// Mechanical translation of JsonSkin.java
// JSON skin model/data classes

mod deserializers;

use deserializers::{
    deserialize_animations_with_conditionals, deserialize_flattened_conditional_destinations,
    deserialize_flattened_conditional_images, deserialize_flattened_conditional_texts,
    deserialize_i32_lenient, deserialize_optional_i32_or_string,
    deserialize_optional_string_from_int, deserialize_vec_string_from_ints,
};
use serde::{Deserialize, Serialize};

/// Corresponds to JsonSkin.Skin
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Skin {
    #[serde(rename = "type")]
    pub skin_type: i32,
    pub name: Option<String>,
    pub author: Option<String>,
    pub w: i32,
    pub h: i32,
    pub fadeout: i32,
    pub input: i32,
    pub scene: i32,
    pub close: i32,
    pub loadend: i32,
    pub playstart: i32,
    pub judgetimer: i32,
    pub finishmargin: i32,

    pub category: Vec<Category>,
    pub property: Vec<Property>,
    pub filepath: Vec<Filepath>,
    pub offset: Vec<Offset>,
    pub source: Vec<Source>,
    pub font: Vec<Font>,
    #[serde(deserialize_with = "deserialize_flattened_conditional_images", default)]
    pub image: Vec<Image>,
    pub imageset: Vec<ImageSet>,
    pub value: Vec<Value>,
    pub floatvalue: Vec<FloatValue>,
    #[serde(deserialize_with = "deserialize_flattened_conditional_texts", default)]
    pub text: Vec<Text>,
    pub slider: Vec<Slider>,
    pub graph: Vec<Graph>,
    pub gaugegraph: Vec<GaugeGraph>,
    pub judgegraph: Vec<JudgeGraph>,
    pub bpmgraph: Vec<BPMGraph>,
    pub hiterrorvisualizer: Vec<HitErrorVisualizer>,
    pub timingvisualizer: Vec<TimingVisualizer>,
    pub timingdistributiongraph: Vec<TimingDistributionGraph>,
    pub note: Option<NoteSet>,
    pub gauge: Option<Gauge>,
    #[serde(rename = "hiddenCover")]
    pub hidden_cover: Vec<HiddenCover>,
    #[serde(rename = "liftCover")]
    pub lift_cover: Vec<LiftCover>,
    pub bga: Option<BGA>,
    pub judge: Vec<Judge>,
    pub songlist: Option<SongList>,
    pub pmchara: Vec<PMchara>,
    #[serde(rename = "skinSelect")]
    pub skin_select: Option<SkinConfigurationProperty>,
    #[serde(rename = "customEvents")]
    pub custom_events: Vec<CustomEvent>,
    #[serde(rename = "customTimers")]
    pub custom_timers: Vec<CustomTimer>,

    #[serde(
        deserialize_with = "deserialize_flattened_conditional_destinations",
        default
    )]
    pub destination: Vec<Destination>,
}

impl Skin {
    fn _default_skin_type() -> i32 {
        -1
    }
}

/// Corresponds to JsonSkin.Property
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Property {
    pub category: Option<String>,
    pub name: Option<String>,
    pub item: Vec<PropertyItem>,
    pub def: Option<String>,
}

/// Corresponds to JsonSkin.PropertyItem
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PropertyItem {
    pub name: Option<String>,
    pub op: i32,
}

/// Corresponds to JsonSkin.Filepath
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Filepath {
    pub category: Option<String>,
    pub name: Option<String>,
    pub path: Option<String>,
    pub def: Option<String>,
}

/// Corresponds to JsonSkin.Offset
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Offset {
    pub category: Option<String>,
    pub name: Option<String>,
    #[serde(deserialize_with = "deserialize_i32_lenient", default)]
    pub id: i32,
    pub x: bool,
    pub y: bool,
    pub w: bool,
    pub h: bool,
    pub r: bool,
    pub a: bool,
}

/// Corresponds to JsonSkin.Category
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Category {
    pub name: Option<String>,
    pub item: Vec<String>,
}

/// Corresponds to JsonSkin.Source
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub path: Option<String>,
}

