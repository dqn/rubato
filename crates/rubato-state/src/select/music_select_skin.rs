use super::stubs::*;

/// Music select skin
/// Translates: bms.player.beatoraja.select.MusicSelectSkin
pub struct MusicSelectSkin {
    pub header: SkinHeader,
    /// Index of the bar where the cursor is
    pub center_bar: i32,
    /// Indices of clickable bars
    pub clickable_bar: Vec<i32>,
    pub search_text: Option<Box<dyn SkinText>>,
    pub search: Option<Rectangle>,
}

impl MusicSelectSkin {
    pub fn new(header: SkinHeader) -> Self {
        Self {
            header,
            center_bar: 0,
            clickable_bar: Vec::new(),
            search_text: None,
            search: None,
        }
    }

    pub fn get_clickable_bar(&self) -> &[i32] {
        &self.clickable_bar
    }

    pub fn set_clickable_bar(&mut self, clickable_bar: Vec<i32>) {
        self.clickable_bar = clickable_bar;
    }

    pub fn get_center_bar(&self) -> i32 {
        self.center_bar
    }

    pub fn set_center_bar(&mut self, center_bar: i32) {
        self.center_bar = center_bar;
    }

    pub fn get_search_text_region(&self) -> Option<&Rectangle> {
        self.search.as_ref()
    }

    pub fn set_search_text_region(&mut self, r: Option<Rectangle>) {
        self.search = r;
    }
}
