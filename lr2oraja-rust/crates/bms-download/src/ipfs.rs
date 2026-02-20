// IPFS download processor
//
// Downloads BMS archives from IPFS gateways and handles diff-append merging.
// Corresponds to Java appendipfs functionality in SongDatabaseAccessor.

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use tracing::info;

use crate::extract;

/// Default IPFS gateway URL.
const DEFAULT_GATEWAY: &str = "https://gateway.ipfs.io";

/// Maximum archive size for IPFS downloads (512 MiB).
const MAX_IPFS_ARCHIVE_BYTES: u64 = 536_870_912;

/// Download timeout in seconds.
const IPFS_TIMEOUT_SECS: u64 = 120;

/// Result of an IPFS download operation.
pub struct IpfsDownloadResult {
    /// Path where the extracted files were placed.
    pub dest_dir: PathBuf,
    /// Whether this was an append (diff) download.
    pub is_append: bool,
}

/// Download a BMS archive from IPFS and extract it.
///
/// If `append_ipfs_path` is set and `original_dir` exists, performs a diff download:
/// downloads only the append archive and merges it into the original directory.
///
/// # Arguments
/// * `gateway_url` - IPFS gateway base URL (e.g. `https://gateway.ipfs.io`)
/// * `ipfs_path` - IPFS CID or path (e.g. `QmXyz...`)
/// * `dest_dir` - Destination directory for extraction
/// * `append_ipfs_path` - Optional append-only IPFS path for diff downloads
/// * `original_dir` - Original song directory for append merging
pub async fn download_ipfs(
    gateway_url: &str,
    ipfs_path: &str,
    dest_dir: &Path,
    append_ipfs_path: Option<&str>,
    original_dir: Option<&Path>,
) -> Result<IpfsDownloadResult> {
    // If append path is available and original directory exists, do diff download
    if let (Some(append_path), Some(orig_dir)) = (append_ipfs_path, original_dir)
        && !append_path.is_empty()
        && orig_dir.exists()
    {
        info!(
            append_path,
            orig = %orig_dir.display(),
            "IPFS: performing diff download"
        );
        return download_and_merge(gateway_url, append_path, orig_dir).await;
    }

    // Full download
    let url = build_gateway_url(gateway_url, ipfs_path);
    info!(url, dest = %dest_dir.display(), "IPFS: downloading full archive");

    let archive_path = download_archive(&url, dest_dir).await?;
    extract::detect_and_extract(&archive_path, dest_dir)?;

    // Clean up the archive file
    let _ = tokio::fs::remove_file(&archive_path).await;

    Ok(IpfsDownloadResult {
        dest_dir: dest_dir.to_path_buf(),
        is_append: false,
    })
}

/// Download append archive and merge into the original directory.
async fn download_and_merge(
    gateway_url: &str,
    append_path: &str,
    original_dir: &Path,
) -> Result<IpfsDownloadResult> {
    let url = build_gateway_url(gateway_url, append_path);

    // Download append archive to a temp directory
    let temp_dir = original_dir.with_extension("_ipfs_tmp");
    tokio::fs::create_dir_all(&temp_dir).await?;

    let archive_path = download_archive(&url, &temp_dir).await?;
    extract::detect_and_extract(&archive_path, &temp_dir)?;

    // Merge extracted files into original directory
    merge_directory(&temp_dir, original_dir).await?;

    // Clean up temp directory
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    info!(
        dest = %original_dir.display(),
        "IPFS: diff merge complete"
    );

    Ok(IpfsDownloadResult {
        dest_dir: original_dir.to_path_buf(),
        is_append: true,
    })
}

/// Build a full gateway URL from base URL and IPFS path.
fn build_gateway_url(gateway_url: &str, ipfs_path: &str) -> String {
    let base = gateway_url.trim_end_matches('/');
    let path = ipfs_path.trim_start_matches('/');

    // If the path already starts with "ipfs/", don't double it
    if path.starts_with("ipfs/") || path.starts_with("ipns/") {
        format!("{base}/{path}")
    } else {
        format!("{base}/ipfs/{path}")
    }
}

