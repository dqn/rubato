// ModMenu overlay plugin — in-game settings overlay using egui.
//
// Provides trainer menus, song info, download status, and notifications
// rendered as an egui overlay on top of the Bevy game window.
// Corresponds to Java `modmenu/ImGuiRenderer.java` and related files.

pub mod main_window;
pub mod menus;
pub mod notify;

use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin};

use self::menus::{
    DownloadTaskState, EventTraceState, FreqTrainerState, GaugeVisualizerState, JudgeTrainerState,
    MiscSettingState, PerformanceMonitorState, ProfilerState, RandomTrainerState, SkinOptionsState,
    SkinWidgetManagerState, SongManagerState, TimerDisplayState, WindowSettingsState,
};
use self::notify::NotificationState;

/// Bevy plugin for the in-game ModMenu overlay.
///
/// Adds an egui-based overlay that can be toggled with the Delete key.
/// Provides trainer menus, misc settings, and toast notifications.
pub struct ModMenuPlugin;

impl Plugin for ModMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin)
            .init_resource::<ModMenuState>()
            .add_systems(Update, mod_menu_system);
    }
}

/// Central state resource for the ModMenu overlay.
///
/// All sub-menu visibility flags and trainer states are stored here
/// to avoid system parameter explosion.
#[derive(Resource, Default)]
pub struct ModMenuState {
    pub visible: bool,

    // Sub-window visibility flags
    pub show_freq_trainer: bool,
    pub show_random_trainer: bool,
    pub show_judge_trainer: bool,
    pub show_song_manager: bool,
    pub show_download_tasks: bool,
    pub show_misc_setting: bool,
    pub show_skin_widget_manager: bool,
    pub show_performance_monitor: bool,
    pub show_window_settings: bool,
    pub show_gauge_visualizer: bool,
    pub show_timer_display: bool,
    pub show_event_trace: bool,
    pub show_profiler: bool,
    pub show_skin_options: bool,

    // Trainer / menu states
    pub freq_trainer: FreqTrainerState,
    pub judge_trainer: JudgeTrainerState,
    pub random_trainer: RandomTrainerState,
    pub misc_setting: MiscSettingState,
    pub song_manager: SongManagerState,
    pub download_tasks: DownloadTaskState,
    pub skin_widget_manager: SkinWidgetManagerState,
    pub performance_monitor: PerformanceMonitorState,
    pub window_settings: WindowSettingsState,
    pub gauge_visualizer: GaugeVisualizerState,
    pub timer_display: TimerDisplayState,
    pub event_trace: EventTraceState,
    pub profiler: ProfilerState,
    pub skin_options: SkinOptionsState,
    pub notifications: NotificationState,

    // Input capture flag (set by render, read by game systems)
    pub wants_keyboard: bool,
    pub wants_pointer: bool,
}

impl ModMenuState {
    /// Toggle main menu visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

fn mod_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut contexts: EguiContexts,
    mut state: ResMut<ModMenuState>,
) {
    // Delete key always toggles the menu
    if keyboard.just_pressed(KeyCode::Delete) {
        state.toggle();
    }

    let ctx = contexts.ctx_mut();

    // Always render notifications (even when menu is hidden)
    notify::render_notifications(ctx, &mut state.notifications);

    if state.visible {
        main_window::render(ctx, &mut state);
    }

    // Update input capture flags after rendering
    state.wants_keyboard = ctx.wants_keyboard_input();
    state.wants_pointer = ctx.wants_pointer_input();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_hidden() {
        let state = ModMenuState::default();
        assert!(!state.visible);
    }

    #[test]
    fn toggle_works() {
        let mut state = ModMenuState::default();
        assert!(!state.visible);
        state.toggle();
        assert!(state.visible);
        state.toggle();
        assert!(!state.visible);
    }
}
