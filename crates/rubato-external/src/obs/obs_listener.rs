use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::warn;

use rubato_core::config::Config;
use rubato_core::main_state::MainStateType;
use rubato_core::main_state_listener::MainStateListener;
use rubato_types::main_state_access::MainStateAccess;
use rubato_types::screen_type::ScreenType;

use super::lock_or_recover;
use super::obs_ws_client::ObsWsClient;
use super::{ACTION_NONE, SCENE_NONE};

/// ObsListener - implements MainStateListener for scene/recording control via OBS WebSocket
pub struct ObsListener {
    config: Config,
    obs_client: Option<Arc<ObsWsClient>>,
    last_state_type: Option<MainStateType>,
    /// Scheduled stop task handle — holds a JoinHandle for cancellation
    scheduled_stop_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ObsListener {
    pub fn new(config: Config) -> Self {
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

        Self {
            config,
            obs_client: client,
            last_state_type: None,
            scheduled_stop_task: Arc::new(Mutex::new(None)),
        }
    }

    pub fn obs_client(&self) -> Option<&Arc<ObsWsClient>> {
        self.obs_client.as_ref()
    }

    fn trigger_replay(&self) {
        let client = match &self.obs_client {
            Some(c) => c,
            None => return,
        };
        if !client.is_connected() {
            return;
        }
        if client.is_recording() {
            client.restart_recording();
        }
        self.trigger_state_change_by_type(MainStateType::MusicSelect);
        let runtime_handle = client.runtime_handle().clone();
        let client_clone = Arc::clone(client);

        // Capture config values for PLAY state before entering async block,
        // since self (ObsListener) is not Send.
        let play_scene = self.config.obs_scene("PLAY").cloned();
        let play_action = self.config.obs_action("PLAY").cloned();
        let stop_wait = self.config.obs.obs_ws_rec_stop_wait;
        let scheduled_stop_task = Arc::clone(&self.scheduled_stop_task);

        runtime_handle.spawn(async move {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            // Trigger PLAY state change after 1 second.
            // Inlined from trigger_state_change since self is not available in async context.
            if !client_clone.is_connected() {
                return;
            }

            // Cancel any scheduled stop and execute immediately if pending
            {
                let mut guard = lock_or_recover(&scheduled_stop_task);
                if let Some(task) = guard.take() {
                    if !task.is_finished() {
                        task.abort();
                    }
                    client_clone.request_stop_record();
                }
            }

            // Set scene if configured
            if let Some(ref scene) = play_scene
                && scene != SCENE_NONE
            {
                client_clone.set_scene(scene);
            }

            // Execute action if configured
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

    fn cancel_scheduled_stop(&self) -> bool {
        let mut guard = lock_or_recover(&self.scheduled_stop_task);
        if let Some(task) = guard.take() {
            if !task.is_finished() {
                task.abort();
            }
            return true;
        }
        false
    }

    pub fn trigger_play_ended(&self) {
        self.trigger_state_change("PLAY_ENDED");
    }

    pub fn trigger_state_change_by_type(&self, state_type: MainStateType) {
        self.trigger_state_change(state_type.obs_key());
    }

    pub fn trigger_state_change(&self, state_name: &str) {
        let client = match &self.obs_client {
            Some(c) => c,
            None => return,
        };
        if !client.is_connected() {
            return;
        }

        let scene = self.config.obs_scene(state_name).cloned();
        let action = self.config.obs_action(state_name).cloned();

        // If a StopRecord action was already scheduled, StopRecord immediately
        let stop_record_now = self.cancel_scheduled_stop();
        if stop_record_now {
            client.request_stop_record();
        }

        // Set scene if configured
        if let Some(ref scene) = scene
            && scene != SCENE_NONE
        {
            client.set_scene(scene);
        }

        // Execute action if configured
        if let Some(ref action) = action
            && action != ACTION_NONE
        {
            if action == "StopRecord" {
                let delay = self.config.obs.obs_ws_rec_stop_wait;
                // We already executed StopRecord above
                if stop_record_now {
                    return;
                }
                let runtime_handle = client.runtime_handle().clone();
                let client_clone = Arc::clone(client);
                let scheduled_stop_task = Arc::clone(&self.scheduled_stop_task);
                let handle = runtime_handle.spawn(async move {
                    tokio::time::sleep(Duration::from_millis(delay.max(0) as u64)).await;
                    client_clone.request_stop_record();
                    // Clear the task handle
                    let mut guard = lock_or_recover(&scheduled_stop_task);
                    *guard = None;
                });
                let mut guard = lock_or_recover(&self.scheduled_stop_task);
                *guard = Some(handle);
            } else {
                client.send_request(action);
            }
        }
    }

    pub fn close(&self) {
        // Cancel scheduled stop task
        {
            let mut guard = lock_or_recover(&self.scheduled_stop_task);
            if let Some(task) = guard.take()
                && !task.is_finished()
            {
                task.abort();
            }
        }

        if let Some(ref client) = self.obs_client {
            client.close();
        }
    }

    /// Test-only constructor that doesn't try to connect to OBS.
    #[cfg(test)]
    fn new_without_client(config: Config) -> Self {
        Self {
            config,
            obs_client: None,
            last_state_type: None,
            scheduled_stop_task: Arc::new(Mutex::new(None)),
        }
    }
}

impl Drop for ObsListener {
    fn drop(&mut self) {
        self.close();
    }
}

impl MainStateListener for ObsListener {
    fn update(&mut self, current_state: &dyn MainStateAccess, _status: i32) {
        if self.obs_client.is_none() {
            return;
        }

        let screen_type = current_state.screen_type();

        // Convert ScreenType back to MainStateType for internal tracking
        let current_state_type = match screen_type {
            ScreenType::MusicSelector => MainStateType::MusicSelect,
            ScreenType::MusicDecide => MainStateType::Decide,
            ScreenType::BMSPlayer => MainStateType::Play,
            ScreenType::MusicResult => MainStateType::Result,
            ScreenType::CourseResult => MainStateType::CourseResult,
            ScreenType::KeyConfiguration => MainStateType::Config,
            ScreenType::Other => return,
        };

        if current_state_type == MainStateType::Play
            && self.last_state_type == Some(MainStateType::Play)
        {
            self.trigger_replay();
        } else if Some(current_state_type) != self.last_state_type {
            self.trigger_state_change_by_type(current_state_type);
        }

        self.last_state_type = Some(current_state_type);
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
        let listener = ObsListener::new_without_client(Config::default());

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
        assert!(listener.last_state_type.is_none());
    }

    // -- trigger_state_change_by_type name mapping (exhaustive) --

    #[test]
    fn trigger_state_change_by_type_maps_all_variants() {
        // Verify the mapping is exhaustive by calling each variant.
        // Since obs_client is None, trigger_state_change returns early,
        // but the match in trigger_state_change_by_type still executes.
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
            // Should not panic for any variant
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

        // cancel_scheduled_stop returns true (slot was occupied) but does NOT
        // call abort on the finished handle
        assert!(listener.cancel_scheduled_stop());

        // Slot is cleared
        let guard = lock_or_recover(&listener.scheduled_stop_task);
        assert!(guard.is_none());
    }

    /// Regression: close() must not abort an already-finished task.
    #[tokio::test]
    async fn close_skips_abort_on_finished_task() {
        let listener = ObsListener::new_without_client(Config::default());

        let handle = tokio::spawn(async {});
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(handle.is_finished());

        {
            let mut guard = lock_or_recover(&listener.scheduled_stop_task);
            *guard = Some(handle);
        }

        // Should not panic when encountering a finished task
        listener.close();

        let guard = lock_or_recover(&listener.scheduled_stop_task);
        assert!(guard.is_none());
    }

    #[test]
    fn scene_none_and_action_none_are_expected_values() {
        assert_eq!(SCENE_NONE, "(No Change)");
        assert_eq!(ACTION_NONE, "(Do Nothing)");
    }
}
