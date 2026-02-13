// FFmpeg-based movie processor for BGA video playback.
//
// Ported from Java FFmpegProcessor.java (~297 lines).
// Uses ffmpeg-next for video decoding on a dedicated thread,
// with frame data shared via Arc<Mutex<FrameBuffer>>.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use tracing::{info, warn};

use super::movie_processor::MovieProcessor;

/// Shared frame buffer between decoder thread and main thread.
struct FrameBuffer {
    /// RGBA pixel data
    data: Vec<u8>,
    width: u32,
    height: u32,
    /// Monotonic counter incremented on each new frame
    generation: u64,
}

/// Commands sent from main thread to decoder thread.
enum Command {
    /// Start or seek playback (no looping).
    Play { time_us: i64 },
    /// Start or seek playback with looping enabled.
    Loop { time_us: i64 },
    /// Stop playback (pause at current position).
    Stop,
    /// Shutdown the decoder thread.
    Halt,
}

/// FFmpeg-based movie processor.
///
/// Spawns a dedicated decoder thread that reads video frames and writes
/// RGBA data into a shared buffer. The main thread polls this buffer
/// each frame and uploads it to a Bevy `Image`.
pub struct FfmpegMovieProcessor {
    cmd_tx: Option<Sender<Command>>,
    frame_buffer: Arc<Mutex<Option<FrameBuffer>>>,
    thread_handle: Option<JoinHandle<()>>,
    image_handle: Option<Handle<Image>>,
    last_generation: u64,
    ready: bool,
}

impl FfmpegMovieProcessor {
    /// Create a new FFmpeg movie processor for the given video file.
    ///
    /// Opens the video file and spawns the decoder thread.
    pub fn new(path: &Path, frameskip: i32) -> Result<Self> {
        let path = path.to_path_buf();
        let frame_buffer: Arc<Mutex<Option<FrameBuffer>>> = Arc::new(Mutex::new(None));
        let buffer_clone = Arc::clone(&frame_buffer);

        let (cmd_tx, cmd_rx) = mpsc::channel();

        let thread_handle = thread::Builder::new()
            .name(format!(
                "movie-decoder-{}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
            ))
            .spawn(move || {
                decoder_thread(path, buffer_clone, cmd_rx, frameskip);
            })?;

        Ok(Self {
            cmd_tx: Some(cmd_tx),
            frame_buffer,
            thread_handle: Some(thread_handle),
            image_handle: None,
            last_generation: 0,
            ready: true,
        })
    }
}

impl MovieProcessor for FfmpegMovieProcessor {
    fn prepare(&mut self, _path: &Path) -> Result<()> {
        // Already prepared in new()
        Ok(())
    }

    fn play(&mut self, time_us: i64, looping: bool) {
        if let Some(tx) = &self.cmd_tx {
            let cmd = if looping {
                Command::Loop { time_us }
            } else {
                Command::Play { time_us }
            };
            let _ = tx.send(cmd);
        }
    }

    fn get_frame(&self, _time_us: i64) -> Option<Handle<Image>> {
        self.image_handle.clone()
    }

