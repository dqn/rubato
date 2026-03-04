/// Corresponds to IpfsInformation interface in Java
/// Interface for obtaining IPFS information
pub trait IpfsInformation: Send + Sync {
    /// Get the IPFS path for the song
    fn get_ipfs(&self) -> String;

    /// Get the IPFS path for the song diff/append
    fn get_append_ipfs(&self) -> String;

    /// Get the song title
    fn get_title(&self) -> String;

    /// Get the song artist name
    fn get_artist(&self) -> String;

    /// Get the md5 list of bundled charts (for diff charts)
    fn get_org_md5(&self) -> Vec<String>;
}
