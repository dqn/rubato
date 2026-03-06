// Translates: bms.player.beatoraja.launcher.ControllerConfigViewModel

use rubato_core::play_mode_config::ControllerConfig;

/// ControllerConfig ViewModel
/// Translates: bms.player.beatoraja.launcher.ControllerConfigViewModel
///
/// Wraps a ControllerConfig and provides observable-like properties
/// for use in table views. In JavaFX these were StringProperty,
/// BooleanProperty, ObjectProperty<Integer>. In Rust these are
/// plain fields that egui will read/write directly.
pub struct ControllerConfigViewModel {
    // private StringProperty nameProperty = new SimpleStringProperty();
    pub name: String,
    // private BooleanProperty isAnalogScratchProperty = new SimpleBooleanProperty();
    pub is_analog_scratch: bool,
    // private ObjectProperty<Integer> analogScratchThresholdProperty = new SimpleIntegerProperty().asObject();
    pub analog_scratch_threshold: i32,
    // private ObjectProperty<Integer> analogScratchModeProperty = new SimpleIntegerProperty().asObject();
    pub analog_scratch_mode: i32,

    // private ControllerConfig config;
    pub config: ControllerConfig,
}

impl ControllerConfigViewModel {
    // public ControllerConfigViewModel(ControllerConfig config)
    pub fn new(config: ControllerConfig) -> Self {
        // this.nameProperty.set(config.getName());
        let name = config.name.clone();
        // this.isAnalogScratchProperty.set(config.isAnalogScratch());
        let is_analog_scratch = config.analog_scratch;
        // this.analogScratchThresholdProperty.set(config.getAnalogScratchThreshold());
        let analog_scratch_threshold = config.analog_scratch_threshold;
        // this.analogScratchModeProperty.set(config.getAnalogScratchMode());
        let analog_scratch_mode = config.analog_scratch_mode;

        ControllerConfigViewModel {
            name,
            is_analog_scratch,
            analog_scratch_threshold,
            analog_scratch_mode,
            config,
        }
    }
}