/// Corresponds to JsonSkin.Font
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Font {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "type")]
    pub font_type: i32,
}

/// Corresponds to JsonSkin.Image
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Image {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default = "default_one")]
    pub divx: i32,
    #[serde(default = "default_one")]
    pub divy: i32,
    pub timer: Option<i32>,
    pub cycle: i32,
    pub len: i32,
    #[serde(rename = "ref")]
    pub ref_id: i32,
    pub act: Option<i32>,
    pub click: i32,
}

fn default_one() -> i32 {
    1
}

/// Corresponds to JsonSkin.ImageSet
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageSet {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    #[serde(rename = "ref")]
    pub ref_id: i32,
    pub value: Option<i32>,
    #[serde(deserialize_with = "deserialize_vec_string_from_ints", default)]
    pub images: Vec<String>,
    pub act: Option<i32>,
    pub click: i32,
}

/// Corresponds to JsonSkin.Value
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Value {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default = "default_one")]
    pub divx: i32,
    #[serde(default = "default_one")]
    pub divy: i32,
    pub timer: Option<i32>,
    pub cycle: i32,
    pub align: i32,
    pub digit: i32,
    pub padding: i32,
    pub zeropadding: i32,
    pub space: i32,
    #[serde(rename = "ref")]
    pub ref_id: i32,
    pub value: Option<i32>,
    pub offset: Option<Vec<Value>>,
}

/// Corresponds to JsonSkin.FloatValue
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FloatValue {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default = "default_one")]
    pub divx: i32,
    #[serde(default = "default_one")]
    pub divy: i32,
    pub timer: Option<i32>,
    pub cycle: i32,
    pub align: i32,
    pub fketa: i32,
    pub iketa: i32,
    #[serde(default = "default_gain")]
    pub gain: f32,
    #[serde(rename = "isSignvisible")]
    pub is_signvisible: bool,
    pub padding: i32,
    pub zeropadding: i32,
    pub space: i32,
    #[serde(rename = "ref")]
    pub ref_id: i32,
    pub value: Option<i32>,
    pub offset: Option<Vec<Value>>,
}

fn default_gain() -> f32 {
    1.0
}

/// Corresponds to JsonSkin.Text
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Text {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub font: Option<String>,
    pub size: i32,
    pub align: i32,
    #[serde(rename = "ref")]
    pub ref_id: i32,
    pub value: Option<i32>,
    #[serde(rename = "constantText")]
    pub constant_text: Option<String>,
    pub wrapping: bool,
    pub overflow: i32,
    #[serde(rename = "outlineColor", default = "default_outline_color")]
    pub outline_color: String,
    #[serde(rename = "outlineWidth")]
    pub outline_width: f32,
    #[serde(rename = "shadowColor", default = "default_shadow_color")]
    pub shadow_color: String,
    #[serde(rename = "shadowOffsetX")]
    pub shadow_offset_x: f32,
    #[serde(rename = "shadowOffsetY")]
    pub shadow_offset_y: f32,
    #[serde(rename = "shadowSmoothness")]
    pub shadow_smoothness: f32,
}

fn default_outline_color() -> String {
    "ffffff00".to_string()
}

fn default_shadow_color() -> String {
    "ffffff00".to_string()
}

/// Corresponds to JsonSkin.Slider
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Slider {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default = "default_one")]
    pub divx: i32,
    #[serde(default = "default_one")]
    pub divy: i32,
    pub timer: Option<i32>,
    pub cycle: i32,
    pub angle: i32,
    pub range: i32,
    #[serde(rename = "type")]
    pub slider_type: i32,
    #[serde(default = "default_true")]
    pub changeable: bool,
    pub value: Option<i32>,
    pub event: Option<i32>,
    #[serde(rename = "isRefNum")]
    pub is_ref_num: bool,
    pub min: i32,
    pub max: i32,
}

fn default_true() -> bool {
    true
}

/// Corresponds to JsonSkin.Graph
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Graph {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default = "default_one")]
    pub divx: i32,
    #[serde(default = "default_one")]
    pub divy: i32,
    pub timer: Option<i32>,
    pub cycle: i32,
    #[serde(default = "default_one")]
    pub angle: i32,
    #[serde(rename = "type")]
    pub graph_type: i32,
    pub value: Option<i32>,
    #[serde(rename = "isRefNum")]
    pub is_ref_num: bool,
    pub min: i32,
    pub max: i32,
}

