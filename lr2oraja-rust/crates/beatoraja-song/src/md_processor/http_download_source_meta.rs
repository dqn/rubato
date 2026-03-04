use super::Config;
use super::http_download_source::HttpDownloadSource;

/// Corresponds to HttpDownloadSourceMeta in Java
pub struct HttpDownloadSourceMeta {
    name: String,
    // TODO: This is a bad design since it doesn't reserved the space for other download strategies
    // (e.g. download through an unique field from IR server or simply sha256). Could be extended
    // in the near future. As for now, keep it simple and stupid
    // However, it's not very easy to give user such flexibility
    default_url: String,
    builder: fn(&Config) -> Box<dyn HttpDownloadSource>,
}

impl HttpDownloadSourceMeta {
    pub fn new(
        name: &str,
        default_url: &str,
        builder: fn(&Config) -> Box<dyn HttpDownloadSource>,
    ) -> Self {
        HttpDownloadSourceMeta {
            name: name.to_string(),
            default_url: default_url.to_string(),
            builder,
        }
    }

    pub fn build(&self, config: &Config) -> Box<dyn HttpDownloadSource> {
        (self.builder)(config)
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_default_url(&self) -> &str {
        &self.default_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_builder(_config: &Config) -> Box<dyn HttpDownloadSource> {
        struct DummySource;
        impl HttpDownloadSource for DummySource {
            fn get_download_url_based_on_md5(&self, md5: &str) -> anyhow::Result<String> {
                Ok(format!("https://dummy/{}", md5))
            }
            fn get_name(&self) -> &str {
                "dummy"
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
        Box::new(DummySource)
    }

    #[test]
    fn new_stores_name_and_url() {
        let meta =
            HttpDownloadSourceMeta::new("test_source", "https://example.com/dl/%s", dummy_builder);
        assert_eq!(meta.get_name(), "test_source");
        assert_eq!(meta.get_default_url(), "https://example.com/dl/%s");
    }

    #[test]
    fn build_creates_source_via_builder() {
        let meta =
            HttpDownloadSourceMeta::new("test_source", "https://example.com/%s", dummy_builder);
        let config = Config::default();
        let source = meta.build(&config);
        assert_eq!(source.get_name(), "dummy");
        let url = source.get_download_url_based_on_md5("abc123").unwrap();
        assert_eq!(url, "https://dummy/abc123");
    }

    #[test]
    fn empty_name_and_url() {
        let meta = HttpDownloadSourceMeta::new("", "", dummy_builder);
        assert_eq!(meta.get_name(), "");
        assert_eq!(meta.get_default_url(), "");
    }
}