/// Download an archive file from a URL to the destination directory.
async fn download_archive(url: &str, dest_dir: &Path) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(IPFS_TIMEOUT_SECS))
        .build()?;

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!("IPFS download failed: HTTP {}", response.status()));
    }

    // Determine filename from Content-Disposition or URL
    let filename = response
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .and_then(extract_filename_from_header)
        .unwrap_or_else(|| "ipfs_archive.tar.gz".to_string());

    let archive_path = dest_dir.join(&filename);
    tokio::fs::create_dir_all(dest_dir).await?;

    let bytes = response.bytes().await?;
    if bytes.len() as u64 > MAX_IPFS_ARCHIVE_BYTES {
        return Err(anyhow!(
            "IPFS archive too large: {} bytes (max {})",
            bytes.len(),
            MAX_IPFS_ARCHIVE_BYTES
        ));
    }

    tokio::fs::write(&archive_path, &bytes).await?;
    info!(
        path = %archive_path.display(),
        size = bytes.len(),
        "IPFS: archive downloaded"
    );

    Ok(archive_path)
}

/// Extract filename from Content-Disposition header.
fn extract_filename_from_header(header: &str) -> Option<String> {
    header.split(';').find_map(|part| {
        let part = part.trim();
        if part.starts_with("filename=") {
            Some(
                part.trim_start_matches("filename=")
                    .trim_matches('"')
                    .to_string(),
            )
        } else {
            None
        }
    })
}

/// Recursively merge source directory contents into destination directory.
async fn merge_directory(src: &Path, dest: &Path) -> Result<()> {
    let mut entries = tokio::fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(&file_name);

        if src_path.is_dir() {
            tokio::fs::create_dir_all(&dest_path).await?;
            Box::pin(merge_directory(&src_path, &dest_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dest_path).await?;
        }
    }
    Ok(())
}

/// Resolve the IPFS gateway URL from config, falling back to the default.
pub fn resolve_gateway(config_url: &str) -> &str {
    if config_url.is_empty() {
        DEFAULT_GATEWAY
    } else {
        config_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_gateway_url_basic() {
        assert_eq!(
            build_gateway_url("https://gateway.ipfs.io", "QmXyz123"),
            "https://gateway.ipfs.io/ipfs/QmXyz123"
        );
    }

    #[test]
    fn build_gateway_url_trailing_slash() {
        assert_eq!(
            build_gateway_url("https://gateway.ipfs.io/", "QmXyz123"),
            "https://gateway.ipfs.io/ipfs/QmXyz123"
        );
    }

    #[test]
    fn build_gateway_url_already_prefixed() {
        assert_eq!(
            build_gateway_url("https://gateway.ipfs.io", "ipfs/QmXyz123"),
            "https://gateway.ipfs.io/ipfs/QmXyz123"
        );
    }

    #[test]
    fn build_gateway_url_ipns() {
        assert_eq!(
            build_gateway_url("https://gateway.ipfs.io", "ipns/example.com"),
            "https://gateway.ipfs.io/ipns/example.com"
        );
    }

    #[test]
    fn resolve_gateway_empty_uses_default() {
        assert_eq!(resolve_gateway(""), DEFAULT_GATEWAY);
    }

    #[test]
    fn resolve_gateway_custom() {
        assert_eq!(
            resolve_gateway("https://my-gateway.io"),
            "https://my-gateway.io"
        );
    }

    #[test]
    fn extract_filename_from_header_basic() {
        assert_eq!(
            extract_filename_from_header("attachment; filename=\"archive.tar.gz\""),
            Some("archive.tar.gz".to_string())
        );
    }

    #[test]
    fn extract_filename_from_header_no_quotes() {
        assert_eq!(
            extract_filename_from_header("attachment; filename=archive.zip"),
            Some("archive.zip".to_string())
        );
    }

    #[test]
    fn extract_filename_from_header_missing() {
        assert_eq!(extract_filename_from_header("attachment"), None);
    }
}
