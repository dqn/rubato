use serde::{Deserialize, Serialize};

/// Random course stage data
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RandomStageData {
    pub title: Option<String>,
    pub sql: Option<String>,
}
