use crate::pomyu_chara_processor::PomyuCharaProcessor;
use rubato_render::color::Rectangle;

/// Play skin
pub struct PlaySkin {
    /// Margin from STATE_READY to STATE_PLAY (ms)
    pub playstart: i32,
    /// Section line images
    pub line: Vec<()>,
    /// Timeline images
    pub time: Vec<()>,
    /// BPM line images
    pub bpm: Vec<()>,
    /// Stop line images
    pub stop: Vec<()>,
    /// Lane region per lane
    pub laneregion: Option<Vec<Rectangle>>,
    /// Lane group region per player
    pub lanegroupregion: Option<Vec<Rectangle>>,
    /// Judge region count
    pub judgeregion: i32,
    /// Margin from STATE_FAILED to exit (ms)
    pub close: i32,
    /// Margin from STATE_FINISHED to fadeout (ms)
    pub finish_margin: i32,
    pub loadstart: i32,
    pub loadend: i32,
    /// Judge timer trigger condition (0:PG, 1:GR, 2:GD, 3:BD)
    pub judgetimer: i32,
    /// PMS rhythm-based note expansion rate (%) [w, h]
    pub note_expansion_rate: [i32; 2],
    /// PMS character processor
    pub pomyu: PomyuCharaProcessor,
}

impl Default for PlaySkin {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaySkin {
    pub fn new() -> Self {
        PlaySkin {
            playstart: 0,
            line: Vec::new(),
            time: Vec::new(),
            bpm: Vec::new(),
            stop: Vec::new(),
            laneregion: None,
            lanegroupregion: None,
            judgeregion: 0,
            close: 0,
            finish_margin: 0,
            loadstart: 0,
            loadend: 0,
            judgetimer: 1,
            note_expansion_rate: [100, 100],
            pomyu: PomyuCharaProcessor::new(),
        }
    }

    pub fn lane_group_region(&self) -> Option<&[Rectangle]> {
        self.lanegroupregion.as_deref()
    }

    pub fn lane_region(&self) -> Option<&[Rectangle]> {
        self.laneregion.as_deref()
    }
}
