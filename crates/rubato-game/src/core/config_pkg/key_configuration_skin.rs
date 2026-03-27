/// Key configuration skin.
/// Translated from Java: KeyConfigurationSkin extends Skin
///
/// This is a minimal skin subclass for the key configuration screen.
/// The actual Skin base type is a Phase 5+ stub.
pub struct KeyConfigurationSkin {
    // Skin header is Phase 5+ type
    _header: (),
}

impl KeyConfigurationSkin {
    pub fn new() -> Self {
        Self { _header: () }
    }
}

impl Default for KeyConfigurationSkin {
    fn default() -> Self {
        Self::new()
    }
}
