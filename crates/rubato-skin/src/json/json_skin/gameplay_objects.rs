use super::deserializers::{
    deserialize_animations_with_conditionals, deserialize_optional_string_from_int,
};
use super::destination::{Animation, Destination};
use super::graph_objects::default_500;
use super::visual_objects::default_one;
use serde::{Deserialize, Serialize};

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
    #[serde(deserialize_with = "deserialize_animations_with_conditionals", default)]
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

fn default_true() -> bool {
    true
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
