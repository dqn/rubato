// BGA (Background Animation) system.
//
// Manages BGA image loading, caching, timeline playback, and layer compositing.
// Ported from Java BGAProcessor/BGImageProcessor/MovieProcessor.

pub mod bg_image_processor;
pub mod bga_processor;
#[cfg(feature = "movie")]
pub mod ffmpeg_movie_processor;
pub mod movie_processor;
