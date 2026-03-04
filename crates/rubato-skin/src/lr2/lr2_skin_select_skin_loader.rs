use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::stubs::Resolution;

/// LR2 skin select skin loader
///
/// Translated from LR2SkinSelectSkinLoader.java (46 lines)
/// Loads LR2 skin configuration/selection skins.
/// Adds SAMPLEBMS command and counts custom property buttons.
///
/// Skin select skin loader state
pub struct LR2SkinSelectSkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,
    pub sample_bms: Vec<String>,
    pub custom_property_count: i32,
}

impl LR2SkinSelectSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
            sample_bms: Vec::new(),
            custom_property_count: 0,
        }
    }

    /// Process skin select-specific commands
    pub fn process_skin_select_command(&mut self, cmd: &str, str_parts: &[String]) {
        match cmd {
            "SAMPLEBMS" => {
                if str_parts.len() > 1 {
                    self.sample_bms = vec![str_parts[1].clone()];
                    // skin.setSampleBMS(new String[] {str[1]})
                }
            }
            _ => {
                self.csv.process_csv_command(cmd, str_parts);
            }
        }
    }

    /// After loading, count skin customize buttons
    pub fn count_custom_properties(&mut self) {
        // int count = 0;
        // for (SkinObject obj : skin.getAllSkinObjects()) {
        //     if (SkinPropertyMapper.isSkinCustomizeButton(obj.getClickeventId())) {
        //         int index = SkinPropertyMapper.getSkinCustomizeIndex(obj.getClickeventId());
        //         if (count <= index)
        //             count = index + 1;
        //     }
        // }
        // skin.setCustomPropertyCount(count);
        // This requires SkinPropertyMapper and full skin object access
    }
}

impl LR2SkinLoaderAccess for LR2SkinSelectSkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn assemble_objects(&mut self, _skin: &mut crate::skin::Skin) {
        // Skin select skin has no LR2-specific objects beyond generic SRC/DST images.
    }
}
