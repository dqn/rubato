//! Audio engine for PCM decoding, key-sound playback, and BGM rendering via Kira.
//!
//! Provides decoders for WAV ([`wav`]), MP3 ([`mp3`]), OGG Vorbis ([`ogg`]),
//! FLAC ([`flac`]), and MS-ADPCM ([`msadpcm`]) formats through a unified [`decode`]
//! interface. [`key_sound::KeySoundManager`] handles per-note audio triggering,
//! [`renderer::AudioRenderer`] manages BGM playback, and [`kira_driver::KiraDriver`]
//! wraps Kira's `AudioManager` with fault-recovery support.

pub mod decode;
pub mod driver;
pub mod flac;
pub mod key_sound;
pub mod kira_driver;
pub mod loudness;
pub mod mp3;
pub mod msadpcm;
pub mod ogg;
pub mod pcm;
pub mod renderer;
pub mod wav;
