use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rubato_audio::audio_system::AudioSystem;
use rubato_skin::audio_config::DEFAULT_AUDIO_VOLUME;
use rubato_skin::sync_utils::lock_or_recover;

use super::*;

/// Preview music processor
/// Translates: bms.player.beatoraja.select.PreviewMusicProcessor
pub struct PreviewMusicProcessor {
    /// Music loading task queue
    commands: Arc<Mutex<VecDeque<String>>>,
    preview_running: Arc<AtomicBool>,
    default_music: String,
    current: Option<SongData>,
    playing: String,
    current_volume: f32,
    default_started: bool,
    /// Non-blocking fade-out state for the render-thread path.
    /// When Some, we're fading out the default music before switching to a new track.
    fade_out: Option<FadeOutState>,
}

/// Tracks non-blocking fade-out progress.
struct FadeOutState {
    /// Remaining fade steps (counting down from 10 to 0).
    remaining_steps: i32,
    /// System volume for scaling.
    sys_vol: f32,
    /// Path to switch to after fade completes.
    next_path: String,
    /// Timestamp of last fade step.
    last_step_time: std::time::Instant,
}

trait PreviewAudioTarget {
    fn play_preview_path(&mut self, path: &str, volume: f32, loop_play: bool);
    fn set_preview_volume(&mut self, path: &str, volume: f32);
    fn is_preview_playing(&self, path: &str) -> bool;
    fn stop_preview_path(&mut self, path: &str);
    fn dispose_preview_path(&mut self, path: &str);
}

struct AudioDriverTarget<'a> {
    inner: &'a mut AudioSystem,
}

impl PreviewAudioTarget for AudioDriverTarget<'_> {
    fn play_preview_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        self.inner.play_path(path, volume, loop_play);
    }

    fn set_preview_volume(&mut self, path: &str, volume: f32) {
        self.inner.set_volume_path(path, volume);
    }

    fn is_preview_playing(&self, path: &str) -> bool {
        self.inner.is_playing_path(path)
    }

    fn stop_preview_path(&mut self, path: &str) {
        self.inner.stop_path(path);
    }

    fn dispose_preview_path(&mut self, path: &str) {
        self.inner.dispose_path(path);
    }
}

impl PreviewMusicProcessor {
    pub fn new(_config: &Config) -> Self {
        Self {
            commands: Arc::new(Mutex::new(VecDeque::new())),
            preview_running: Arc::new(AtomicBool::new(false)),
            default_music: String::new(),
            current: None,
            playing: String::new(),
            current_volume: 0.0,
            default_started: false,
            fade_out: None,
        }
    }

    pub fn set_default(&mut self, path: &str) {
        self.default_music = path.to_string();
    }

    pub fn start(&mut self, song: Option<&SongData>) {
        if !self.preview_running.load(Ordering::SeqCst) {
            self.preview_running.store(true, Ordering::SeqCst);
        }
        self.current = song.cloned();

        let mut preview_path = String::new();
        if let Some(song) = song
            && !song.file.preview.is_empty()
            && let Some(song_path) = song.file.path()
            && let Some(parent) = Path::new(song_path).parent()
        {
            preview_path = parent
                .join(&song.file.preview)
                .to_string_lossy()
                .to_string();
        }

        lock_or_recover(&self.commands).push_back(preview_path);
    }

    pub fn song_data(&self) -> Option<&SongData> {
        self.current.as_ref()
    }

    pub fn stop(&mut self) {
        self.preview_running.store(false, Ordering::SeqCst);
    }

    pub fn tick_preview(&mut self, audio: &mut AudioSystem, config: &Config) {
        let mut target = AudioDriverTarget { inner: audio };
        self.tick_with_target(&mut target, config);
    }