/// Corresponds to JsonSkin.GaugeGraph
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GaugeGraph {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub color: Option<Vec<String>>,
    #[serde(rename = "assistClearBGColor", default = "default_assist_clear_bg")]
    pub assist_clear_bg_color: String,
    #[serde(
        rename = "assistAndEasyFailBGColor",
        default = "default_assist_easy_fail_bg"
    )]
    pub assist_and_easy_fail_bg_color: String,
    #[serde(rename = "grooveFailBGColor", default = "default_groove_fail_bg")]
    pub groove_fail_bg_color: String,
    #[serde(
        rename = "grooveClearAndHardBGColor",
        default = "default_groove_clear_hard_bg"
    )]
    pub groove_clear_and_hard_bg_color: String,
    #[serde(rename = "exHardBGColor", default = "default_exhard_bg")]
    pub ex_hard_bg_color: String,
    #[serde(rename = "hazardBGColor", default = "default_hazard_bg")]
    pub hazard_bg_color: String,
    #[serde(rename = "assistClearLineColor", default = "default_assist_clear_line")]
    pub assist_clear_line_color: String,
    #[serde(
        rename = "assistAndEasyFailLineColor",
        default = "default_assist_easy_fail_line"
    )]
    pub assist_and_easy_fail_line_color: String,
    #[serde(rename = "grooveFailLineColor", default = "default_groove_fail_line")]
    pub groove_fail_line_color: String,
    #[serde(
        rename = "grooveClearAndHardLineColor",
        default = "default_groove_clear_hard_line"
    )]
    pub groove_clear_and_hard_line_color: String,
    #[serde(rename = "exHardLineColor", default = "default_exhard_line")]
    pub ex_hard_line_color: String,
    #[serde(rename = "hazardLineColor", default = "default_hazard_line")]
    pub hazard_line_color: String,
    #[serde(rename = "borderlineColor", default = "default_borderline")]
    pub borderline_color: String,
    #[serde(rename = "borderColor", default = "default_border")]
    pub border_color: String,
}

fn default_assist_clear_bg() -> String {
    "440044".to_string()
}
fn default_assist_easy_fail_bg() -> String {
    "004444".to_string()
}
fn default_groove_fail_bg() -> String {
    "004400".to_string()
}
fn default_groove_clear_hard_bg() -> String {
    "440000".to_string()
}
fn default_exhard_bg() -> String {
    "444400".to_string()
}
fn default_hazard_bg() -> String {
    "444444".to_string()
}
fn default_assist_clear_line() -> String {
    "ff00ff".to_string()
}
fn default_assist_easy_fail_line() -> String {
    "00ffff".to_string()
}
fn default_groove_fail_line() -> String {
    "00ff00".to_string()
}
fn default_groove_clear_hard_line() -> String {
    "ff0000".to_string()
}
fn default_exhard_line() -> String {
    "ffff00".to_string()
}
fn default_hazard_line() -> String {
    "cccccc".to_string()
}
fn default_borderline() -> String {
    "ff0000".to_string()
}
fn default_border() -> String {
    "440000".to_string()
}

/// Corresponds to JsonSkin.JudgeGraph
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct JudgeGraph {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub graph_type: i32,
    #[serde(rename = "backTexOff")]
    pub back_tex_off: i32,
    #[serde(default = "default_500")]
    pub delay: i32,
    #[serde(rename = "orderReverse")]
    pub order_reverse: i32,
    #[serde(rename = "noGap")]
    pub no_gap: i32,
    #[serde(rename = "noGapX")]
    pub no_gap_x: i32,
}

fn default_500() -> i32 {
    500
}

