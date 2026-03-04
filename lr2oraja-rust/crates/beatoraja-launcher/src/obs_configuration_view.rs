// Translated from ObsConfigurationView.java

use std::collections::HashMap;

use beatoraja_core::config::Config;
use beatoraja_core::main_state::MainStateType;
use beatoraja_external::obs::obs_ws_client::{
    ObsVersionInfo, ObsWsClient, get_action_label, obs_actions,
};
use egui;

use crate::play_configuration_view::PlayConfigurationView;

/// SCENE_NONE - label for no scene change
pub const SCENE_NONE: &str = "(No Change)";
/// ACTION_NONE - label for no action
pub const ACTION_NONE: &str = "(Do Nothing)";

/// ObsConfigurationView - OBS WebSocket configuration view
///
/// JavaFX UI widgets are translated to data structs.
/// Rendering will be implemented via egui when the OBS config tab is wired.
pub struct ObsConfigurationView {
    // JavaFX @FXML fields → egui widget state
    obs_ws_enabled: bool,
    obs_ws_host: String,
    obs_ws_port: i32,
    obs_ws_pass: String,
    obs_ws_connect_label: String,
    obs_ws_rec_mode: i32,
    obs_ws_rec_mode_items: Vec<String>,
    obs_ws_rec_stop_wait: i32,
    // VBox listContainer children are represented as state data
    // (actual rendering is egui)
    config: Option<Config>,
    status: String,
    obs_cfg_client: Option<ObsWsClient>,

    states: Vec<String>,
    scene_boxes: HashMap<String, ComboBoxState>,
    action_boxes: HashMap<String, ComboBoxState>,
}

/// ComboBoxState - represents the state of a ComboBox widget
#[derive(Clone, Debug, Default)]
pub struct ComboBoxState {
    pub items: Vec<String>,
    pub value: Option<String>,
    pub disabled: bool,
    pub min_width: f32,
}

impl ObsConfigurationView {
    /// Constructor (corresponds to JavaFX controller instantiation)
    pub fn new() -> Self {
        Self {
            obs_ws_enabled: false,
            obs_ws_host: String::new(),
            obs_ws_port: 4455,
            obs_ws_pass: String::new(),
            obs_ws_connect_label: String::new(),
            obs_ws_rec_mode: 0,
            obs_ws_rec_mode_items: Vec::new(),
            obs_ws_rec_stop_wait: 5000,

            config: None,
            status: String::new(),
            obs_cfg_client: None,

            states: Vec::new(),
            scene_boxes: HashMap::new(),
            action_boxes: HashMap::new(),
        }
    }

    /// initialize - corresponds to Initializable.initialize(URL, ResourceBundle)
    /// In Java, this populates the obsWsRecMode ComboBox items from resource bundle.
    pub fn initialize(&mut self) {
        // obsWsRecMode.getItems().addAll(
        //     resources.getString("OBSWS_REC_DEFAULT"),
        //     resources.getString("OBSWS_REC_ONSCREENSHOT"),
        //     resources.getString("OBSWS_REC_ONREPLAY")
        // );
        self.obs_ws_rec_mode_items = vec![
            "Default".to_string(),
            "On Screenshot".to_string(),
            "On Replay".to_string(),
        ];
        // egui: items populated above; combo box renders at frame time — see launcher_ui.rs render_obs_tab()
    }

    /// init - called from PlayConfigurationView to set up state rows
    pub fn init(&mut self, _main: &PlayConfigurationView) {
        let main_state_types = [
            MainStateType::MusicSelect,
            MainStateType::Decide,
            MainStateType::Play,
            MainStateType::Result,
            MainStateType::CourseResult,
            MainStateType::Config,
            MainStateType::SkinConfig,
        ];

        for state in &main_state_types {
            let name = format!("{:?}", state);
            self.states.push(name.clone());
            self.create_state_row(&name);
            if name == "Play" {
                self.states.push("PLAY_ENDED".to_string());
                self.create_state_row("PLAY_ENDED");
            }
        }
    }

    /// createStateRow - creates a row with label, scene ComboBox, and action ComboBox
    fn create_state_row(&mut self, state_name: &str) {
        // HBox row = new HBox(10);
        // Label label = new Label(stateName);
        // label.setMinWidth(100);

        // ComboBox<String> sceneBox = new ComboBox<>();
        // sceneBox.setDisable(true);
        // sceneBox.setMinWidth(150);
        // sceneBox.getItems().add(SCENE_NONE);
        let scene_box = ComboBoxState {
            items: vec![SCENE_NONE.to_string()],
            value: None,
            disabled: true,
            min_width: 150.0,
        };
        self.scene_boxes.insert(state_name.to_string(), scene_box);

        // ComboBox<String> actionBox = new ComboBox<>();
        // actionBox.setMinWidth(150);
        let action_box = ComboBoxState {
            items: Vec::new(),
            value: None,
            disabled: false,
            min_width: 150.0,
        };
        self.action_boxes.insert(state_name.to_string(), action_box);

        // row.getChildren().addAll(label, sceneBox, actionBox);
        // listContainer.getChildren().add(row);
        // (egui rendering deferred)
    }

