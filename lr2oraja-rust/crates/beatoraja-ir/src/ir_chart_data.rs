use std::collections::HashMap;

use bms_model::mode::Mode;

use crate::SongData;

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
        let lntype = if let Some(model) = song.get_bms_model() {
            model.get_lntype()
        } else {
            0
        };
        Self::new_with_lntype(song, lntype)
    }

    /// Convert IRChartData back to SongData.
    /// Translated from: Java BarManager.java inline mapping (lines 141-152, 160-172)
    pub fn to_song_data(&self) -> SongData {
        let mut sd = SongData::default();
        sd.sha256 = self.sha256.clone();
        sd.md5 = self.md5.clone();
        sd.title = self.title.clone();
        sd.artist = self.artist.clone();
        sd.genre = self.genre.clone();
        sd.set_url(self.url.clone());
        sd.set_appendurl(self.appendurl.clone());
        if let Some(ref mode) = self.mode {
            sd.mode = mode.id();
        }
        sd
    }

    pub fn new_with_lntype(song: &SongData, lntype: i32) -> Self {
        let model = song.get_bms_model();
        let total = if let Some(m) = model {
            m.get_total() as i32
        } else {
            0
        };
        let mode = model.and_then(|m| m.get_mode().cloned());

        let mut values = HashMap::new();
        if let Some(m) = model {
            for (k, v) in m.get_values() {
                values.insert(k.clone(), v.clone());
            }
        }

        Self {
            title: song.get_title().to_string(),
            subtitle: song.get_subtitle().to_string(),
            genre: song.get_genre().to_string(),
            artist: song.get_artist().to_string(),
            subartist: song.get_subartist().to_string(),
            md5: song.get_md5().to_string(),
            sha256: song.get_sha256().to_string(),
            url: song.get_url().to_string(),
            appendurl: song.get_appendurl().to_string(),
            level: song.get_level(),
            total,
            mode,
            judge: song.get_judge(),
            minbpm: song.get_minbpm(),
            maxbpm: song.get_maxbpm(),
            notes: song.get_notes(),
            has_undefined_ln: song.has_undefined_long_note(),
            has_ln: song.has_long_note(),
            has_cn: song.has_charge_note(),
            has_hcn: song.has_hell_charge_note(),
            has_mine: song.has_mine_note(),
            has_random: song.has_random_sequence(),
            has_stop: song.is_bpmstop(),
            lntype,
            values,
        }
    }
}