/// Corresponds to JsonSkin.BPMGraph
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BPMGraph {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub delay: i32,
    #[serde(rename = "lineWidth", default = "default_two")]
    pub line_width: i32,
    #[serde(rename = "mainBPMColor", default = "default_main_bpm_color")]
    pub main_bpm_color: String,
    #[serde(rename = "minBPMColor", default = "default_min_bpm_color")]
    pub min_bpm_color: String,
    #[serde(rename = "maxBPMColor", default = "default_max_bpm_color")]
    pub max_bpm_color: String,
    #[serde(rename = "otherBPMColor", default = "default_other_bpm_color")]
    pub other_bpm_color: String,
    #[serde(rename = "stopLineColor", default = "default_stop_line_color")]
    pub stop_line_color: String,
    #[serde(
        rename = "transitionLineColor",
        default = "default_transition_line_color"
    )]
    pub transition_line_color: String,
}

fn default_two() -> i32 {
    2
}
fn default_main_bpm_color() -> String {
    "00ff00".to_string()
}
fn default_min_bpm_color() -> String {
    "0000ff".to_string()
}
fn default_max_bpm_color() -> String {
    "ff0000".to_string()
}
fn default_other_bpm_color() -> String {
    "ffff00".to_string()
}
fn default_stop_line_color() -> String {
    "ff00ff".to_string()
}
fn default_transition_line_color() -> String {
    "7f7f7f".to_string()
}

/// Corresponds to JsonSkin.HitErrorVisualizer
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HitErrorVisualizer {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub width: i32,
    #[serde(rename = "judgeWidthMillis")]
    pub judge_width_millis: i32,
    #[serde(rename = "lineWidth")]
    pub line_width: i32,
    #[serde(rename = "colorMode")]
    pub color_mode: i32,
    #[serde(rename = "hiterrorMode")]
    pub hiterror_mode: i32,
    #[serde(rename = "emaMode")]
    pub ema_mode: i32,
    #[serde(rename = "lineColor")]
    pub line_color: String,
    #[serde(rename = "centerColor")]
    pub center_color: String,
    #[serde(rename = "PGColor")]
    pub pg_color: String,
    #[serde(rename = "GRColor")]
    pub gr_color: String,
    #[serde(rename = "GDColor")]
    pub gd_color: String,
    #[serde(rename = "BDColor")]
    pub bd_color: String,
    #[serde(rename = "PRColor")]
    pub pr_color: String,
    #[serde(rename = "emaColor")]
    pub ema_color: String,
    pub alpha: f32,
    #[serde(rename = "windowLength")]
    pub window_length: i32,
    pub transparent: i32,
    #[serde(rename = "drawDecay")]
    pub draw_decay: i32,
}

impl Default for HitErrorVisualizer {
    fn default() -> Self {
        Self {
            id: None,
            width: 301,
            judge_width_millis: 150,
            line_width: 1,
            color_mode: 1,
            hiterror_mode: 1,
            ema_mode: 1,
            line_color: "99CCFF80".to_string(),
            center_color: "FFFFFFFF".to_string(),
            pg_color: "99CCFF80".to_string(),
            gr_color: "F2CB3080".to_string(),
            gd_color: "14CC8f80".to_string(),
            bd_color: "FF1AB380".to_string(),
            pr_color: "CC292980".to_string(),
            ema_color: "FF0000FF".to_string(),
            alpha: 0.1,
            window_length: 30,
            transparent: 0,
            draw_decay: 1,
        }
    }
}

/// Corresponds to JsonSkin.TimingVisualizer
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TimingVisualizer {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub width: i32,
    #[serde(rename = "judgeWidthMillis")]
    pub judge_width_millis: i32,
    #[serde(rename = "lineWidth")]
    pub line_width: i32,
    #[serde(rename = "lineColor")]
    pub line_color: String,
    #[serde(rename = "centerColor")]
    pub center_color: String,
    #[serde(rename = "PGColor")]
    pub pg_color: String,
    #[serde(rename = "GRColor")]
    pub gr_color: String,
    #[serde(rename = "GDColor")]
    pub gd_color: String,
    #[serde(rename = "BDColor")]
    pub bd_color: String,
    #[serde(rename = "PRColor")]
    pub pr_color: String,
    pub transparent: i32,
    #[serde(rename = "drawDecay")]
    pub draw_decay: i32,
}

