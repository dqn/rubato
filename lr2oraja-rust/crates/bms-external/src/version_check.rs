use serde::Deserialize;
use tracing::{info, warn};

const DEFAULT_REPO: &str = "dqn/brs";
const GITHUB_API_BASE: &str = "https://api.github.com";

/// Result of comparing the current version against the latest GitHub release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionStatus {
    /// Current version matches the latest release.
    UpToDate,
    /// A newer version is available on GitHub.
    UpdateAvailable {
        current: String,
        latest: String,
        download_url: String,
    },
    /// Current version is newer than the latest release (development build).
    Development { current: String, latest: String },
    /// Version check failed (network error, parse error, etc.).
    CheckFailed(String),
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

/// Compare two semver version strings (MAJOR.MINOR.PATCH).
///
/// Returns `None` if either string cannot be parsed as semver.
fn version_cmp(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Option<(u64, u64, u64)> {
        let s = s.strip_prefix('v').unwrap_or(s);
        // Strip pre-release suffix (e.g., "-beta.1")
        let s = s.split('-').next()?;
        let mut parts = s.split('.');
        let major = parts.next()?.parse::<u64>().ok()?;
        let minor = parts.next()?.parse::<u64>().ok()?;
        let patch = parts.next()?.parse::<u64>().ok()?;
        // Reject extra components
        if parts.next().is_some() {
            return None;
        }
        Some((major, minor, patch))
    };

    let a = parse(a)?;
    let b = parse(b)?;
    Some(a.cmp(&b))
}

/// Check the latest release version from GitHub and compare with the current version.
///
/// `repo` defaults to `"dqn/brs"` when `None`.
pub async fn check_latest_version(current_version: &str, repo: Option<&str>) -> VersionStatus {
    check_latest_version_with_base(current_version, repo, GITHUB_API_BASE).await
}

