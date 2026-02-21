// Translates: bms.player.beatoraja.launcher.InputConfigurationView

use bms_model::mode::Mode;

use beatoraja_core::play_mode_config::{
    PlayModeConfig, ANALOG_SCRATCH_VER_1, ANALOG_SCRATCH_VER_2,
};
use beatoraja_core::player_config::PlayerConfig;

use crate::controller_config_view_model::ControllerConfigViewModel;

/// PlayMode enum
/// Translates: PlayConfigurationView.PlayMode (inner enum)
/// Defined here to avoid circular dependency on play_configuration_view.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code, non_camel_case_types)]
pub enum PlayMode {
    BEAT_5K,
    BEAT_7K,
    BEAT_10K,
    BEAT_14K,
    POPN_9K,
    KEYBOARD_24K,
    KEYBOARD_24K_DOUBLE,
}

#[allow(dead_code)]
impl PlayMode {
    pub fn values() -> &'static [PlayMode] {
        &[
            PlayMode::BEAT_5K,
            PlayMode::BEAT_7K,
            PlayMode::BEAT_10K,
            PlayMode::BEAT_14K,
            PlayMode::POPN_9K,
            PlayMode::KEYBOARD_24K,
            PlayMode::KEYBOARD_24K_DOUBLE,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PlayMode::BEAT_5K => "5KEYS",
            PlayMode::BEAT_7K => "7KEYS",
            PlayMode::BEAT_10K => "10KEYS",
            PlayMode::BEAT_14K => "14KEYS",
            PlayMode::POPN_9K => "9KEYS",
            PlayMode::KEYBOARD_24K => "24KEYS",
            PlayMode::KEYBOARD_24K_DOUBLE => "24KEYS DOUBLE",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            PlayMode::BEAT_5K => "BEAT_5K",
            PlayMode::BEAT_7K => "BEAT_7K",
            PlayMode::BEAT_10K => "BEAT_10K",
            PlayMode::BEAT_14K => "BEAT_14K",
            PlayMode::POPN_9K => "POPN_9K",
            PlayMode::KEYBOARD_24K => "KEYBOARD_24K",
            PlayMode::KEYBOARD_24K_DOUBLE => "KEYBOARD_24K_DOUBLE",
        }
    }

    pub fn to_mode(&self) -> Mode {
        match self {
            PlayMode::BEAT_5K => Mode::BEAT_5K,
            PlayMode::BEAT_7K => Mode::BEAT_7K,
            PlayMode::BEAT_10K => Mode::BEAT_10K,
            PlayMode::BEAT_14K => Mode::BEAT_14K,
            PlayMode::POPN_9K => Mode::POPN_9K,
            PlayMode::KEYBOARD_24K => Mode::KEYBOARD_24K,
            PlayMode::KEYBOARD_24K_DOUBLE => Mode::KEYBOARD_24K_DOUBLE,
        }
    }
}

impl std::fmt::Display for PlayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Translates: InputConfigurationView (JavaFX → egui)
///
/// Input configuration UI: play mode selection, controller table,
/// keyboard input duration, JKOC hack, mouse scratch settings.
#[allow(dead_code)]
pub struct InputConfigurationView {
    // TODO 各デバイス毎の最小入力間隔設定

    // @FXML private ComboBox<PlayConfigurationView.PlayMode> inputconfig;
    inputconfig: Option<PlayMode>,

    // @FXML private Spinner<Integer> inputduration;
    inputduration: i32,
    // @FXML private CheckBox jkoc_hack;
    jkoc_hack: bool,
    // @FXML private TableView<ControllerConfigViewModel> controller_tableView;
    controller_table_view: Vec<ControllerConfigViewModel>,
    // Table column configuration:
    // @FXML private TableColumn<ControllerConfigViewModel, String> playsideCol;
    // @FXML private TableColumn<ControllerConfigViewModel, String> nameCol;
    // @FXML private TableColumn<ControllerConfigViewModel, Boolean> isAnalogCol;
    // @FXML private TableColumn<ControllerConfigViewModel, Integer> analogThresholdCol;
    // @FXML private TableColumn<ControllerConfigViewModel, Integer> analogModeCol;
    // (Table columns are configured during egui rendering)

    // @FXML private CheckBox mouseScratch;
    mouse_scratch: bool,
    // @FXML private NumericSpinner<Integer> mouseScratchTimeThreshold;
    mouse_scratch_time_threshold: i32,
    // @FXML private NumericSpinner<Integer> mouseScratchDistance;
    mouse_scratch_distance: i32,
    // @FXML private ComboBox<Integer> mouseScratchMode;
    mouse_scratch_mode: i32,

    // private PlayerConfig player;
    player: Option<PlayerConfig>,
    // private PlayConfigurationView.PlayMode mode;
    mode: Option<PlayMode>,
}

