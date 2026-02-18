// Download processor
//
// Manages download tasks with concurrent execution.
// Corresponds to Java HttpDownloadProcessor.java.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use tokio::sync::{Mutex, Semaphore};
use tracing::{info, warn};

use crate::extract;
use crate::task::{DownloadTask, DownloadTaskStatus};

/// Default maximum concurrent downloads.
const DEFAULT_MAX_CONCURRENT: usize = 5;
const DOWNLOAD_CONNECT_TIMEOUT_SECS: u64 = 10;
const DOWNLOAD_REQUEST_TIMEOUT_SECS: u64 = 120;
const MAX_ARCHIVE_BYTES: u64 = 1_073_741_824; // 1 GiB

/// Manages download tasks with concurrent execution limits.
pub struct HttpDownloadProcessor {
    tasks: Arc<Mutex<Vec<DownloadTask>>>,
    id_counter: AtomicUsize,
    semaphore: Arc<Semaphore>,
    download_dir: PathBuf,
    pub max_concurrent: usize,
}

impl HttpDownloadProcessor {
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        Self::with_max_concurrent(download_dir, DEFAULT_MAX_CONCURRENT)
    }

    pub fn with_max_concurrent(download_dir: impl Into<PathBuf>, max_concurrent: usize) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            id_counter: AtomicUsize::new(0),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            download_dir: download_dir.into(),
            max_concurrent,
        }
    }

    /// Add a new download task. Returns the task ID.
    pub async fn add_task(&self, url: String, name: String, hash: String) -> usize {
        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);
        let task = DownloadTask::new(id, url, name, hash);
        self.tasks.lock().await.push(task);
        id
    }

    /// Start downloading a task by ID.
    /// Spawns a tokio task that respects the concurrency semaphore.
    pub fn start_download(&self, task_id: usize) {
        let tasks = self.tasks.clone();
        let semaphore = self.semaphore.clone();
        let download_dir = self.download_dir.clone();

        tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(permit) => permit,
                Err(_) => {
                    warn!("Semaphore closed for task {}", task_id);
                    return;
                }
            };

            // Update status to Downloading
            {
                let mut tasks = tasks.lock().await;
                if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                    if task.status == DownloadTaskStatus::Cancel {
                        return;
                    }
                    task.set_status(DownloadTaskStatus::Downloading);
                } else {
                    warn!("Task {} not found", task_id);
                    return;
                }
            }

            // Get the URL
            let url = {
                let tasks = tasks.lock().await;
                match tasks.iter().find(|t| t.id == task_id) {
                    Some(task) => task.url.clone(),
                    None => return,
                }
            };

            // Download
            let client = match reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(DOWNLOAD_CONNECT_TIMEOUT_SECS))
                .timeout(Duration::from_secs(DOWNLOAD_REQUEST_TIMEOUT_SECS))
                .build()
            {
                Ok(client) => client,
                Err(e) => {
                    let mut tasks = tasks.lock().await;
                    if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                        task.set_error(format!("failed to create HTTP client: {}", e));
                    }
                    return;
                }
            };
            match download_file(&client, &url, &tasks, task_id).await {
                Ok(archive_path) => {
                    if is_cancelled(&tasks, task_id).await {
                        let _ = tokio::fs::remove_file(&archive_path).await;
                        return;
                    }

                    // Update status to Downloaded
                    {
                        let mut tasks = tasks.lock().await;
                        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                            task.set_status(DownloadTaskStatus::Downloaded);
                        }
                    }

                    if is_cancelled(&tasks, task_id).await {
                        let _ = tokio::fs::remove_file(&archive_path).await;
                        return;
                    }

                    // Extract
                    match extract::detect_and_extract(&archive_path, &download_dir) {
                        Ok(()) => {
                            let mut tasks = tasks.lock().await;
                            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                                task.set_status(DownloadTaskStatus::Extracted);
                            }
                            info!("Task {} extracted successfully", task_id);

                            // Clean up archive
                            if let Err(e) = tokio::fs::remove_file(&archive_path).await {
                                warn!("Failed to remove archive {:?}: {}", archive_path, e);
                            }
                        }
                        Err(e) => {
                            let mut tasks = tasks.lock().await;
                            if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                                task.set_error(format!("extraction failed: {}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    let mut tasks = tasks.lock().await;
                    if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                        task.set_error(format!("download failed: {}", e));
                    }
                }
            }
        });
    }

    /// Retry a failed task by ID.
    /// Resets the task to Prepare state and restarts the download.
    pub async fn retry_task(&self, task_id: usize) -> anyhow::Result<()> {
        {
            let mut tasks = self.tasks.lock().await;
            let task = tasks
                .iter_mut()
                .find(|t| t.id == task_id)
                .ok_or_else(|| anyhow!("task {} not found", task_id))?;
            if task.status != DownloadTaskStatus::Error {
                anyhow::bail!(
                    "task {} is not in Error state (current: {:?})",
                    task_id,
                    task.status
                );
            }
            task.status = DownloadTaskStatus::Prepare;
            task.error_message = None;
            task.download_size = 0;
            task.time_finished = None;
        }
        self.start_download(task_id);
        Ok(())
    }

    /// Cancel a task by ID.
    pub async fn cancel_task(&self, task_id: usize) {
        let mut tasks = self.tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.set_status(DownloadTaskStatus::Cancel);
        }
    }

    /// Get a snapshot of all tasks.
    pub async fn get_tasks(&self) -> Vec<DownloadTask> {
        self.tasks.lock().await.clone()
    }

    /// Get a snapshot of a single task by ID.
    pub async fn get_task(&self, task_id: usize) -> Option<DownloadTask> {
        self.tasks
            .lock()
            .await
            .iter()
            .find(|t| t.id == task_id)
            .cloned()
    }
}

