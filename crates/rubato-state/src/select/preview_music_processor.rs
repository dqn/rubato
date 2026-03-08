use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rubato_audio::audio_driver::AudioDriver;
use rubato_types::main_controller_access::MainControllerAccess;

use super::stubs::*;

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
}

trait PreviewAudioTarget {
    fn play_preview_path(&mut self, path: &str, volume: f32, loop_play: bool);
    fn set_preview_volume(&mut self, path: &str, volume: f32);
    fn is_preview_playing(&self, path: &str) -> bool;
    fn stop_preview_path(&mut self, path: &str);
    fn dispose_preview_path(&mut self, path: &str);
}

struct AudioDriverTarget<'a> {
    inner: &'a mut dyn AudioDriver,
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

struct MainControllerTarget<'a> {
    inner: &'a mut dyn MainControllerAccess,
}

impl PreviewAudioTarget for MainControllerTarget<'_> {
    fn play_preview_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        self.inner.play_audio_path(path, volume, loop_play);
    }

    fn set_preview_volume(&mut self, path: &str, volume: f32) {
        self.inner.set_audio_path_volume(path, volume);
    }

    fn is_preview_playing(&self, path: &str) -> bool {
        self.inner.is_audio_path_playing(path)
    }

    fn stop_preview_path(&mut self, path: &str) {
        self.inner.stop_audio_path(path);
    }

    fn dispose_preview_path(&mut self, path: &str) {
        self.inner.dispose_audio_path(path);
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
        }
    }

    pub fn set_default(&mut self, path: &str) {
        self.default_music = path.to_string();
    }

    pub fn start(&mut self, song: Option<&SongData>) {
        if !self.preview_running.load(Ordering::SeqCst) {
            self.preview_running.store(true, Ordering::SeqCst);
            // In Java: starts PreviewThread. Here we would spawn a thread.
            // Stubbed since audio playback requires runtime integration.
        }
        self.current = song.cloned();

        let mut preview_path = String::new();
        if let Some(song) = song
            && !song.file.preview.is_empty()
            && let Some(song_path) = song.path()
            && let Some(parent) = Path::new(song_path).parent()
        {
            preview_path = parent
                .join(&song.file.preview)
                .to_string_lossy()
                .to_string();
        }

        if let Ok(mut cmds) = self.commands.lock() {
            cmds.push_back(preview_path);
        }
    }

    pub fn song_data(&self) -> Option<&SongData> {
        self.current.as_ref()
    }

    pub fn stop(&mut self) {
        self.preview_running.store(false, Ordering::SeqCst);
    }

    pub fn tick_preview(&mut self, audio: &mut dyn AudioDriver, config: &Config) {
        let mut target = AudioDriverTarget { inner: audio };
        self.tick_with_target(&mut target, config);
    }

    pub fn tick_preview_with_main(&mut self, main: &mut dyn MainControllerAccess, config: &Config) {
        let mut target = MainControllerTarget { inner: main };
        self.tick_with_target(&mut target, config);
    }

    fn tick_with_target<T: PreviewAudioTarget + ?Sized>(&mut self, audio: &mut T, config: &Config) {
        let sys_vol = config.audio.as_ref().map(|a| a.systemvolume).unwrap_or(0.5);

        if !self.preview_running.load(Ordering::SeqCst) {
            if !self.playing.is_empty() {
                Self::stop_preview_internal(
                    audio,
                    &self.playing,
                    &self.default_music,
                    sys_vol,
                    false,
                );
                self.playing.clear();
                self.default_started = false;
            }
            return;
        }

        if !self.default_started {
            audio.play_preview_path(&self.default_music, sys_vol, true);
            self.playing = self.default_music.clone();
            self.current_volume = sys_vol;
            self.default_started = true;
        }

        let next_path = self
            .commands
            .lock()
            .ok()
            .and_then(|mut cmds| cmds.pop_front());

        if let Some(path) = next_path {
            let path = if path.is_empty() {
                self.default_music.clone()
            } else {
                path
            };
            if path != self.playing {
                Self::stop_preview_internal(
                    audio,
                    &self.playing,
                    &self.default_music,
                    sys_vol,
                    true,
                );
                if path != self.default_music {
                    let looping = matches!(config.select.song_preview, SongPreview::LOOP);
                    audio.play_preview_path(&path, sys_vol, looping);
                } else {
                    audio.set_preview_volume(&self.default_music, sys_vol);
                }
                self.playing = path;
                self.current_volume = sys_vol;
            }
        } else if self.playing != self.default_music && !audio.is_preview_playing(&self.playing) {
            Self::stop_preview_internal(audio, &self.playing, &self.default_music, sys_vol, true);
            audio.set_preview_volume(&self.default_music, sys_vol);
            self.playing = self.default_music.clone();
            self.current_volume = sys_vol;
        } else if (self.current_volume - sys_vol).abs() > f32::EPSILON {
            audio.set_preview_volume(&self.playing, sys_vol);
            self.current_volume = sys_vol;
        }
    }

    /// Run the preview thread main loop.
    /// Corresponds to Java PreviewThread.run()
    /// In Java this is the inner thread's run() method that:
    /// 1. Plays default music
    /// 2. Polls commands queue for preview path changes
    /// 3. Stops preview and switches back to default when preview ends
    /// 4. Updates volume when system volume changes
    pub fn run_preview_loop(&self, audio: &mut dyn AudioDriver, config: &Config) {
        let sys_vol = config.audio.as_ref().map(|a| a.systemvolume).unwrap_or(0.5);
        audio.play_path(&self.default_music, sys_vol, true);
        let mut playing = self.default_music.clone();
        let mut current_volume = sys_vol;

        while self.preview_running.load(Ordering::SeqCst) {
            let sys_vol = config.audio.as_ref().map(|a| a.systemvolume).unwrap_or(0.5);
            if let Ok(mut cmds) = self.commands.lock() {
                if let Some(path) = cmds.pop_front() {
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
                    drop(cmds);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }
            }
        }
        let sys_vol = config.audio.as_ref().map(|a| a.systemvolume).unwrap_or(0.5);
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
    use ::bms_model::bms_model::BMSModel;
    use ::bms_model::note::Note;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::AtomicI32;

    /// Mock AudioDriver for testing PreviewMusicProcessor.
    struct MockAudioDriver {
        play_count: AtomicI32,
        stop_count: AtomicI32,
    }

    impl MockAudioDriver {
        fn new() -> Self {
            Self {
                play_count: AtomicI32::new(0),
                stop_count: AtomicI32::new(0),
            }
        }
    }

    impl AudioDriver for MockAudioDriver {
        fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {
            self.play_count.fetch_add(1, Ordering::SeqCst);
        }
        fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
        fn is_playing_path(&self, _path: &str) -> bool {
            false
        }
        fn stop_path(&mut self, _path: &str) {
            self.stop_count.fetch_add(1, Ordering::SeqCst);
        }
        fn dispose_path(&mut self, _path: &str) {}
        fn set_model(&mut self, _model: &BMSModel) {}
        fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
        fn abort(&mut self) {}
        fn get_progress(&self) -> f32 {
            1.0
        }
        fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {}
        fn play_judge(&mut self, _judge: i32, _fast: bool) {}
        fn stop_note(&mut self, _n: Option<&Note>) {}
        fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}
        fn set_global_pitch(&mut self, _pitch: f32) {}
        fn get_global_pitch(&self) -> f32 {
            1.0
        }
        fn dispose_old(&mut self) {}
        fn dispose(&mut self) {}
    }

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
        let audio = MockAudioDriver::new();
        let config = Config::default();
        let processor = PreviewMusicProcessor::new(&config);
        assert!(processor.song_data().is_none());
    }

    #[test]
    fn test_set_default() {
        let audio = MockAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/path/to/bgm.ogg");
        assert_eq!(processor.default_music, "/path/to/bgm.ogg");
    }

    #[test]
    fn test_start_with_none_song() {
        let audio = MockAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.start(None);
        assert!(processor.song_data().is_none());
        // Command queue should have one entry (empty path)
        assert_eq!(processor.commands.lock().expect("mutex poisoned").len(), 1);
    }

    #[test]
    fn test_start_with_song() {
        let audio = MockAudioDriver::new();
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
        let audio = MockAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.preview_running.store(true, Ordering::SeqCst);
        processor.stop();
        assert!(!processor.preview_running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_run_preview_loop_immediate_stop() {
        let mut audio = MockAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/bgm/default.ogg");
        // preview_running is false, so run_preview_loop should exit immediately
        // after playing default music and then calling stop_preview_internal
        processor.run_preview_loop(&mut audio, &config);
        // Should have played the default music
        assert!(audio.play_count.load(Ordering::SeqCst) >= 1);
        // Should have stopped the default music on exit
        assert!(audio.stop_count.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn test_tick_preview_starts_default_music_when_running() {
        let mut audio = MockAudioDriver::new();
        let config = Config::default();
        let mut processor = PreviewMusicProcessor::new(&config);
        processor.set_default("/bgm/default.ogg");
        processor.preview_running.store(true, Ordering::SeqCst);

        processor.tick_preview(&mut audio, &config);

        assert_eq!(audio.play_count.load(Ordering::SeqCst), 1);
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
}