/// Internal implementation that accepts a custom API base URL for testing.
async fn check_latest_version_with_base(
    current_version: &str,
    repo: Option<&str>,
    api_base: &str,
) -> VersionStatus {
    let repo = repo.unwrap_or(DEFAULT_REPO);
    let url = format!("{api_base}/repos/{repo}/releases/latest");

    let client = reqwest::Client::new();
    let response = match client
        .get(&url)
        .header("User-Agent", format!("brs/{current_version}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => return VersionStatus::CheckFailed(e.to_string()),
    };

    if !response.status().is_success() {
        return VersionStatus::CheckFailed(format!(
            "GitHub API returned status {}",
            response.status()
        ));
    }

    let release: GitHubRelease = match response.json().await {
        Ok(r) => r,
        Err(e) => return VersionStatus::CheckFailed(format!("Failed to parse response: {e}")),
    };

    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);

    match version_cmp(current_version, latest) {
        Some(std::cmp::Ordering::Equal) => {
            info!("brs is up to date (v{current_version})");
            VersionStatus::UpToDate
        }
        Some(std::cmp::Ordering::Less) => {
            warn!("Update available: v{current_version} -> v{latest}");
            VersionStatus::UpdateAvailable {
                current: current_version.to_string(),
                latest: latest.to_string(),
                download_url: release.html_url,
            }
        }
        Some(std::cmp::Ordering::Greater) => {
            info!("Running development version (v{current_version} > v{latest})");
            VersionStatus::Development {
                current: current_version.to_string(),
                latest: latest.to_string(),
            }
        }
        None => VersionStatus::CheckFailed(format!(
            "Failed to compare versions: current={current_version}, latest={latest}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn version_cmp_equal() {
        assert_eq!(version_cmp("1.2.3", "1.2.3"), Some(Ordering::Equal));
    }

    #[test]
    fn version_cmp_with_v_prefix() {
        assert_eq!(version_cmp("v1.2.3", "1.2.3"), Some(Ordering::Equal));
        assert_eq!(version_cmp("1.2.3", "v1.2.3"), Some(Ordering::Equal));
        assert_eq!(version_cmp("v1.2.3", "v1.2.3"), Some(Ordering::Equal));
    }

    #[test]
    fn version_cmp_newer() {
        assert_eq!(version_cmp("2.0.0", "1.9.9"), Some(Ordering::Greater));
        assert_eq!(version_cmp("1.3.0", "1.2.9"), Some(Ordering::Greater));
        assert_eq!(version_cmp("1.2.4", "1.2.3"), Some(Ordering::Greater));
    }

    #[test]
    fn version_cmp_older() {
        assert_eq!(version_cmp("1.0.0", "2.0.0"), Some(Ordering::Less));
        assert_eq!(version_cmp("1.2.3", "1.2.4"), Some(Ordering::Less));
        assert_eq!(version_cmp("0.9.9", "1.0.0"), Some(Ordering::Less));
    }

    #[test]
    fn version_cmp_prerelease_stripped() {
        // Pre-release suffixes are stripped, so base versions are compared
        assert_eq!(version_cmp("1.2.3-beta.1", "1.2.3"), Some(Ordering::Equal));
        assert_eq!(version_cmp("1.2.3-alpha", "1.2.4"), Some(Ordering::Less));
    }

    #[test]
    fn version_cmp_invalid() {
        assert_eq!(version_cmp("abc", "1.2.3"), None);
        assert_eq!(version_cmp("1.2", "1.2.3"), None);
        assert_eq!(version_cmp("1.2.3.4", "1.2.3"), None);
        assert_eq!(version_cmp("", "1.2.3"), None);
    }

    #[tokio::test]
    async fn check_latest_version_update_available() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/repos/dqn/brs/releases/latest"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "tag_name": "v2.0.0",
                    "html_url": "https://github.com/dqn/brs/releases/tag/v2.0.0"
                })),
            )
            .mount(&mock_server)
            .await;

        let status = check_latest_version_with_base("1.0.0", None, &mock_server.uri()).await;

        assert_eq!(
            status,
            VersionStatus::UpdateAvailable {
                current: "1.0.0".to_string(),
                latest: "2.0.0".to_string(),
                download_url: "https://github.com/dqn/brs/releases/tag/v2.0.0".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn check_latest_version_up_to_date() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/repos/dqn/brs/releases/latest"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "tag_name": "v1.0.0",
                    "html_url": "https://github.com/dqn/brs/releases/tag/v1.0.0"
                })),
            )
            .mount(&mock_server)
            .await;

        let status = check_latest_version_with_base("1.0.0", None, &mock_server.uri()).await;

        assert_eq!(status, VersionStatus::UpToDate);
    }

    #[tokio::test]
    async fn check_latest_version_development() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/repos/dqn/brs/releases/latest"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "tag_name": "v0.9.0",
                    "html_url": "https://github.com/dqn/brs/releases/tag/v0.9.0"
                })),
            )
            .mount(&mock_server)
            .await;

        let status = check_latest_version_with_base("1.0.0", None, &mock_server.uri()).await;

        assert_eq!(
            status,
            VersionStatus::Development {
                current: "1.0.0".to_string(),
                latest: "0.9.0".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn check_latest_version_404() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/repos/dqn/brs/releases/latest"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let status = check_latest_version_with_base("1.0.0", None, &mock_server.uri()).await;

        match status {
            VersionStatus::CheckFailed(msg) => {
                assert!(msg.contains("404"), "Expected 404 in message: {msg}");
            }
            other => panic!("Expected CheckFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn check_latest_version_network_error() {
        // Use an invalid URL to trigger a network error
        let status = check_latest_version_with_base("1.0.0", None, "http://127.0.0.1:1").await;

        match status {
            VersionStatus::CheckFailed(_) => {}
            other => panic!("Expected CheckFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn check_latest_version_custom_repo() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/repos/other/repo/releases/latest",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "tag_name": "v1.0.0",
                    "html_url": "https://github.com/other/repo/releases/tag/v1.0.0"
                })),
            )
            .mount(&mock_server)
            .await;

        let status =
            check_latest_version_with_base("1.0.0", Some("other/repo"), &mock_server.uri()).await;

        assert_eq!(status, VersionStatus::UpToDate);
    }
}
