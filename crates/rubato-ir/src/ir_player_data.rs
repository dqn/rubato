/// IR player data
///
/// Translated from: IRPlayerData.java
#[derive(Clone, Debug)]
pub struct IRPlayerData {
    /// Player ID
    pub id: String,
    /// Player name
    pub name: String,
    /// Rank
    pub rank: String,
}

impl IRPlayerData {
    pub fn new(id: String, name: String, rank: String) -> Self {
        Self { id, name, rank }
    }
}
