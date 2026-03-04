use serde::{Deserialize, Serialize};

/// Player information
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerInformation {
    pub id: Option<String>,
    pub name: Option<String>,
    pub rank: Option<String>,
}

impl PlayerInformation {
    pub fn get_name(&self) -> &str {
        self.name.as_deref().unwrap_or("")
    }
}
