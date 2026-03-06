// Translates: bms.player.beatoraja.launcher.AudioConfigurationView

use rubato_core::audio_config::{AudioConfig, DriverType, FrequencyType};

use egui;

use crate::stubs::port_audio_devices;

/// Translates: AudioConfigurationView (JavaFX → egui)
///
/// Audio configuration UI with DriverType, FrequencyType combos,
/// volume sliders, and PortAudio device selection.
pub struct AudioConfigurationView {
    // @FXML private ComboBox<DriverType> audio;
    audio: Option<DriverType>,
    // @FXML private ComboBox<String> audioname;
    audioname: Option<String>,
    audioname_items: Vec<String>,
    audioname_disabled: bool,
    // @FXML private Spinner<Integer> audiobuffer;
    audiobuffer: i32,
    audiobuffer_disabled: bool,
    // @FXML private Spinner<Integer> audiosim;
    audiosim: i32,
    audiosim_disabled: bool,
    // @FXML private ComboBox<Integer> audiosamplerate;
    audiosamplerate: Option<i32>,
    // @FXML private Slider systemvolume;
    systemvolume: f64,
    // @FXML private Slider keyvolume;
    keyvolume: f64,
    keyvolume_disabled: bool,
    // @FXML private Slider bgvolume;
    bgvolume: f64,
    bgvolume_disabled: bool,
    // @FXML private CheckBox normalizeVolume;
    normalize_volume: bool,
    // @FXML private ComboBox<FrequencyType> audioFreqOption;
    audio_freq_option: Option<FrequencyType>,
    // @FXML private ComboBox<FrequencyType> audioFastForward;
    audio_fast_forward: Option<FrequencyType>,
    // @FXML private CheckBox loopResultSound;
    loop_result_sound: bool,
    // @FXML private CheckBox loopCourseResultSound;
    loop_course_result_sound: bool,

    config: Option<AudioConfig>,
}

impl Default for AudioConfigurationView {
    fn default() -> Self {
        AudioConfigurationView {
            audio: None,
            audioname: None,
            audioname_items: Vec::new(),
            audioname_disabled: false,
            audiobuffer: 0,
            audiobuffer_disabled: false,
            audiosim: 0,
            audiosim_disabled: false,
            audiosamplerate: None,
            systemvolume: 0.0,
            keyvolume: 0.0,
            keyvolume_disabled: false,
            bgvolume: 0.0,
            bgvolume_disabled: false,
            normalize_volume: false,
            audio_freq_option: None,
            audio_fast_forward: None,
            loop_result_sound: false,
            loop_course_result_sound: false,
            config: None,
        }
    }
}

impl AudioConfigurationView {
    // public void initialize(URL arg0, ResourceBundle arg1)
    pub fn initialize(&mut self) {
        // audio.getItems().setAll(DriverType.OpenAL , DriverType.PortAudio);
        // (available driver types: OpenAL, PortAudio)
        // audiosamplerate.getItems().setAll(null, 44100, 48000);
        // (available sample rates: None, 44100, 48000)

        // audioFreqOption.getItems().setAll(FrequencyType.UNPROCESSED, FrequencyType.FREQUENCY);
        // audioFastForward.getItems().setAll(FrequencyType.UNPROCESSED, FrequencyType.FREQUENCY);

        // egui: combo boxes render items at frame time — see launcher_ui.rs render_audio_tab()
    }

    // public void update(AudioConfig config)
    pub fn update(&mut self, config: AudioConfig) {
        self.config = Some(config.clone());

        // audio.setValue(config.getDriver());
        self.audio = Some(config.driver.clone());
        // audiobuffer.getValueFactory().setValue(config.getDeviceBufferSize());
        self.audiobuffer = config.device_buffer_size;
        // audiosim.getValueFactory().setValue(config.getDeviceSimultaneousSources());
        self.audiosim = config.device_simultaneous_sources;
        // audiosamplerate.setValue(config.getSampleRate() > 0 ? config.getSampleRate() : null);
        self.audiosamplerate = if config.sample_rate > 0 {
            Some(config.sample_rate)
        } else {
            None
        };
        // audioFreqOption.setValue(config.getFreqOption());
        self.audio_freq_option = Some(config.freq_option.clone());
        // audioFastForward.setValue(config.getFastForward());
        self.audio_fast_forward = Some(config.fast_forward.clone());
        // systemvolume.setValue((double)config.getSystemvolume());
        self.systemvolume = config.systemvolume as f64;
        // keyvolume.setValue((double)config.getKeyvolume());
        self.keyvolume = config.keyvolume as f64;
        // bgvolume.setValue((double)config.getBgvolume());
        self.bgvolume = config.bgvolume as f64;
        // normalizeVolume.setSelected(config.isNormalizeVolume());
        self.normalize_volume = config.normalize_volume;
        // loopResultSound.setSelected(config.isLoopResultSound());
        self.loop_result_sound = config.is_loop_result_sound;
        // loopCourseResultSound.setSelected(config.isLoopCourseResultSound());
        self.loop_course_result_sound = config.is_loop_course_result_sound;

        self.update_audio_driver();
        self.update_normalize_volume();
    }

