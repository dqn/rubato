/// Skin configuration skin.
/// Translated from Java: SkinConfigurationSkin extends Skin
pub struct SkinConfigurationSkin {
    sample_bms: Vec<String>,
    default_skin_type: i32,
    custom_property_count: i32,
    custom_offset_style: i32,
    // Skin header is Phase 5+ type
    _header: (),
}

impl SkinConfigurationSkin {
    pub fn new() -> Self {
        Self {
            sample_bms: Vec::new(),
            default_skin_type: 0,
            custom_property_count: -1,
            custom_offset_style: 0,
            _header: (),
        }
    }

    pub fn set_sample_bms(&mut self, sample_bms: Vec<String>) {
        self.sample_bms = sample_bms;
    }

    pub fn get_sample_bms(&self) -> &[String] {
        &self.sample_bms
    }

    pub fn set_default_skin_type(&mut self, default_skin_type: i32) {
        self.default_skin_type = default_skin_type;
    }

    pub fn get_default_skin_type(&self) -> i32 {
        self.default_skin_type
    }

    pub fn set_custom_offset_style(&mut self, custom_offset_style: i32) {
        self.custom_offset_style = custom_offset_style;
    }

    pub fn get_custom_offset_style(&self) -> i32 {
        self.custom_offset_style
    }

    pub fn set_custom_property_count(&mut self, count: i32) {
        self.custom_property_count = count;
    }

    pub fn get_custom_property_count(&self) -> i32 {
        self.custom_property_count
    }
}

impl Default for SkinConfigurationSkin {
    fn default() -> Self {
        Self::new()
    }
}
