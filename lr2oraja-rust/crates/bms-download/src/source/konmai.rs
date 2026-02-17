// Konmai download source
//
// Queries the Konmai API for BMS chart metadata and returns the download URL.
// Default API: https://bms.alvorna.com/api/hash?md5={md5}

use anyhow::{anyhow, bail};
use serde::Deserialize;

use super::DownloadSource;

const DEFAULT_URL: &str = "https://bms.alvorna.com/api/hash?md5={}";

/// Response wrapper from the Konmai API.
#[derive(Debug, Deserialize)]
struct KonmaiResponse {
    result: String,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    data: Option<ChartMeta>,
}

/// Chart metadata from the Konmai API.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Parsed for completeness (Konmai API response fields)
struct ChartMeta {
    #[serde(default)]
    chart_name: Option<String>,
    #[serde(default)]
    md5: Option<String>,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    song_name: Option<String>,
    #[serde(default)]
    song_url: Option<String>,
}

/// Konmai download source that queries a metadata API to resolve download URLs.
pub struct KonmaiDownloadSource {
    /// API URL pattern with `{}` placeholder for the hash.
    query_url: String,
    client: reqwest::Client,
}

impl KonmaiDownloadSource {
    pub fn new(override_url: Option<&str>) -> Self {
        let query_url = override_url
            .filter(|u| !u.is_empty())
            .map(String::from)
            .unwrap_or_else(|| DEFAULT_URL.to_string());
        Self {
            query_url,
            client: reqwest::Client::new(),
        }
    }

    /// Return the default API URL.
    pub fn default_url() -> &'static str {
        DEFAULT_URL
    }
}

impl DownloadSource for KonmaiDownloadSource {
    fn name(&self) -> &str {
        "konmai"
    }

    async fn get_download_url(&self, hash: &str) -> anyhow::Result<String> {
        let url = self.query_url.replace("{}", hash);
        let resp = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json::<KonmaiResponse>()
            .await?;

        if resp.result != "success" {
            let msg = resp.msg.unwrap_or_else(|| "unknown error".into());
            bail!("Konmai API error: {}", msg);
        }

        let data = resp
            .data
            .ok_or_else(|| anyhow!("missing data in response"))?;
        let song_url = data
            .song_url
            .filter(|u| !u.is_empty())
            .ok_or_else(|| anyhow!("song not found for hash {}", hash))?;

        Ok(song_url)
    }

    fn allow_md5(&self) -> bool {
        true
    }

    fn allow_sha256(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_url() {
        let source = KonmaiDownloadSource::new(None);
        assert_eq!(source.query_url, DEFAULT_URL);
        assert_eq!(source.name(), "konmai");
    }

    #[test]
    fn test_override_url() {
        let source = KonmaiDownloadSource::new(Some("https://custom.api/hash?md5={}"));
        assert_eq!(source.query_url, "https://custom.api/hash?md5={}");
    }

    #[test]
    fn test_empty_override_uses_default() {
        let source = KonmaiDownloadSource::new(Some(""));
        assert_eq!(source.query_url, DEFAULT_URL);
    }

    #[test]
    fn test_allow_md5() {
        let source = KonmaiDownloadSource::new(None);
        assert!(source.allow_md5());
        assert!(!source.allow_sha256());
    }

    #[test]
    fn test_deserialize_success_response() {
        let json = r#"{
            "result": "success",
            "data": {
                "chart_name": "test",
                "md5": "abc123",
                "sha256": "def456",
                "song_name": "Test Song",
                "song_url": "https://example.com/download/test.7z"
            }
        }"#;
        let resp: KonmaiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.result, "success");
        let data = resp.data.unwrap();
        assert_eq!(
            data.song_url.as_deref(),
            Some("https://example.com/download/test.7z")
        );
        assert_eq!(data.song_name.as_deref(), Some("Test Song"));
    }

    #[test]
    fn test_deserialize_failure_response() {
        let json = r#"{"result": "fail", "msg": "not found"}"#;
        let resp: KonmaiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.result, "fail");
        assert_eq!(resp.msg.as_deref(), Some("not found"));
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_deserialize_empty_song_url() {
        let json = r#"{
            "result": "success",
            "data": {
                "song_url": ""
            }
        }"#;
        let resp: KonmaiResponse = serde_json::from_str(json).unwrap();
        let data = resp.data.unwrap();
        assert_eq!(data.song_url.as_deref(), Some(""));
    }
}