/// Download a file from the given URL, streaming to disk.
/// Updates download_size and content_length on the task as data arrives.
async fn download_file(
    client: &reqwest::Client,
    url: &str,
    tasks: &Arc<Mutex<Vec<DownloadTask>>>,
    task_id: usize,
) -> anyhow::Result<PathBuf> {
    use tokio::io::AsyncWriteExt;

    if is_cancelled(tasks, task_id).await {
        anyhow::bail!("download cancelled");
    }
    let resp = client.get(url).send().await?.error_for_status()?;
    if is_cancelled(tasks, task_id).await {
        anyhow::bail!("download cancelled");
    }

    let content_length = resp.content_length().unwrap_or(0);
    if content_length != 0 {
        validate_download_size_limit(content_length)?;
    }

    // Determine filename from Content-Disposition header or URL
    let filename = resp
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .and_then(extract_filename_from_header)
        .and_then(|name| sanitize_download_filename(&name))
        .or_else(|| {
            url.split('?')
                .next()
                .and_then(|v| v.rsplit('/').next())
                .and_then(sanitize_download_filename)
        })
        .unwrap_or_else(|| "download".to_string());

    // Update content length
    {
        let mut tasks = tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.content_length = content_length;
        }
    }

    // Determine task hash for unique temporary file naming.
    let task_hash = {
        let tasks = tasks.lock().await;
        tasks
            .iter()
            .find(|t| t.id == task_id)
            .map(|t| t.hash.clone())
            .ok_or_else(|| anyhow!("task {} not found", task_id))?
    };

    // Stream to a unique temporary file path to avoid collisions between concurrent tasks.
    let file_path = build_temp_archive_path(task_id, &task_hash, &filename);
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::File::create(&file_path).await?;
    let mut downloaded: u64 = 0;
    let mut resp = resp;

    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        validate_download_size_limit(downloaded)?;

        // Update progress
        let mut tasks = tasks.lock().await;
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.download_size = downloaded;
            // Check for cancellation
            if task.status == DownloadTaskStatus::Cancel {
                drop(tasks);
                let _ = tokio::fs::remove_file(&file_path).await;
                anyhow::bail!("download cancelled");
            }
        }
    }

    file.flush().await?;

    Ok(file_path)
}

