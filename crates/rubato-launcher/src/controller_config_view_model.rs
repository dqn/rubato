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

    // public String getName()
    pub fn get_name(&self) -> &str {
        // return this.nameProperty.get();
        &self.name
    }

    // public StringProperty getNameProperty()
    pub fn get_name_property(&self) -> &str {
        &self.name
    }

    // public boolean getIsAnalogScratch()
    pub fn get_is_analog_scratch(&self) -> bool {
        // return isAnalogScratchProperty.get();
        self.is_analog_scratch
    }

    // public void setIsAnalogScratch(boolean isAnalogScratch)
    pub fn set_is_analog_scratch(&mut self, is_analog_scratch: bool) {
        // this.isAnalogScratchProperty.set(isAnalogScratch);
        self.is_analog_scratch = is_analog_scratch;
    }

    // public BooleanProperty getIsAnalogScratchProperty()
    pub fn get_is_analog_scratch_property(&self) -> bool {
        self.is_analog_scratch
    }

    // public int getAnalogScratchThreshold()
    pub fn get_analog_scratch_threshold(&self) -> i32 {
        // return analogScratchThresholdProperty.get();
        self.analog_scratch_threshold
    }

    // public void setAnalogScratchThreshold(Integer analogScratchThreshold)
    pub fn set_analog_scratch_threshold(&mut self, analog_scratch_threshold: i32) {
        // this.analogScratchThresholdProperty.set(analogScratchThreshold);
        self.analog_scratch_threshold = analog_scratch_threshold;
    }

    // public ObjectProperty<Integer> getAnalogScratchThresholdProperty()
    pub fn get_analog_scratch_threshold_property(&self) -> i32 {
        self.analog_scratch_threshold
    }

    // public int getAnalogScratchMode()
    pub fn get_analog_scratch_mode(&self) -> i32 {
        // return this.analogScratchModeProperty.get();
        self.analog_scratch_mode
    }

    // public void setAnalogScratchMode(int analogScratchMode)
    pub fn set_analog_scratch_mode(&mut self, analog_scratch_mode: i32) {
        // this.analogScratchModeProperty.set(analogScratchMode);
        self.analog_scratch_mode = analog_scratch_mode;
    }

    // public ObjectProperty<Integer> getAnalogScratchModeProperty()
    pub fn get_analog_scratch_mode_property(&self) -> i32 {
        self.analog_scratch_mode
    }

    // public ControllerConfig getConfig()
    pub fn get_config(&self) -> &ControllerConfig {
        // return this.config;
        &self.config
    }

    // Mutable version for commit
    pub fn get_config_mut(&mut self) -> &mut ControllerConfig {
        &mut self.config
    }
}
