use super::Config;
use super::http_download_source::HttpDownloadSource;
use super::http_download_source_meta::HttpDownloadSourceMeta;

use std::sync::LazyLock;

use serde::Deserialize;

/// Corresponds to KonmaiDownloadSource in Java
pub struct KonmaiDownloadSource {
    download_query_url: String,
}

pub static META: LazyLock<HttpDownloadSourceMeta> = LazyLock::new(|| {
    HttpDownloadSourceMeta::new(
        "konmai",
        "https://bms.alvorna.com/api/hash?md5=%s",
        |config| Box::new(KonmaiDownloadSource::new(config)),
    )
});

pub static SUCCESS_RESULT: &str = "success";

impl KonmaiDownloadSource {
    pub fn new(config: &Config) -> Self {
        // override download url if user ask to do so
        let override_download_url = config.get_override_download_url();
        let download_query_url = match override_download_url {
            Some(url) if !url.is_empty() => url.to_string(),
            _ => META.get_default_url().to_string(),
        };
        KonmaiDownloadSource { download_query_url }
    }
}

impl HttpDownloadSource for KonmaiDownloadSource {
    fn get_name(&self) -> &str {
        META.get_name()
    }

    /// Konmai backend uses a meta query endpoint instead of direct download link.
    /// Similar to wriggle, the url must be a pattern string with only one %s placeholder and anything could happen
    /// if not. It also requires authentication so we have to grab token if we don't have one or the server reports
    /// that it's expired.
    fn get_download_url_based_on_md5(&self, md5: &str) -> anyhow::Result<String> {
        let meta_url = self.download_query_url.replace("%s", md5);
        // TODO: Server side doesn't provide auth currently
        let response = reqwest::blocking::get(&meta_url)?;
        let response_code = response.status();

        // Konmai backend doesn't offer an 404 status code
        if response_code != reqwest::StatusCode::OK {
            if response_code == reqwest::StatusCode::NOT_FOUND {
                return Err(anyhow::anyhow!("FileNotFound"));
            }
            return Err(anyhow::anyhow!(
                "Unexpected http response code: {}",
                response_code.as_u16()
            ));
        }

        let resp_data: RespData<ChartMeta> = response.json()?;
        // Instead, Konmai returns an empty 'song_url' or 'result: fail' to indicate song is not exist
        if resp_data.result.as_deref() != Some(SUCCESS_RESULT) {
            return Err(anyhow::anyhow!(
                "Unexpected error: {}",
                resp_data.msg.as_deref().unwrap_or("")
            ));
        }
        let chart_meta = resp_data
            .data
            .ok_or_else(|| anyhow::anyhow!("Missing chart meta data"))?;
        match chart_meta.song_url {
            Some(ref url) if !url.is_empty() => Ok(url.clone()),
            _ => Err(anyhow::anyhow!("FileNotFound")),
        }
    }

    fn is_allow_download_through_md5(&self) -> bool {
        true
    }

    fn is_allow_download_through_sha256(&self) -> bool {
        false
    }

    fn is_allow_meta_query(&self) -> bool {
        true
    }
}

/// Response wrapper from Konmai
#[derive(Deserialize)]
struct RespData<T> {
    #[serde(default)]
    result: Option<String>,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    chart: Option<String>,
    #[serde(default)]
    data: Option<T>,
}

/// Represents one chart meta info from Konmai
#[derive(Deserialize, Default)]
struct ChartMeta {
    #[serde(default)]
    #[allow(dead_code)]
    chart_name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    md5: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    sha256: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    song_name: Option<String>,
    #[serde(default)]
    song_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_success_response() {
        let json = r#"{
            "result": "success",
            "msg": null,
            "chart": "test_chart",
            "data": {
                "chart_name": "Test Chart",
                "md5": "abc123def456",
                "sha256": "sha256hash",
                "song_name": "Test Song",
                "song_url": "https://example.com/download/test.7z"
            }
        }"#;
        let resp: RespData<ChartMeta> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.result.as_deref(), Some("success"));
        assert!(resp.msg.is_none());
        assert_eq!(resp.chart.as_deref(), Some("test_chart"));
        let data = resp.data.unwrap();
        assert_eq!(data.chart_name.as_deref(), Some("Test Chart"));
        assert_eq!(data.md5.as_deref(), Some("abc123def456"));
        assert_eq!(data.sha256.as_deref(), Some("sha256hash"));
        assert_eq!(data.song_name.as_deref(), Some("Test Song"));
        assert_eq!(
            data.song_url.as_deref(),
            Some("https://example.com/download/test.7z")
        );
    }

    #[test]
    fn deserialize_failure_response() {
        let json = r#"{
            "result": "fail",
            "msg": "Song not found"
        }"#;
        let resp: RespData<ChartMeta> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.result.as_deref(), Some("fail"));
        assert_eq!(resp.msg.as_deref(), Some("Song not found"));
        assert!(resp.data.is_none());
        assert!(resp.chart.is_none());
    }

    #[test]
    fn deserialize_empty_json_object() {
        let json = r#"{}"#;
        let resp: RespData<ChartMeta> = serde_json::from_str(json).unwrap();
        assert!(resp.result.is_none());
        assert!(resp.msg.is_none());
        assert!(resp.chart.is_none());
        assert!(resp.data.is_none());
    }

    #[test]
    fn deserialize_data_with_empty_song_url() {
        let json = r#"{
            "result": "success",
            "data": {
                "song_url": ""
            }
        }"#;
        let resp: RespData<ChartMeta> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.result.as_deref(), Some("success"));
        let data = resp.data.unwrap();
        assert_eq!(data.song_url.as_deref(), Some(""));
        // Empty song_url should be treated as "not found" by the download logic
    }

    #[test]
    fn deserialize_data_with_null_song_url() {
        let json = r#"{
            "result": "success",
            "data": {
                "song_url": null
            }
        }"#;
        let resp: RespData<ChartMeta> = serde_json::from_str(json).unwrap();
        let data = resp.data.unwrap();
        assert!(data.song_url.is_none());
    }

    #[test]
    fn chart_meta_default() {
        let meta = ChartMeta::default();
        assert!(meta.chart_name.is_none());
        assert!(meta.md5.is_none());
        assert!(meta.sha256.is_none());
        assert!(meta.song_name.is_none());
        assert!(meta.song_url.is_none());
    }

    #[test]
    fn deserialize_ignores_unknown_fields() {
        let json = r#"{
            "result": "success",
            "unknown_field": 42,
            "data": {
                "song_url": "https://example.com/dl.7z",
                "extra_field": true
            }
        }"#;
        let resp: RespData<ChartMeta> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.result.as_deref(), Some("success"));
        assert_eq!(
            resp.data.unwrap().song_url.as_deref(),
            Some("https://example.com/dl.7z")
        );
    }

    #[test]
    fn meta_name_and_default_url() {
        let meta = &*META;
        assert_eq!(meta.get_name(), "konmai");
        assert_eq!(
            meta.get_default_url(),
            "https://bms.alvorna.com/api/hash?md5=%s"
        );
    }

    #[test]
    fn success_result_constant() {
        assert_eq!(SUCCESS_RESULT, "success");
    }
}