    /// update - loads config values into UI state
    pub fn update(&mut self, config: Config) {
        self.obs_ws_enabled = config.use_obs_ws;
        self.obs_ws_host = config.obs_ws_host.clone();
        self.obs_ws_port = config.obs_ws_port;
        self.obs_ws_pass = config.obs_ws_pass.clone();
        self.obs_ws_rec_stop_wait = config.obs_ws_rec_stop_wait;
        self.obs_ws_rec_mode = config.obs_ws_rec_mode;
        self.reset_connection_status();

        self.config = Some(config);

        self.load_saved_selections();
    }

    /// loadSavedSelections - loads saved scene/action selections from config
    fn load_saved_selections(&mut self) {
        let config = match &self.config {
            Some(c) => c.clone(),
            None => return,
        };

        for state in self.states.clone() {
            if let Some(scene_box) = self.scene_boxes.get_mut(&state) {
                let saved_scene = config.get_obs_scene(&state);
                if let Some(saved) = saved_scene {
                    if !saved.is_empty() {
                        scene_box.value = Some(saved.clone());
                    } else {
                        scene_box.value = Some(SCENE_NONE.to_string());
                    }
                } else {
                    scene_box.value = Some(SCENE_NONE.to_string());
                }
            }

            if let Some(action_box) = self.action_boxes.get_mut(&state) {
                let saved_action = config.get_obs_action(&state);
                let saved_action_label = if let Some(action) = saved_action {
                    get_action_label(action)
                } else {
                    None
                };
                if let Some(label) = saved_action_label {
                    if !label.is_empty() {
                        action_box.value = Some(label);
                    } else {
                        action_box.value = Some(ACTION_NONE.to_string());
                    }
                } else {
                    action_box.value = Some(ACTION_NONE.to_string());
                }
            }
        }
    }

    /// commit - saves UI state back to config
    pub fn commit(&mut self) {
        if let Some(config) = &mut self.config {
            config.use_obs_ws = self.obs_ws_enabled;
            config.obs_ws_host = self.obs_ws_host.clone();
            config.obs_ws_port = self.obs_ws_port;
            config.obs_ws_pass = self.obs_ws_pass.clone();
            config.obs_ws_rec_stop_wait = self.obs_ws_rec_stop_wait;
            config.obs_ws_rec_mode = self.obs_ws_rec_mode;
        }

        self.save_selections();
    }

    /// saveSelections - saves scene/action selections to config
    fn save_selections(&mut self) {
        if self.obs_cfg_client.is_none() {
            return;
        }
        if let Some(ref client) = self.obs_cfg_client
            && !client.is_connected()
        {
            return;
        }

        let actions = obs_actions();
        let states_clone = self.states.clone();

        for state in &states_clone {
            if let Some(scene_box) = self.scene_boxes.get(state) {
                let value = scene_box.value.as_deref();
                if value.is_none() || value == Some(SCENE_NONE) {
                    if let Some(config) = &mut self.config {
                        config.set_obs_scene(state.clone(), Some(String::new()));
                    }
                } else if let Some(v) = value
                    && let Some(config) = &mut self.config
                {
                    config.set_obs_scene(state.clone(), Some(v.to_string()));
                }
            }

            if let Some(action_box) = self.action_boxes.get(state) {
                let value = action_box.value.as_deref();
                if value.is_none() || value == Some(ACTION_NONE) {
                    if let Some(config) = &mut self.config {
                        config.set_obs_action(state.clone(), Some(String::new()));
                    }
                } else if let Some(v) = value {
                    let req = actions.get(v);
                    if let Some(req) = req
                        && let Some(config) = &mut self.config
                    {
                        config.set_obs_action(state.clone(), Some(req.clone()));
                    }
                }
            }
        }

        self.close_existing_connection();
    }