    fn tick_with_target<T: PreviewAudioTarget + ?Sized>(&mut self, audio: &mut T, config: &Config) {
        let sys_vol = config
            .audio
            .as_ref()
            .map(|a| a.systemvolume)
            .unwrap_or(DEFAULT_AUDIO_VOLUME);

        if !self.preview_running.load(Ordering::SeqCst) {
            self.fade_out = None;
            if !self.playing.is_empty() {
                Self::stop_preview_nonblocking(audio, &self.playing, &self.default_music, false);
                self.playing.clear();
                self.default_started = false;
            }
            return;
        }

        // Process ongoing fade-out animation (one step per tick, ~15ms apart).
        if let Some(ref mut fade) = self.fade_out {
            if fade.last_step_time.elapsed() >= std::time::Duration::from_millis(15) {
                fade.remaining_steps -= 1;
                let vol = fade.remaining_steps as f32 * 0.1 * fade.sys_vol;
                audio.set_preview_volume(&self.default_music, vol.max(0.0));
                fade.last_step_time = std::time::Instant::now();
            }
            if fade.remaining_steps <= 0 {
                // Fade complete -- switch to the next track.
                let next_path = std::mem::take(&mut fade.next_path);
                let fade_sys_vol = fade.sys_vol;
                self.fade_out = None;
                if next_path != self.default_music {
                    let looping = matches!(config.select.song_preview, SongPreview::LOOP);
                    audio.play_preview_path(&next_path, fade_sys_vol, looping);
                } else {
                    audio.set_preview_volume(&self.default_music, fade_sys_vol);
                }
                self.playing = next_path;
                self.current_volume = fade_sys_vol;
            }
            return;
        }

        if !self.default_started {
            audio.play_preview_path(&self.default_music, sys_vol, true);
            self.playing = self.default_music.clone();
            self.current_volume = sys_vol;
            self.default_started = true;
        }

        let next_path = lock_or_recover(&self.commands).pop_front();

        if let Some(path) = next_path {
            let path = if path.is_empty() {
                self.default_music.clone()
            } else {
                path
            };
            if path != self.playing {
                if self.playing == self.default_music {
                    // Fade out default music before switching (non-blocking).
                    self.fade_out = Some(FadeOutState {
                        remaining_steps: 10,
                        sys_vol,
                        next_path: path,
                        last_step_time: std::time::Instant::now(),
                    });
                } else {
                    // Non-default track: stop immediately, no fade.
                    audio.stop_preview_path(&self.playing);
                    audio.dispose_preview_path(&self.playing);
                    if path != self.default_music {
                        let looping = matches!(config.select.song_preview, SongPreview::LOOP);
                        audio.play_preview_path(&path, sys_vol, looping);
                    } else {
                        audio.set_preview_volume(&self.default_music, sys_vol);
                    }
                    self.playing = path;
                    self.current_volume = sys_vol;
                }
            }
        } else if self.playing != self.default_music && !audio.is_preview_playing(&self.playing) {
            // Preview finished, return to default music.
            audio.stop_preview_path(&self.playing);
            audio.dispose_preview_path(&self.playing);
            audio.set_preview_volume(&self.default_music, sys_vol);
            self.playing = self.default_music.clone();
            self.current_volume = sys_vol;
        } else if (self.current_volume - sys_vol).abs() > f32::EPSILON {
            audio.set_preview_volume(&self.playing, sys_vol);
            self.current_volume = sys_vol;
        }
    }

    /// Non-blocking stop: stops or disposes the track without fade-out sleep.
    /// Used when preview is stopping entirely (not pausing).
    fn stop_preview_nonblocking<T: PreviewAudioTarget + ?Sized>(
        audio: &mut T,
        playing: &str,
        default_music: &str,
        _pause: bool,
    ) {
        if !playing.is_empty() {
            if playing != default_music {
                audio.stop_preview_path(playing);
                audio.dispose_preview_path(playing);
            } else {
                audio.stop_preview_path(playing);
            }
        }
    }

