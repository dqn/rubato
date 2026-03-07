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

    pub fn judgeregion(&self) -> i32 {
        self.judgeregion
    }

    pub fn get_close(&self) -> i32 {
        self.close
    }

    pub fn get_finish_margin(&self) -> i32 {
        self.finish_margin
    }

    pub fn get_playstart(&self) -> i32 {
        self.playstart
    }

    pub fn get_loadstart(&self) -> i32 {
        self.loadstart
    }

    pub fn get_loadend(&self) -> i32 {
        self.loadend
    }

    pub fn get_judgetimer(&self) -> i32 {
        self.judgetimer
    }

    pub fn get_note_expansion_rate(&self) -> &[i32; 2] {
        &self.note_expansion_rate
    }

    pub fn get_lane_group_region(&self) -> Option<&[Rectangle]> {
        self.lanegroupregion.as_deref()
    }

    pub fn get_lane_region(&self) -> Option<&[Rectangle]> {
        self.laneregion.as_deref()
    }

    pub fn get_line(&self) -> &[()] {
        &self.line
    }

    pub fn get_bpm_line(&self) -> &[()] {
        &self.bpm
    }

    pub fn get_stop_line(&self) -> &[()] {
        &self.stop
    }

    pub fn get_time_line(&self) -> &[()] {
        &self.time
    }
}