async fn is_cancelled(tasks: &Arc<Mutex<Vec<DownloadTask>>>, task_id: usize) -> bool {
    let tasks = tasks.lock().await;
    tasks
        .iter()
        .find(|t| t.id == task_id)
        .is_some_and(|task| task.status == DownloadTaskStatus::Cancel)
}

fn validate_download_size_limit(size: u64) -> anyhow::Result<()> {
    if size > MAX_ARCHIVE_BYTES {
        anyhow::bail!(
            "archive size exceeds the limit: {} > {}",
            size,
            MAX_ARCHIVE_BYTES
        );
    }
    Ok(())
}

fn build_temp_archive_path(task_id: usize, hash: &str, filename: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let hash_part: String = hash
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(16)
        .collect();
    let hash_part = if hash_part.is_empty() {
        "unknown".to_string()
    } else {
        hash_part
    };
    let filename = sanitize_download_filename(filename).unwrap_or_else(|| "download".to_string());
    std::env::temp_dir()
        .join("brs_download")
        .join(format!("{hash_part}_{task_id}_{nanos}_{filename}"))
}

/// Extract filename from Content-Disposition header value.
fn extract_filename_from_header(header: &str) -> Option<String> {
    // Match: filename="name.ext" or filename=name.ext
    let filename_prefix = "filename=";
    let pos = header.find(filename_prefix)?;
    let value = &header[pos + filename_prefix.len()..];
    let value = value.trim();
    if let Some(stripped) = value.strip_prefix('"') {
        // Quoted value
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        // Unquoted value - take until whitespace or semicolon
        let end = value
            .find(|c: char| c.is_whitespace() || c == ';')
            .unwrap_or(value.len());
        Some(value[..end].to_string())
    }
}

