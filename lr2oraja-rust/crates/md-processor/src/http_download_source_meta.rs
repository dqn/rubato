use crate::Config;
use crate::http_download_source::HttpDownloadSource;

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
