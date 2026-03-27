use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::warn;

use crate::core::config::Config;
use crate::core::main_state::MainStateType;
use rubato_types::app_event::{AppEvent, StateChangedData};
use rubato_types::screen_type::ScreenType;

use super::lock_or_recover;
use super::obs_ws_client::ObsWsClient;
use super::{ACTION_NONE, SCENE_NONE};

/// ObsListener - scene/recording control via OBS WebSocket.
///
/// Receives `AppEvent::StateChanged` events via a channel and triggers
/// OBS scene changes and recording actions accordingly.
pub struct ObsListener {
    config: Config,
    obs_client: Option<Arc<ObsWsClient>>,
    /// Scheduled stop task handle -- holds a JoinHandle for cancellation
    scheduled_stop_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Bridge thread that reads AppEvent and calls OBS methods.
    bridge_thread: Option<std::thread::JoinHandle<()>>,
}

impl ObsListener {
    /// Create a new ObsListener and return `(app_event_sender, listener)`.
    ///
    /// The caller should register `app_event_sender` with `MainController::add_event_sender()`.
    /// The listener must be kept alive (not dropped) for the background thread to run.
    pub fn new(config: Config) -> (std::sync::mpsc::SyncSender<AppEvent>, Self) {
        let client = match ObsWsClient::new(&config) {
            Ok(client) => {
                let client = Arc::new(client);
                client.connect_async();
                Some(client)
            }
            Err(e) => {
                warn!("Failed to initialize OBS client: {}", e);
                None
            }
        };

        // AppEvent channel: MainController -> bridge thread
        let (app_tx, app_rx) = std::sync::mpsc::sync_channel::<AppEvent>(256);

        // Clone state for the bridge thread
        let config_clone = config.clone();
        let client_clone = client.clone();
        let scheduled_stop_task = Arc::new(Mutex::new(None));
        let scheduled_stop_clone = Arc::clone(&scheduled_stop_task);

        let bridge_handle = std::thread::Builder::new()
            .name("obs-bridge".to_string())
            .spawn(move || {
                Self::bridge_loop(app_rx, config_clone, client_clone, scheduled_stop_clone);
            })
            .ok();

        (
            app_tx,
            Self {
                config,
                obs_client: client,
                scheduled_stop_task,
                bridge_thread: bridge_handle,
            },
        )
    }

    /// Bridge thread: reads `AppEvent`s and handles OBS state changes.
    fn bridge_loop(
        rx: std::sync::mpsc::Receiver<AppEvent>,
        config: Config,
        obs_client: Option<Arc<ObsWsClient>>,
        scheduled_stop_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ) {
        let mut last_state_type: Option<MainStateType> = None;

        loop {
            match rx.recv() {
                Ok(AppEvent::StateChanged(data)) => {
                    Self::handle_state_changed(
                        &data,
                        &config,
                        &obs_client,
                        &scheduled_stop_task,
                        &mut last_state_type,
                    );
                }
                Ok(AppEvent::Lifecycle(_)) => {
                    // Lifecycle events are not relevant for OBS.
                }
                Err(_) => {
                    // Channel disconnected; clean up and exit.
                    Self::close_impl(&scheduled_stop_task, &obs_client);
                    break;
                }
            }
        }
    }

    /// Handle a `StateChanged` event by triggering appropriate OBS actions.
    fn handle_state_changed(
        data: &StateChangedData,
        config: &Config,
        obs_client: &Option<Arc<ObsWsClient>>,
        scheduled_stop_task: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        last_state_type: &mut Option<MainStateType>,
    ) {
        if obs_client.is_none() {
            return;
        }

        // Use state_type from the event directly (avoids ScreenType->MainStateType roundtrip)
        let current_state_type = match data.state_type {
            Some(st) => st,
            None => return,
        };

        // SkinConfig maps to ScreenType::Other, which we skip
        if data.screen_type == ScreenType::Other {
            return;
        }

        if current_state_type == MainStateType::Play
            && *last_state_type == Some(MainStateType::Play)
        {
            Self::trigger_replay_static(config, obs_client, scheduled_stop_task);
        } else if Some(current_state_type) != *last_state_type {
            Self::trigger_state_change_by_type_static(
                current_state_type,
                config,
                obs_client,
                scheduled_stop_task,
            );
        }

        *last_state_type = Some(current_state_type);
    }

