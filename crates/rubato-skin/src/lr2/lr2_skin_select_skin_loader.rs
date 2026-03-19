use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::reexports::Resolution;

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
                self.csv.process_csv_command(cmd, str_parts, None);
            }
        }
    }

    /// After loading, count skin customize buttons by scanning all skin objects
    /// for the highest-indexed BUTTON_SKIN_CUSTOMIZE click event.
    pub fn count_custom_properties(&mut self, skin: &crate::skin::Skin) {
        use crate::skin_property_mapper::{is_skin_customize_button, skin_customize_index};

        let mut count = 0i32;
        for obj in skin.objects() {
            let id = obj.data().clickevent_id();
            if is_skin_customize_button(id) {
                let index = skin_customize_index(id);
                if count <= index {
                    count = index + 1;
                }
            }
        }
        self.custom_property_count = count;
    }
}

impl LR2SkinLoaderAccess for LR2SkinSelectSkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn load_skin_data(
        &mut self,
        path: &std::path::Path,
        state: Option<&dyn crate::reexports::MainState>,
    ) -> anyhow::Result<()> {
        let raw_bytes = std::fs::read(path)?;
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&raw_bytes);
        let content = decoded.into_owned();

        for line in content.lines() {
            self.csv.line = Some(line.to_string());
            if let Some((cmd, str_parts)) = self.csv.base.process_line_directives(line, state) {
                self.process_skin_select_command(&cmd, &str_parts);
            }
        }

        self.csv.finalize_active_objects();
        Ok(())
    }

    fn assemble_objects(&mut self, skin: &mut crate::skin::Skin) {
        // Transfer generic objects from base CSV parser.
        for obj in self.csv.collected_objects.drain(..) {
            skin.add(obj);
        }

        // Count custom property buttons after all objects are assembled.
        self.count_custom_properties(skin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::objects::skin_image::SkinImage;
    use crate::skin::Skin;
    use crate::skin_header::SkinHeader;
    use crate::skin_property::{BUTTON_SKIN_CUSTOMIZE1, BUTTON_SKIN_CUSTOMIZE10};
    use crate::types::skin::SkinObject;

    fn make_image_with_clickevent(event_id: i32) -> SkinObject {
        let mut img = SkinImage::new_empty();
        img.data.set_clickevent_by_id(event_id);
        SkinObject::Image(img)
    }

    fn make_loader() -> LR2SkinSelectSkinLoaderState {
        let res = Resolution {
            width: 640.0,
            height: 480.0,
        };
        LR2SkinSelectSkinLoaderState::new(res.clone(), res, false, String::new())
    }

    #[test]
    fn count_custom_properties_empty_skin() {
        let mut loader = make_loader();
        let skin = Skin::new(SkinHeader::new());
        loader.count_custom_properties(&skin);
        assert_eq!(loader.custom_property_count, 0);
    }

    #[test]
    fn count_custom_properties_single_slot() {
        let mut loader = make_loader();
        let mut skin = Skin::new(SkinHeader::new());
        // Add object with customize button slot 1 (ID 220, index 0)
        skin.add(make_image_with_clickevent(BUTTON_SKIN_CUSTOMIZE1));
        loader.count_custom_properties(&skin);
        assert_eq!(loader.custom_property_count, 1);
    }

    #[test]
    fn count_custom_properties_slot_10() {
        let mut loader = make_loader();
        let mut skin = Skin::new(SkinHeader::new());
        // Add object with customize button slot 10 (ID 229, index 9)
        skin.add(make_image_with_clickevent(BUTTON_SKIN_CUSTOMIZE10));
        loader.count_custom_properties(&skin);
        assert_eq!(loader.custom_property_count, 10);
    }

    #[test]
    fn count_custom_properties_highest_wins() {
        let mut loader = make_loader();
        let mut skin = Skin::new(SkinHeader::new());
        // Add slots 1, 5, 3 (IDs 220, 224, 222)
        skin.add(make_image_with_clickevent(BUTTON_SKIN_CUSTOMIZE1));
        skin.add(make_image_with_clickevent(BUTTON_SKIN_CUSTOMIZE1 + 4));
        skin.add(make_image_with_clickevent(BUTTON_SKIN_CUSTOMIZE1 + 2));
        loader.count_custom_properties(&skin);
        // Highest index is 4 (slot 5), so count = 4 + 1 = 5
        assert_eq!(loader.custom_property_count, 5);
    }

    #[test]
    fn count_custom_properties_ignores_non_customize_buttons() {
        let mut loader = make_loader();
        let mut skin = Skin::new(SkinHeader::new());
        // Add a non-customize click event (ID 100)
        skin.add(make_image_with_clickevent(100));
        // Add one just below range
        skin.add(make_image_with_clickevent(BUTTON_SKIN_CUSTOMIZE1 - 1));
        // Add one just above range
        skin.add(make_image_with_clickevent(BUTTON_SKIN_CUSTOMIZE10 + 1));
        loader.count_custom_properties(&skin);
        assert_eq!(loader.custom_property_count, 0);
    }
}