impl Default for TimingVisualizer {
    fn default() -> Self {
        Self {
            id: None,
            width: 301,
            judge_width_millis: 150,
            line_width: 1,
            line_color: "00FF00FF".to_string(),
            center_color: "FFFFFFFF".to_string(),
            pg_color: "000088FF".to_string(),
            gr_color: "008800FF".to_string(),
            gd_color: "888800FF".to_string(),
            bd_color: "880000FF".to_string(),
            pr_color: "000000FF".to_string(),
            transparent: 0,
            draw_decay: 1,
        }
    }
}

/// Corresponds to JsonSkin.TimingDistributionGraph
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TimingDistributionGraph {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub width: i32,
    #[serde(rename = "lineWidth")]
    pub line_width: i32,
    #[serde(rename = "graphColor")]
    pub graph_color: String,
    #[serde(rename = "averageColor")]
    pub average_color: String,
    #[serde(rename = "devColor")]
    pub dev_color: String,
    #[serde(rename = "PGColor")]
    pub pg_color: String,
    #[serde(rename = "GRColor")]
    pub gr_color: String,
    #[serde(rename = "GDColor")]
    pub gd_color: String,
    #[serde(rename = "BDColor")]
    pub bd_color: String,
    #[serde(rename = "PRColor")]
    pub pr_color: String,
    #[serde(rename = "drawAverage")]
    pub draw_average: i32,
    #[serde(rename = "drawDev")]
    pub draw_dev: i32,
}

impl Default for TimingDistributionGraph {
    fn default() -> Self {
        Self {
            id: None,
            width: 301,
            line_width: 1,
            graph_color: "00FF00FF".to_string(),
            average_color: "FFFFFFFF".to_string(),
            dev_color: "FFFFFFFF".to_string(),
            pg_color: "000088FF".to_string(),
            gr_color: "008800FF".to_string(),
            gd_color: "888800FF".to_string(),
            bd_color: "880000FF".to_string(),
            pr_color: "000000FF".to_string(),
            draw_average: 1,
            draw_dev: 1,
        }
    }
}

/// Corresponds to JsonSkin.NoteSet
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteSet {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub note: Vec<String>,
    pub lnstart: Vec<String>,
    pub lnend: Vec<String>,
    pub lnbody: Vec<String>,
    #[serde(rename = "lnbodyActive")]
    pub lnbody_active: Vec<String>,
    pub lnactive: Vec<String>,
    pub hcnstart: Vec<String>,
    pub hcnend: Vec<String>,
    pub hcnbody: Vec<String>,
    pub hcnactive: Vec<String>,
    #[serde(rename = "hcnbodyActive")]
    pub hcnbody_active: Vec<String>,
    pub hcndamage: Vec<String>,
    #[serde(rename = "hcnbodyMiss")]
    pub hcnbody_miss: Vec<String>,
    pub hcnreactive: Vec<String>,
    #[serde(rename = "hcnbodyReactive")]
    pub hcnbody_reactive: Vec<String>,
    pub mine: Vec<String>,
    pub hidden: Vec<String>,
    pub processed: Vec<String>,
    pub dst: Vec<Animation>,
    #[serde(default = "default_i32_min")]
    pub dst2: i32,
    #[serde(default = "default_expansion_rate")]
    pub expansionrate: Vec<i32>,
    pub size: Vec<f32>,
    pub group: Vec<Destination>,
    pub bpm: Vec<Destination>,
    pub stop: Vec<Destination>,
    pub time: Vec<Destination>,
}

fn default_i32_min() -> i32 {
    i32::MIN
}

fn default_expansion_rate() -> Vec<i32> {
    vec![100, 100]
}

/// Corresponds to JsonSkin.Gauge
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Gauge {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub nodes: Vec<String>,
    #[serde(default = "default_50")]
    pub parts: i32,
    #[serde(rename = "type")]
    pub gauge_type: i32,
    #[serde(default = "default_3")]
    pub range: i32,
    #[serde(default = "default_33")]
    pub cycle: i32,
    pub starttime: i32,
    #[serde(default = "default_500")]
    pub endtime: i32,
}

