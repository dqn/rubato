use crate::bms_table_element::BmsTableElement;

#[derive(Debug, Clone)]
pub struct EventTableElement {
    pub element: BmsTableElement,
    team: Option<String>,
    artist: Option<String>,
}

impl EventTableElement {
    pub fn new() -> Self {
        Self {
            element: BmsTableElement::new(),
            team: None,
            artist: None,
        }
    }

    pub fn get_artist(&self) -> Option<&str> {
        self.artist.as_deref()
    }

    pub fn set_artist(&mut self, artist: Option<&str>) {
        self.artist = artist.map(|s| s.to_string());
    }

    pub fn get_team(&self) -> Option<&str> {
        self.team.as_deref()
    }

    pub fn set_team(&mut self, team: Option<&str>) {
        self.team = team.map(|s| s.to_string());
    }
}

impl Default for EventTableElement {
    fn default() -> Self {
        Self::new()
    }
}
