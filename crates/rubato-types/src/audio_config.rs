use crate::validatable::Validatable;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum DriverType {
    #[default]
    OpenAL,
    PortAudio,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum FrequencyType {
    UNPROCESSED,
    #[default]
    FREQUENCY,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub driver: DriverType,
    #[serde(rename = "driverName")]
    pub driver_name: Option<String>,
    #[serde(rename = "deviceBufferSize")]
    pub device_buffer_size: i32,
    #[serde(rename = "deviceSimultaneousSources")]
    pub device_simultaneous_sources: i32,
    #[serde(rename = "sampleRate")]
    pub sample_rate: i32,
    #[serde(rename = "freqOption")]
    pub freq_option: FrequencyType,
    #[serde(rename = "fastForward")]
    pub fast_forward: FrequencyType,
    pub systemvolume: f32,
    pub keyvolume: f32,
    pub bgvolume: f32,
    #[serde(rename = "normalizeVolume")]
    pub normalize_volume: bool,
    #[serde(rename = "isLoopResultSound")]
    pub is_loop_result_sound: bool,
    #[serde(rename = "isLoopCourseResultSound")]
    pub is_loop_course_result_sound: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        AudioConfig {
            driver: DriverType::OpenAL,
            driver_name: None,
            device_buffer_size: 384,
            device_simultaneous_sources: 128,
            sample_rate: 0,
            freq_option: FrequencyType::FREQUENCY,
            fast_forward: FrequencyType::FREQUENCY,
            systemvolume: 0.5,
            keyvolume: 0.5,
            bgvolume: 0.5,
            normalize_volume: false,
            is_loop_result_sound: false,
            is_loop_course_result_sound: false,
        }
    }
}

// Compatibility getters for stub API
impl AudioConfig {
    pub fn is_normalize_volume(&self) -> bool {
        self.normalize_volume
    }

    pub fn get_driver_name(&self) -> Option<&str> {
        self.driver_name.as_deref()
    }

    pub fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    pub fn get_device_buffer_size(&self) -> i32 {
        self.device_buffer_size
    }

    pub fn get_device_simultaneous_sources(&self) -> i32 {
        self.device_simultaneous_sources
    }
}

impl Validatable for AudioConfig {
    fn validate(&mut self) -> bool {
        self.device_buffer_size = self.device_buffer_size.clamp(4, 4096);
        self.device_simultaneous_sources = self.device_simultaneous_sources.clamp(16, 1024);
        self.systemvolume = self.systemvolume.clamp(0.0, 1.0);
        self.keyvolume = self.keyvolume.clamp(0.0, 1.0);
        self.bgvolume = self.bgvolume.clamp(0.0, 1.0);
        true
    }
}