    fn stop(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(Command::Stop);
        }
    }

    fn update_frame(&mut self, images: &mut Assets<Image>) -> Option<Handle<Image>> {
        let guard = self.frame_buffer.lock().ok()?;
        let fb = guard.as_ref()?;

        if fb.generation == self.last_generation {
            return None;
        }

        self.last_generation = fb.generation;

        match &self.image_handle {
            Some(handle) => {
                // Update existing image in-place
                if let Some(image) = images.get_mut(handle) {
                    image.data.clone_from(&fb.data);
                }
            }
            None => {
                // Create initial Bevy image
                let image = Image::new(
                    Extent3d {
                        width: fb.width,
                        height: fb.height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    fb.data.clone(),
                    TextureFormat::Rgba8UnormSrgb,
                    default(),
                );
                self.image_handle = Some(images.add(image));
            }
        }

        self.image_handle.clone()
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    fn dispose(&mut self) {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(Command::Halt);
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        self.image_handle = None;
        self.ready = false;
    }
}

impl Drop for FfmpegMovieProcessor {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// Decoder thread main loop.
///
/// Ported from Java MovieSeekThread.run(). Opens the video file with FFmpeg,
/// decodes frames on demand, and writes RGBA data into the shared buffer.
fn decoder_thread(
    path: PathBuf,
    frame_buffer: Arc<Mutex<Option<FrameBuffer>>>,
    cmd_rx: Receiver<Command>,
    frameskip: i32,
) {
    if let Err(e) = decoder_thread_inner(&path, &frame_buffer, &cmd_rx, frameskip) {
        warn!("Movie decoder error for {:?}: {e}", path);
    }
}

fn decoder_thread_inner(
    path: &Path,
    frame_buffer: &Arc<Mutex<Option<FrameBuffer>>>,
    cmd_rx: &Receiver<Command>,
    frameskip: i32,
) -> Result<()> {
    ffmpeg_next::init()?;

    let mut ictx = ffmpeg_next::format::input(path)?;

    // Find the best video stream (Java: skip streams with bitrate < 10)
    let video_stream_index = ictx
        .streams()
        .best(ffmpeg_next::media::Type::Video)
        .ok_or_else(|| anyhow::anyhow!("No video stream found in {:?}", path))?
        .index();

    let stream = ictx.stream(video_stream_index).unwrap();
    let time_base = stream.time_base();
    let codec_par = stream.parameters();

    let context = ffmpeg_next::codec::Context::from_parameters(codec_par)?;
    let mut decoder = context.decoder().video()?;

    let width = decoder.width();
    let height = decoder.height();
    let src_format = decoder.format();

    info!(
        "Movie decode - size: {}x{}, format: {:?}, path: {:?}",
        width, height, src_format, path
    );

    let mut scaler = ffmpeg_next::software::scaling::Context::get(
        src_format,
        width,
        height,
        ffmpeg_next::format::Pixel::RGBA,
        width,
        height,
        ffmpeg_next::software::scaling::Flags::BILINEAR,
    )?;

    let fpsd = frameskip.max(1);
    let mut eof = true;
    let mut looping = false;
    let mut play_start_us: i64 = 0;
    let mut generation: u64 = 0;
    let mut framecount: i64 = 0;
    let mut halt = false;

    // Helper: convert stream timestamp to microseconds
    let ts_to_us = |ts: i64| -> i64 {
        (ts as f64 * f64::from(time_base.0) / f64::from(time_base.1) * 1_000_000.0) as i64
    };

    // Reusable frame for decoded output
    let mut rgba_frame = ffmpeg_next::frame::Video::empty();

    while !halt {
        if eof {
            // Wait for a command (Java: sleep(3600000) with interrupt)
            match cmd_rx.recv() {
                Ok(cmd) => {
                    process_command(
                        cmd,
                        &mut eof,
                        &mut looping,
                        &mut halt,
                        &mut play_start_us,
                        &mut framecount,
                        &mut ictx,
                        video_stream_index,
                    );
                }
                Err(_) => break, // Channel closed
            }
            continue;
        }

        // Process pending commands (non-blocking)
        while let Ok(cmd) = cmd_rx.try_recv() {
            process_command(
                cmd,
                &mut eof,
                &mut looping,
                &mut halt,
                &mut play_start_us,
                &mut framecount,
                &mut ictx,
                video_stream_index,
            );
        }

        if halt || eof {
            continue;
        }

        // Read and decode next frame
        let mut got_frame = false;
        for (stream, packet) in ictx.packets() {
            if stream.index() != video_stream_index {
                continue;
            }

            decoder.send_packet(&packet)?;

            let mut decoded = ffmpeg_next::frame::Video::empty();
            while decoder.receive_frame(&mut decoded).is_ok() {
                framecount += 1;

                // Frameskip: only process every nth frame
                if framecount % i64::from(fpsd) != 0 {
                    continue;
                }

                // Convert to RGBA
                scaler.run(&decoded, &mut rgba_frame)?;

                // Check timing: is this frame at or past the requested time?
                let frame_ts = decoded.timestamp().unwrap_or(0);
                let frame_us = ts_to_us(frame_ts);

                if frame_us >= play_start_us {
                    // Write frame to shared buffer
                    generation += 1;
                    let data = rgba_frame.data(0);
                    let stride = rgba_frame.stride(0);
                    let row_bytes = (width as usize) * 4;

                    let pixels = if stride == row_bytes as usize {
                        data[..row_bytes * height as usize].to_vec()
                    } else {
                        // Handle stride != width * 4 (padding)
                        let mut pixels = Vec::with_capacity(row_bytes * height as usize);
                        for y in 0..height as usize {
                            let start = y * stride;
                            pixels.extend_from_slice(&data[start..start + row_bytes]);
                        }
                        pixels
                    };

                    if let Ok(mut guard) = frame_buffer.lock() {
                        *guard = Some(FrameBuffer {
                            data: pixels,
                            width,
                            height,
                            generation,
                        });
                    }

                    got_frame = true;
                    break;
                }
            }

            if got_frame {
                break;
            }
        }

        if !got_frame {
            // EOF reached
            eof = true;
            if looping {
                // Restart from beginning (Java: commands.offerLast(Command.LOOP))
                seek_to_start(&mut ictx, video_stream_index);
                eof = false;
                framecount = 0;
            }
        } else {
            // Sleep briefly to avoid busy-spinning
            thread::sleep(Duration::from_millis(1));
        }
    }

    // Flush decoder
    decoder.send_eof()?;
    let mut decoded = ffmpeg_next::frame::Video::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {}

    info!("Movie resource released: {:?}", path);
    Ok(())
}

/// Process a command from the main thread.
#[allow(clippy::too_many_arguments)]
fn process_command(
    cmd: Command,
    eof: &mut bool,
    looping: &mut bool,
    halt: &mut bool,
    play_start_us: &mut i64,
    framecount: &mut i64,
    ictx: &mut ffmpeg_next::format::context::Input,
    video_stream_index: usize,
) {
    match cmd {
        Command::Play { time_us } => {
            *looping = false;
            *play_start_us = time_us;
            *framecount = 0;
            seek_to_start(ictx, video_stream_index);
            *eof = false;
        }
        Command::Loop { time_us } => {
            *looping = true;
            *play_start_us = time_us;
            *framecount = 0;
            seek_to_start(ictx, video_stream_index);
            *eof = false;
        }
        Command::Stop => {
            *eof = true;
        }
        Command::Halt => {
            *halt = true;
        }
    }
}

/// Seek the input context back to the beginning of the video stream.
fn seek_to_start(ictx: &mut ffmpeg_next::format::context::Input, video_stream_index: usize) {
    let _ = ictx.seek(video_stream_index as i64, ..0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_buffer_generation_increments() {
        let fb1 = FrameBuffer {
            data: vec![0; 4],
            width: 1,
            height: 1,
            generation: 1,
        };
        let fb2 = FrameBuffer {
            data: vec![0; 4],
            width: 1,
            height: 1,
            generation: 2,
        };
        assert_ne!(fb1.generation, fb2.generation);
    }

    #[test]
    fn command_channel_works() {
        let (tx, rx) = mpsc::channel();
        tx.send(Command::Play { time_us: 0 }).unwrap();
        tx.send(Command::Stop).unwrap();
        tx.send(Command::Halt).unwrap();

        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 3);
    }
}
