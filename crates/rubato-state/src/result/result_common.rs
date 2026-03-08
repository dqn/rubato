// Shared helper functions for music_result and course_result mod.rs.
// These are identical between the two result screen implementations.

use rubato_core::system_sound_manager::SoundType;

use super::stubs::MainController;
use super::stubs::PlayerResource;

/// Check whether a sound asset exists for the given SoundType.
#[inline]
pub fn has_sound(main: &MainController, sound: &SoundType) -> bool {
    main.sound_path(sound).is_some()
}

/// Play a sound (non-looping).
#[inline]
pub fn play_sound(main: &mut MainController, sound: &SoundType) {
    main.play_sound(sound, false);
}

/// Stop a sound.
#[inline]
pub fn stop_sound(main: &mut MainController, sound: &SoundType) {
    main.stop_sound(sound);
}

/// Play a sound with optional looping.
#[inline]
pub fn play_sound_loop(main: &mut MainController, sound: &SoundType, loop_sound: bool) {
    main.play_sound(sound, loop_sound);
}

/// Set gauge_type from the player resource's groove gauge.
/// Returns the gauge type value (0 if no groove gauge is available).
#[inline]
pub fn set_gauge_type(resource: &PlayerResource) -> i32 {
    resource.groove_gauge().map(|g| g.gauge_type()).unwrap_or(0)
}