    // public void commit()
    pub fn commit(&mut self) {
        if let Some(ref mut config) = self.config {
            // config.setDriver(audio.getValue());
            if let Some(ref driver) = self.audio {
                config.driver = driver.clone();
            }
            // config.setDriverName(audioname.getValue());
            config.driver_name = self.audioname.clone();
            // config.setDeviceBufferSize(audiobuffer.getValue());
            config.device_buffer_size = self.audiobuffer;
            // config.setDeviceSimultaneousSources(audiosim.getValue());
            config.device_simultaneous_sources = self.audiosim;
            // config.setSampleRate(audiosamplerate.getValue() != null ? audiosamplerate.getValue() : 0);
            config.sample_rate = self.audiosamplerate.unwrap_or(0);
            // config.setFreqOption(audioFreqOption.getValue());
            if let Some(ref freq) = self.audio_freq_option {
                config.freq_option = freq.clone();
            }
            // config.setFastForward(audioFastForward.getValue());
            if let Some(ref ff) = self.audio_fast_forward {
                config.fast_forward = ff.clone();
            }
            // config.setSystemvolume((float) systemvolume.getValue());
            config.systemvolume = self.systemvolume as f32;
            // config.setKeyvolume((float) keyvolume.getValue());
            config.keyvolume = self.keyvolume as f32;
            // config.setBgvolume((float) bgvolume.getValue());
            config.bgvolume = self.bgvolume as f32;
            // config.setNormalizeVolume(normalizeVolume.isSelected());
            config.normalize_volume = self.normalize_volume;
            // config.setLoopResultSound(loopResultSound.isSelected());
            config.is_loop_result_sound = self.loop_result_sound;
            // config.setLoopCourseResultSound(loopCourseResultSound.isSelected());
            config.is_loop_course_result_sound = self.loop_course_result_sound;
        }
    }

    // @FXML public void updateNormalizeVolume()
    pub fn update_normalize_volume(&mut self) {
        // boolean enabled = normalizeVolume.isSelected();
        let enabled = self.normalize_volume;
        // keyvolume.setDisable(enabled);
        self.keyvolume_disabled = enabled;
        // bgvolume.setDisable(enabled);
        self.bgvolume_disabled = enabled;
    }

