use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Bmson {
    pub version: Option<String>,
    pub info: BMSInfo,
    pub lines: Vec<BarLine>,
    pub bpm_events: Vec<BpmEvent>,
    pub stop_events: Vec<StopEvent>,
    pub scroll_events: Vec<ScrollEvent>,
    pub sound_channels: Vec<SoundChannel>,
    pub bga: Option<BGA>,
    pub mine_channels: Vec<MineChannel>,
    pub key_channels: Vec<MineChannel>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct BMSInfo {
    pub title: String,
    pub subtitle: Option<String>,
    pub genre: String,
    pub artist: String,
    pub subartists: Vec<String>,
    pub mode_hint: String,
    pub chart_name: Option<String>,
    pub judge_rank: i32,
    pub total: f64,
    pub init_bpm: f64,
    pub level: i32,
    pub back_image: String,
    pub eyecatch_image: String,
    pub banner_image: String,
    pub preview_music: String,
    pub resolution: i32,
    pub ln_type: i32,
}

impl Default for BMSInfo {
    fn default() -> Self {
        BMSInfo {
            title: String::new(),
            subtitle: Some(String::new()),
            genre: String::new(),
            artist: String::new(),
            subartists: Vec::new(),
            mode_hint: "beat-7k".to_string(),
            chart_name: Some(String::new()),
            judge_rank: 100,
            total: 100.0,
            init_bpm: 0.0,
            level: 0,
            back_image: String::new(),
            eyecatch_image: String::new(),
            banner_image: String::new(),
            preview_music: String::new(),
            resolution: 240,
            ln_type: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct BarLine {
    pub y: i32,
    pub k: i32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct BpmEvent {
    pub y: i32,
    pub bpm: f64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct StopEvent {
    pub y: i32,
    pub duration: i64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct ScrollEvent {
    pub y: i32,
    pub rate: f64,
}

impl Default for ScrollEvent {
    fn default() -> Self {
        ScrollEvent { y: 0, rate: 1.0 }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Note {
    pub y: i32,
    pub x: i32,
    pub l: i32,
    pub c: bool,
    pub t: i32,
    pub up: bool,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct MineNote {
    pub y: i32,
    pub x: i32,
    pub damage: f64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct BNote {
    pub y: i32,
    pub id: i32,
    pub id_set: Option<Vec<i32>>,
    pub condition: Option<String>,
    pub interval: i32,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct BGAHeader {
    pub id: i32,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(default)]
pub struct Sequence {
    pub time: i64,
    pub id: i32,
}

impl Default for Sequence {
    fn default() -> Self {
        Sequence {
            time: 0,
            id: i32::MIN,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct BGASequence {
    pub id: i32,
    pub sequence: Vec<Sequence>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SoundChannel {
    pub name: String,
    pub notes: Vec<Note>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct MineChannel {
    pub name: String,
    pub notes: Vec<MineNote>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct BGA {
    pub bga_header: Option<Vec<BGAHeader>>,
    pub bga_sequence: Option<Vec<BGASequence>>,
    pub bga_events: Option<Vec<BNote>>,
    pub layer_events: Option<Vec<BNote>>,
    pub poor_events: Option<Vec<BNote>>,
}
