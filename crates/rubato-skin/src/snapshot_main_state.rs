use std::collections::HashMap;

use rubato_core::bms_player_mode::BMSPlayerMode;
use rubato_types::property_snapshot::PropertySnapshot;
use rubato_types::skin_render_context::SkinRenderContext;
use rubato_types::timer_access::TimerAccess;
use rubato_types::timer_id::TimerId;

use crate::render_reexports::TextureRegion;

/// Thin wrapper around `PropertySnapshot` that implements `MainState`.
///
/// `PropertySnapshot` (in rubato-types) already implements `SkinRenderContext`
/// and `TimerAccess`. This wrapper adds the two skin-crate-local methods that
/// `MainState` requires:
///
/// - `skin_image(id)` -- looks up system-defined images from an image registry
/// - `select_song(mode)` -- queues song selection requests for later draining
///
/// ## Usage
///
/// ```ignore
/// let mut snapshot = build_property_snapshot(&self);
/// let mut state = SnapshotMainState::new(&mut snapshot, &image_registry);
/// skin.draw_all_objects(&state);
/// // Drain queued actions
/// let actions = std::mem::take(&mut state.snapshot.actions);
/// let song_selections = state.take_select_song_requests();
/// apply_actions(actions, song_selections);
/// ```
pub struct SnapshotMainState<'a> {
    /// The underlying property snapshot (all SkinRenderContext methods delegate here).
    pub snapshot: &'a mut PropertySnapshot,
    /// Image registry for `skin_image()` lookups.
    image_registry: &'a HashMap<i32, TextureRegion>,
    /// Queued `select_song` requests (collected during skin event execution).
    select_song_requests: Vec<BMSPlayerMode>,
}

impl<'a> SnapshotMainState<'a> {
    pub fn new(
        snapshot: &'a mut PropertySnapshot,
        image_registry: &'a HashMap<i32, TextureRegion>,
    ) -> Self {
        Self {
            snapshot,
            image_registry,
            select_song_requests: Vec::new(),
        }
    }

    /// Takes and returns all queued select_song requests, leaving the internal
    /// buffer empty.
    pub fn take_select_song_requests(&mut self) -> Vec<BMSPlayerMode> {
        std::mem::take(&mut self.select_song_requests)
    }
}

// ================================================================
// TimerAccess -- delegate to snapshot
// ================================================================
impl TimerAccess for SnapshotMainState<'_> {
    fn now_time(&self) -> i64 {
        self.snapshot.now_time()
    }

    fn now_micro_time(&self) -> i64 {
        self.snapshot.now_micro_time()
    }

    fn micro_timer(&self, timer_id: TimerId) -> i64 {
        self.snapshot.micro_timer(timer_id)
    }

    fn timer(&self, timer_id: TimerId) -> i64 {
        self.snapshot.timer(timer_id)
    }

    fn now_time_for(&self, timer_id: TimerId) -> i64 {
        self.snapshot.now_time_for(timer_id)
    }

    fn is_timer_on(&self, timer_id: TimerId) -> bool {
        self.snapshot.is_timer_on(timer_id)
    }
}