fn default_50() -> i32 {
    50
}
fn default_3() -> i32 {
    3
}
fn default_33() -> i32 {
    33
}

/// Corresponds to JsonSkin.HiddenCover
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HiddenCover {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default = "default_one")]
    pub divx: i32,
    #[serde(default = "default_one")]
    pub divy: i32,
    pub timer: Option<i32>,
    pub cycle: i32,
    #[serde(rename = "disapearLine", default = "default_neg_one")]
    pub disapear_line: i32,
    #[serde(rename = "isDisapearLineLinkLift", default = "default_true")]
    pub is_disapear_line_link_lift: bool,
}

fn default_neg_one() -> i32 {
    -1
}

/// Corresponds to JsonSkin.LiftCover
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LiftCover {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    #[serde(default = "default_one")]
    pub divx: i32,
    #[serde(default = "default_one")]
    pub divy: i32,
    pub timer: Option<i32>,
    pub cycle: i32,
    #[serde(rename = "disapearLine", default = "default_neg_one")]
    pub disapear_line: i32,
    #[serde(rename = "isDisapearLineLinkLift")]
    pub is_disapear_line_link_lift: bool,
}

/// Corresponds to JsonSkin.BGA
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BGA {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
}

/// Corresponds to JsonSkin.Judge
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Judge {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub index: i32,
    pub images: Vec<Destination>,
    pub numbers: Vec<Destination>,
    pub shift: bool,
}

/// Corresponds to JsonSkin.SongList
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SongList {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub center: i32,
    pub clickable: Vec<i32>,
    pub listoff: Vec<Destination>,
    pub liston: Vec<Destination>,
    pub text: Vec<Destination>,
    pub level: Vec<Destination>,
    pub lamp: Vec<Destination>,
    pub playerlamp: Vec<Destination>,
    pub rivallamp: Vec<Destination>,
    pub trophy: Vec<Destination>,
    pub label: Vec<Destination>,
    pub graph: Option<Destination>,
}

/// Corresponds to JsonSkin.Destination
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Destination {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub blend: i32,
    pub filter: i32,
    pub timer: Option<i32>,
    #[serde(rename = "loop")]
    pub loop_val: i32,
    pub center: i32,
    pub offset: i32,
    pub offsets: Vec<i32>,
    #[serde(default = "default_neg_one")]
    pub stretch: i32,
    pub op: Vec<i32>,
    #[serde(deserialize_with = "deserialize_optional_i32_or_string", default)]
    pub draw: Option<i32>,
    #[serde(deserialize_with = "deserialize_animations_with_conditionals", default)]
    pub dst: Vec<Animation>,
    #[serde(rename = "mouseRect")]
    pub mouse_rect: Option<Rect>,
}

/// Corresponds to JsonSkin.Rect
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Corresponds to JsonSkin.Animation
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Animation {
    pub time: i32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub acc: i32,
    pub a: i32,
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub angle: i32,
}

impl Default for Animation {
    fn default() -> Self {
        Self {
            time: i32::MIN,
            x: i32::MIN,
            y: i32::MIN,
            w: i32::MIN,
            h: i32::MIN,
            acc: i32::MIN,
            a: i32::MIN,
            r: i32::MIN,
            g: i32::MIN,
            b: i32::MIN,
            angle: i32::MIN,
        }
    }
}

/// Corresponds to JsonSkin.PMchara
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PMchara {
    #[serde(deserialize_with = "deserialize_optional_string_from_int", default)]
    pub id: Option<String>,
    pub src: Option<String>,
    pub color: i32,
    #[serde(rename = "type")]
    pub chara_type: i32,
    pub side: i32,
}

impl Default for PMchara {
    fn default() -> Self {
        Self {
            id: None,
            src: None,
            color: 1,
            chara_type: i32::MIN,
            side: 1,
        }
    }
}

/// Corresponds to JsonSkin.SkinConfigurationProperty
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SkinConfigurationProperty {
    #[serde(rename = "customBMS")]
    pub custom_bms: Option<Vec<String>>,
    #[serde(rename = "defaultCategory")]
    pub default_category: i32,
    #[serde(rename = "customPropertyCount", default = "default_neg_one")]
    pub custom_property_count: i32,
    #[serde(rename = "customOffsetStyle")]
    pub custom_offset_style: i32,
}

