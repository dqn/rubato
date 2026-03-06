/// Corresponds to IpfsInformation interface in Java
/// Interface for obtaining IPFS information
pub trait IpfsInformation: Send + Sync {
    /// Get the IPFS path for the song
    fn ipfs(&self) -> String;
    /// Get the IPFS path for the song diff/append
    fn append_ipfs(&self) -> String;
    /// Get the song title
    fn title(&self) -> String;
    /// Get the song artist name
    fn artist(&self) -> String;
    /// Get the md5 list of bundled charts (for diff charts)
    fn org_md5(&self) -> Vec<String>;
}