impl Default for InputConfigurationView {
    fn default() -> Self {
        InputConfigurationView {
            inputconfig: None,
            inputduration: 0,
            jkoc_hack: false,
            controller_table_view: Vec::new(),
            mouse_scratch: false,
            mouse_scratch_time_threshold: 0,
            mouse_scratch_distance: 0,
            mouse_scratch_mode: 0,
            player: None,
            mode: None,
        }
    }
}

#[allow(dead_code)]
impl InputConfigurationView {
    pub fn new() -> Self {
        Self::default()
    }

    // public void initialize(URL location, ResourceBundle resources)
    pub fn initialize(&mut self) {
        // inputconfig.getItems().setAll(PlayConfigurationView.PlayMode.values());
        // (PlayMode items set during egui rendering)

        // PlayConfigurationView.initComboBox(mouseScratchMode, new String[] { "Ver. 2 (Newest)", "Ver. 1 (~0.8.3)" });
        // (ComboBox with integer indices 0, 1 mapped to label strings)
    }

    // @FXML public void changeMode()
    pub fn change_mode(&mut self) {
        // commitMode();
        self.commit_mode();
        // updateMode(inputconfig.getValue());
        if let Some(ref mode) = self.inputconfig.clone() {
            self.update_mode(mode);
        }
    }

    // public void update(PlayerConfig player)
    pub fn update(&mut self, player: &mut PlayerConfig) {
        // commitMode();
        self.commit_mode();
        // this.player = player;
        self.player = Some(player.clone());
        // updateMode(PlayConfigurationView.PlayMode.BEAT_7K);
        self.update_mode(&PlayMode::BEAT_7K);
        // inputconfig.setValue(PlayConfigurationView.PlayMode.BEAT_7K);
        self.inputconfig = Some(PlayMode::BEAT_7K);
    }

    // public void commit()
    pub fn commit(&mut self) {
        // commitMode();
        self.commit_mode();
    }

    // public void updateMode(PlayConfigurationView.PlayMode mode)
    pub fn update_mode(&mut self, mode: &PlayMode) {
        // this.mode = mode;
        self.mode = Some(mode.clone());

        // PlayModeConfig conf = player.getPlayConfig(Mode.valueOf(mode.name()));
        let bms_mode = mode.to_mode();
        let player = self.player.as_mut().expect("player must be set before updateMode");
        let conf: PlayModeConfig = player.get_play_config(bms_mode).clone();

        // List<ControllerConfigViewModel> listControllerConfigViewModel = Arrays.asList(conf.getController()).stream()
        //     .map(config -> new ControllerConfigViewModel(config)).collect(Collectors.toList());
        let list_controller_config_view_model: Vec<ControllerConfigViewModel> = conf
            .controller
            .iter()
            .map(|config| ControllerConfigViewModel::new(config.clone()))
            .collect();

        // inputduration.getValueFactory().setValue(conf.getKeyboardConfig().getDuration());
        self.inputduration = conf.keyboard.duration;
        // mouseScratch.setSelected(conf.getKeyboardConfig().getMouseScratchConfig().isMouseScratchEnabled());
        self.mouse_scratch = conf.keyboard.mouse_scratch_config.mouse_scratch_enabled;
        // mouseScratchTimeThreshold.getValueFactory().setValue(conf.getKeyboardConfig().getMouseScratchConfig().getMouseScratchTimeThreshold());
        self.mouse_scratch_time_threshold =
            conf.keyboard.mouse_scratch_config.mouse_scratch_time_threshold;
        // mouseScratchDistance.getValueFactory().setValue(conf.getKeyboardConfig().getMouseScratchConfig().getMouseScratchDistance());
        self.mouse_scratch_distance = conf.keyboard.mouse_scratch_config.mouse_scratch_distance;
        // mouseScratchMode.getSelectionModel().select(conf.getKeyboardConfig().getMouseScratchConfig().getMouseScratchMode());
        self.mouse_scratch_mode = conf.keyboard.mouse_scratch_config.mouse_scratch_mode;

        // controller_tableView.setEditable(true);
        // playsideCol.setEditable(false);
        // nameCol.setEditable(false);
        // playsideCol.setSortable(false);
        // nameCol.setSortable(false);
        // isAnalogCol.setSortable(false);
        // analogThresholdCol.setSortable(false);
        // analogModeCol.setSortable(false);
        // (Table column configuration deferred to egui rendering)

        // playsideCol.setCellValueFactory(col -> new SimpleStringProperty(...));
        // nameCol.setCellValueFactory(col -> col.getValue().getNameProperty());
        // isAnalogCol.setCellValueFactory(col -> col.getValue().getIsAnalogScratchProperty());
        // analogThresholdCol.setCellValueFactory(col -> col.getValue().getAnalogScratchThresholdProperty());
        // analogModeCol.setCellValueFactory(col -> col.getValue().getAnalogScratchModeProperty());
        // (Cell value factories deferred to egui rendering)

        // nameCol.setCellFactory(TextFieldTableCell.forTableColumn());
        // isAnalogCol.setCellFactory(CheckBoxTableCell.forTableColumn(isAnalogCol));
        // analogThresholdCol.setCellFactory(col -> new SpinnerCell(1, 1000, 100, 1));
        // analogModeCol.setCellFactory(ComboBoxTableCell.forTableColumn(new IntegerStringConverter() {
        //     private String v2String = "Ver. 2 (Newest)";
        //     private String v1String = "Ver. 1 (~0.6.9)";
        //     @Override public Integer fromString(String arg0) { ... }
        //     @Override public String toString(Integer arg0) { ... }
        // }, PlayModeConfig.ControllerConfig.ANALOG_SCRATCH_VER_2, PlayModeConfig.ControllerConfig.ANALOG_SCRATCH_VER_1));
        // (Cell factories deferred to egui rendering)

        // ObservableList<ControllerConfigViewModel> data = FXCollections.observableArrayList(listControllerConfigViewModel);
        // controller_tableView.setItems(data);
        self.controller_table_view = list_controller_config_view_model;

        // for (PlayModeConfig.ControllerConfig controller : conf.getController()) {
        for controller in &conf.controller {
            // inputduration.getValueFactory().setValue(controller.getDuration());
            self.inputduration = controller.duration;
            // jkoc_hack.setSelected(controller.getJKOC());
            self.jkoc_hack = controller.jkoc_hack;
        }
    }

