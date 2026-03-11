// Mechanical translation of JsonSkin.java
// JSON skin model/data classes

mod deserializers;
mod destination;
mod gameplay_objects;
mod graph_objects;
mod skin_structs;
mod visual_objects;

pub use deserializers::set_enabled_options;
pub use destination::*;
pub use gameplay_objects::*;
pub use graph_objects::*;
pub use skin_structs::*;
pub use visual_objects::*;

use deserializers::{
    deserialize_flattened_conditional_destinations, deserialize_flattened_conditional_images,
    deserialize_flattened_conditional_texts, deserialize_i32_lenient,
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

fn default_neg_one() -> i32 {
    -1
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
