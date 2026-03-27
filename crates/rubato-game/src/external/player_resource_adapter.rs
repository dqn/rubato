// PlayerResource wrapper delegating to Box<dyn PlayerResourceAccess>.

use crate::core::config::Config;
use crate::core::replay_data::ReplayData;
use crate::song::song_data::SongData;
use bms::model::mode::Mode;
use rubato_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};

/// Wrapper for bms.player.beatoraja.PlayerResource.
/// Delegates to `Box<dyn PlayerResourceAccess>` for trait methods.
/// `original_mode()` is crate-local (not on trait, since Mode lives in bms-model).
pub struct PlayerResource {
    pub(crate) inner: Box<dyn PlayerResourceAccess>,
    original_mode: Mode,
}

impl PlayerResource {
    pub fn new(inner: Box<dyn PlayerResourceAccess>, original_mode: Mode) -> Self {
        Self {
            inner,
            original_mode,
        }
    }

    pub fn config(&self) -> &Config {
        self.inner.config()
    }

    pub fn songdata(&self) -> Option<&SongData> {
        self.inner.songdata()
    }

    pub fn replay_data(&self) -> Option<&ReplayData> {
        self.inner.replay_data()
    }

    pub fn reverse_lookup_levels(&self) -> Vec<String> {
        self.inner.reverse_lookup_levels()
    }

    pub fn original_mode(&self) -> &Mode {
        &self.original_mode
    }
}

impl Default for PlayerResource {
    fn default() -> Self {
        Self {
            inner: Box::new(NullPlayerResource::new()),
            original_mode: Mode::BEAT_7K,
        }
    }
}
