use crate::skin_source::SkinSource;
use crate::stubs::{MainState, TextureRegion};

#[cfg(feature = "ffmpeg")]
use std::sync::Mutex;

// ============================================================
// MovieDecoder — ffmpeg-backed video frame decoder
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
// accessed through a Mutex<Option<MovieDecoder>> inside SkinSourceMovie, so concurrent
// access is impossible. We assert Send so Mutex<MovieDecoder> can be Sync.
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
// SkinSourceMovie
// ============================================================

/// Skin source movie (SkinSourceMovie.java)
pub struct SkinSourceMovie {
    path: String,
    _timer: i32,
    _playing: bool,
    disposed: bool,
    region: TextureRegion,
    #[cfg(feature = "ffmpeg")]
    decoder: Mutex<Option<MovieDecoder>>,
}

impl SkinSourceMovie {
    pub fn new(path: &str) -> Self {
        Self::new_with_timer(path, 0)
    }

    pub fn new_with_timer(path: &str, timer: i32) -> Self {
        #[cfg(feature = "ffmpeg")]
        let decoder = Mutex::new(MovieDecoder::new(path));

        Self {
            path: path.to_string(),
            _timer: timer,
            _playing: false,
            disposed: false,
            region: TextureRegion::new(),
            #[cfg(feature = "ffmpeg")]
            decoder,
        }
    }
}

impl SkinSource for SkinSourceMovie {
    fn get_image(&self, time: i64, _state: &dyn MainState) -> Option<TextureRegion> {
        #[cfg(feature = "ffmpeg")]
        {
            use crate::stubs::Texture;

            let mut guard = self.decoder.lock().ok()?;
            let decoder = guard.as_mut()?;
            let (rgba_data, width, height) = decoder.decode_frame(time)?;

            // Build a Texture from the decoded dimensions.
            // Actual GPU upload happens later when a GpuContext is available.
            let _ = &rgba_data; // RGBA data available for future GPU upload
            let texture = Texture {
                width: width as i32,
                height: height as i32,
                disposed: false,
                rgba_data: Some(std::sync::Arc::new(rgba_data)),
                ..Default::default()
            };
            Some(TextureRegion::from_texture(texture))
        }

        #[cfg(not(feature = "ffmpeg"))]
        {
            let _ = (time, _state);
            // FFmpeg video decoding requires feature = "ffmpeg"
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
                // Drop the decoder to release ffmpeg resources
                if let Ok(mut guard) = self.decoder.lock() {
                    *guard = None;
                }
            }
            self.disposed = true;
            log::debug!("Disposed movie source: {}", self.path);
        }
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}
