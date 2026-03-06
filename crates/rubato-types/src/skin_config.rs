use crate::skin_type::SkinType;
use crate::validatable::{Validatable, remove_invalid_elements};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SkinConfig {
    pub path: Option<String>,
    pub properties: Option<SkinProperty>,
}

impl SkinConfig {
    pub fn new_with_path(path: &str) -> Self {
        SkinConfig {
            path: Some(path.to_string()),
            properties: None,
        }
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn properties(&self) -> Option<&SkinProperty> {
        self.properties.as_ref()
    }

    pub fn properties_mut(&mut self) -> Option<&mut SkinProperty> {
        self.properties.as_mut()
    }

    pub fn default_for_id(id: i32) -> SkinConfig {
        let mut skin = SkinConfig::default();
        if let Some(skin_type) = SkinType::skin_type_by_id(id)
            && let Some(dskin) = SkinDefault::get(skin_type)
        {
            skin.path = Some(dskin.path.to_string());
            skin.validate();
        }
        skin
    }
}

impl Validatable for SkinConfig {
    fn validate(&mut self) -> bool {
        match &self.path {
            None => return false,
            Some(p) if p.is_empty() => return false,
            _ => {}
        }
        if self.properties.is_none() {
            self.properties = Some(SkinProperty::default());
        }
        if let Some(ref mut props) = self.properties {
            props.validate();
        }
        true
    }
}

// -- Property --

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SkinProperty {
    pub option: Vec<Option<SkinOption>>,
    pub file: Vec<Option<SkinFilePath>>,
    pub offset: Vec<Option<SkinOffset>>,
}

impl SkinProperty {
    pub fn validate(&mut self) -> bool {
        self.option = remove_invalid_elements(std::mem::take(&mut self.option))
            .into_iter()
            .map(Some)
            .collect();

        self.file = remove_invalid_elements(std::mem::take(&mut self.file))
            .into_iter()
            .map(Some)
            .collect();

        self.offset = remove_invalid_elements(std::mem::take(&mut self.offset))
            .into_iter()
            .map(Some)
            .collect();

        true
    }
}

// -- Option (renamed to SkinOption to avoid conflict) --

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SkinOption {
    pub name: Option<String>,
    pub value: i32,
}

impl Validatable for SkinOption {
    fn validate(&mut self) -> bool {
        match &self.name {
            None => false,
            Some(n) => !n.is_empty(),
        }
    }
}

// -- FilePath --

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SkinFilePath {
    pub name: Option<String>,
    pub path: Option<String>,
}

impl Validatable for SkinFilePath {
    fn validate(&mut self) -> bool {
        let name_valid = self.name.as_ref().is_some_and(|n| !n.is_empty());
        let path_valid = self.path.as_ref().is_some_and(|p| !p.is_empty());
        name_valid && path_valid
    }
}

// -- Offset --

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SkinOffset {
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub r: i32,
    pub a: i32,
}

impl Validatable for SkinOffset {
    fn validate(&mut self) -> bool {
        match &self.name {
            None => false,
            Some(n) => !n.is_empty(),
        }
    }
}

// -- Default skin paths --

pub struct SkinDefault {
    pub skin_type: SkinType,
    pub path: &'static str,
}

impl SkinDefault {
    const ALL: &'static [SkinDefault] = &[
        SkinDefault {
            skin_type: SkinType::Play7Keys,
            path: "skin/default/play/play7.luaskin",
        },
        SkinDefault {
            skin_type: SkinType::Play5Keys,
            path: "skin/default/play5.json",
        },
        SkinDefault {
            skin_type: SkinType::Play14Keys,
            path: "skin/default/play14.json",
        },
        SkinDefault {
            skin_type: SkinType::Play10Keys,
            path: "skin/default/play10.json",
        },
        SkinDefault {
            skin_type: SkinType::Play9Keys,
            path: "skin/default/play9.json",
        },
        SkinDefault {
            skin_type: SkinType::MusicSelect,
            path: "skin/default/select.json",
        },
        SkinDefault {
            skin_type: SkinType::Decide,
            path: "skin/default/decide/decide.luaskin",
        },
        SkinDefault {
            skin_type: SkinType::Result,
            path: "skin/default/result/result.luaskin",
        },
        SkinDefault {
            skin_type: SkinType::CourseResult,
            path: "skin/default/graderesult.json",
        },
        SkinDefault {
            skin_type: SkinType::Play24Keys,
            path: "skin/default/play24.json",
        },
        SkinDefault {
            skin_type: SkinType::Play24KeysDouble,
            path: "skin/default/play24double.json",
        },
        SkinDefault {
            skin_type: SkinType::KeyConfig,
            path: "skin/default/keyconfig/keyconfig.luaskin",
        },
        SkinDefault {
            skin_type: SkinType::SkinSelect,
            path: "skin/default/skinselect/skinselect.luaskin",
        },
    ];

    pub fn get(skin_type: SkinType) -> Option<&'static SkinDefault> {
        SkinDefault::ALL.iter().find(|s| s.skin_type == skin_type)
    }
}