    // @FXML public void updateAudioDriver()
    pub fn update_audio_driver(&mut self) {
        // switch(audio.getValue())
        match self.audio {
            Some(DriverType::OpenAL) => {
                // case OpenAL:
                // audioname.setDisable(true);
                self.audioname_disabled = true;
                // audioname.getItems().clear();
                self.audioname_items.clear();
                // audiobuffer.setDisable(false);
                self.audiobuffer_disabled = false;
                // audiosim.setDisable(false);
                self.audiosim_disabled = false;
            }
            Some(DriverType::PortAudio) => {
                // case PortAudio:
                // try {
                match port_audio_devices() {
                    Ok(devices) => {
                        // DeviceInfo[] devices = PortAudioDriver.getDevices();
                        // List<String> drivers = new ArrayList<String>(devices.length);
                        let mut drivers: Vec<String> = Vec::with_capacity(devices.len());
                        // for(int i = 0;i < devices.length;i++) {
                        for device in &devices {
                            // drivers.add(devices[i].name);
                            drivers.push(device.name.clone());
                        }
                        // if(drivers.size() == 0) {
                        if drivers.is_empty() {
                            // throw new RuntimeException("ドライバが見つかりません");
                            log::error!("PortAudioは選択できません : ドライバが見つかりません");
                            self.audio = Some(DriverType::OpenAL);
                            return;
                        }
                        // audioname.getItems().setAll(drivers);
                        self.audioname_items = drivers.clone();
                        // if(drivers.contains(config.getDriverName())) {
                        if let Some(ref config) = self.config {
                            if let Some(ref driver_name) = config.driver_name {
                                if drivers.contains(driver_name) {
                                    // audioname.setValue(config.getDriverName());
                                    self.audioname = Some(driver_name.clone());
                                } else {
                                    // audioname.setValue(drivers.get(0));
                                    self.audioname = Some(drivers[0].clone());
                                }
                            } else {
                                self.audioname = Some(drivers[0].clone());
                            }
                        } else {
                            self.audioname = Some(drivers[0].clone());
                        }
                        // audioname.setDisable(false);
                        self.audioname_disabled = false;
                        // audiobuffer.setDisable(false);
                        self.audiobuffer_disabled = false;
                        // audiosim.setDisable(false);
                        self.audiosim_disabled = false;
                    }
                    Err(e) => {
                        // } catch(Throwable e) {
                        // logger.error("PortAudioは選択できません : {}", e.getMessage());
                        log::error!("PortAudioは選択できません : {}", e);
                        // audio.setValue(DriverType.OpenAL);
                        self.audio = Some(DriverType::OpenAL);
                    }
                }
            }
            None => {}
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let old_driver_label = self
            .audio
            .as_ref()
            .map(|d| format!("{:?}", d))
            .unwrap_or_default();

        ui.heading("Audio Driver");
        egui::Grid::new("audio_driver_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Driver:");
                let driver_label = old_driver_label.clone();
                egui::ComboBox::from_id_salt("audio_driver")
                    .selected_text(&driver_label)
                    .show_ui(ui, |ui| {
                        let drivers = [DriverType::OpenAL, DriverType::PortAudio];
                        for driver in &drivers {
                            let label = format!("{:?}", driver);
                            let selected = driver_label == label;
                            if ui.selectable_label(selected, &label).clicked() {
                                self.audio = Some(driver.clone());
                            }
                        }
                    });
                ui.end_row();

                if !self.audioname_disabled {
                    ui.label("Device:");
                    let name_label = self.audioname.clone().unwrap_or_default();
                    egui::ComboBox::from_id_salt("audio_device_name")
                        .selected_text(&name_label)
                        .show_ui(ui, |ui| {
                            for name in &self.audioname_items.clone() {
                                ui.selectable_value(&mut self.audioname, Some(name.clone()), name);
                            }
                        });
                    ui.end_row();
                }

                if !self.audiobuffer_disabled {
                    ui.label("Buffer Size:");
                    ui.add(egui::DragValue::new(&mut self.audiobuffer).range(0..=65536));
                    ui.end_row();
                }

                if !self.audiosim_disabled {
                    ui.label("Simultaneous Sources:");
                    ui.add(egui::DragValue::new(&mut self.audiosim).range(0..=256));
                    ui.end_row();
                }

                ui.label("Sample Rate:");
                let sr_label = self
                    .audiosamplerate
                    .map(|sr| sr.to_string())
                    .unwrap_or_else(|| "Auto".to_string());
                egui::ComboBox::from_id_salt("audio_sample_rate")
                    .selected_text(&sr_label)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.audiosamplerate, None, "Auto");
                        ui.selectable_value(&mut self.audiosamplerate, Some(44100), "44100");
                        ui.selectable_value(&mut self.audiosamplerate, Some(48000), "48000");
                    });
                ui.end_row();
            });

        // Update driver settings when driver selection changes
        let new_driver_label = self
            .audio
            .as_ref()
            .map(|d| format!("{:?}", d))
            .unwrap_or_default();
        if old_driver_label != new_driver_label {
            self.update_audio_driver();
        }

        ui.separator();
        ui.heading("Volume");
        egui::Grid::new("audio_volume_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("System Volume:");
                ui.add(egui::Slider::new(&mut self.systemvolume, 0.0..=1.0));
                ui.end_row();

                ui.label("Normalize Volume:");
                if ui.checkbox(&mut self.normalize_volume, "").changed() {
                    self.update_normalize_volume();
                }
                ui.end_row();

                if !self.keyvolume_disabled {
                    ui.label("Key Volume:");
                    ui.add(egui::Slider::new(&mut self.keyvolume, 0.0..=1.0));
                    ui.end_row();
                }

                if !self.bgvolume_disabled {
                    ui.label("BG Volume:");
                    ui.add(egui::Slider::new(&mut self.bgvolume, 0.0..=1.0));
                    ui.end_row();
                }
            });

        ui.separator();
        ui.heading("Frequency / Playback");
        egui::Grid::new("audio_freq_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Frequency Option:");
                let freq_label = self
                    .audio_freq_option
                    .as_ref()
                    .map(|f| format!("{:?}", f))
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("audio_freq_option")
                    .selected_text(&freq_label)
                    .show_ui(ui, |ui| {
                        let options = [FrequencyType::UNPROCESSED, FrequencyType::FREQUENCY];
                        for opt in &options {
                            let label = format!("{:?}", opt);
                            let selected = freq_label == label;
                            if ui.selectable_label(selected, &label).clicked() {
                                self.audio_freq_option = Some(opt.clone());
                            }
                        }
                    });
                ui.end_row();

                ui.label("Fast Forward:");
                let ff_label = self
                    .audio_fast_forward
                    .as_ref()
                    .map(|f| format!("{:?}", f))
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("audio_fast_forward")
                    .selected_text(&ff_label)
                    .show_ui(ui, |ui| {
                        let options = [FrequencyType::UNPROCESSED, FrequencyType::FREQUENCY];
                        for opt in &options {
                            let label = format!("{:?}", opt);
                            let selected = ff_label == label;
                            if ui.selectable_label(selected, &label).clicked() {
                                self.audio_fast_forward = Some(opt.clone());
                            }
                        }
                    });
                ui.end_row();

                ui.label("Loop Result Sound:");
                ui.checkbox(&mut self.loop_result_sound, "");
                ui.end_row();

                ui.label("Loop Course Result Sound:");
                ui.checkbox(&mut self.loop_course_result_sound, "");
                ui.end_row();
            });
    }
}