fn sanitize_download_filename(filename: &str) -> Option<String> {
    // Normalize path separators so Windows-style paths are treated as paths too.
    let normalized = filename.trim().replace('\\', "/");
    let normalized = normalized.trim_matches('/');
    if normalized.is_empty() {
        return None;
    }

    let base_name = Path::new(normalized).file_name()?.to_str()?;
    if matches!(base_name, "" | "." | "..") {
        return None;
    }
    Some(base_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_filename_quoted() {
        let header = r#"attachment; filename="test.7z""#;
        assert_eq!(extract_filename_from_header(header), Some("test.7z".into()));
    }

    #[test]
    fn test_extract_filename_unquoted() {
        let header = "attachment; filename=test.7z";
        assert_eq!(extract_filename_from_header(header), Some("test.7z".into()));
    }

    #[test]
    fn test_extract_filename_with_semicolon() {
        let header = "attachment; filename=test.7z; size=12345";
        assert_eq!(extract_filename_from_header(header), Some("test.7z".into()));
    }

    #[test]
    fn test_extract_filename_none() {
        let header = "inline";
        assert!(extract_filename_from_header(header).is_none());
    }

    #[test]
    fn test_sanitize_download_filename_removes_directories() {
        assert_eq!(
            sanitize_download_filename("../evil.7z"),
            Some("evil.7z".into())
        );
        assert_eq!(
            sanitize_download_filename("/tmp/evil.7z"),
            Some("evil.7z".into())
        );
        assert_eq!(
            sanitize_download_filename("C:\\temp\\evil.7z"),
            Some("evil.7z".into())
        );
    }

    #[test]
    fn test_sanitize_download_filename_rejects_invalid_names() {
        assert_eq!(sanitize_download_filename(""), None);
        assert_eq!(sanitize_download_filename("/"), None);
        assert_eq!(sanitize_download_filename("."), None);
        assert_eq!(sanitize_download_filename(".."), None);
    }

    #[test]
    fn test_validate_download_size_limit_allows_within_limit() {
        assert!(validate_download_size_limit(MAX_ARCHIVE_BYTES).is_ok());
    }

    #[test]
    fn test_validate_download_size_limit_rejects_over_limit() {
        let result = validate_download_size_limit(MAX_ARCHIVE_BYTES + 1);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("archive size exceeds the limit")
        );
    }

    #[test]
    fn test_extract_filename_then_sanitize() {
        let header = r#"attachment; filename="../safe.7z""#;
        let safe =
            extract_filename_from_header(header).and_then(|n| sanitize_download_filename(&n));
        assert_eq!(safe, Some("safe.7z".into()));
    }

    #[test]
    fn test_build_temp_archive_path_is_unique_per_invocation() {
        let p1 = build_temp_archive_path(1, "abc123", "pkg.7z");
        let p2 = build_temp_archive_path(1, "abc123", "pkg.7z");
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_build_temp_archive_path_includes_task_and_hash_prefix() {
        let path = build_temp_archive_path(42, "abcdef1234567890", "pkg.7z");
        let file = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
        assert!(file.starts_with("abcdef1234567890_42_"), "actual: {file}");
        assert!(file.ends_with("_pkg.7z"), "actual: {file}");
    }

    #[tokio::test]
    async fn test_add_task() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());

        let id = processor
            .add_task(
                "http://example.com/test.7z".into(),
                "test song".into(),
                "abc123".into(),
            )
            .await;
        assert_eq!(id, 0);

        let tasks = processor.get_tasks().await;
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].url, "http://example.com/test.7z");
        assert_eq!(tasks[0].name, "test song");
        assert_eq!(tasks[0].hash, "abc123");
        assert_eq!(tasks[0].status, DownloadTaskStatus::Prepare);
    }

    #[tokio::test]
    async fn test_add_multiple_tasks() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());

        let id1 = processor
            .add_task("http://a.com".into(), "song1".into(), "h1".into())
            .await;
        let id2 = processor
            .add_task("http://b.com".into(), "song2".into(), "h2".into())
            .await;

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);

        let tasks = processor.get_tasks().await;
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());

        let id = processor
            .add_task("http://example.com".into(), "test".into(), "hash".into())
            .await;
        processor.cancel_task(id).await;

        let task = processor.get_task(id).await.unwrap();
        assert_eq!(task.status, DownloadTaskStatus::Cancel);
        assert!(task.time_finished.is_some());
    }

    #[tokio::test]
    async fn test_get_nonexistent_task() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());

        assert!(processor.get_task(999).await.is_none());
    }

    #[test]
    fn test_default_max_concurrent() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());
        assert_eq!(processor.max_concurrent, DEFAULT_MAX_CONCURRENT);
    }

    #[test]
    fn test_custom_max_concurrent() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::with_max_concurrent(tmp.path(), 10);
        assert_eq!(processor.max_concurrent, 10);
    }

    #[tokio::test]
    async fn test_retry_task_from_error() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());

        let id = processor
            .add_task("http://example.com".into(), "test".into(), "hash".into())
            .await;

        // Put the task into Error state
        {
            let mut tasks = processor.tasks.lock().await;
            let task = tasks.iter_mut().find(|t| t.id == id).unwrap();
            task.set_error("connection timeout".into());
            task.download_size = 500;
        }

        // Retry should succeed and reset the task
        processor.retry_task(id).await.unwrap();

        let task = processor.get_task(id).await.unwrap();
        assert_eq!(task.status, DownloadTaskStatus::Prepare);
        assert!(task.error_message.is_none());
        assert_eq!(task.download_size, 0);
        assert!(task.time_finished.is_none());
    }

    #[tokio::test]
    async fn test_retry_task_non_error_state() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());

        let id = processor
            .add_task("http://example.com".into(), "test".into(), "hash".into())
            .await;

        // Task is in Prepare state, retry should fail
        let result = processor.retry_task(id).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not in Error state")
        );
    }

    #[tokio::test]
    async fn test_retry_task_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let processor = HttpDownloadProcessor::new(tmp.path());

        let result = processor.retry_task(999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