    /// connect - initiates OBS WebSocket connection
    /// In Java, this spawns a new Thread. In Rust, this would use tokio::spawn.
    pub fn connect(&mut self) {
        self.set_connection_status("connecting", "Connecting...");

        // new Thread(() -> { ... }).start();
        // In Rust, we would use tokio::spawn or std::thread::spawn
        // For now, the connection logic is represented as a method body

        self.close_existing_connection();

        let temp_config = Config {
            obs_ws_host: self.obs_ws_host.clone(),
            obs_ws_port: self.obs_ws_port,
            obs_ws_pass: self.obs_ws_pass.clone(),
            ..Default::default()
        };

        let client = match ObsWsClient::new(&temp_config) {
            Ok(c) => c,
            Err(_ex) => {
                self.handle_obs_error("Failed to create client");
                return;
            }
        };
        client.set_auto_reconnect(false);

        // obsCfgClient.setOnError(this::handleObsError);
        // obsCfgClient.setOnClose(this::handleObsClose);
        // obsCfgClient.setOnVersionReceived(this::handleVersionReceived);
        // obsCfgClient.setOnScenesReceived(this::handleScenesReceived);
        // Note: In Rust, closures capturing &mut self require careful handling.
        // The callbacks are set but actual error/close/version/scene handling
        // would need Arc<Mutex<Self>> or channel-based communication.
        // Stubbed here as todo.

        // client.set_on_error(...);
        // client.set_on_close(...);
        // client.set_on_version_received(...);
        // client.set_on_scenes_received(...);

        match client.connect() {
            Ok(()) => {}
            Err(_ex) => {
                self.handle_obs_error("Connection error");
            }
        }

        self.obs_cfg_client = Some(client);
    }

    /// closeExistingConnection - closes the existing OBS connection if active
    fn close_existing_connection(&mut self) {
        if let Some(ref client) = self.obs_cfg_client
            && client.is_connected()
        {
            client.close();
        }
        self.obs_cfg_client = None;
    }

    /// handleObsError - called when OBS connection encounters an error
    fn handle_obs_error(&mut self, _ex: &str) {
        self.set_connection_status("connect_fail", "Failed to connect!");
    }

    /// handleObsClose - called when OBS connection is closed
    #[allow(dead_code)]
    fn handle_obs_close(&mut self) {
        if self.status == "connecting" {
            self.set_connection_status("auth_fail", "Authentication failed!");
        }
    }

    /// handleVersionReceived - called when OBS version info is received
    #[allow(dead_code)]
    fn handle_version_received(&mut self, version: &ObsVersionInfo) {
        self.set_connection_status("version_received", &version.to_string());
    }

    /// handleScenesReceived - called when OBS scene list is received
    /// In Java, this uses Platform.runLater() to update UI on the JavaFX thread.
    #[allow(dead_code)]
    fn handle_scenes_received(&mut self, scenes: &[String]) {
        // Platform.runLater(() -> { ... })
        // In egui, UI updates happen on the main thread during frame rendering.

        let config = match &self.config {
            Some(c) => c.clone(),
            None => return,
        };

        for (state_name, scene_box) in &mut self.scene_boxes {
            let previous_value = scene_box.value.clone();
            let saved_scene = config.get_obs_scene(state_name).cloned();

            scene_box.items.clear();
            scene_box.items.push(SCENE_NONE.to_string());
            scene_box.items.extend(scenes.iter().cloned());
            scene_box.disabled = false;

            if let Some(ref saved) = saved_scene {
                if !saved.is_empty() && scenes.contains(saved) {
                    scene_box.value = Some(saved.clone());
                } else if let Some(ref prev) = previous_value {
                    if scenes.contains(prev) {
                        scene_box.value = previous_value.clone();
                    } else {
                        scene_box.value = Some(SCENE_NONE.to_string());
                    }
                } else {
                    scene_box.value = Some(SCENE_NONE.to_string());
                }
            } else if let Some(ref prev) = previous_value {
                if scenes.contains(prev) {
                    scene_box.value = previous_value.clone();
                } else {
                    scene_box.value = Some(SCENE_NONE.to_string());
                }
            } else {
                scene_box.value = Some(SCENE_NONE.to_string());
            }
        }

        let actions = obs_actions();
        let action_keys: Vec<String> = actions.keys().cloned().collect();

        for (state_name, action_box) in &mut self.action_boxes {
            let previous_value = action_box.value.clone();
            let saved_action_label = config
                .get_obs_action(state_name)
                .and_then(|a| get_action_label(a));

            action_box.items.clear();
            action_box.items.push(ACTION_NONE.to_string());
            action_box.items.extend(action_keys.iter().cloned());

            if let Some(ref saved_label) = saved_action_label {
                if !saved_label.is_empty() && action_keys.contains(saved_label) {
                    action_box.value = Some(saved_label.clone());
                } else if let Some(ref prev) = previous_value {
                    if action_keys.contains(prev) {
                        action_box.value = previous_value.clone();
                    } else {
                        action_box.value = Some(ACTION_NONE.to_string());
                    }
                } else {
                    action_box.value = Some(ACTION_NONE.to_string());
                }
            } else if let Some(ref prev) = previous_value {
                if action_keys.contains(prev) {
                    action_box.value = previous_value.clone();
                } else {
                    action_box.value = Some(ACTION_NONE.to_string());
                }
            } else {
                action_box.value = Some(ACTION_NONE.to_string());
            }
        }
    }

