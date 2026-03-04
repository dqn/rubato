use super::bar::Bar;
use super::directory_bar::DirectoryBarData;

/// Bar containing specified child bars
/// Translates: bms.player.beatoraja.select.bar.ContainerBar
#[derive(Clone)]
pub struct ContainerBar {
    pub directory: DirectoryBarData,
    pub title: String,
    pub childbar: Vec<Bar>,
}

impl ContainerBar {
    pub fn new(title: String, bar: Vec<Bar>) -> Self {
        Self {
            directory: DirectoryBarData::default(),
            title,
            childbar: bar,
        }
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_children(&self) -> &[Bar] {
        &self.childbar
    }
}
