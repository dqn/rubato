use std::sync::Mutex;

use bms::model::mode::Mode;
use rubato_types::play_config::PlayConfig;
use rubato_types::player_config::PlayerConfig;
use rubato_types::skin_config::SkinConfig;

/// Pending actions collected from modmenu egui callbacks.
///
/// Modmenus write to this from egui closures (which cannot carry `&mut` references);
/// `MainController` drains each frame in `lifecycle.rs`.
#[derive(Default)]
pub struct ModmenuOutboxInner {
    pub play_config_updates: Vec<(Mode, Box<PlayConfig>)>,
    pub load_new_profile: Option<Box<PlayerConfig>>,
    pub save_config: bool,
    pub skin_config_updates: Vec<(usize, Option<Box<SkinConfig>>)>,
    pub skin_history_updates: Vec<(String, Box<SkinConfig>)>,
}

/// Thread-safe outbox for modmenu actions.
///
/// Modmenus write to this from egui callbacks; MainController drains each frame.
pub struct ModmenuOutbox {
    inner: Mutex<ModmenuOutboxInner>,
}

impl Default for ModmenuOutbox {
    fn default() -> Self {
        Self::new()
    }
}

impl ModmenuOutbox {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ModmenuOutboxInner::default()),
        }
    }

    pub fn push_play_config_update(&self, mode: Mode, config: PlayConfig) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.play_config_updates.push((mode, Box::new(config)));
    }

    pub fn push_load_new_profile(&self, pc: PlayerConfig) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.load_new_profile = Some(Box::new(pc));
    }

    pub fn push_save_config(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.save_config = true;
    }

    pub fn push_skin_config_update(&self, id: usize, config: Option<SkinConfig>) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.skin_config_updates.push((id, config.map(Box::new)));
    }

    pub fn push_skin_history_update(&self, path: String, config: SkinConfig) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.skin_history_updates.push((path, Box::new(config)));
    }

    /// Drain all pending actions. Called by MainController each frame.
    pub fn drain(&self) -> ModmenuOutboxInner {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *inner)
    }

    /// Check if there are no pending actions.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.play_config_updates.is_empty()
            && inner.load_new_profile.is_none()
            && !inner.save_config
            && inner.skin_config_updates.is_empty()
            && inner.skin_history_updates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_outbox_is_empty() {
        let outbox = ModmenuOutbox::new();
        assert!(outbox.is_empty());
    }

    #[test]
    fn push_and_drain_play_config_update() {
        let outbox = ModmenuOutbox::new();
        let config = PlayConfig::default();
        outbox.push_play_config_update(Mode::BEAT_7K, config);
        assert!(!outbox.is_empty());

        let drained = outbox.drain();
        assert_eq!(drained.play_config_updates.len(), 1);
        assert_eq!(drained.play_config_updates[0].0, Mode::BEAT_7K);
        assert!(outbox.is_empty());
    }

    #[test]
    fn push_and_drain_load_new_profile() {
        let outbox = ModmenuOutbox::new();
        let pc = PlayerConfig::default();
        outbox.push_load_new_profile(pc);
        assert!(!outbox.is_empty());

        let drained = outbox.drain();
        assert!(drained.load_new_profile.is_some());
        assert!(outbox.is_empty());
    }

    #[test]
    fn push_and_drain_save_config() {
        let outbox = ModmenuOutbox::new();
        outbox.push_save_config();
        assert!(!outbox.is_empty());

        let drained = outbox.drain();
        assert!(drained.save_config);
        assert!(outbox.is_empty());
    }

    #[test]
    fn push_and_drain_skin_config_update() {
        let outbox = ModmenuOutbox::new();
        let config = SkinConfig::default();
        outbox.push_skin_config_update(3, Some(config));
        assert!(!outbox.is_empty());

        let drained = outbox.drain();
        assert_eq!(drained.skin_config_updates.len(), 1);
        assert_eq!(drained.skin_config_updates[0].0, 3);
        assert!(drained.skin_config_updates[0].1.is_some());
        assert!(outbox.is_empty());
    }

    #[test]
    fn push_and_drain_skin_history_update() {
        let outbox = ModmenuOutbox::new();
        let config = SkinConfig::default();
        outbox.push_skin_history_update("/skins/test.json".to_string(), config);
        assert!(!outbox.is_empty());

        let drained = outbox.drain();
        assert_eq!(drained.skin_history_updates.len(), 1);
        assert_eq!(drained.skin_history_updates[0].0, "/skins/test.json");
        assert!(outbox.is_empty());
    }

    #[test]
    fn drain_returns_all_and_resets() {
        let outbox = ModmenuOutbox::new();
        outbox.push_play_config_update(Mode::BEAT_7K, PlayConfig::default());
        outbox.push_save_config();
        outbox.push_skin_config_update(0, None);

        let drained = outbox.drain();
        assert_eq!(drained.play_config_updates.len(), 1);
        assert!(drained.save_config);
        assert_eq!(drained.skin_config_updates.len(), 1);

        // Second drain should be empty
        let drained2 = outbox.drain();
        assert!(drained2.play_config_updates.is_empty());
        assert!(!drained2.save_config);
        assert!(drained2.skin_config_updates.is_empty());
    }
}
