use rubato_skin::reexports::Rectangle;

use super::SkinHeader;

/// Skin stub for modmenu
#[derive(Clone, Default)]
pub struct Skin {
    pub header: SkinHeader,
    objects: Vec<SkinObject>,
}

impl Skin {
    pub fn all_skin_objects(&self) -> &[SkinObject] {
        &self.objects
    }
}

/// SkinObject stub for modmenu
#[derive(Clone, Debug, Default)]
pub struct SkinObject {
    pub name: Option<String>,
    pub draw: bool,
    pub visible: bool,
    pub destinations: Vec<SkinObjectDestination>,
}

impl SkinObject {
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn all_destination(&self) -> &[SkinObjectDestination] {
        &self.destinations
    }
}

#[derive(Clone, Debug, Default)]
pub struct SkinObjectDestination {
    pub time: i32,
    pub region: Rectangle,
    pub color: Option<[f32; 4]>,
    pub angle: f32,
    pub alpha: f32,
}
