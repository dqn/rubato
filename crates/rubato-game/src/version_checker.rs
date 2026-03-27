// VersionChecker: queries GitHub API for the latest release.

use std::io::Write;

pub use crate::core::version::Version;
use anyhow::{Result, bail};

/// A `Write` adapter that caps buffered data at a fixed size.
/// Returns `WriteZero` when the accumulated bytes would exceed the limit,
/// causing `Response::copy_to()` to abort the transfer early.
struct LimitedWriter {
    buf: Vec<u8>,
    limit: usize,
}

impl Write for LimitedWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.buf.len() + data.len() > self.limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "response exceeded size limit",
            ));
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Read response body with streaming size enforcement.
///
/// Unlike `response.bytes()`, this rejects oversized responses during streaming,
/// preventing memory exhaustion from chunked responses that omit Content-Length.
fn read_response_bytes_limited(
    mut response: reqwest::blocking::Response,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    if let Some(content_length) = response.content_length()
        && content_length > max_bytes
    {
        bail!("Response too large: {} bytes", content_length);
    }

    let mut writer = LimitedWriter {
        buf: Vec::new(),
        limit: max_bytes as usize,
    };

    match response.copy_to(&mut writer) {
        Ok(_) => Ok(writer.buf),
        Err(_) if writer.buf.len() >= writer.limit => {
            bail!("Response too large (>{} bytes)", max_bytes)
        }
        Err(e) => bail!("Failed to read response: {}", e),
    }
}

/// Version checker that queries GitHub API for the latest release.
///
/// Translated from: MainLoader.GithubVersionChecker
///
/// Lazily fetches version info from GitHub API on first access.
///
/// **Threading**: `message()` and `download_url()` perform blocking HTTP on first call.
/// Must be called from a background thread, not the UI/render thread.
#[derive(Clone, Debug, Default)]
pub struct VersionChecker {
    message: Option<String>,
    download_url: Option<String>,
}

impl VersionChecker {
    pub fn message(&mut self) -> &str {
        if self.message.is_none() {
            self.information();
        }
        self.message.as_deref().unwrap_or("")
    }

    pub fn download_url(&mut self) -> Option<&str> {
        if self.message.is_none() {
            self.information();
        }
        self.download_url.as_deref()
    }

    fn information(&mut self) {
        let result = self.fetch_latest_release();
        match result {
            Ok((name, html_url)) => {
                let cmp = Version::compare_to_string(Some(&name));
                if cmp == 0 {
                    self.message = Some("Already on the latest version".to_string());
                } else if cmp == -1 {
                    self.message = Some(format!("Version [{}] is available to download", name));
                    self.download_url = Some(html_url);
                } else {
                    self.message = Some(format!(
                        "On Development Build for {}",
                        Version::get_version()
                    ));
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch version info: {}", e);
                self.message = Some("Could not retrieve version information".to_string());
            }
        }
    }

    fn fetch_latest_release(&self) -> anyhow::Result<(String, String)> {
        const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024; // 4 MB

        let client = reqwest::blocking::Client::builder()
            .user_agent("rubato")
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let response = client
            .get("https://api.github.com/repos/seraxis/lr2oraja-endlessdream/releases/latest")
            .send()?;
        let bytes = read_response_bytes_limited(response, MAX_RESPONSE_BYTES)?;
        let resp: serde_json::Value = serde_json::from_slice(&bytes)?;
        let name = resp["name"].as_str().unwrap_or("").to_string();
        let html_url = resp["html_url"].as_str().unwrap_or("").to_string();
        Ok((name, html_url))
    }
}
