//! FFmpeg-based movie processor with background video decoding thread.
//!
//! Translated from: FFmpegProcessor.java (inner class MovieSeekThread)
//! In Rust, std::thread + mpsc channels replace Java Thread + LinkedBlockingDeque.

#[cfg(feature = "ffmpeg")]
use std::sync::Arc;

use crate::play::Texture;
use crate::play::bga::movie_processor::MovieProcessor;

/// Timer observer for movie playback
pub trait TimerObserver {
    fn micro_time(&self) -> i64;
}

/// Commands sent to the background movie seek thread.
/// Translated from: FFmpegProcessor.Command enum in Java
#[cfg(feature = "ffmpeg")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Play,
    Loop,
    Stop,
    Halt,
}

/// Status of the FFmpeg processor texture.
/// Translated from: FFmpegProcessor texture state in Java
#[cfg(feature = "ffmpeg")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessorStatus {
    TextureInactive,
    TextureActive,
    Disposed,
}

// ============================================================
// FFmpeg feature: background video decoding thread
// ============================================================

#[cfg(feature = "ffmpeg")]
mod ffmpeg_impl {
    use super::{Command, ProcessorStatus};
    use ffmpeg_next as ffmpeg;
    use rubato_types::sync_utils::lock_or_recover;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    /// Decoded video frame data.
    pub(super) struct DecodedFrame {
        pub pixels: Vec<u8>,
        pub width: u32,
        pub height: u32,
    }

    /// Mutable video playback state shared across handle_command / restart.
    struct VideoPlaybackState {
        eof: bool,
        loop_play: bool,
        offset: i64,
        framecount: i64,
        current_ts_us: i64,
    }

    /// Immutable stream metadata used by grab_frame / convert_frame.
    struct VideoStreamInfo {
        stream_index: usize,
        width: u32,
        height: u32,
        time_base_num: i64,
        time_base_den: i64,
    }

    /// Shared state between FFmpegProcessor and MovieSeekThread.
    pub(super) struct SharedState {
        pub status: ProcessorStatus,
        pub frame: Option<DecodedFrame>,
    }

    /// Handle to the background movie seek thread.
    pub(super) struct MovieSeekHandle {
        pub cmd_tx: mpsc::Sender<Command>,
        pub shared: Arc<Mutex<SharedState>>,
        pub time: Arc<AtomicI64>,
        pub thread: Option<JoinHandle<()>>,
    }

    /// Start the background movie seek thread.
    /// Translated from: FFmpegProcessor.create() + MovieSeekThread constructor
    pub(super) fn start_movie_seek(filepath: &str, fpsd: i32) -> Option<MovieSeekHandle> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let time = Arc::new(AtomicI64::new(0));
        let shared = Arc::new(Mutex::new(SharedState {
            status: ProcessorStatus::TextureInactive,
            frame: None,
        }));

        let filepath_owned = filepath.to_string();
        let time_clone = Arc::clone(&time);
        let shared_clone = Arc::clone(&shared);
        let fpsd = fpsd.max(1);

        let thread = thread::Builder::new()
            .name("movie-seek".into())
            .spawn(move || {
                movie_seek_main(filepath_owned, fpsd, time_clone, shared_clone, cmd_rx);
            })
            .ok()?;

