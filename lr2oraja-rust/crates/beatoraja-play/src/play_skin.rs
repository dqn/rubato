use crate::pomyu_chara_processor::PomyuCharaProcessor;
use beatoraja_render::color::Rectangle;

/// Play skin
pub struct PlaySkin {
    /// Margin from STATE_READY to STATE_PLAY (ms)
    playstart: i32,
    /// Section line images
    line: Vec<()>,
    /// Timeline images
    time: Vec<()>,
    /// BPM line images
    bpm: Vec<()>,
    /// Stop line images
    stop: Vec<()>,
    /// Lane region per lane
    laneregion: Option<Vec<Rectangle>>,
    /// Lane group region per player
    lanegroupregion: Option<Vec<Rectangle>>,
    /// Judge region count
    judgeregion: i32,
    /// Margin from STATE_FAILED to exit (ms)
    close: i32,
    /// Margin from STATE_FINISHED to fadeout (ms)
    finish_margin: i32,
    loadstart: i32,
    loadend: i32,
    /// Judge timer trigger condition (0:PG, 1:GR, 2:GD, 3:BD)
    judgetimer: i32,
    /// PMS rhythm-based note expansion rate (%) [w, h]
    note_expansion_rate: [i32; 2],
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

    pub fn get_judgeregion(&self) -> i32 {
        self.judgeregion
    }

    pub fn set_judgeregion(&mut self, jr: i32) {
        self.judgeregion = jr;
    }

    pub fn get_close(&self) -> i32 {
        self.close
    }

    pub fn set_close(&mut self, close: i32) {
        self.close = close;
    }

    pub fn get_finish_margin(&self) -> i32 {
        self.finish_margin
    }

    pub fn set_finish_margin(&mut self, finish_margin: i32) {
        self.finish_margin = finish_margin;
    }

    pub fn get_playstart(&self) -> i32 {
        self.playstart
    }

    pub fn set_playstart(&mut self, playstart: i32) {
        self.playstart = playstart;
    }

    pub fn get_loadstart(&self) -> i32 {
        self.loadstart
    }

    pub fn set_loadstart(&mut self, loadstart: i32) {
        self.loadstart = loadstart;
    }

    pub fn get_loadend(&self) -> i32 {
        self.loadend
    }

    pub fn set_loadend(&mut self, loadend: i32) {
        self.loadend = loadend;
    }

    pub fn get_judgetimer(&self) -> i32 {
        self.judgetimer
    }

    pub fn set_judgetimer(&mut self, judgetimer: i32) {
        self.judgetimer = judgetimer;
    }

    pub fn get_note_expansion_rate(&self) -> &[i32; 2] {
        &self.note_expansion_rate
    }

    pub fn set_note_expansion_rate(&mut self, rate: [i32; 2]) {
        self.note_expansion_rate = rate;
    }

    pub fn get_lane_group_region(&self) -> Option<&[Rectangle]> {
        self.lanegroupregion.as_deref()
    }

    pub fn set_lane_group_region(&mut self, r: Option<Vec<Rectangle>>) {
        self.lanegroupregion = r;
    }

    pub fn get_lane_region(&self) -> Option<&[Rectangle]> {
        self.laneregion.as_deref()
    }

    pub fn set_lane_region(&mut self, r: Option<Vec<Rectangle>>) {
        self.laneregion = r;
    }

    pub fn get_line(&self) -> &[()] {
        &self.line
    }

    pub fn set_line(&mut self, line: Vec<()>) {
        self.line = line;
    }

    pub fn get_bpm_line(&self) -> &[()] {
        &self.bpm
    }

    pub fn set_bpm_line(&mut self, bpm: Vec<()>) {
        self.bpm = bpm;
    }

    pub fn get_stop_line(&self) -> &[()] {
        &self.stop
    }

    pub fn set_stop_line(&mut self, stop: Vec<()>) {
        self.stop = stop;
    }

    pub fn get_time_line(&self) -> &[()] {
        &self.time
    }

    pub fn set_time_line(&mut self, time: Vec<()>) {
        self.time = time;
    }
}