// ================================================================
// SkinRenderContext -- delegate to snapshot
// ================================================================
impl SkinRenderContext for SnapshotMainState<'_> {
    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        self.snapshot.execute_event(id, arg1, arg2);
    }

    fn change_state(&mut self, state: rubato_types::main_state_type::MainStateType) {
        self.snapshot.change_state(state);
    }

    fn set_timer_micro(&mut self, timer_id: TimerId, micro_time: i64) {
        self.snapshot.set_timer_micro(timer_id, micro_time);
    }

    fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
        self.snapshot.audio_play(path, volume, is_loop);
    }

    fn audio_stop(&mut self, path: &str) {
        self.snapshot.audio_stop(path);
    }

    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        self.snapshot.current_state_type()
    }

    fn recent_judges(&self) -> &[i64] {
        self.snapshot.recent_judges()
    }

    fn recent_judges_index(&self) -> usize {
        self.snapshot.recent_judges_index()
    }

    fn boot_time_millis(&self) -> i64 {
        self.snapshot.boot_time_millis()
    }

    fn integer_value(&self, id: i32) -> i32 {
        self.snapshot.integer_value(id)
    }

    fn image_index_value(&self, id: i32) -> i32 {
        self.snapshot.image_index_value(id)
    }

    fn boolean_value(&self, id: i32) -> bool {
        self.snapshot.boolean_value(id)
    }

    fn float_value(&self, id: i32) -> f32 {
        self.snapshot.float_value(id)
    }

    fn string_value(&self, id: i32) -> String {
        self.snapshot.string_value(id)
    }

    fn set_float_value(&mut self, id: i32, value: f32) {
        self.snapshot.set_float_value(id, value);
    }

    fn replay_option_data(&self) -> Option<&rubato_types::replay_data::ReplayData> {
        self.snapshot.replay_option_data()
    }

    fn target_score_data(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.snapshot.target_score_data()
    }

    fn score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.snapshot.score_data_ref()
    }

    fn rival_score_data_ref(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.snapshot.rival_score_data_ref()
    }

    fn ranking_score_clear_type(&self, slot: i32) -> i32 {
        self.snapshot.ranking_score_clear_type(slot)
    }

    fn ranking_offset(&self) -> i32 {
        self.snapshot.ranking_offset()
    }

    fn current_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        self.snapshot.current_play_config_ref()
    }

    fn song_data_ref(&self) -> Option<&rubato_types::song_data::SongData> {
        self.snapshot.song_data_ref()
    }

    fn lane_shuffle_pattern_value(&self, player: usize, lane: usize) -> i32 {
        self.snapshot.lane_shuffle_pattern_value(player, lane)
    }

    fn mode_image_index(&self) -> Option<i32> {
        self.snapshot.mode_image_index()
    }

    fn sort_image_index(&self) -> Option<i32> {
        self.snapshot.sort_image_index()
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        self.snapshot.judge_count(judge, fast)
    }

    fn gauge_value(&self) -> f32 {
        self.snapshot.gauge_value()
    }

    fn gauge_type(&self) -> i32 {
        self.snapshot.gauge_type()
    }

    fn is_mode_changed(&self) -> bool {
        self.snapshot.is_mode_changed()
    }

    fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
        self.snapshot.gauge_element_borders()
    }

    fn now_judge(&self, player: i32) -> i32 {
        self.snapshot.now_judge(player)
    }

    fn now_combo(&self, player: i32) -> i32 {
        self.snapshot.now_combo(player)
    }

    fn player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        self.snapshot.player_config_ref()
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        self.snapshot.player_config_mut()
    }

    fn config_ref(&self) -> Option<&rubato_types::config::Config> {
        self.snapshot.config_ref()
    }

    fn config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        self.snapshot.config_mut()
    }

    fn selected_play_config_mut(&mut self) -> Option<&mut rubato_types::play_config::PlayConfig> {
        self.snapshot.selected_play_config_mut()
    }

    fn notify_audio_config_changed(&mut self) {
        self.snapshot.notify_audio_config_changed();
    }

    fn play_option_change_sound(&mut self) {
        self.snapshot.play_option_change_sound();
    }

    fn update_bar_after_change(&mut self) {
        self.snapshot.update_bar_after_change();
    }

    fn select_song_mode(&mut self, event_id: i32) {
        self.snapshot.select_song_mode(event_id);
    }

    fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.snapshot.get_offset_value(id)
    }

    fn mouse_x(&self) -> f32 {
        self.snapshot.mouse_x()
    }

    fn mouse_y(&self) -> f32 {
        self.snapshot.mouse_y()
    }

    fn is_debug(&self) -> bool {
        self.snapshot.is_debug()
    }

    fn get_timing_distribution(
        &self,
    ) -> Option<&rubato_types::timing_distribution::TimingDistribution> {
        self.snapshot.get_timing_distribution()
    }

    fn judge_area(&self) -> Option<Vec<Vec<i32>>> {
        self.snapshot.judge_area()
    }

    fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
        self.snapshot.score_data_property()
    }

    fn gauge_history(&self) -> Option<&Vec<Vec<f32>>> {
        self.snapshot.gauge_history()
    }

    fn course_gauge_history(&self) -> &[Vec<Vec<f32>>] {
        self.snapshot.course_gauge_history()
    }

    fn gauge_border_max(&self) -> Option<(f32, f32)> {
        self.snapshot.gauge_border_max()
    }

    fn gauge_min(&self) -> f32 {
        self.snapshot.gauge_min()
    }

    fn gauge_transition_last_value(&self, gauge_type: i32) -> Option<f32> {
        self.snapshot.gauge_transition_last_value(gauge_type)
    }

    fn result_gauge_type(&self) -> i32 {
        self.snapshot.result_gauge_type()
    }

    fn is_gauge_max(&self) -> bool {
        self.snapshot.is_gauge_max()
    }

    fn is_media_load_finished(&self) -> bool {
        self.snapshot.is_media_load_finished()
    }

    fn is_practice_mode(&self) -> bool {
        self.snapshot.is_practice_mode()
    }

    fn get_distribution_data(&self) -> Option<rubato_types::distribution_data::DistributionData> {
        self.snapshot.get_distribution_data()
    }
}

