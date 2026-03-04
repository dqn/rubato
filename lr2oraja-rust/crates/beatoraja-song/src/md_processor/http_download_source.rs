/// Corresponds to HttpDownloadSource interface in Java
/// Defines a http download source
pub trait HttpDownloadSource: Send + Sync {
    /// Construct download url based on md5
    ///
    /// # Arguments
    /// * `md5` - missing sabun's md5
    ///
    /// # Returns
    /// download url, based on download source
    fn get_download_url_based_on_md5(&self, md5: &str) -> anyhow::Result<String>;

    /// Name is an unique symbol, also the option from 'otherTab'
    fn get_name(&self) -> &str;

    // For further implementations

    fn is_allow_download_through_md5(&self) -> bool;

    fn is_allow_download_through_sha256(&self) -> bool;

    fn is_allow_meta_query(&self) -> bool;
}
