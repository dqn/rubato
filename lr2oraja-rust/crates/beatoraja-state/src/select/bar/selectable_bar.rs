use super::bar::BarData;
use crate::select::music_selector::REPLAY;

/// Selectable bar shared data
/// Translates: bms.player.beatoraja.select.bar.SelectableBar
#[derive(Clone, Debug)]
pub struct SelectableBarData {
    pub bar_data: BarData,
    /// Whether replay data exists for each replay slot
    pub exists_replay: [bool; REPLAY],
}

impl Default for SelectableBarData {
    fn default() -> Self {
        Self {
            bar_data: BarData::default(),
            exists_replay: [false; REPLAY],
        }
    }
}

impl SelectableBarData {
    pub fn exists_replay_data(&self) -> bool {
        for b in &self.exists_replay {
            if *b {
                return true;
            }
        }
        false
    }

    pub fn exists_replay(&self, index: i32) -> bool {
        if index >= 0 && (index as usize) < self.exists_replay.len() {
            self.exists_replay[index as usize]
        } else {
            false
        }
    }

    pub fn set_exists_replay(&mut self, index: i32, exists: bool) {
        if index >= 0 && (index as usize) < self.exists_replay.len() {
            self.exists_replay[index as usize] = exists;
        }
    }
}