    /// Run the preview thread main loop.
    /// Corresponds to Java PreviewThread.run()
    /// In Java this is the inner thread's run() method that:
    /// 1. Plays default music
    /// 2. Polls commands queue for preview path changes
    /// 3. Stops preview and switches back to default when preview ends
    /// 4. Updates volume when system volume changes
    pub fn run_preview_loop(&self, audio: &mut AudioSystem, config: &Config) {
        let sys_vol = config
            .audio
            .as_ref()
            .map(|a| a.systemvolume)
            .unwrap_or(DEFAULT_AUDIO_VOLUME);
        audio.play_path(&self.default_music, sys_vol, true);
        let mut playing = self.default_music.clone();
        let mut current_volume = sys_vol;

        while self.preview_running.load(Ordering::SeqCst) {
            let sys_vol = config
                .audio
                .as_ref()
                .map(|a| a.systemvolume)
                .unwrap_or(DEFAULT_AUDIO_VOLUME);
            // Drain command with lock held only briefly, then perform audio work unlocked.
            let next_cmd = lock_or_recover(&self.commands).pop_front();

            if let Some(path) = next_cmd {
                let path = if path.is_empty() {
                    self.default_music.clone()
                } else {
                    path
                };
                if path != playing {
                    let mut target = AudioDriverTarget { inner: audio };
                    Self::stop_preview_internal(
                        &mut target,
                        &playing,
                        &self.default_music,
                        sys_vol,
                        true,
                    );
                    if path != self.default_music {
                        let looping = matches!(config.select.song_preview, SongPreview::LOOP);
                        audio.play_path(&path, sys_vol, looping);
                    } else {
                        audio.set_volume_path(&self.default_music, sys_vol);
                    }
                    playing = path;
                }
            } else if playing != self.default_music && !audio.is_playing_path(&playing) {
                // Preview finished, return to default music
                let mut target = AudioDriverTarget { inner: audio };
                Self::stop_preview_internal(
                    &mut target,
                    &playing,
                    &self.default_music,
                    sys_vol,
                    true,
                );
                audio.set_volume_path(&self.default_music, sys_vol);
                playing = self.default_music.clone();
            } else if (current_volume - sys_vol).abs() > f32::EPSILON {
                audio.set_volume_path(&playing, sys_vol);
                current_volume = sys_vol;
            } else {
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }
        }
        let sys_vol = config
            .audio
            .as_ref()
            .map(|a| a.systemvolume)
            .unwrap_or(DEFAULT_AUDIO_VOLUME);
        let mut target = AudioDriverTarget { inner: audio };
        Self::stop_preview_internal(&mut target, &playing, &self.default_music, sys_vol, false);
    }

