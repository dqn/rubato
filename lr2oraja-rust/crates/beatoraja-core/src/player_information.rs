use serde::{Deserialize, Serialize};

/// Player information
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerInformation {
    pub id: Option<String>,
    pub name: Option<String>,
    pub rank: Option<String>,
}