// ================================================================
// MainState -- adds skin_image + select_song on top of snapshot
// ================================================================
impl crate::main_state::MainState for SnapshotMainState<'_> {
    fn skin_image(&self, id: i32) -> Option<TextureRegion> {
        self.image_registry.get(&id).cloned()
    }

    fn select_song(&mut self, mode: BMSPlayerMode) {
        self.select_song_requests.push(mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::main_state_type::MainStateType;

    #[test]
    fn skin_image_returns_none_when_not_registered() {
        let mut snapshot = PropertySnapshot::new();
        let registry = HashMap::new();
        let state = SnapshotMainState::new(&mut snapshot, &registry);

        assert!(crate::main_state::MainState::skin_image(&state, 100).is_none());
    }

    #[test]
    fn skin_image_returns_registered_texture() {
        let mut snapshot = PropertySnapshot::new();
        let mut registry = HashMap::new();
        let texture = TextureRegion::default();
        registry.insert(42, texture.clone());

        let state = SnapshotMainState::new(&mut snapshot, &registry);
        let result = crate::main_state::MainState::skin_image(&state, 42);
        assert!(result.is_some());
    }

    #[test]
    fn select_song_queues_requests() {
        let mut snapshot = PropertySnapshot::new();
        let registry = HashMap::new();
        let mut state = SnapshotMainState::new(&mut snapshot, &registry);

        crate::main_state::MainState::select_song(&mut state, BMSPlayerMode::PLAY);
        crate::main_state::MainState::select_song(&mut state, BMSPlayerMode::AUTOPLAY);

        let requests = state.take_select_song_requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], BMSPlayerMode::PLAY);
        assert_eq!(requests[1], BMSPlayerMode::AUTOPLAY);
    }

    #[test]
    fn take_select_song_requests_clears_buffer() {
        let mut snapshot = PropertySnapshot::new();
        let registry = HashMap::new();
        let mut state = SnapshotMainState::new(&mut snapshot, &registry);

        crate::main_state::MainState::select_song(&mut state, BMSPlayerMode::PRACTICE);
        let _ = state.take_select_song_requests();

        let requests = state.take_select_song_requests();
        assert!(requests.is_empty());
    }

    #[test]
    fn delegates_skin_render_context_to_snapshot() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.state_type = Some(MainStateType::Play);
        snapshot.now_time = 5000;
        snapshot.gauge_value = 0.75;
        snapshot.integers.insert(100, 42);

        let registry = HashMap::new();
        let state = SnapshotMainState::new(&mut snapshot, &registry);

        assert_eq!(state.current_state_type(), Some(MainStateType::Play));
        assert_eq!(state.now_time(), 5000);
        assert_eq!(state.gauge_value(), 0.75);
        assert_eq!(state.integer_value(100), 42);
        assert!(state.is_bms_player());
    }

    #[test]
    fn write_actions_flow_through_to_snapshot() {
        let mut snapshot = PropertySnapshot::new();
        let registry = HashMap::new();
        let mut state = SnapshotMainState::new(&mut snapshot, &registry);

        state.set_float_value(1, 0.5);
        state.change_state(MainStateType::Result);
        state.execute_event(100, 1, 2);

        assert_eq!(state.snapshot.actions.float_writes, vec![(1, 0.5)]);
        assert_eq!(
            state.snapshot.actions.state_changes,
            vec![MainStateType::Result]
        );
        assert_eq!(state.snapshot.actions.custom_events, vec![(100, 1, 2)]);
    }
}
