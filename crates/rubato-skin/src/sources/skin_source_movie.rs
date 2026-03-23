use crate::reexports::{MainState, TextureRegion};
use crate::sources::skin_source::SkinSource;

#[cfg(feature = "ffmpeg")]
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
#[cfg(feature = "ffmpeg")]
use std::sync::{mpsc, Arc, Mutex};

// ============================================================
// MovieDecoder -- ffmpeg-backed video frame decoder
// ============================================================

#[cfg(feature = "ffmpeg")]
use ffmpeg_next as ffmpeg;

#[cfg(feature = "ffmpeg")]
struct MovieDecoder {
    input_context: ffmpeg::format::context::Input,
    video_stream_index: usize,
    decoder: ffmpeg::codec::decoder::Video,
    scaler: ffmpeg::software::scaling::Context,
    width: u32,
    height: u32,
    time_base: ffmpeg::Rational,
}

// SAFETY: MovieDecoder contains ffmpeg types (scaling::Context, codec::decoder::Video,
// format::context::Input) that are !Send due to internal raw pointers to C FFmpeg structs
// (SwsContext, AVCodecContext, AVFormatContext). However, MovieDecoder is only ever
// accessed from its owning background thread after creation, so concurrent
// access is impossible. We assert Send so it can be moved into the thread.
#[cfg(feature = "ffmpeg")]
unsafe impl Send for MovieDecoder {}

#[cfg(feature = "ffmpeg")]
impl MovieDecoder {
    fn new(path: &str) -> Option<Self> {
        if let Err(e) = ffmpeg::init() {
            log::warn!("Failed to initialize ffmpeg: {}", e);
            return None;
        }

        let input_context = match ffmpeg::format::input(&path) {
            Ok(ctx) => ctx,
            Err(e) => {
                log::warn!("Failed to open video file '{}': {}", path, e);
                return None;
            }
        };

        let video_stream = match input_context.streams().best(ffmpeg::media::Type::Video) {
            Some(stream) => stream,
            None => {
                log::warn!("No video stream found in '{}'", path);
                return None;
            }
        };

        let video_stream_index = video_stream.index();
        let time_base = video_stream.time_base();

        let context_decoder =
            match ffmpeg::codec::context::Context::from_parameters(video_stream.parameters()) {
                Ok(ctx) => ctx,
                Err(e) => {
                    log::warn!("Failed to create decoder context for '{}': {}", path, e);
                    return None;
                }
            };

        let decoder = match context_decoder.decoder().video() {
            Ok(dec) => dec,
            Err(e) => {
                log::warn!("Failed to open video decoder for '{}': {}", path, e);
                return None;
            }
        };

        let width = decoder.width();
        let height = decoder.height();

        let scaler = match ffmpeg::software::scaling::Context::get(
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
                log::warn!("Failed to create pixel format scaler for '{}': {}", path, e);
                return None;
            }
        };

        Some(Self {
            input_context,
            video_stream_index,
            decoder,
            scaler,
            width,
            height,
            time_base,
        })
    }

    /// Decode a single frame at the given time (microseconds).
    /// Returns RGBA pixel data and (width, height).
    fn decode_frame(&mut self, time_us: i64) -> Option<(Vec<u8>, u32, u32)> {
        // Convert microseconds to stream time_base units for seeking.
        // time_base = num/den, so timestamp = time_us * den / (num * 1_000_000)
        let tb_num = self.time_base.numerator() as i64;
        let tb_den = self.time_base.denominator() as i64;
        if tb_num == 0 || tb_den == 0 {
            return None;
        }
        let seek_ts = time_us * tb_den / (tb_num * 1_000_000);

        // Seek to the requested position (seek backward to nearest keyframe)
        if self.input_context.seek(seek_ts, ..seek_ts).is_err() {
            // If seek fails, try from the beginning
            let _ = self.input_context.seek(0, ..0);
        }
        self.decoder.flush();

        // Read packets until we decode a frame
        let mut decoded_frame = ffmpeg::frame::Video::empty();
        for (stream, packet) in self.input_context.packets() {
            if stream.index() != self.video_stream_index {
                continue;
            }
            if self.decoder.send_packet(&packet).is_err() {
                continue;
            }
            while self.decoder.receive_frame(&mut decoded_frame).is_ok() {
                // Convert to RGBA
                let mut rgba_frame = ffmpeg::frame::Video::empty();
                if self.scaler.run(&decoded_frame, &mut rgba_frame).is_ok() {
                    let data = rgba_frame.data(0);
                    let stride = rgba_frame.stride(0);
                    let w = self.width as usize;
                    let h = self.height as usize;

                    // Copy row-by-row in case stride != width * 4
                    let mut pixels = Vec::with_capacity(w * h * 4);
                    for row in 0..h {
                        let start = row * stride;
                        let end = start + w * 4;
                        if end <= data.len() {
                            pixels.extend_from_slice(&data[start..end]);
                        }
                    }

                    return Some((pixels, self.width, self.height));
                }
            }
        }

        None
    }
}

// ============================================================
// Background thread decode loop
// ============================================================

#[cfg(feature = "ffmpeg")]
enum MovieCommand {
    Halt,
}

#[cfg(feature = "ffmpeg")]
struct DecodedFrame {
    rgba_data: Arc<Vec<u8>>,
    width: u32,
    height: u32,
}

