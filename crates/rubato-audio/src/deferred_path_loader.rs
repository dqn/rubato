/// Deferred path sound loader - loads audio files on a background thread
/// to avoid blocking the render thread on cache misses.
///
/// Used by GdxSoundDriver, GdxAudioDeviceDriver, and PortAudioDriver.
use std::collections::HashSet;
use std::sync::mpsc;

use kira::sound::static_sound::StaticSoundData;

/// Result of a background path sound load.
struct PathLoadResult {
    path: String,
    sound: Option<StaticSoundData>,
}

/// A loaded sound with its path and pending play requests (volume, loop).
type LoadedSoundEntry = (String, StaticSoundData, Vec<(f32, bool)>);

/// Manages deferred (non-blocking) loading of path-based audio files.
///
/// On cache miss, spawns a background thread to load the file. The loaded
/// sound data is received via channel on the next `poll()` call and cached
/// for immediate playback.
pub(crate) struct DeferredPathLoader {
    tx: mpsc::Sender<PathLoadResult>,
    rx: mpsc::Receiver<PathLoadResult>,
    /// Paths currently being loaded in background threads.
    loading: HashSet<String>,
    /// Play requests waiting for their path to finish loading.
    pending_plays: Vec<(String, f32, bool)>,
}

impl DeferredPathLoader {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            loading: HashSet::new(),
            pending_plays: Vec::new(),
        }
    }

    /// Maximum number of concurrent background load threads.
    const MAX_CONCURRENT_LOADS: usize = 8;

    /// Queue a background load for the given path if not already loading.
    /// Also records the play request so it can be fulfilled when loading completes.
    ///
    /// When the maximum number of concurrent loads is reached, the play request
    /// is recorded but the load is skipped. During rapid song scrolling, this
    /// prevents spawning unbounded OS threads for each unique preview track.
    pub fn request_load(&mut self, path: &str, volume: f32, loop_play: bool) {
        if !self.loading.contains(path) && self.loading.len() < Self::MAX_CONCURRENT_LOADS {
            self.loading.insert(path.to_string());
            let tx = self.tx.clone();
            let path_owned = path.to_string();
            std::thread::Builder::new()
                .name(format!("path-audio-load:{}", path))
                .spawn(move || {
                    let candidates = crate::audio_driver::paths(&path_owned);
                    let mut loaded = None;
                    for candidate in &candidates {
                        if let Ok(sound_data) = StaticSoundData::from_file(candidate) {
                            loaded = Some(sound_data);
                            break;
                        }
                    }
                    let _ = tx.send(PathLoadResult {
                        path: path_owned,
                        sound: loaded,
                    });
                })
                .ok();
        }
        self.pending_plays
            .push((path.to_string(), volume, loop_play));
    }

    /// Poll for completed background loads. Returns newly loaded sounds
    /// and their pending play requests.
    ///
    /// Caller is responsible for inserting into `path_sound_cache` and playing.
    pub fn poll(&mut self) -> Vec<LoadedSoundEntry> {
        let mut results = Vec::new();

        while let Ok(result) = self.rx.try_recv() {
            self.loading.remove(&result.path);
            if let Some(sound) = result.sound {
                // Collect all pending plays for this path
                let plays: Vec<(f32, bool)> = self
                    .pending_plays
                    .iter()
                    .filter(|(p, _, _)| *p == result.path)
                    .map(|(_, v, l)| (*v, *l))
                    .collect();
                self.pending_plays.retain(|(p, _, _)| *p != result.path);
                results.push((result.path, sound, plays));
            } else {
                // Load failed - discard pending plays for this path
                self.pending_plays.retain(|(p, _, _)| *p != result.path);
            }
        }

        results
    }

    /// Drain all pending state (e.g., on dispose).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.loading.clear();
        self.pending_plays.clear();
        // Drain any remaining messages
        while self.rx.try_recv().is_ok() {}
    }
}
