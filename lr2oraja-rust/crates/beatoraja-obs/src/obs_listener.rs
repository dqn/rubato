use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::warn;

use beatoraja_core::config::Config;
use beatoraja_core::main_state::{MainState, MainStateType};
use beatoraja_core::main_state_listener::MainStateListener;

use crate::obs_ws_client::ObsWsClient;
use crate::stubs::{ACTION_NONE, MainControllerRef, SCENE_NONE};

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

    pub fn get_obs_client(&self) -> Option<&Arc<ObsWsClient>> {
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
        let client_clone = Arc::clone(client);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            // Trigger PLAY state change after 1 second
            // NOTE: We cannot call self.trigger_state_change_by_type here since self is not Send.
            // In the Java code, this calls triggerStateChange(MainStateType.PLAY).
            // For the Rust translation, we inline the essential logic.
            if !client_clone.is_connected() {
                return;
            }
            // This is a simplified version — the full version would need config access
            // to look up scene/action for PLAY state.
            log::warn!(
                "not yet implemented: delayed triggerStateChange(PLAY) needs config access in async context"
            );
        });
    }

    fn cancel_scheduled_stop(&self) -> bool {
        let mut guard = self.scheduled_stop_task.lock().unwrap();
        if let Some(task) = guard.take() {
            task.abort();
            return true;
        }
        false
    }

    pub fn trigger_play_ended(&self) {
        self.trigger_state_change("PLAY_ENDED");
    }

    pub fn trigger_state_change_by_type(&self, state_type: MainStateType) {
        let name = match state_type {
            MainStateType::MusicSelect => "MUSICSELECT",
            MainStateType::Decide => "DECIDE",
            MainStateType::Play => "PLAY",
            MainStateType::Result => "RESULT",
            MainStateType::CourseResult => "COURSERESULT",
            MainStateType::Config => "CONFIG",
            MainStateType::SkinConfig => "SKINCONFIG",
        };
        self.trigger_state_change(name);
    }

    pub fn trigger_state_change(&self, state_name: &str) {
        let client = match &self.obs_client {
            Some(c) => c,
            None => return,
        };
        if !client.is_connected() {
            return;
        }

        let scene = self.config.get_obs_scene(state_name).cloned();
        let action = self.config.get_obs_action(state_name).cloned();

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
                let delay = self.config.obs_ws_rec_stop_wait;
                // We already executed StopRecord above
                if stop_record_now {
                    return;
                }
                let client_clone = Arc::clone(client);
                let scheduled_stop_task = Arc::clone(&self.scheduled_stop_task);
                let handle = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                    client_clone.request_stop_record();
                    // Clear the task handle
                    let mut guard = scheduled_stop_task.lock().unwrap();
                    *guard = None;
                });
                let mut guard = self.scheduled_stop_task.lock().unwrap();
                *guard = Some(handle);
            } else {
                client.send_request(action);
            }
        }
    }

    pub fn close(&self) {
        // Cancel scheduled stop task
        {
            let mut guard = self.scheduled_stop_task.lock().unwrap();
            if let Some(task) = guard.take() {
                task.abort();
            }
        }

        if let Some(ref client) = self.obs_client {
            client.close();
        }
    }
}

impl MainStateListener for ObsListener {
    fn update(&mut self, current_state: &dyn MainState, _status: i32) {
        if self.obs_client.is_none() {
            return;
        }

        let current_state_type = MainControllerRef::get_state_type(current_state);
        let current_state_type = match current_state_type {
            Some(t) => t,
            None => return,
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