    pub fn obs_client(&self) -> Option<&Arc<ObsWsClient>> {
        self.obs_client.as_ref()
    }

    fn trigger_replay_static(
        config: &Config,
        obs_client: &Option<Arc<ObsWsClient>>,
        scheduled_stop_task: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ) {
        let client = match obs_client {
            Some(c) => c,
            None => return,
        };
        if !client.is_connected() {
            return;
        }
        if client.is_recording() {
            client.restart_recording();
        }
        Self::trigger_state_change_static(
            MainStateType::MusicSelect.obs_key(),
            config,
            obs_client,
            scheduled_stop_task,
        );
        let Some(runtime_handle) = client.runtime_handle() else {
            return;
        };
        let runtime_handle = runtime_handle.clone();
        let client_clone = Arc::clone(client);

        let play_scene = config.obs_scene("PLAY").cloned();
        let play_action = config.obs_action("PLAY").cloned();
        let stop_wait = config.obs.obs_ws_rec_stop_wait;
        let scheduled_stop_task = Arc::clone(scheduled_stop_task);

        runtime_handle.spawn(async move {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            if !client_clone.is_connected() {
                return;
            }

            {
                let mut guard = lock_or_recover(&scheduled_stop_task);
                if let Some(task) = guard.take() {
                    if !task.is_finished() {
                        task.abort();
                    }
                    client_clone.request_stop_record();
                }
            }

            if let Some(ref scene) = play_scene
                && scene != SCENE_NONE
            {
                client_clone.set_scene(scene);
            }

            if let Some(ref action) = play_action
                && action != ACTION_NONE
            {
                if action == "StopRecord" {
                    let client_for_stop = Arc::clone(&client_clone);
                    let scheduled_stop_task_clone = Arc::clone(&scheduled_stop_task);
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(stop_wait.max(0) as u64)).await;
                        client_for_stop.request_stop_record();
                        let mut guard = lock_or_recover(&scheduled_stop_task_clone);
                        *guard = None;
                    });
                    let mut guard = lock_or_recover(&scheduled_stop_task);
                    *guard = Some(handle);
                } else {
                    client_clone.send_request(action);
                }
            }
        });
    }

    fn cancel_scheduled_stop_impl(
        scheduled_stop_task: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ) -> bool {
        let mut guard = lock_or_recover(scheduled_stop_task);
        if let Some(task) = guard.take() {
            if !task.is_finished() {
                task.abort();
            }
            return true;
        }
        false
    }

    #[cfg(test)]
    fn cancel_scheduled_stop(&self) -> bool {
        Self::cancel_scheduled_stop_impl(&self.scheduled_stop_task)
    }

    pub fn trigger_play_ended(&self) {
        self.trigger_state_change("PLAY_ENDED");
    }

    pub fn trigger_state_change_by_type(&self, state_type: MainStateType) {
        Self::trigger_state_change_by_type_static(
            state_type,
            &self.config,
            &self.obs_client,
            &self.scheduled_stop_task,
        );
    }

    fn trigger_state_change_by_type_static(
        state_type: MainStateType,
        config: &Config,
        obs_client: &Option<Arc<ObsWsClient>>,
        scheduled_stop_task: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ) {
        Self::trigger_state_change_static(
            state_type.obs_key(),
            config,
            obs_client,
            scheduled_stop_task,
        );
    }

    pub fn trigger_state_change(&self, state_name: &str) {
        Self::trigger_state_change_static(
            state_name,
            &self.config,
            &self.obs_client,
            &self.scheduled_stop_task,
        );
    }

    fn trigger_state_change_static(
        state_name: &str,
        config: &Config,
        obs_client: &Option<Arc<ObsWsClient>>,
        scheduled_stop_task: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    ) {
        let client = match obs_client {
            Some(c) => c,
            None => return,
        };
        if !client.is_connected() {
            return;
        }

        let scene = config.obs_scene(state_name).cloned();
        let action = config.obs_action(state_name).cloned();

        let stop_record_now = Self::cancel_scheduled_stop_impl(scheduled_stop_task);
        if stop_record_now {
            client.request_stop_record();
        }

        if let Some(ref scene) = scene
            && scene != SCENE_NONE
        {
            client.set_scene(scene);
        }

        if let Some(ref action) = action
            && action != ACTION_NONE
        {
            if action == "StopRecord" {
                let delay = config.obs.obs_ws_rec_stop_wait;
                if stop_record_now {
                    return;
                }
                let Some(runtime_handle) = client.runtime_handle() else {
                    return;
                };
                let runtime_handle = runtime_handle.clone();
                let client_clone = Arc::clone(client);
                let stop_task_clone = Arc::clone(scheduled_stop_task);
                let handle = runtime_handle.spawn(async move {
                    tokio::time::sleep(Duration::from_millis(delay.max(0) as u64)).await;
                    client_clone.request_stop_record();
                    let mut guard = lock_or_recover(&stop_task_clone);
                    *guard = None;
                });
                let mut guard = lock_or_recover(scheduled_stop_task);
                *guard = Some(handle);
            } else {
                client.send_request(action);
            }
        }
    }

    fn close_impl(
        scheduled_stop_task: &Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
        obs_client: &Option<Arc<ObsWsClient>>,
    ) {
        {
            let mut guard = lock_or_recover(scheduled_stop_task);
            if let Some(task) = guard.take()
                && !task.is_finished()
            {
                task.abort();
            }
        }

        if let Some(client) = obs_client {
            client.close();
        }
    }

    pub fn close(&mut self) {
        Self::close_impl(&self.scheduled_stop_task, &self.obs_client);
        if let Some(handle) = self.bridge_thread.take()
            && let Err(e) = handle.join()
        {
            log::warn!("OBS bridge thread panicked: {:?}", e);
        }
    }

    /// Test-only constructor that doesn't try to connect to OBS.
    #[cfg(test)]
    fn new_without_client(config: Config) -> Self {
        Self {
            config,
            obs_client: None,
            scheduled_stop_task: Arc::new(Mutex::new(None)),
            bridge_thread: None,
        }
    }
}