/// Corresponds to JsonSkin.CustomEvent
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomEvent {
    #[serde(deserialize_with = "deserialize_i32_lenient", default)]
    pub id: i32,
    pub action: Option<i32>,
    pub condition: Option<i32>,
    #[serde(rename = "minInterval")]
    pub min_interval: i32,
}

/// Corresponds to JsonSkin.CustomTimer
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomTimer {
    #[serde(deserialize_with = "deserialize_i32_lenient", default)]
    pub id: i32,
    pub timer: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_id_from_string() {
        let json = r#"{"id": "myimage"}"#;
        let img: Image = serde_json::from_str(json).unwrap();
        assert_eq!(img.id, Some("myimage".to_string()));
    }

    #[test]
    fn image_id_from_integer() {
        let json = r#"{"id": 150}"#;
        let img: Image = serde_json::from_str(json).unwrap();
        assert_eq!(img.id, Some("150".to_string()));
    }

    #[test]
    fn image_id_null() {
        let json = r#"{"id": null}"#;
        let img: Image = serde_json::from_str(json).unwrap();
        assert_eq!(img.id, None);
    }

    #[test]
    fn image_id_absent() {
        let json = r#"{}"#;
        let img: Image = serde_json::from_str(json).unwrap();
        assert_eq!(img.id, None);
    }

    #[test]
    fn destination_draw_from_integer() {
        let json = r#"{"draw": 1}"#;
        let dst: Destination = serde_json::from_str(json).unwrap();
        assert_eq!(dst.draw, Some(1));
    }

    #[test]
    fn destination_draw_from_lua_expression() {
        let json = r#"{"draw": "gauge() >= 75"}"#;
        let dst: Destination = serde_json::from_str(json).unwrap();
        // Lua expressions are not yet evaluable, so they become None
        assert_eq!(dst.draw, None);
    }

    #[test]
    fn destination_draw_from_string_integer() {
        let json = r#"{"draw": "42"}"#;
        let dst: Destination = serde_json::from_str(json).unwrap();
        // String-encoded integers are parsed successfully
        assert_eq!(dst.draw, Some(42));
    }

    #[test]
    fn destination_draw_null() {
        let json = r#"{"draw": null}"#;
        let dst: Destination = serde_json::from_str(json).unwrap();
        assert_eq!(dst.draw, None);
    }

    #[test]
    fn destination_draw_absent() {
        let json = r#"{}"#;
        let dst: Destination = serde_json::from_str(json).unwrap();
        assert_eq!(dst.draw, None);
    }

    #[test]
    fn skin_image_array_direct_items() {
        let json = r#"{"image": [{"id": "a"}, {"id": 10}]}"#;
        let skin: Skin = serde_json::from_str(json).unwrap();
        assert_eq!(skin.image.len(), 2);
        assert_eq!(skin.image[0].id, Some("a".to_string()));
        assert_eq!(skin.image[1].id, Some("10".to_string()));
    }

    #[test]
    fn skin_image_array_with_conditional() {
        let json = r#"{
            "image": [
                {"id": "a"},
                {"if": [920], "values": [{"id": "b"}, {"id": "c"}]},
                {"id": "d"}
            ]
        }"#;
        let skin: Skin = serde_json::from_str(json).unwrap();
        assert_eq!(skin.image.len(), 4);
        assert_eq!(skin.image[0].id, Some("a".to_string()));
        assert_eq!(skin.image[1].id, Some("b".to_string()));
        assert_eq!(skin.image[2].id, Some("c".to_string()));
        assert_eq!(skin.image[3].id, Some("d".to_string()));
    }

    #[test]
    fn skin_image_array_empty() {
        let json = r#"{"image": []}"#;
        let skin: Skin = serde_json::from_str(json).unwrap();
        assert!(skin.image.is_empty());
    }

    #[test]
    fn skin_image_array_absent() {
        let json = r#"{}"#;
        let skin: Skin = serde_json::from_str(json).unwrap();
        assert!(skin.image.is_empty());
    }
}
