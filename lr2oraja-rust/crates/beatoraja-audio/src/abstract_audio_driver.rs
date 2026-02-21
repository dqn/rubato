use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicI32, Ordering};

use bms_model::note::Note;

use crate::pcm::PCM;

/// Abstract audio driver with caching.
///
/// Translated from: AbstractAudioDriver.java
///
/// In Java, this is a generic abstract class `AbstractAudioDriver<T>`.
/// In Rust, we use a trait `AbstractAudioDriverBackend<T>` for the abstract methods
/// and a struct `AbstractAudioDriverState<T>` for the shared state.
///
/// Backend trait that concrete audio drivers must implement.
pub trait AbstractAudioDriverBackend<T> {
    fn get_key_sound_from_path(&mut self, p: &Path) -> Option<T>;
    fn get_key_sound_from_pcm(&mut self, pcm: &PCM) -> Option<T>;
    fn dispose_key_sound(&mut self, pcm: T);
    fn play_wav(&mut self, wav: &T, channel: i32, volume: f32, pitch: f32);
    fn play_element(&mut self, id: &mut AudioElement<T>, volume: f32, loop_play: bool);
    fn set_volume_element(&mut self, id: &AudioElement<T>, volume: f32);
    fn is_playing_wav(&self, id: &T) -> bool;
    fn stop_wav(&mut self, id: &T);
    fn stop_wav_channel(&mut self, id: &T, channel: i32);
    fn set_volume_wav(&mut self, id: &T, channel: i32, volume: f32);
}

/// Slice wav data.
///
/// Translated from: AbstractAudioDriver.SliceWav
#[derive(Clone)]
pub struct SliceWav<T> {
    pub starttime: i64,
    pub duration: i64,
    pub wav: T,
    pub playid: i64,
}

impl<T> SliceWav<T> {
    pub fn new(starttime: i64, duration: i64, wav: T) -> Self {
        SliceWav {
            starttime,
            duration,
            wav,
            playid: -1,
        }
    }
}

/// Audio element wrapper.
///
/// Translated from: AbstractAudioDriver.AudioElement
pub struct AudioElement<T> {
    pub id: i64,
    pub audio: T,
}

impl<T> AudioElement<T> {
    pub fn new(audio: T) -> Self {
        AudioElement { id: 0, audio }
    }
}

/// Audio cache key.
///
/// Translated from: AbstractAudioDriver.AudioKey
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AudioKey {
    pub path: String,
    pub start: i64,
    pub duration: i64,
}

impl AudioKey {
    pub fn new(path: String, n: &Note) -> Self {
        AudioKey {
            path,
            start: n.get_micro_starttime(),
            duration: n.get_micro_duration(),
        }
    }
}

/// Shared state for AbstractAudioDriver.
///
/// Translated from the fields of AbstractAudioDriver.java
pub struct AbstractAudioDriverState<T> {
    pub soundmap: HashMap<String, Option<AudioElement<T>>>,
    pub wavmap: Vec<Option<T>>,
    pub slicesound: Vec<Vec<SliceWav<T>>>,
    pub additional_key_sounds: [[Option<T>; 2]; 6],
    pub progress: AtomicI32,
    pub note_map_size: i32,
    pub volume: f32,
    pub global_pitch: f32,
    pub sample_rate: i32,
    pub channels: i32,
    maxgen: i32,
    // AudioCache simplified: HashMap<AudioKey, T>
    pub pcm_cache: HashMap<String, PCM>,
    pub audio_cache: HashMap<AudioKey, T>,
}

impl<T> AbstractAudioDriverState<T> {
    pub fn new(maxgen: i32) -> Self {
        let maxgen = maxgen.max(1);
        AbstractAudioDriverState {
            soundmap: HashMap::new(),
            wavmap: Vec::new(),
            slicesound: Vec::new(),
            additional_key_sounds: Default::default(),
            progress: AtomicI32::new(0),
            note_map_size: 0,
            volume: 1.0,
            global_pitch: 1.0,
            sample_rate: 0,
            channels: 0,
            maxgen,
            pcm_cache: HashMap::new(),
            audio_cache: HashMap::new(),
        }
    }

    pub fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    pub fn set_sample_rate(&mut self, sample_rate: i32) {
        self.sample_rate = sample_rate;
    }

    pub fn get_progress(&self) -> f32 {
        if self.note_map_size == 0 {
            return 0.0;
        }
        self.progress.load(Ordering::Relaxed) as f32 / self.note_map_size as f32
    }

    pub fn abort(&self) {
        self.progress.store(self.note_map_size, Ordering::Relaxed);
    }

    fn channel(id: i32, pitch: i32) -> i32 {
        id * 256 + pitch + 128
    }
}

// Default impl required for the const generic array
impl<T> Default for AbstractAudioDriverState<T> {
    fn default() -> Self {
        Self::new(1)
    }
}

// We need Default for Option<T> arrays, so use a helper
fn default_additional_key_sounds<T>() -> [[Option<T>; 2]; 6] {
    [
        [None, None],
        [None, None],
        [None, None],
        [None, None],
        [None, None],
        [None, None],
    ]
}
