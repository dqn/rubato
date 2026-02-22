use crate::bar_renderer::BarRenderer;
use crate::skin_distribution_graph::SkinDistributionGraph;
use crate::stubs::*;

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
    pub text: Vec<Option<SkinText>>,
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
        Self {
            barimageon: vec![None; Self::BAR_COUNT],
            barimageoff: vec![None; Self::BAR_COUNT],
            trophy: vec![None; Self::BARTROPHY_COUNT],
            text: vec![None; Self::BARTEXT_COUNT],
            barlevel: vec![None; Self::BARLEVEL_COUNT],
            label: vec![None; Self::BARLABEL_COUNT],
            graph: None,
            position,
            lamp: vec![None; Self::BARLAMP_COUNT],
            mylamp: vec![None; Self::BARLAMP_COUNT],
            rivallamp: vec![None; Self::BARLAMP_COUNT],
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

    pub fn get_bar_images(&self, on: bool, index: usize) -> Option<&SkinImage> {
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

    pub fn get_lamp(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.lamp.len() {
            self.lamp[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn get_player_lamp(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.mylamp.len() {
            self.mylamp[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn get_rival_lamp(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.rivallamp.len() {
            self.rivallamp[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn get_trophy(&self, id: i32) -> Option<&SkinImage> {
        if id >= 0 && (id as usize) < self.trophy.len() {
            self.trophy[id as usize].as_ref()
        } else {
            None
        }
    }

    pub fn get_text(&self, id: usize) -> Option<&SkinText> {
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

    pub fn set_text(&mut self, id: usize, text: SkinText) {
        if id < self.text.len() {
            self.text[id] = Some(text);
        }
    }

    pub fn set_rival_lamp(&mut self, id: i32, rivallamp: SkinImage) {
        if id >= 0 && (id as usize) < self.rivallamp.len() {
            self.rivallamp[id as usize] = Some(rivallamp);
        }
    }

    pub fn validate(&mut self) -> bool {
        // In Java: validates all sub-objects, removing invalid ones
        // Stub: always valid
        true
    }

    pub fn prepare(&mut self, _time: i64, _state: &dyn MainState) {
        // In Java: prepares all sub-objects and calls render.prepare(this, time)
        log::warn!(
            "not yet implemented: SkinBar.prepare - requires BarRenderer and rendering integration"
        );
    }

    pub fn draw(&self, _sprite: &SkinObjectRenderer) {
        // In Java: render.render(sprite, this)
        log::warn!(
            "not yet implemented: SkinBar.draw - requires BarRenderer and rendering integration"
        );
    }

    pub fn dispose(&self) {
        // In Java: disposes all sub-objects
    }

    pub fn get_barlevel(&self, id: i32) -> Option<&SkinNumber> {
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

    pub fn get_position(&self) -> i32 {
        self.position
    }

    pub fn get_label(&self, id: i32) -> Option<&SkinImage> {
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

    pub fn mouse_pressed(&self, state: &dyn MainState, button: i32, x: i32, y: i32) -> bool {
        // In Java: return ((MusicSelector) state).getBarRender().mousePressed(this, button, x, y)
        // Stubbed since we don't have downcast to MusicSelector
        log::warn!("not yet implemented: SkinBar.mousePressed - requires MusicSelector downcast");
        false
    }

    pub fn get_graph(&self) -> Option<&SkinDistributionGraph> {
        self.graph.as_ref()
    }

    pub fn set_graph(&mut self, graph: SkinDistributionGraph) {
        self.graph = Some(graph);
    }
}