    // public void commitMode()
    pub fn commit_mode(&mut self) {
        // if (mode != null) {
        if let Some(ref mode) = self.mode.clone() {
            // PlayModeConfig conf = player.getPlayConfig(Mode.valueOf(mode.name()));
            let bms_mode = mode.to_mode();
            let player = self
                .player
                .as_mut()
                .expect("player must be set before commitMode");
            let conf = player.get_play_config(bms_mode);

            // conf.getKeyboardConfig().setDuration(inputduration.getValue());
            conf.keyboard.duration = self.inputduration;
            // conf.getKeyboardConfig().getMouseScratchConfig().setMouseScratchEnabled(mouseScratch.isSelected());
            conf.keyboard.mouse_scratch_config.mouse_scratch_enabled = self.mouse_scratch;
            // conf.getKeyboardConfig().getMouseScratchConfig().setMouseScratchTimeThreshold(mouseScratchTimeThreshold.getValue());
            conf.keyboard
                .mouse_scratch_config
                .set_mouse_scratch_time_threshold(self.mouse_scratch_time_threshold);
            // conf.getKeyboardConfig().getMouseScratchConfig().setMouseScratchDistance(mouseScratchDistance.getValue());
            conf.keyboard
                .mouse_scratch_config
                .set_mouse_scratch_distance(self.mouse_scratch_distance);
            // conf.getKeyboardConfig().getMouseScratchConfig().setMouseScratchMode(mouseScratchMode.getValue());
            conf.keyboard.mouse_scratch_config.mouse_scratch_mode = self.mouse_scratch_mode;

            // for(ControllerConfigViewModel vm : this.controller_tableView.getItems()) {
            //     PlayModeConfig.ControllerConfig controller = vm.getConfig();
            //     controller.setDuration(inputduration.getValue());
            //     controller.setJKOC(jkoc_hack.isSelected());
            //     controller.setAnalogScratch(vm.getIsAnalogScratchProperty().get());
            //     controller.setAnalogScratchThreshold(vm.getAnalogScratchThreshold());
            //     controller.setAnalogScratchMode(vm.getAnalogScratchMode());
            // }
            // Update controllers from view model data
            for (i, vm) in self.controller_table_view.iter().enumerate() {
                if i < conf.controller.len() {
                    conf.controller[i].duration = self.inputduration;
                    conf.controller[i].jkoc_hack = self.jkoc_hack;
                    conf.controller[i].analog_scratch = vm.get_is_analog_scratch_property();
                    conf.controller[i]
                        .set_analog_scratch_threshold(vm.get_analog_scratch_threshold());
                    conf.controller[i].analog_scratch_mode = vm.get_analog_scratch_mode();
                }
            }
        }
    }

    /// Helper: Get analog scratch mode display string
    /// Translates the IntegerStringConverter used in Java's ComboBoxTableCell
    pub fn analog_scratch_mode_to_string(mode: i32) -> &'static str {
        if mode == ANALOG_SCRATCH_VER_2 {
            "Ver. 2 (Newest)"
        } else {
            "Ver. 1 (~0.6.9)"
        }
    }

    /// Helper: Get analog scratch mode from display string
    pub fn analog_scratch_mode_from_string(s: &str) -> i32 {
        if s == "Ver. 2 (Newest)" {
            ANALOG_SCRATCH_VER_2
        } else {
            ANALOG_SCRATCH_VER_1
        }
    }

    /// Helper: Get play side display string for table column
    /// Translates: playsideCol.setCellValueFactory(col -> ... (index+1) + "P")
    pub fn play_side_string(index: usize) -> String {
        format!("{}P", index + 1)
    }
}
