use super::Config;
use super::http_download_source::HttpDownloadSource;
use super::http_download_source_meta::HttpDownloadSourceMeta;

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
        let override_download_url = config.override_download_url();
        let download_url = match override_download_url {
            Some(url) if !url.is_empty() => url.to_string(),
            _ => META.default_url().to_string(),
        };
        WriggleDownloadSource { download_url }
    }
}

impl HttpDownloadSource for WriggleDownloadSource {
    /// The download url should be a pattern with only one %s placeholder. If not, anything could happen.
    fn get_download_url_based_on_md5(&self, md5: &str) -> anyhow::Result<String> {
        Ok(self.download_url.replace("%s", md5))
    }

    fn name(&self) -> &str {
        META.name()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_name_and_default_url() {
        let meta = &*META;
        assert_eq!(meta.name(), "wriggle");
        assert_eq!(
            meta.default_url(),
            "https://bms.wrigglebug.xyz/download/package/%s"
        );
    }

    #[test]
    fn url_substitution_with_md5() {
        let source = WriggleDownloadSource {
            download_url: "https://bms.wrigglebug.xyz/download/package/%s".to_string(),
        };
        let url = source
            .get_download_url_based_on_md5("deadbeef1234")
            .unwrap();
        assert_eq!(
            url,
            "https://bms.wrigglebug.xyz/download/package/deadbeef1234"
        );
    }

    #[test]
    fn url_substitution_with_empty_md5() {
        let source = WriggleDownloadSource {
            download_url: "https://example.com/%s".to_string(),
        };
        let url = source.get_download_url_based_on_md5("").unwrap();
        assert_eq!(url, "https://example.com/");
    }

    #[test]
    fn url_substitution_with_special_characters() {
        let source = WriggleDownloadSource {
            download_url: "https://example.com/dl/%s/file".to_string(),
        };
        let url = source.get_download_url_based_on_md5("abc+def/ghi").unwrap();
        assert_eq!(url, "https://example.com/dl/abc+def/ghi/file");
    }

    #[test]
    fn download_source_trait_methods() {
        let source = WriggleDownloadSource {
            download_url: "https://example.com/%s".to_string(),
        };
        assert_eq!(source.name(), "wriggle");
        assert!(source.is_allow_download_through_md5());
        assert!(!source.is_allow_download_through_sha256());
        assert!(!source.is_allow_meta_query());
    }

    #[test]
    fn url_without_placeholder_returns_unchanged() {
        let source = WriggleDownloadSource {
            download_url: "https://example.com/static-url".to_string(),
        };
        let url = source.get_download_url_based_on_md5("anything").unwrap();
        assert_eq!(url, "https://example.com/static-url");
    }
}
