use super::skin_distribution_graph::SkinDistributionGraph;
use super::*;

/// Skin object for rendering song bars
/// Translates: bms.player.beatoraja.select.SkinBar
pub struct SkinBar {
    /// SkinImage for selected bars
    pub barimageon: Vec<Option<SkinImage>>,
    /// SkinImage for non-selected bars
    pub barimageoff: Vec<Option<SkinImage>>,
    /// Trophy SkinImages (relative to bar position)
    pub trophy: Vec<Option<SkinImage>>,
    /// Bar text SkinText objects
    /// Index: 0=normal, 1=new, 2=SongBar(normal), 3=SongBar(new), 4=FolderBar(normal),
    /// 5=FolderBar(new), 6=TableBar/HashBar, 7=GradeBar(songs exist),
    /// 8=(SongBar/GradeBar)(songs missing), 9=CommandBar/ContainerBar, 10=SearchWordBar
    pub text: Vec<Option<SkinTextEnum>>,
    /// Level SkinNumbers (relative to bar position)
    pub barlevel: Vec<Option<SkinNumber>>,
    /// Label SkinImages (relative to bar position)
    pub label: Vec<Option<SkinImage>>,
    /// Distribution graph
    pub graph: Option<SkinDistributionGraph>,
    /// Position mode
    pub position: i32,
    /// Lamp images
    pub lamp: Vec<Option<SkinImage>>,
    /// Player lamp images (for rival display)
    pub mylamp: Vec<Option<SkinImage>>,
    /// Rival lamp images (for rival display)
    pub rivallamp: Vec<Option<SkinImage>>,
    /// SkinObject base data
    pub draw: bool,
    pub region: SkinRegion,
}

impl SkinBar {
    pub const BAR_COUNT: usize = 60;
    pub const BARTROPHY_COUNT: usize = 3;
    pub const BARTEXT_NORMAL: usize = 0;
    pub const BARTEXT_NEW: usize = 1;
    pub const BARTEXT_SONG_NORMAL: usize = 2;
    pub const BARTEXT_SONG_NEW: usize = 3;
    pub const BARTEXT_FOLDER_NORMAL: usize = 4;
    pub const BARTEXT_FOLDER_NEW: usize = 5;
    pub const BARTEXT_TABLE: usize = 6;
    pub const BARTEXT_GRADE: usize = 7;
    pub const BARTEXT_NO_SONGS: usize = 8;
    pub const BARTEXT_COMMAND: usize = 9;
    pub const BARTEXT_SEARCH: usize = 10;
    pub const BARTEXT_COUNT: usize = 11;
    pub const BARLEVEL_COUNT: usize = 7;
    pub const BARLABEL_COUNT: usize = 5;
    pub const BARLAMP_COUNT: usize = 11;

    pub fn new(position: i32) -> Self {
        // Real SkinImage/SkinNumber/SkinTextEnum are not Clone,
        // so use repeat_with instead of vec![None; N].
        let none_images = |n| {
            std::iter::repeat_with(|| None::<SkinImage>)
                .take(n)
                .collect()
        };
        let none_numbers = |n| {
            std::iter::repeat_with(|| None::<SkinNumber>)
                .take(n)
                .collect()
        };
        let text: Vec<Option<SkinTextEnum>> = std::iter::repeat_with(|| None)
            .take(Self::BARTEXT_COUNT)
            .collect();
        Self {
            barimageon: none_images(Self::BAR_COUNT),
            barimageoff: none_images(Self::BAR_COUNT),
            trophy: none_images(Self::BARTROPHY_COUNT),
            text,
            barlevel: none_numbers(Self::BARLEVEL_COUNT),
            label: none_images(Self::BARLABEL_COUNT),
            graph: None,
            position,
            lamp: none_images(Self::BARLAMP_COUNT),
            mylamp: none_images(Self::BARLAMP_COUNT),
            rivallamp: none_images(Self::BARLAMP_COUNT),
            draw: false,
            region: SkinRegion::default(),
        }
    }

    pub fn set_bar_image(
        &mut self,
        onimage: Vec<Option<SkinImage>>,
        offimage: Vec<Option<SkinImage>>,
    ) {
        self.barimageon = onimage;
        self.barimageoff = offimage;
    }

    pub fn bar_images(&self, on: bool, index: usize) -> Option<&SkinImage> {
        if index < self.barimageoff.len() {
            if on {
                self.barimageon[index].as_ref()
            } else {
                self.barimageoff[index].as_ref()
            }
        } else {
            None
        }
    }

