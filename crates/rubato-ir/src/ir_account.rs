/// IR account data
///
/// Translated from: IRAccount.java
#[derive(Clone, Debug)]
pub struct IRAccount {
    /// Player ID
    pub id: String,
    /// Password
    pub password: String,
    /// Player name
    pub name: String,
}

impl IRAccount {
    pub fn new(id: String, password: String, name: String) -> Self {
        Self { id, password, name }
    }
}