/// Background decode thread main loop.
/// Watches `requested_time` for changes. When the time changes, decodes the
/// frame at that time and stores it in `shared_frame`.
#[cfg(feature = "ffmpeg")]
fn movie_decode_main(
    mut decoder: MovieDecoder,
    requested_time: Arc<AtomicI64>,
    shared_frame: Arc<Mutex<Option<DecodedFrame>>>,
    cmd_rx: mpsc::Receiver<MovieCommand>,
) {
    let mut last_decoded_time: i64 = i64::MIN;

    loop {
        // Check for halt command (non-blocking)
        match cmd_rx.try_recv() {
            Ok(MovieCommand::Halt) | Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        let time = requested_time.load(Ordering::Acquire);

        // Only decode if time has changed
        if time != last_decoded_time && time >= 0 {
            if let Some((pixels, width, height)) = decoder.decode_frame(time) {
                let frame = DecodedFrame {
                    rgba_data: Arc::new(pixels),
                    width,
                    height,
                };
                if let Ok(mut guard) = shared_frame.lock() {
                    *guard = Some(frame);
                }
                last_decoded_time = time;
            }
        }

        // Sleep briefly to avoid spinning
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
}

// ============================================================
// SkinSourceMovie
// ============================================================

/// Skin source movie (SkinSourceMovie.java)
/// Decodes video frames in a background thread to avoid blocking the render thread.
pub struct SkinSourceMovie {
    path: String,
    _timer: i32,
    disposed: bool,
    #[cfg(feature = "ffmpeg")]
    requested_time: Arc<AtomicI64>,
    #[cfg(feature = "ffmpeg")]
    shared_frame: Arc<Mutex<Option<DecodedFrame>>>,
    #[cfg(feature = "ffmpeg")]
    cmd_tx: Option<mpsc::Sender<MovieCommand>>,
    #[cfg(feature = "ffmpeg")]
    thread_handle: Option<std::thread::JoinHandle<()>>,
    #[cfg(feature = "ffmpeg")]
    has_decoder: bool,
}

impl SkinSourceMovie {
    pub fn new(path: &str) -> Self {
        Self::new_with_timer(path, 0)
    }

    pub fn new_with_timer(path: &str, timer: i32) -> Self {
        #[cfg(feature = "ffmpeg")]
        {
            let requested_time = Arc::new(AtomicI64::new(i64::MIN));
            let shared_frame: Arc<Mutex<Option<DecodedFrame>>> = Arc::new(Mutex::new(None));

            if let Some(decoder) = MovieDecoder::new(path) {
                let (cmd_tx, cmd_rx) = mpsc::channel();
                let time_clone = Arc::clone(&requested_time);
                let frame_clone = Arc::clone(&shared_frame);
                let path_owned = path.to_string();

                let thread_handle = std::thread::Builder::new()
                    .name(format!("movie-decode:{}", path))
                    .spawn(move || {
                        movie_decode_main(decoder, time_clone, frame_clone, cmd_rx);
                        log::debug!("Movie decode thread exited: {}", path_owned);
                    })
                    .ok();

                Self {
                    path: path.to_string(),
                    _timer: timer,
                    disposed: false,
                    requested_time,
                    shared_frame,
                    cmd_tx: Some(cmd_tx),
                    thread_handle,
                    has_decoder: true,
                }
            } else {
                Self {
                    path: path.to_string(),
                    _timer: timer,
                    disposed: false,
                    requested_time,
                    shared_frame,
                    cmd_tx: None,
                    thread_handle: None,
                    has_decoder: false,
                }
            }
        }

        #[cfg(not(feature = "ffmpeg"))]
        Self {
            path: path.to_string(),
            _timer: timer,
            disposed: false,
        }
    }
}

// SAFETY: SkinSourceMovie fields are all Send+Sync:
// - Arc<AtomicI64>, Arc<Mutex<...>>, mpsc::Sender are Send+Sync
// - Option<JoinHandle<()>> is Send
// - The thread_handle is only joined in dispose() which takes &mut self
unsafe impl Sync for SkinSourceMovie {}

impl SkinSource for SkinSourceMovie {
    fn get_image(&self, time: i64, _state: &dyn MainState) -> Option<TextureRegion> {
        #[cfg(feature = "ffmpeg")]
        {
            if !self.has_decoder {
                return None;
            }

            use crate::reexports::Texture;

            // Update requested time for background thread
            self.requested_time.store(time, Ordering::Release);

            // Pick up latest decoded frame
            let guard = rubato_types::sync_utils::lock_or_recover(&self.shared_frame);
            let frame = guard.as_ref()?;

            let texture = Texture {
                width: frame.width as i32,
                height: frame.height as i32,
                disposed: false,
                rgba_data: Some(Arc::clone(&frame.rgba_data)),
                ..Default::default()
            };
            Some(TextureRegion::from_texture(texture))
        }

        #[cfg(not(feature = "ffmpeg"))]
        {
            let _ = (time, _state);
            None
        }
    }

    fn validate(&self) -> bool {
        true
    }

    fn dispose(&mut self) {
        if !self.disposed {
            #[cfg(feature = "ffmpeg")]
            {
                // Send halt command and drop the sender
                if let Some(tx) = self.cmd_tx.take() {
                    let _ = tx.send(MovieCommand::Halt);
                }
                // Drop the thread handle to detach (don't join -- decode can be slow)
                self.thread_handle.take();
            }
            self.disposed = true;
            log::debug!("Disposed movie source: {}", self.path);
        }
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}