impl Drop for ObsListener {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- cancel_scheduled_stop --

    #[tokio::test]
    async fn cancel_scheduled_stop_returns_false_when_empty() {
        let listener = ObsListener::new_without_client(Config::default());
        assert!(!listener.cancel_scheduled_stop());
    }

    #[tokio::test]
    async fn cancel_scheduled_stop_returns_true_and_aborts_task() {
        let listener = ObsListener::new_without_client(Config::default());

        // Spawn a long-running task and store its handle
        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        {
            let mut guard = lock_or_recover(&listener.scheduled_stop_task);
            *guard = Some(handle);
        }

        // cancel_scheduled_stop should return true and abort the task
        assert!(listener.cancel_scheduled_stop());

        // After cancellation, the slot should be empty
        let guard = lock_or_recover(&listener.scheduled_stop_task);
        assert!(guard.is_none());
    }

    #[tokio::test]
    async fn cancel_scheduled_stop_is_idempotent() {
        let listener = ObsListener::new_without_client(Config::default());

        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        {
            let mut guard = lock_or_recover(&listener.scheduled_stop_task);
            *guard = Some(handle);
        }

        // First cancel returns true
        assert!(listener.cancel_scheduled_stop());
        // Second cancel returns false (already consumed)
        assert!(!listener.cancel_scheduled_stop());
    }

    // -- close cancels scheduled stop --

    #[tokio::test]
    async fn close_cancels_scheduled_stop_task() {
        let mut listener = ObsListener::new_without_client(Config::default());

        let handle = tokio::spawn(async {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        });
        assert!(!handle.is_finished());
        {
            let mut guard = lock_or_recover(&listener.scheduled_stop_task);
            *guard = Some(handle);
        }

        listener.close();

        // After close, the slot should be empty
        let guard = lock_or_recover(&listener.scheduled_stop_task);
        assert!(guard.is_none());
    }