    /// setConnectionStatus - updates connection status string and label
    fn set_connection_status(&mut self, status: &str, label_text: &str) {
        self.status = status.to_string();
        // Platform.runLater(() -> obsWsConnectLabel.setText(labelText));
        self.obs_ws_connect_label = label_text.to_string();
    }

    /// resetConnectionStatus - clears the connection status label
    fn reset_connection_status(&mut self) {
        self.obs_ws_connect_label = String::new();
    }

    /// Render the OBS configuration UI.
    ///
    /// Shows WebSocket connection settings, connect button with status,
    /// and per-state scene/action selectors.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("OBS WebSocket Configuration");

        egui::Grid::new("obs_config_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Enable:");
                ui.checkbox(&mut self.obs_ws_enabled, "");
                ui.end_row();

                if self.obs_ws_enabled {
                    ui.label("Host:");
                    ui.text_edit_singleline(&mut self.obs_ws_host);
                    ui.end_row();

                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut self.obs_ws_port).range(1..=65535));
                    ui.end_row();

                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.obs_ws_pass).password(true));
                    ui.end_row();

                    ui.label("Recording Mode:");
                    let rec_mode_label = self
                        .obs_ws_rec_mode_items
                        .get(self.obs_ws_rec_mode as usize)
                        .cloned()
                        .unwrap_or_else(|| "Default".to_string());
                    egui::ComboBox::from_id_salt("obs_config_rec_mode")
                        .selected_text(&rec_mode_label)
                        .show_ui(ui, |ui| {
                            for (i, label) in self.obs_ws_rec_mode_items.iter().enumerate() {
                                ui.selectable_value(&mut self.obs_ws_rec_mode, i as i32, label);
                            }
                        });
                    ui.end_row();

                    ui.label("Rec Stop Wait (ms):");
                    ui.add(egui::DragValue::new(&mut self.obs_ws_rec_stop_wait).range(0..=60000));
                    ui.end_row();
                }
            });

        if self.obs_ws_enabled {
            ui.separator();

            // Connect button + status
            ui.horizontal(|ui| {
                if ui.button("Connect").clicked() {
                    self.connect();
                }
                if !self.obs_ws_connect_label.is_empty() {
                    ui.label(&self.obs_ws_connect_label);
                }
            });

            ui.separator();

            // State scene/action selectors
            ui.heading("State Actions");

            let states = self.states.clone();
            let actions = obs_actions();
            let action_labels: Vec<String> = std::iter::once(ACTION_NONE.to_string())
                .chain(actions.keys().cloned())
                .collect();

            egui::Grid::new("obs_state_action_grid")
                .min_col_width(100.0)
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("State");
                    ui.label("Scene");
                    ui.label("Action");
                    ui.end_row();

                    for state in &states {
                        ui.label(state);

                        // Scene combo box
                        if let Some(scene_box) = self.scene_boxes.get_mut(state) {
                            let selected_text =
                                scene_box.value.as_deref().unwrap_or(SCENE_NONE).to_string();
                            let mut new_value = selected_text.clone();
                            let combo =
                                egui::ComboBox::from_id_salt(format!("obs_cfg_scene_{}", state))
                                    .selected_text(&selected_text);
                            if scene_box.disabled {
                                ui.add_enabled(false, egui::Label::new(&selected_text));
                            } else {
                                combo.show_ui(ui, |ui| {
                                    for item in &scene_box.items {
                                        ui.selectable_value(
                                            &mut new_value,
                                            item.clone(),
                                            item.as_str(),
                                        );
                                    }
                                });
                                if new_value != selected_text {
                                    scene_box.value = Some(new_value);
                                }
                            }
                        } else {
                            ui.label("-");
                        }

                        // Action combo box
                        if let Some(action_box) = self.action_boxes.get_mut(state) {
                            let selected_text = action_box
                                .value
                                .as_deref()
                                .unwrap_or(ACTION_NONE)
                                .to_string();
                            let mut new_value = selected_text.clone();
                            egui::ComboBox::from_id_salt(format!("obs_cfg_action_{}", state))
                                .selected_text(&selected_text)
                                .show_ui(ui, |ui| {
                                    for label in &action_labels {
                                        ui.selectable_value(
                                            &mut new_value,
                                            label.clone(),
                                            label.as_str(),
                                        );
                                    }
                                });
                            if new_value != selected_text {
                                action_box.value = Some(new_value);
                            }
                        } else {
                            ui.label("-");
                        }

                        ui.end_row();
                    }
                });
        }
    }
}

impl Default for ObsConfigurationView {
    fn default() -> Self {
        Self::new()
    }
}
