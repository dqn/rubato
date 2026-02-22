use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::stubs::*;

/// Preview music processor
/// Translates: bms.player.beatoraja.select.PreviewMusicProcessor
pub struct PreviewMusicProcessor {
    /// Music loading task queue
    commands: Arc<Mutex<VecDeque<String>>>,
    preview_running: Arc<AtomicBool>,
    default_music: String,
    current: Option<SongData>,
}

impl PreviewMusicProcessor {
    pub fn new(_audio: &AudioDriver, _config: &Config) -> Self {
        Self {
            commands: Arc::new(Mutex::new(VecDeque::new())),
            preview_running: Arc::new(AtomicBool::new(false)),
            default_music: String::new(),
            current: None,
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
            && !song.get_preview().is_empty()
            && let Some(song_path) = song.get_path()
            && let Some(parent) = Path::new(song_path).parent()
        {
            preview_path = parent
                .join(song.get_preview())
                .to_string_lossy()
                .to_string();
        }

        if let Ok(mut cmds) = self.commands.lock() {
            cmds.push_back(preview_path);
        }
    }

    pub fn get_song_data(&self) -> Option<&SongData> {
        self.current.as_ref()
    }

    pub fn stop(&mut self) {
        self.preview_running.store(false, Ordering::SeqCst);
    }

    /// Run the preview thread main loop.
    /// Corresponds to Java PreviewThread.run()
    /// In Java this is the inner thread's run() method that:
    /// 1. Plays default music
    /// 2. Polls commands queue for preview path changes
    /// 3. Stops preview and switches back to default when preview ends
    /// 4. Updates volume when system volume changes
    pub fn run_preview_loop(&self, audio: &AudioDriver, config: &Config) {
        let sys_vol = config.audio.as_ref().map(|a| a.systemvolume).unwrap_or(0.5);
        audio.play(&self.default_music, sys_vol, true);
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
                        Self::stop_preview_internal(
                            audio,
                            &playing,
                            &self.default_music,
                            sys_vol,
                            true,
                        );
                        if path != self.default_music {
                            let looping = matches!(config.song_preview, SongPreview::LOOP);
                            audio.play(&path, sys_vol, looping);
                        } else {
                            audio.set_volume(&self.default_music, sys_vol);
                        }
                        playing = path;
                    }
                } else if playing != self.default_music && !audio.is_playing(&playing) {
                    // Preview finished, return to default music
                    Self::stop_preview_internal(
                        audio,
                        &playing,
                        &self.default_music,
                        sys_vol,
                        true,
                    );
                    audio.set_volume(&self.default_music, sys_vol);
                    playing = self.default_music.clone();
                } else if (current_volume - sys_vol).abs() > f32::EPSILON {
                    audio.set_volume(&playing, sys_vol);
                    current_volume = sys_vol;
                } else {
                    drop(cmds);
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }
            }
        }
        let sys_vol = config.audio.as_ref().map(|a| a.systemvolume).unwrap_or(0.5);
        Self::stop_preview_internal(audio, &playing, &self.default_music, sys_vol, false);
    }

    /// Stop the currently playing preview.
    /// Corresponds to Java PreviewThread.stopPreview(boolean pause)
    fn stop_preview_internal(
        audio: &AudioDriver,
        playing: &str,
        default_music: &str,
        sys_vol: f32,
        pause: bool,
    ) {
        if !playing.is_empty() {
            if playing != default_music {
                audio.stop(playing);
                audio.dispose(playing);
            } else if pause {
                // Fade out
                for i in (0..=10).rev() {
                    let vol = i as f32 * 0.1 * sys_vol;
                    audio.set_volume(playing, vol);
                    std::thread::sleep(std::time::Duration::from_millis(15));
                }
            } else {
                audio.stop(playing);
            }
        }
    }
}
