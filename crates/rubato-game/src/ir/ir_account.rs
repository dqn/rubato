/// IR account data
///
/// Translated from: IRAccount.java
///
/// Accepted trade-off: password is stored as plaintext `String`, matching the Java
/// original. The IR password is serialized to the player config JSON on disk in
/// cleartext. Using OS keychain or hashing would improve security but diverges from
/// the original design and the LR2IR protocol requires the raw password for auth.
#[derive(Clone, Debug)]
pub struct IRAccount {
    /// Player ID
    pub id: String,
    /// Password (stored as plaintext; see struct-level docs for rationale)
    pub password: String,
    /// Player name
    pub name: String,
}

impl IRAccount {
    pub fn new(id: String, password: String, name: String) -> Self {
        Self { id, password, name }
    }
}
