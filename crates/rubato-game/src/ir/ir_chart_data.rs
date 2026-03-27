use std::collections::HashMap;

use bms::model::mode::Mode;

use crate::ir::SongData;

/// IR chart data
///
/// Translated from: IRChartData.java
#[derive(Clone, Debug, Default)]
pub struct IRChartData {
    /// Chart MD5 hash
    pub md5: String,
    /// Chart SHA256 hash
    pub sha256: String,
    /// Chart title
    pub title: String,
    /// Chart subtitle
    pub subtitle: String,
    /// Chart genre
    pub genre: String,
    /// Chart artist
    pub artist: String,
    /// Chart subartist
    pub subartist: String,
    /// Song download URL
    pub url: String,
    /// Append chart download URL
    pub appendurl: String,
    /// Level
    pub level: i32,
    /// TOTAL value
    pub total: i32,
    /// Mode
    pub mode: Option<Mode>,
    /// LN TYPE (-1: unspecified, 0: LN, 1: CN, 2: HCN)
    pub lntype: i32,
    /// Judge width (bmson judgerank notation)
    pub judge: i32,
    /// Minimum BPM
    pub minbpm: i32,
    /// Maximum BPM
    pub maxbpm: i32,
    /// Total note count
    pub notes: i32,
    /// Whether undefined LN type long notes exist
    pub has_undefined_ln: bool,
    /// Whether LN exists
    pub has_ln: bool,
    /// Whether CN exists
    pub has_cn: bool,
    /// Whether HCN exists
    pub has_hcn: bool,
    /// Whether mine notes exist
    pub has_mine: bool,
    /// Whether RANDOM definitions exist
    pub has_random: bool,
    /// Whether stop sequences exist
    pub has_stop: bool,
    /// Additional values
    pub values: HashMap<String, String>,
}

impl IRChartData {
    pub fn new(song: &SongData) -> Self {
        let lntype = if let Some(model) = song.bms_model() {
            model.lntype().as_i32()
        } else {
            0
        };
        Self::new_with_lntype(song, lntype)
    }

    /// Convert IRChartData back to SongData.
    /// Translated from: Java BarManager.java inline mapping (lines 141-152, 160-172)
    pub fn to_song_data(&self) -> SongData {
        let mut sd = SongData::default();
        sd.file.sha256 = self.sha256.clone();
        sd.file.md5 = self.md5.clone();
        sd.metadata.title = self.title.clone();
        sd.metadata.artist = self.artist.clone();
        sd.metadata.genre = self.genre.clone();
        sd.url = Some(self.url.clone());
        sd.appendurl = Some(self.appendurl.clone());
        if let Some(ref mode) = self.mode {
            sd.chart.mode = mode.id();
        }
        sd
    }

    pub fn new_with_lntype(song: &SongData, lntype: i32) -> Self {
        let model = song.bms_model();
        let total = if let Some(m) = model {
            m.total as i32
        } else {
            0
        };
        let mode = model.and_then(|m| m.mode().copied());

        let values = if let Some(m) = model {
            m.values.clone()
        } else {
            HashMap::new()
        };

        Self {
            title: song.metadata.title.clone(),
            subtitle: song.metadata.subtitle.clone(),
            genre: song.metadata.genre.clone(),
            artist: song.metadata.artist.clone(),
            subartist: song.metadata.subartist.clone(),
            md5: song.file.md5.clone(),
            sha256: song.file.sha256.clone(),
            url: song.url().to_string(),
            appendurl: song.appendurl().to_string(),
            level: song.chart.level,
            total,
            mode,
            judge: song.chart.judge,
            minbpm: song.chart.minbpm,
            maxbpm: song.chart.maxbpm,
            notes: song.chart.notes,
            has_undefined_ln: song.chart.has_undefined_long_note(),
            has_ln: song.chart.has_long_note(),
            has_cn: song.chart.has_charge_note(),
            has_hcn: song.chart.has_hell_charge_note(),
            has_mine: song.chart.has_mine_note(),
            has_random: song.chart.has_random_sequence(),
            has_stop: song.chart.is_bpmstop(),
            lntype,
            values,
        }
    }
}