        Some(MovieSeekHandle {
            cmd_tx,
            shared,
            time,
            thread: Some(thread),
        })
    }

    /// Background thread main loop.
    /// Translated from: MovieSeekThread.run()
    fn movie_seek_main(
        filepath: String,
        fpsd: i32,
        time: Arc<AtomicI64>,
        shared: Arc<Mutex<SharedState>>,
        cmd_rx: mpsc::Receiver<Command>,
    ) {
        // Initialize ffmpeg
        if let Err(e) = ffmpeg::init() {
            log::warn!("Failed to initialize ffmpeg: {}", e);
            return;
        }

        // Open video file
        let mut input_context = match ffmpeg::format::input(&filepath) {
            Ok(ctx) => ctx,
            Err(e) => {
                log::warn!("Failed to open video file '{}': {}", filepath, e);
                return;
            }
        };

        let video_stream = match input_context.streams().best(ffmpeg::media::Type::Video) {
            Some(s) => s,
            None => {
                log::warn!("No video stream found in '{}'", filepath);
                return;
            }
        };

        let video_stream_index = video_stream.index();
        let time_base_num = video_stream.time_base().numerator() as i64;
        let time_base_den = video_stream.time_base().denominator() as i64;

        let ctx = match ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create decoder context for '{}': {}", filepath, e);
                return;
            }
        };

        let mut decoder = match ctx.decoder().video() {
            Ok(d) => d,
            Err(e) => {
                log::warn!("Failed to open video decoder for '{}': {}", filepath, e);
                return;
            }
        };

        let width = decoder.width();
        let height = decoder.height();

        let mut scaler = match ffmpeg::software::scaling::Context::get(
            decoder.format(),
            width,
            height,
            ffmpeg::format::Pixel::RGBA,
            width,
            height,
            ffmpeg::software::scaling::Flags::BILINEAR,
        ) {
            Ok(s) => s,
            Err(e) => {
                log::warn!(
                    "Failed to create pixel format scaler for '{}': {}",
                    filepath,
                    e
                );
                return;
            }
        };

        log::info!(
            "movie decode - size: {}x{} file: {}",
            width,
            height,
            filepath
        );

        let mut state = VideoPlaybackState {
            eof: true,
            loop_play: false,
            offset: 0,
            framecount: 0,
            current_ts_us: 0,
        };
        let stream_info = VideoStreamInfo {
            stream_index: video_stream_index,
            width,
            height,
            time_base_num,
            time_base_den,
        };
        let fpsd_i64 = fpsd as i64;

        // Main loop — translated from MovieSeekThread.run()
        loop {
            if state.eof {
                // Set status to inactive
                {
                    let mut s = lock_or_recover(&shared);
                    if s.status != ProcessorStatus::Disposed {
                        s.status = ProcessorStatus::TextureInactive;
                    }
                }
                // Wait for command (blocking, like Java Thread.sleep(3600000) + interrupt)
                match cmd_rx.recv() {
                    Ok(cmd) => {
                        if !handle_command(cmd, &mut state, &time, &mut input_context, &mut decoder)
                        {
                            break; // Halt
                        }
                        continue;
                    }
                    Err(_) => break, // Channel closed
                }
            }

            let current_time = time.load(Ordering::Acquire);
            let microtime = current_time * 1000 + state.offset;

            if microtime >= state.current_ts_us {
                // Catch up: grab frames until video position >= playback time
                // Translated from: while (microtime >= grabber.getTimestamp() || framecount % fpsd != 0)
                let mut latest_pixels: Option<Vec<u8>> = None;
                loop {
                    // Break condition: caught up AND at display interval
                    if microtime < state.current_ts_us && state.framecount % fpsd_i64 == 0 {
                        break;
                    }
                    match grab_frame(&mut input_context, &mut decoder, &mut scaler, &stream_info) {
                        Some((pixels, ts_us)) => {
                            state.current_ts_us = ts_us;
                            state.framecount += 1;
                            latest_pixels = Some(pixels);
                        }
                        None => {
                            // End of file
                            state.eof = true;
                            if state.loop_play {
                                // Auto-restart (like Java: commands.offerLast(Command.LOOP))
                                restart(&mut state, &time, &mut input_context, &mut decoder);
                            }
                            break;
                        }
                    }
                }

                // Update shared state with latest decoded frame
                // Translated from: Gdx.app.postRunnable() in MovieSeekThread
                if let Some(pixels) = latest_pixels {
                    let mut s = lock_or_recover(&shared);
                    if s.status != ProcessorStatus::Disposed {
                        s.frame = Some(DecodedFrame {
                            pixels,
                            width: stream_info.width,
                            height: stream_info.height,
                        });
                        s.status = ProcessorStatus::TextureActive;
                    }
                }
            } else {
                // Video is ahead of playback — sleep with command check
                // Translated from: sleep((grabber.getTimestamp() - microtime) / 1000 - 1)
                let sleep_us = (state.current_ts_us - microtime).max(1000);
                let sleep_ms = ((sleep_us / 1000) - 1).max(1) as u64;
                match cmd_rx.recv_timeout(Duration::from_millis(sleep_ms.min(100))) {
                    Ok(cmd) => {
                        if !handle_command(cmd, &mut state, &time, &mut input_context, &mut decoder)
                        {
                            break;
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            // Non-blocking command check (like Java: if (!commands.isEmpty()) { ... })
            while let Ok(cmd) = cmd_rx.try_recv() {
                if !handle_command(cmd, &mut state, &time, &mut input_context, &mut decoder) {
                    log::info!("Video resource released: {}", filepath);
                    return;
                }
            }
        }

        log::info!("Video resource released: {}", filepath);
    }

    /// Process a command. Returns false if Halt (thread should exit).
    /// Translated from: the switch(commands.pollFirst()) block in MovieSeekThread.run()
    fn handle_command(
        cmd: Command,
        state: &mut VideoPlaybackState,
        time: &Arc<AtomicI64>,
        input_context: &mut ffmpeg::format::context::Input,
        decoder: &mut ffmpeg::codec::decoder::Video,
    ) -> bool {
        match cmd {
            Command::Play => {
                state.loop_play = false;
                restart(state, time, input_context, decoder);
                true
            }
            Command::Loop => {
                state.loop_play = true;
                restart(state, time, input_context, decoder);
                true
            }
            Command::Stop => {
                state.eof = true;
                true
            }
            Command::Halt => false,
        }
    }

    /// Restart video from beginning.
    /// Translated from: MovieSeekThread.restart()
    fn restart(
        state: &mut VideoPlaybackState,
        time: &Arc<AtomicI64>,
        input_context: &mut ffmpeg::format::context::Input,
        decoder: &mut ffmpeg::codec::decoder::Video,
    ) {
        let _ = input_context.seek(0, ..0);
        decoder.flush();
        state.eof = false;
        let current_time = time.load(Ordering::Acquire);
        state.offset = -current_time * 1000;
        state.framecount = 1;
        state.current_ts_us = 0;
    }

    /// Grab one decoded video frame from the stream.
    /// Translated from: grabber.grabImage() in MovieSeekThread
    fn grab_frame(
        input_context: &mut ffmpeg::format::context::Input,
        decoder: &mut ffmpeg::codec::decoder::Video,
        scaler: &mut ffmpeg::software::scaling::Context,
        info: &VideoStreamInfo,
    ) -> Option<(Vec<u8>, i64)> {
        let mut decoded_frame = ffmpeg::frame::Video::empty();

        // Try to receive buffered frames first
        if decoder.receive_frame(&mut decoded_frame).is_ok() {
            return convert_frame(&decoded_frame, scaler, info);
        }

        // Read packets until we decode a video frame
        for (stream, packet) in input_context.packets() {
            if stream.index() != info.stream_index {
                continue;
            }
            let _ = decoder.send_packet(&packet);
            if decoder.receive_frame(&mut decoded_frame).is_ok() {
                return convert_frame(&decoded_frame, scaler, info);
            }
        }

        None
    }

    /// Convert a decoded video frame to RGBA pixels.
    fn convert_frame(
        frame: &ffmpeg::frame::Video,
        scaler: &mut ffmpeg::software::scaling::Context,
        info: &VideoStreamInfo,
    ) -> Option<(Vec<u8>, i64)> {
        let mut rgba_frame = ffmpeg::frame::Video::empty();
        if scaler.run(frame, &mut rgba_frame).is_err() {
            return None;
        }

        let data = rgba_frame.data(0);
        let stride = rgba_frame.stride(0);
        let w = info.width as usize;
        let h = info.height as usize;

        // Copy row-by-row in case stride != width * 4
        let mut pixels = Vec::with_capacity(w * h * 4);
        for row in 0..h {
            let start = row * stride;
            let end = start + w * 4;
            if end <= data.len() {
                pixels.extend_from_slice(&data[start..end]);
            }
        }

        // Convert frame timestamp to microseconds
        let ts = frame.timestamp().unwrap_or(0);
        let ts_us = if info.time_base_num > 0 && info.time_base_den > 0 {
            ts * 1_000_000 * info.time_base_num / info.time_base_den
        } else {
            0
        };

        Some((pixels, ts_us))
    }
}

// ============================================================
// FFmpegProcessor (public API)
// ============================================================

/// FFmpeg-based movie processor with background decoding thread.
/// Translated from: FFmpegProcessor.java
pub struct FFmpegProcessor {
    /// Frame display rate (1/n)
    _fpsd: i32,
    /// Background thread handle (only when ffmpeg feature is enabled)
    #[cfg(feature = "ffmpeg")]
    handle: Option<ffmpeg_impl::MovieSeekHandle>,
    /// Cached texture from last decoded frame
    _showing_tex: Option<Texture>,
}

impl FFmpegProcessor {
    pub fn new(fpsd: i32) -> Self {
        FFmpegProcessor {
            _fpsd: fpsd,
            #[cfg(feature = "ffmpeg")]
            handle: None,
            _showing_tex: None,
        }
    }

    /// Open a video file and start the background decoding thread.
    /// Translated from: FFmpegProcessor.create()
    pub fn create(&mut self, filepath: &str) {
        #[cfg(feature = "ffmpeg")]
        {
            self.handle = ffmpeg_impl::start_movie_seek(filepath, self._fpsd);
        }
        #[cfg(not(feature = "ffmpeg"))]
        {
            let _ = filepath;
            log::warn!("FFmpeg video decoding not available (ffmpeg feature not enabled)");
        }
    }
}

impl Default for FFmpegProcessor {
    fn default() -> Self {
        Self::new(1)
    }
}

impl MovieProcessor for FFmpegProcessor {
    fn frame(&mut self, time: i64) -> Option<Texture> {
        #[cfg(feature = "ffmpeg")]
        {
            if let Some(ref handle) = self.handle {
                // Update time for background thread
                handle
                    .time
                    .store(time, std::sync::atomic::Ordering::Release);
                // Check for new decoded frame.
                // Take the frame out of the shared state to release the lock
                // before building the Texture. The background thread will
                // repopulate the frame on the next decode cycle.
                let taken_frame = {
                    let mut s = rubato_types::sync_utils::lock_or_recover(&handle.shared);
                    if s.status == ProcessorStatus::TextureActive {
                        s.frame.take()
                    } else {
                        None
                    }
                };
                if let Some(frame) = taken_frame {
                    self._showing_tex = Some(Texture {
                        width: frame.width as i32,
                        height: frame.height as i32,
                        disposed: false,
                        rgba_data: Some(Arc::new(frame.pixels)),
                        ..Default::default()
                    });
                }
            }
            self._showing_tex.clone()
        }
        #[cfg(not(feature = "ffmpeg"))]
        {
            let _ = time;
            None
        }
    }

    fn play(&mut self, time: i64, loop_play: bool) {
        #[cfg(feature = "ffmpeg")]
        {
            if let Some(ref handle) = self.handle {
                // Set time before sending command (matches Java: this.time = time; movieseek.exec())
                handle
                    .time
                    .store(time, std::sync::atomic::Ordering::Release);
                let cmd = if loop_play {
                    Command::Loop
                } else {
                    Command::Play
                };
                let _ = handle.cmd_tx.send(cmd);
            }
        }
        #[cfg(not(feature = "ffmpeg"))]
        {
            let _ = (time, loop_play);
        }
    }

    fn stop(&mut self) {
        #[cfg(feature = "ffmpeg")]
        {
            if let Some(ref handle) = self.handle {
                let _ = handle.cmd_tx.send(Command::Stop);
            }
        }
    }

    fn dispose(&mut self) {
        #[cfg(feature = "ffmpeg")]
        {
            // Set disposed status first
            if let Some(ref handle) = self.handle {
                {
                    let mut s = rubato_types::sync_utils::lock_or_recover(&handle.shared);
                    s.status = ProcessorStatus::Disposed;
                }
                let _ = handle.cmd_tx.send(Command::Halt);
            }
            // Join thread for clean shutdown
            if let Some(mut handle) = self.handle.take() {
                if let Some(thread) = handle.thread.take() {
                    let _ = thread.join();
                }
            }
            self._showing_tex = None;
        }
    }
}

impl Drop for FFmpegProcessor {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispose_without_create() {
        // Calling dispose() without calling create() should not panic
        let mut proc = FFmpegProcessor::new(30);
        proc.dispose();
    }

    #[test]
    fn test_double_dispose() {
        // Calling dispose() twice should not panic
        let mut proc = FFmpegProcessor::new(30);
        // First dispose (no background thread exists)
        proc.dispose();
        // Second dispose
        proc.dispose();
    }

    #[test]
    fn test_frame_before_create() {
        // Calling frame() before create() should return None
        let mut proc = FFmpegProcessor::new(30);
        let result = proc.frame(0);
        assert!(
            result.is_none(),
            "frame() before create() should return None"
        );

        let result2 = proc.frame(1000);
        assert!(
            result2.is_none(),
            "frame() before create() should return None for any time"
        );
    }

    #[test]
    fn test_stop_before_create() {
        // Calling stop() before create() should not panic
        let mut proc = FFmpegProcessor::new(30);
        proc.stop();
    }

    #[test]
    fn test_play_before_create() {
        // Calling play() before create() should not panic
        let mut proc = FFmpegProcessor::new(30);
        proc.play(0, false);
        proc.play(0, true);
    }

    #[test]
    fn test_drop_without_dispose() {
        // Dropping without explicit dispose() should not leak the background thread.
        // Drop impl calls dispose() automatically.
        let proc = FFmpegProcessor::new(30);
        drop(proc);
    }

    #[test]
    fn test_drop_after_dispose_is_safe() {
        // Calling dispose() then dropping should not panic (double dispose is idempotent).
        let mut proc = FFmpegProcessor::new(30);
        proc.dispose();
        drop(proc);
    }
}
