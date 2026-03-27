// Shared helper functions for music_result and course_result mod.rs.
// These are identical between the two result screen implementations.

use crate::core::system_sound_manager::SoundType;

use super::MainController;
use super::PlayerResource;

/// Check whether a sound asset exists for the given SoundType.
#[inline]
pub fn has_sound(main: &MainController, sound: &SoundType) -> bool {
    main.sound_path(sound).is_some()
}

/// Set gauge_type from the player resource's groove gauge.
/// Returns the gauge type value (0 if no groove gauge is available).
#[inline]
pub fn set_gauge_type(resource: &PlayerResource) -> i32 {
    resource.groove_gauge().map(|g| g.gauge_type()).unwrap_or(0)
}