    /// Stop the currently playing preview.
    /// Corresponds to Java PreviewThread.stopPreview(boolean pause)
    fn stop_preview_internal<T: PreviewAudioTarget + ?Sized>(
        audio: &mut T,
        playing: &str,
        default_music: &str,
        sys_vol: f32,
        pause: bool,
    ) {
        if !playing.is_empty() {
            if playing != default_music {
                audio.stop_preview_path(playing);
                audio.dispose_preview_path(playing);
            } else if pause {
                // Fade out
                for i in (0..=10).rev() {
                    let vol = i as f32 * 0.1 * sys_vol;
                    audio.set_preview_volume(playing, vol);
                    std::thread::sleep(std::time::Duration::from_millis(15));
                }
            } else {
                audio.stop_preview_path(playing);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_audio::recording_audio_driver::RecordingAudioDriver;
    use std::sync::atomic::AtomicBool;

    struct LockCheckingTarget {
        commands: Arc<Mutex<VecDeque<String>>>,
        queue_was_unlocked_during_fade: AtomicBool,
    }

    impl LockCheckingTarget {
        fn new(commands: Arc<Mutex<VecDeque<String>>>) -> Self {
            Self {
                commands,
                queue_was_unlocked_during_fade: AtomicBool::new(true),
            }
        }
    }

    impl PreviewAudioTarget for LockCheckingTarget {
        fn play_preview_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {}

        fn set_preview_volume(&mut self, _path: &str, _volume: f32) {
            if self.commands.try_lock().is_err() {
                self.queue_was_unlocked_during_fade
                    .store(false, Ordering::SeqCst);
            }
        }

        fn is_preview_playing(&self, _path: &str) -> bool {
            false
        }

        fn stop_preview_path(&mut self, _path: &str) {}

        fn dispose_preview_path(&mut self, _path: &str) {}
    }

    #[test]
    fn test_new_with_audio_driver_trait() {
        let _audio = RecordingAudioDriver::new();
        let config = Config::default();
        let processor = PreviewMusicProcessor::new(&config);
        assert!(processor.song_data().is_none());
    }

    #[test]
    fn test_set_default() {
        let _audio = RecordingAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/path/to/bgm.ogg");
        assert_eq!(processor.default_music, "/path/to/bgm.ogg");
    }

    #[test]
    fn test_start_with_none_song() {
        let _audio = RecordingAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.start(None);
        assert!(processor.song_data().is_none());
        // Command queue should have one entry (empty path)
        assert_eq!(processor.commands.lock().expect("mutex poisoned").len(), 1);
    }

    #[test]
    fn test_start_with_song() {
        let _audio = RecordingAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);

        let mut song = SongData::default();
        song.file.sha256 = "abc".to_string();
        processor.start(Some(&song));
        assert!(processor.song_data().is_some());
        assert_eq!(processor.song_data().unwrap().file.sha256, "abc");
    }

    #[test]
    fn test_stop() {
        let _audio = RecordingAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.preview_running.store(true, Ordering::SeqCst);
        processor.stop();
        assert!(!processor.preview_running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_run_preview_loop_immediate_stop() {
        let mut audio = AudioSystem::Recording(RecordingAudioDriver::new());
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/bgm/default.ogg");
        // preview_running is false, so run_preview_loop should exit immediately
        // after playing default music and then calling stop_preview_internal
        processor.run_preview_loop(&mut audio, &config);
        // Should have played the default music
        if let AudioSystem::Recording(ref inner) = audio {
            assert!(inner.play_path_count() >= 1);
            // Should have stopped the default music on exit
            assert!(inner.stop_path_count() >= 1);
        } else {
            panic!("expected Recording variant");
        }
    }

    #[test]
    fn test_tick_preview_starts_default_music_when_running() {
        let mut audio = AudioSystem::Recording(RecordingAudioDriver::new());
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/bgm/default.ogg");
        processor.preview_running.store(true, Ordering::SeqCst);

        processor.tick_preview(&mut audio, &config);

        if let AudioSystem::Recording(ref inner) = audio {
            assert_eq!(inner.play_path_count(), 1);
        } else {
            panic!("expected Recording variant");
        }
    }

    #[test]
    fn test_tick_preview_releases_command_queue_before_fade() {
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/bgm/default.ogg");
        processor.preview_running.store(true, Ordering::SeqCst);
        processor.playing = "/bgm/default.ogg".to_string();
        processor.default_started = true;
        processor.current_volume = 0.5;
        processor
            .commands
            .lock()
            .unwrap()
            .push_back("/preview/song.ogg".to_string());

        let mut target = LockCheckingTarget::new(Arc::clone(&processor.commands));
        processor.tick_with_target(&mut target, &config);

        assert!(
            target.queue_was_unlocked_during_fade.load(Ordering::SeqCst),
            "command queue should not stay locked while fade-out work runs"
        );
    }

    #[test]
    fn test_tick_preview_uses_default_volume_when_audio_config_missing() {
        let mut config = Config::default();
        config.audio = None;

        let mut audio = AudioSystem::Recording(RecordingAudioDriver::new());
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/bgm/default.ogg");
        processor.preview_running.store(true, Ordering::SeqCst);

        processor.tick_preview(&mut audio, &config);

        if let AudioSystem::Recording(ref inner) = audio {
            assert!(matches!(
                inner.events().first(),
                Some(rubato_audio::recording_audio_driver::AudioEvent::PlayPath {
                    volume,
                    ..
                }) if (*volume - 0.1).abs() < f32::EPSILON
            ));
        } else {
            panic!("expected Recording variant");
        }
    }
}