    // -- trigger_state_change with no client is a no-op --

    #[test]
    fn trigger_state_change_without_client_does_not_panic() {
        let listener = ObsListener::new_without_client(Config::default());
        // Should return early without panic when obs_client is None
        listener.trigger_state_change("PLAY");
        listener.trigger_state_change_by_type(MainStateType::Play);
        listener.trigger_play_ended();
    }

    // -- new_without_client initial state --

    #[test]
    fn new_without_client_has_no_obs_client() {
        let listener = ObsListener::new_without_client(Config::default());
        assert!(listener.obs_client().is_none());
    }

    // -- trigger_state_change_by_type name mapping (exhaustive) --

    #[test]
    fn trigger_state_change_by_type_maps_all_variants() {
        let listener = ObsListener::new_without_client(Config::default());
        let variants = [
            MainStateType::MusicSelect,
            MainStateType::Decide,
            MainStateType::Play,
            MainStateType::Result,
            MainStateType::CourseResult,
            MainStateType::Config,
            MainStateType::SkinConfig,
        ];
        for variant in variants {
            listener.trigger_state_change_by_type(variant);
        }
    }

    // -- SCENE_NONE / ACTION_NONE constants --

    /// Regression: cancel_scheduled_stop must not abort an already-finished task,
    /// which would cause a spurious request_stop_record call at the call site.
    #[tokio::test]
    async fn cancel_scheduled_stop_skips_abort_on_finished_task() {
        let listener = ObsListener::new_without_client(Config::default());

        // Spawn a task that completes immediately
        let handle = tokio::spawn(async {});
        // Wait for it to finish
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(handle.is_finished(), "task should have completed");

        {
            let mut guard = lock_or_recover(&listener.scheduled_stop_task);
            *guard = Some(handle);
        }

        assert!(listener.cancel_scheduled_stop());

        let guard = lock_or_recover(&listener.scheduled_stop_task);
        assert!(guard.is_none());
    }

    /// Regression: close() must not abort an already-finished task.
    #[tokio::test]
    async fn close_skips_abort_on_finished_task() {
        let mut listener = ObsListener::new_without_client(Config::default());

        let handle = tokio::spawn(async {});
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(handle.is_finished());

        {
            let mut guard = lock_or_recover(&listener.scheduled_stop_task);
            *guard = Some(handle);
        }

        listener.close();

        let guard = lock_or_recover(&listener.scheduled_stop_task);
        assert!(guard.is_none());
    }

    #[test]
    fn scene_none_and_action_none_are_expected_values() {
        assert_eq!(SCENE_NONE, "(No Change)");
        assert_eq!(ACTION_NONE, "(Do Nothing)");
    }

    #[test]
    fn handle_state_changed_skips_when_no_client() {
        let scheduled = Arc::new(Mutex::new(None));
        let mut last_state: Option<MainStateType> = None;
        let data = StateChangedData {
            screen_type: ScreenType::BMSPlayer,
            state_type: Some(MainStateType::Play),
            status: 0,
            song_info: None,
        };
        // Should not panic with obs_client=None
        ObsListener::handle_state_changed(
            &data,
            &Config::default(),
            &None,
            &scheduled,
            &mut last_state,
        );
        // last_state not updated because obs_client is None
        assert!(last_state.is_none());
    }

    #[test]
    fn handle_state_changed_skips_screen_type_other() {
        let scheduled = Arc::new(Mutex::new(None));
        let mut last_state: Option<MainStateType> = None;
        let data = StateChangedData {
            screen_type: ScreenType::Other,
            state_type: Some(MainStateType::SkinConfig),
            status: 0,
            song_info: None,
        };
        ObsListener::handle_state_changed(
            &data,
            &Config::default(),
            &None,
            &scheduled,
            &mut last_state,
        );
        assert!(last_state.is_none());
    }
}
