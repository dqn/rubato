use crate::main_state_type::MainStateType;
use crate::screen_type::ScreenType;
use crate::state_event::StateEvent;

/// Unified application event delivered via channels to external listeners
/// and test harnesses.
///
/// Replaces the `MainStateListener` trait callback pattern and the
/// `Arc<Mutex<Vec<StateEvent>>>` event log with a single channel-based
/// delivery mechanism.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// A MainStateListener-equivalent notification sent after each state
    /// transition completes. Carries a snapshot of all data that listeners
    /// previously queried from `&dyn MainStateAccess`.
    StateChanged(StateChangedData),

    /// A state machine lifecycle event for E2E test observability.
    /// Wraps the existing `StateEvent` variants.
    Lifecycle(StateEvent),
}

/// Snapshot of state data sent with `AppEvent::StateChanged`.
///
/// Contains all fields that DiscordListener and ObsListener previously
/// extracted from `&dyn MainStateAccess` during their `update()` calls.
#[derive(Debug, Clone)]
pub struct StateChangedData {
    /// The screen type (external-facing state classification).
    pub screen_type: ScreenType,
    /// The internal state type (for OBS listener state tracking).
    pub state_type: Option<MainStateType>,
    /// The listener status code (Java parity: `updateMainStateListener(int)`).
    pub status: i32,
    /// Song metadata, present when a song is loaded (Play/Result screens).
    pub song_info: Option<SongInfo>,
}

/// Song information snapshot for Discord Rich Presence.
#[derive(Debug, Clone)]
pub struct SongInfo {
    pub title: String,
    pub subtitle: String,
    pub artist: String,
    /// The key mode (e.g. 7, 14) from `SongData.chart.mode`.
    pub mode: i32,
}