    pub fn lamp(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.lamp.len() {
            self.lamp[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn player_lamp(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.mylamp.len() {
            self.mylamp[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn rival_lamp(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.rivallamp.len() {
            self.rivallamp[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn trophy(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.trophy.len() {
            self.trophy[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn text(&self, id: usize) -> Option<&SkinTextEnum> {
        if id < self.text.len() {
            self.text[id].as_ref()
        } else {
            None
        }
    }

    pub fn set_trophy(&mut self, id: i32, trophy: SkinImage) {
        if id >= 0 && (id as usize) < self.trophy.len() {
            self.trophy[id as usize] = Some(trophy);
        }
    }

    pub fn set_lamp_image(&mut self, id: i32, lamp: SkinImage) {
        if id >= 0 && (id as usize) < self.lamp.len() {
            self.lamp[id as usize] = Some(lamp);
        }
    }

    pub fn set_player_lamp(&mut self, id: i32, mylamp: SkinImage) {
        if id >= 0 && (id as usize) < self.mylamp.len() {
            self.mylamp[id as usize] = Some(mylamp);
        }
    }

    pub fn set_text(&mut self, id: usize, text: SkinTextEnum) {
        if id < self.text.len() {
            self.text[id] = Some(text);
        }
    }

    pub fn set_rival_lamp(&mut self, id: i32, rivallamp: SkinImage) {
        if id >= 0 && (id as usize) < self.rivallamp.len() {
            self.rivallamp[id as usize] = Some(rivallamp);
        }
    }

    /// Validate all sub-objects, removing invalid ones.
    /// Translates: Java SkinBar.validate()
    pub fn validate(&mut self) -> bool {
        fn validate_images(images: &mut [Option<SkinImage>]) {
            for img in images.iter_mut() {
                if img.as_mut().is_some_and(|i| !i.validate()) {
                    *img = None;
                }
            }
        }

        validate_images(&mut self.barimageon);
        validate_images(&mut self.barimageoff);
        validate_images(&mut self.trophy);
        validate_images(&mut self.label);
        validate_images(&mut self.lamp);
        validate_images(&mut self.mylamp);
        validate_images(&mut self.rivallamp);
        // SkinText trait doesn't expose validate; validate underlying SkinObjectData
        for txt in self.text.iter_mut() {
            if txt
                .as_ref()
                .is_some_and(|t| !t.get_text_data().data.validate())
            {
                *txt = None;
            }
        }
        true
    }

    /// Prepare all sub-objects for rendering.
    /// In Java: prepares all child SkinImage/SkinText/SkinNumber, then calls render.prepare(this, time).
    /// In Rust: sub-object preparation is done here. BarRenderer.prepare() is called separately
    /// by MusicSelector since it requires context (center_bar, currentsongs, selectedindex)
    /// that can't be obtained from &dyn MainState without downcasting.
    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        // Prepare all child skin objects (real types need &mut self)
        for bar in self.barimageon.iter_mut().flatten() {
            bar.prepare(time, state);
        }
        for bar in self.barimageoff.iter_mut().flatten() {
            bar.prepare(time, state);
        }
        for trophy in self.trophy.iter_mut().flatten() {
            trophy.prepare(time, state);
        }
        for text in self.text.iter_mut().flatten() {
            text.get_text_data_mut().prepare(time, state);
        }
        for barlevel in self.barlevel.iter_mut().flatten() {
            barlevel.prepare(time, state);
        }
        for label in self.label.iter_mut().flatten() {
            label.prepare(time, state);
        }
        for lamp in self.lamp.iter_mut().flatten() {
            lamp.prepare(time, state);
        }
        for mylamp in self.mylamp.iter_mut().flatten() {
            mylamp.prepare(time, state);
        }
        for rivallamp in self.rivallamp.iter_mut().flatten() {
            rivallamp.prepare(time, state);
        }
        if let Some(ref mut graph) = self.graph {
            graph.prepare(time, state);
        }
        // NOTE: BarRenderer.prepare(baro, time, ctx) is called by MusicSelector
        // after this method, since it requires PrepareContext with center_bar, etc.
    }

    /// Draw all bar elements.
    /// In Java: render.render(sprite, this).
    /// In Rust: BarRenderer.render() is called separately by MusicSelector
    /// since it requires RenderContext with center_bar, currentsongs, rival, etc.
    pub fn draw(&mut self, _sprite: &mut SkinObjectRenderer) {
        // NOTE: BarRenderer.render(sprite, baro, ctx) is called by MusicSelector
        // after prepare(), since it requires RenderContext.
    }

    pub fn dispose(&mut self) {
        for img in self.barimageon.iter_mut().flatten() {
            img.dispose();
        }
        for img in self.barimageoff.iter_mut().flatten() {
            img.dispose();
        }
        for img in self.trophy.iter_mut().flatten() {
            img.dispose();
        }
        for txt in self.text.iter_mut().flatten() {
            txt.dispose();
        }
        for num in self.barlevel.iter_mut().flatten() {
            num.dispose();
        }
        for img in self.label.iter_mut().flatten() {
            img.dispose();
        }
        for img in self.lamp.iter_mut().flatten() {
            img.dispose();
        }
        for img in self.mylamp.iter_mut().flatten() {
            img.dispose();
        }
        for img in self.rivallamp.iter_mut().flatten() {
            img.dispose();
        }
    }

    pub fn barlevel(&self, id: i32) -> Option<&SkinNumber> {
        if id >= 0 && (id as usize) < self.barlevel.len() {
            self.barlevel[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn set_barlevel(&mut self, id: i32, barlevel: SkinNumber) {
        if id >= 0 && (id as usize) < self.barlevel.len() {
            self.barlevel[id as usize] = Some(barlevel);
        }
    }

    pub fn position(&self) -> i32 {
        self.position
    }

    pub fn label(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.label.len() {
            self.label[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn set_label(&mut self, id: i32, label: SkinImage) {
        if id >= 0 && (id as usize) < self.label.len() {
            self.label[id as usize] = Some(label);
        }
    }

    /// Handle mouse press on bar.
    /// In Java: return ((MusicSelector) state).getBarRender().mousePressed(this, button, x, y).
    /// In Rust: BarRenderer.mouse_pressed() is called separately by MusicSelector
    /// since it requires MousePressedContext. This stub returns false for API compatibility.
    pub fn mouse_pressed(&self, _state: &dyn MainState, _button: i32, _x: i32, _y: i32) -> bool {
        // NOTE: BarRenderer.mouse_pressed(baro, button, x, y, ctx) is called by MusicSelector.
        false
    }

    pub fn graph(&self) -> Option<&SkinDistributionGraph> {
        self.graph.as_ref()
    }

    pub fn set_graph(&mut self, graph: SkinDistributionGraph) {
        self.graph = Some(graph);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skin_bar_new_initializes_arrays() {
        let bar = SkinBar::new(0);
        assert_eq!(bar.barimageon.len(), SkinBar::BAR_COUNT);
        assert_eq!(bar.barimageoff.len(), SkinBar::BAR_COUNT);
        assert_eq!(bar.trophy.len(), SkinBar::BARTROPHY_COUNT);
        assert_eq!(bar.text.len(), SkinBar::BARTEXT_COUNT);
        assert_eq!(bar.barlevel.len(), SkinBar::BARLEVEL_COUNT);
        assert_eq!(bar.label.len(), SkinBar::BARLABEL_COUNT);
        assert_eq!(bar.lamp.len(), SkinBar::BARLAMP_COUNT);
        assert_eq!(bar.position, 0);
        assert!(!bar.draw);
    }

    #[test]
    fn test_skin_bar_two_phase_prepare_draw_signatures() {
        let mut bar = SkinBar::new(0);
        assert!(!bar.draw);
        let mut renderer = SkinObjectRenderer::new();
        bar.draw(&mut renderer);
    }

    #[test]
    fn test_skin_bar_position_preserved() {
        let bar = SkinBar::new(1);
        assert_eq!(bar.position(), 1);
    }

    #[test]
    fn test_skin_bar_get_bar_images_bounds() {
        let bar = SkinBar::new(0);
        assert!(bar.bar_images(true, 0).is_none());
        assert!(bar.bar_images(false, 0).is_none());
        assert!(bar.bar_images(true, SkinBar::BAR_COUNT).is_none());
    }

    #[test]
    fn test_skin_bar_accessors_bounds_checked() {
        let bar = SkinBar::new(0);
        assert!(bar.lamp(-1).is_none());
        assert!(bar.lamp(0).is_none());
        assert!(bar.lamp(SkinBar::BARLAMP_COUNT as i32).is_none());
        assert!(bar.trophy(-1).is_none());
        assert!(bar.trophy(0).is_none());
        assert!(bar.text(SkinBar::BARTEXT_COUNT).is_none());
        assert!(bar.barlevel(-1).is_none());
        assert!(bar.label(-1).is_none());
    }
}
