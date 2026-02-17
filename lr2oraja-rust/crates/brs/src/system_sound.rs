// System sound playback queue manager.

/// System sound types for state transitions.
#[allow(dead_code)] // TODO: integrate with audio system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemSound {
    Decide,
    ResultClear,
    ResultFail,
    Select,
    Scratch,
    Folder,
    OptionChange,
}

/// Manages system sound playback queue.
#[allow(dead_code)] // TODO: integrate with audio system
#[derive(Default)]
pub struct SystemSoundManager {
    /// Queue of sounds to play this frame.
    queue: Vec<SystemSound>,
}

#[allow(dead_code)] // TODO: integrate with audio system
impl SystemSoundManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a sound for playback.
    pub fn play(&mut self, sound: SystemSound) {
        self.queue.push(sound);
    }

    /// Drain the queue (consumed by audio system each frame).
    pub fn drain(&mut self) -> Vec<SystemSound> {
        std::mem::take(&mut self.queue)
    }

    /// Check if queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        let mgr = SystemSoundManager::new();
        assert!(mgr.is_empty());
    }

    #[test]
    fn play_adds_to_queue() {
        let mut mgr = SystemSoundManager::new();
        mgr.play(SystemSound::Decide);
        mgr.play(SystemSound::Select);
        assert!(!mgr.is_empty());
    }

    #[test]
    fn drain_returns_and_clears_queue() {
        let mut mgr = SystemSoundManager::new();
        mgr.play(SystemSound::ResultClear);
        mgr.play(SystemSound::Folder);
        let drained = mgr.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0], SystemSound::ResultClear);
        assert_eq!(drained[1], SystemSound::Folder);
        assert!(mgr.is_empty());
    }
}
