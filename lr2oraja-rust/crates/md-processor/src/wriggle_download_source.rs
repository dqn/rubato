use crate::Config;
use crate::http_download_source::HttpDownloadSource;
use crate::http_download_source_meta::HttpDownloadSourceMeta;

use std::sync::LazyLock;

/// Corresponds to WriggleDownloadSource in Java
pub struct WriggleDownloadSource {
    download_url: String,
}

pub static META: LazyLock<HttpDownloadSourceMeta> = LazyLock::new(|| {
    HttpDownloadSourceMeta::new(
        "wriggle",
        "https://bms.wrigglebug.xyz/download/package/%s",
        |config| Box::new(WriggleDownloadSource::new(config)),
    )
});

impl WriggleDownloadSource {
    pub fn new(config: &Config) -> Self {
        // override download url if user ask to do so
        let override_download_url = config.get_override_download_url();
        let download_url = match override_download_url {
            Some(url) if !url.is_empty() => url.to_string(),
            _ => META.get_default_url().to_string(),
        };
        WriggleDownloadSource { download_url }
    }
}

impl HttpDownloadSource for WriggleDownloadSource {
    /// The download url should be a pattern with only one %s placeholder. If not, anything could happen.
    fn get_download_url_based_on_md5(&self, md5: &str) -> anyhow::Result<String> {
        Ok(self.download_url.replace("%s", md5))
    }

    fn get_name(&self) -> &str {
        META.get_name()
    }

    fn is_allow_download_through_md5(&self) -> bool {
        true
    }

    fn is_allow_download_through_sha256(&self) -> bool {
        false
    }

    fn is_allow_meta_query(&self) -> bool {
        false
    }
}
