// VersionChecker: queries GitHub API for the latest release.

pub use rubato_core::version::Version;

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
        let client = reqwest::blocking::Client::builder()
            .user_agent("rubato")
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let response = client
            .get("https://api.github.com/repos/seraxis/lr2oraja-endlessdream/releases/latest")
            .send()?;
        let bytes = response.bytes()?;
        if bytes.len() > 4 * 1024 * 1024 {
            anyhow::bail!("Response too large: {} bytes", bytes.len());
        }
        let resp: serde_json::Value = serde_json::from_slice(&bytes)?;
        let name = resp["name"].as_str().unwrap_or("").to_string();
        let html_url = resp["html_url"].as_str().unwrap_or("").to_string();
        Ok((name, html_url))
    }
}
