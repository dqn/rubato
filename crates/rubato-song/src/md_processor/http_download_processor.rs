use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::Duration;

use regex::Regex;
use rubato_types::sync_utils::lock_or_recover;

use super::download_task::{DownloadTask, DownloadTaskStatus};
use super::http_download_source::HttpDownloadSource;
use super::http_download_source_meta::HttpDownloadSourceMeta;
use super::{ImGuiNotify, MainControllerRef};
use super::{konmai_download_source, wriggle_download_source};

pub static DOWNLOAD_SOURCES: LazyLock<HashMap<String, &'static HttpDownloadSourceMeta>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        // Wriggle
        let wriggle_meta: &HttpDownloadSourceMeta = &wriggle_download_source::META;
        map.insert(wriggle_meta.name().to_string(), wriggle_meta);
        // Konmai
        let konmai_meta: &HttpDownloadSourceMeta = &konmai_download_source::META;
        map.insert(konmai_meta.name().to_string(), konmai_meta);
        map
    });

pub const MAXIMUM_DOWNLOAD_COUNT: usize = 5;

/// Corresponds to HttpDownloadProcessor in Java
///
/// In-game download processor. In charge of:
/// - Manage all download tasks(stored in memory)
/// - Accept download task submission
/// - Download compressed files from remote http server
/// - Extract & update the 'songdata.db' automatically
pub struct HttpDownloadProcessor {
    download_directory: String,
    // id => task
    tasks: Arc<Mutex<HashMap<i32, Arc<Mutex<DownloadTask>>>>>,
    // O(1) duplicate URL check without iterating/locking individual tasks
    submitted_urls: Arc<Mutex<HashSet<String>>>,
    // O(1) duplicate MD5 check on the calling thread (no I/O) to avoid redundant spawns
    submitted_md5s: Arc<Mutex<HashSet<String>>>,
    // In-memory self-add id generator
    id_generator: Arc<AtomicI32>,
    // Active download thread count, enforces MAXIMUM_DOWNLOAD_COUNT
    active_downloads: Arc<AtomicUsize>,
    // A reference to the main controller, only used for updating folder and rendering the message
    main: Arc<dyn MainControllerRef>,
    http_download_source: Arc<dyn HttpDownloadSource>,
}

impl HttpDownloadProcessor {
    pub fn new(
        main: Arc<dyn MainControllerRef>,
        http_download_source: Arc<dyn HttpDownloadSource>,
        download_directory: String,
    ) -> Self {
        HttpDownloadProcessor {
            download_directory,
            tasks: Arc::new(Mutex::new(HashMap::new())),
            active_downloads: Arc::new(AtomicUsize::new(0)),
            submitted_urls: Arc::new(Mutex::new(HashSet::new())),
            submitted_md5s: Arc::new(Mutex::new(HashSet::new())),
            id_generator: Arc::new(AtomicI32::new(0)),
            main,
            http_download_source,
        }
    }

    pub fn default_download_source() -> &'static HttpDownloadSourceMeta {
        &wriggle_download_source::META
    }

    // Would be best if this returned an immutable view over the tasks,
    // without creating a copy, in the interest of efficiency,
    // however I'm not sure if that is possible in java
    pub fn all_tasks(&self) -> Arc<Mutex<HashMap<i32, Arc<Mutex<DownloadTask>>>>> {
        self.tasks.clone()
    }

    /// Submit a download task based on md5
    ///
    /// # Arguments
    /// * `md5` - missing sabun's md5
    /// * `task_name` - task name, normally sabun's name
    pub fn submit_md5_task(&self, md5: &str, task_name: &str) {
        log::info!(
            "[HttpDownloadProcessor] Trying to submit new download task[{}](based on md5: {})",
            task_name,
            md5
        );

        // Early md5-based dedup on the calling thread (cheap, no I/O).
        {
            let mut md5s = lock_or_recover(&self.submitted_md5s);
            if md5s.contains(md5) {
                log::info!(
                    "[HttpDownloadProcessor] Rejecting download task[{}] because md5 {} is already being resolved",
                    task_name,
                    md5
                );
                ImGuiNotify::warning("Already submitted");
                return;
            }
            md5s.insert(md5.to_string());
        }

        // Move the blocking get_download_url_based_on_md5() call off the calling thread.
        let http_download_source = self.http_download_source.clone();
        let submitted_md5s = self.submitted_md5s.clone();
        let submitted_urls = self.submitted_urls.clone();
        let tasks = self.tasks.clone();
        let id_generator = self.id_generator.clone();
        let active_downloads = self.active_downloads.clone();
        let download_directory = self.download_directory.clone();
        let main = self.main.clone();
        let md5 = md5.to_string();
        let task_name = task_name.to_string();

        thread::spawn(move || {
            // Guard that cleans up the md5 from submitted_md5s when this thread exits
            // (whether via success, error, or panic).
            struct Md5Guard {
                submitted_md5s: Arc<Mutex<HashSet<String>>>,
                md5: String,
            }
            impl Drop for Md5Guard {
                fn drop(&mut self) {
                    if let Ok(mut md5s) = self.submitted_md5s.lock() {
                        md5s.remove(&self.md5);
                    }
                }
            }
            let _md5_guard = Md5Guard {
                submitted_md5s: submitted_md5s.clone(),
                md5: md5.clone(),
            };

            let source_name = http_download_source.name().to_string();

            // Blocking HTTP call to resolve the download URL from the md5.
            let download_url = match http_download_source.get_download_url_based_on_md5(&md5) {
                Ok(url) => url,
                Err(e) => {
                    // Fragile: uses string comparison for error discrimination.
                    // Typed error enum would be more robust, but this matches the existing protocol.
                    let err_msg = e.to_string();
                    if err_msg == "FileNotFound" {
                        log::error!(
                            "[HttpDownloadProcessor] Remote server[{}] reports no such data",
                            source_name
                        );
                        ImGuiNotify::error(&format!(
                            "Cannot find specified song from {}",
                            source_name
                        ));
                    } else {
                        log::error!(
                            "[HttpDownloadProcessor] Cannot get download url from remote server[{}] due to unexpected exception: {}",
                            source_name,
                            err_msg
                        );
                        ImGuiNotify::error(&format!(
                            "{} returns a severe error: {}",
                            source_name, err_msg
                        ));
                    }
                    return;
                }
            };

            // URL-based dedup (prevents duplicate downloads of the same URL from different md5s).
            let download_task = {
                let mut urls = lock_or_recover(&submitted_urls);
                if urls.contains(&download_url) {
                    log::error!(
                        "[HttpDownloadProcessor] Rejecting download task[{}] because duplication has been found",
                        download_url
                    );
                    ImGuiNotify::warning("Already submitted");
                    return;
                }
                let task_id = id_generator.fetch_add(1, Ordering::SeqCst) + 1;
                let download_task = Arc::new(Mutex::new(DownloadTask::new(
                    task_id,
                    download_url.clone(),
                    task_name.clone(),
                    md5.clone(),
                )));
                urls.insert(download_url);
                drop(urls);
                let mut all_tasks = lock_or_recover(&tasks);
                all_tasks.insert(task_id, download_task.clone());
                ImGuiNotify::info(&format!("New download task[{}] submitted", task_name));
                download_task
            };

            // Execute the download (reserve slot, spawn download thread).
            execute_download_task_static(
                download_task,
                &active_downloads,
                &submitted_urls,
                &download_directory,
                &main,
                &source_name,
            );
        });
    }

    /// Execute the download task, which are chained steps:
    /// 1. Download the archive file from url
    /// 2. Extract the package
    /// 3. Update download directory
    /// 4. Delete the archive file
    pub fn execute_download_task(&self, download_task: Arc<Mutex<DownloadTask>>) {
        let source_name = self.http_download_source.name().to_string();
        execute_download_task_static(
            download_task,
            &self.active_downloads,
            &self.submitted_urls,
            &self.download_directory,
            &self.main,
            &source_name,
        );
    }

    /// Retry a download task
    pub fn retry_download_task(&self, download_task: Arc<Mutex<DownloadTask>>) {
        {
            let mut task = lock_or_recover(&download_task);
            task.set_download_task_status(DownloadTaskStatus::Prepare);
        }
        self.execute_download_task(download_task);
    }
}

/// Static helper for `execute_download_task` so it can be called both from `&self` methods
/// and from inside spawned threads (where `&self` is not available).
fn execute_download_task_static(
    download_task: Arc<Mutex<DownloadTask>>,
    active_downloads: &Arc<AtomicUsize>,
    submitted_urls: &Arc<Mutex<HashSet<String>>>,
    download_directory: &str,
    main: &Arc<dyn MainControllerRef>,
    source_name: &str,
) {
    // Reserve a download slot atomically using compare_exchange to prevent
    // concurrent threads from exceeding MAXIMUM_DOWNLOAD_COUNT.
    loop {
        let current = active_downloads.load(Ordering::Acquire);
        if current >= MAXIMUM_DOWNLOAD_COUNT {
            log::warn!(
                "[HttpDownloadProcessor] Maximum concurrent downloads ({}) reached, rejecting task",
                MAXIMUM_DOWNLOAD_COUNT
            );
            ImGuiNotify::warning("Download queue is full, try again later");
            let mut task = lock_or_recover(&download_task);
            // Release the URL from submitted_urls so the user can retry later
            if let Ok(mut urls) = submitted_urls.lock() {
                urls.remove(task.url());
            }
            task.set_download_task_status(DownloadTaskStatus::Error);
            return;
        }
        if active_downloads
            .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            break;
        }
    }

    let download_directory = download_directory.to_string();
    let main = main.clone();
    let source_name = source_name.to_string();
    let active_downloads = active_downloads.clone();
    let submitted_urls = submitted_urls.clone();

    thread::spawn(move || {
        struct DownloadGuard {
            active_downloads: Arc<AtomicUsize>,
            submitted_urls: Arc<Mutex<HashSet<String>>>,
            download_url: String,
        }
        impl Drop for DownloadGuard {
            fn drop(&mut self) {
                self.active_downloads.fetch_sub(1, Ordering::AcqRel);
                // Remove URL from submitted set so it can be retried
                if let Ok(mut urls) = self.submitted_urls.lock() {
                    urls.remove(&self.download_url);
                }
            }
        }
        let (task_name, download_url, hash) = {
            let task = lock_or_recover(&download_task);
            (
                task.name().to_string(),
                task.url().to_string(),
                task.hash().to_string(),
            )
        };
        let _guard = DownloadGuard {
            active_downloads,
            submitted_urls,
            download_url: download_url.clone(),
        };
        log::info!(
            "[HttpDownloadProcessor] Trying to kick new download task[{}]({})",
            task_name,
            download_url
        );
        {
            let mut task = lock_or_recover(&download_task);
            task.set_download_task_status(DownloadTaskStatus::Downloading);
        }
        // 1) Download file from remote http server
        let result = match download_file_from_url(
            &download_task,
            &format!("{}.7z", hash),
            &download_directory,
            &source_name,
        ) {
            Ok(path) => Some(path),
            Err(e) => {
                log::error!("{}", e);
                ImGuiNotify::error(&format!(
                    "Failed downloading from {} due to {}",
                    source_name, e
                ));
                None
            }
        };
        if result.is_none() {
            // Download failed, skip the remaining steps
            let mut task = lock_or_recover(&download_task);
            task.set_download_task_status(DownloadTaskStatus::Error);
            return;
        }
        let result = result.expect("result");
        // 2) Extract the compressed archive & update download directory automatically
        let mut successfully_extracted = false;
        let mut bms_directory: Option<String> = None;
        match extract_compressed_file(&result, None, &download_directory) {
            Ok(dir) => {
                bms_directory = dir;
                successfully_extracted = true;
                let mut task = lock_or_recover(&download_task);
                task.set_download_task_status(DownloadTaskStatus::Extracted);
            }
            Err(e) => {
                log::error!("{}", e);
                ImGuiNotify::error(&format!(
                    "Failed extracting file: {} due to {}",
                    result.display(),
                    e
                ));
            }
        }
        if successfully_extracted {
            // Note: Directory update is protected, this might cause some uncovered situation. Personally speaking,
            // I don't think this has any issue since user can always turn back to root directory
            // and update the download directory manually
            ImGuiNotify::info(
                "Successfully downloaded & extracted. Trying to rebuild download directory",
            );
            if let Some(ref dir) = bms_directory {
                main.update_song(dir, true);
            }
            // If everything works well, trying to delete the downloaded archive
            if let Err(e) = fs::remove_file(&result) {
                log::error!("{}", e);
                ImGuiNotify::error("Failed deleting archive file automatically");
            }
        }
    });
}

impl rubato_types::http_download_submitter::HttpDownloadSubmitter for HttpDownloadProcessor {
    fn submit_md5_task(&self, md5: &str, task_name: &str) {
        HttpDownloadProcessor::submit_md5_task(self, md5, task_name);
    }
}

/// Download a file from url
///
/// Writes to a temporary `.tmp` file first and renames to the final path on
/// success, so a partial download never destroys an existing file.
///
/// # Arguments
/// * `fallback_file_name` - fallback file name if remote server's response doesn't contain a valid file name
///
/// # Returns
/// result file path
fn download_file_from_url(
    task: &Arc<Mutex<DownloadTask>>,
    fallback_file_name: &str,
    download_directory: &str,
    source_name: &str,
) -> anyhow::Result<PathBuf> {
    let url = {
        let t = lock_or_recover(task);
        t.url().to_string()
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let response = client.get(&url).send()?;
    let response_code = response.status();
    if response_code != reqwest::StatusCode::OK {
        if response_code == reqwest::StatusCode::NOT_FOUND {
            return Err(anyhow::anyhow!("Package not found at {}", source_name));
        }
        return Err(anyhow::anyhow!(
            "Unexpected http response code: {}",
            response_code.as_u16()
        ));
    }

    // Prepare the file name
    let mut file_name = fallback_file_name.to_string();
    let content_disposition = response
        .headers()
        .get("Content-Disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let mut candidate_file_name = String::new();
    if !content_disposition.is_empty() {
        let re = Regex::new(r#"filename="?([^"]+)"?"#).expect("valid regex");
        if let Some(caps) = re.captures(&content_disposition)
            && let Some(m) = caps.get(1)
        {
            candidate_file_name = m.as_str().to_string();
        }
    }
    if !candidate_file_name.is_empty() {
        // Sanitize filename to prevent path traversal from malicious Content-Disposition headers
        file_name = std::path::Path::new(&candidate_file_name)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(candidate_file_name);
    }

    let content_length = response.content_length().map(|l| l as i64).unwrap_or(-1);

    let result = Path::new(download_directory).join(&file_name);
    let tmp_path = result.with_extension(format!(
        "{}.tmp",
        result
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default()
    ));

    // Stream body in chunks to a temporary file to avoid destroying an
    // existing file if the download fails partway through.
    let write_result = (|| -> anyhow::Result<()> {
        let mut fos = fs::File::create(&tmp_path)?;
        let mut download_bytes: i64 = 0;
        let mut buf = [0u8; 8192];
        let mut reader = response;
        loop {
            let read = std::io::Read::read(&mut reader, &mut buf)?;
            if read == 0 {
                break;
            }
            fos.write_all(&buf[..read])?;
            download_bytes += read as i64;
            {
                let mut t = lock_or_recover(task);
                t.download_size = download_bytes;
                t.content_length = content_length;
            }
        }
        Ok(())
    })();

    if let Err(e) = write_result {
        // Clean up the partial temp file on failure
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    // Atomically move the completed download to the final path
    fs::rename(&tmp_path, &result)?;

    log::info!(
        "[HttpDownloadProcessor] Download successfully to {}",
        result.display()
    );
    {
        let mut t = lock_or_recover(task);
        t.set_download_task_status(DownloadTaskStatus::Downloaded);
    }

    Ok(result)
}

/// Extract a compressed file into target_path
///
/// # Arguments
/// * `file` - compressed archive
/// * `target_path` - target directory, fallback to download_directory if None
///
/// # Returns
/// the path to the directory just extracted
fn extract_compressed_file(
    file: &Path,
    target_path: Option<&Path>,
    download_directory: &str,
) -> anyhow::Result<Option<String>> {
    let dest = target_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(download_directory));

    if !dest.exists() {
        fs::create_dir_all(&dest)?;
    }

    // Pre-validate 7z entry names from archive metadata BEFORE extraction.
    // This prevents path-traversal attacks via ../ entries in the archive;
    // malicious entries are rejected before any bytes are written to disk.
    validate_archive_entry_names(file)?;

    // Extract into a temp directory first, then validate paths before moving.
    // This prevents symlink-based escapes that metadata validation cannot catch.
    let staging_dir = tempfile::tempdir_in(&dest)?;

    sevenz_rust::decompress_file(file, staging_dir.path())
        .map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;

    // Defense-in-depth: validate all extracted paths stay within the staging
    // directory (catches symlink escapes that entry-name checks cannot detect).
    let canonical_staging = staging_dir.path().canonicalize()?;
    validate_extracted_paths(&canonical_staging)?;

    // Record pre-existing entries so we can detect what was newly added.
    let pre_existing: std::collections::HashSet<_> = fs::read_dir(&dest)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name())
        .collect();

    // Move validated contents from staging into dest.
    let mut has_new_files = false;
    for entry in fs::read_dir(staging_dir.path())?.flatten() {
        let target = dest.join(entry.file_name());
        // rename may fail across mount points; fall back to copy+remove
        if fs::rename(entry.path(), &target).is_err() {
            if entry.path().is_dir() {
                copy_dir_recursive(&entry.path(), &target)?;
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::copy(entry.path(), &target)?;
                fs::remove_file(entry.path())?;
            }
        }
        has_new_files = true;
    }

    // Find the first newly created subdirectory.
    let mut extracted_dir = None;
    if let Ok(entries) = fs::read_dir(&dest) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && !pre_existing.contains(&entry.file_name()) {
                extracted_dir = Some(path.to_string_lossy().to_string());
                break;
            }
        }
    }

    // If files were extracted at root level (no subdirectory), return download_directory
    // so that the song updater still rescans the location.
    if extracted_dir.is_none() && has_new_files {
        extracted_dir = Some(dest.to_string_lossy().to_string());
    }

    Ok(extracted_dir)
}

/// Pre-validate 7z archive entry names for path traversal before extraction.
///
/// Rejects entries whose names contain parent-directory components (`..`) or
/// are absolute paths, which could write files outside the intended directory.
fn validate_archive_entry_names(file: &Path) -> anyhow::Result<()> {
    let reader = sevenz_rust::SevenZReader::open(file, sevenz_rust::Password::empty())
        .map_err(|e| anyhow::anyhow!("Failed to open 7z archive for validation: {}", e))?;
    for entry in &reader.archive().files {
        let entry_path = Path::new(&entry.name);
        if entry_path.is_absolute() {
            return Err(anyhow::anyhow!(
                "Path traversal detected in archive: absolute path '{}'",
                entry.name
            ));
        }
        for component in entry_path.components() {
            if matches!(component, Component::ParentDir) {
                return Err(anyhow::anyhow!(
                    "Path traversal detected in archive: parent directory component in '{}'",
                    entry.name
                ));
            }
        }
    }
    Ok(())
}

/// Recursively validate that all paths under `root` are within `root` (no symlink escapes).
fn validate_extracted_paths(root: &Path) -> anyhow::Result<()> {
    validate_extracted_paths_recursive(root, root)
}

fn validate_extracted_paths_recursive(root: &Path, dir: &Path) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let canonical = entry.path().canonicalize()?;
        if !canonical.starts_with(root) {
            return Err(anyhow::anyhow!(
                "Path traversal detected: {} escapes {}",
                entry.path().display(),
                root.display()
            ));
        }
        if entry.path().is_dir() {
            validate_extracted_paths_recursive(root, &entry.path())?;
        }
    }
    Ok(())
}

/// Recursively copy a directory tree.
pub(super) fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)?.flatten() {
        let target = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    struct FakeMainControllerRef;
    impl MainControllerRef for FakeMainControllerRef {
        fn update_song(&self, _path: &str, _force: bool) {}
    }

    /// A fake download source that records calls and blocks until signaled.
    struct FakeHttpDownloadSource {
        call_count: AtomicUsize,
        /// When set, `get_download_url_based_on_md5` blocks until this is true.
        unblock: Arc<AtomicBool>,
        url_to_return: String,
    }

    impl FakeHttpDownloadSource {
        fn new(url: &str) -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                unblock: Arc::new(AtomicBool::new(true)),
                url_to_return: url.to_string(),
            }
        }

        fn new_blocking(url: &str, unblock: Arc<AtomicBool>) -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                unblock,
                url_to_return: url.to_string(),
            }
        }
    }

    impl HttpDownloadSource for FakeHttpDownloadSource {
        fn get_download_url_based_on_md5(&self, _md5: &str) -> anyhow::Result<String> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            // Spin-wait until unblocked
            while !self.unblock.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(5));
            }
            Ok(self.url_to_return.clone())
        }

        fn name(&self) -> &str {
            "FakeSource"
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

    fn make_processor(source: Arc<dyn HttpDownloadSource>) -> HttpDownloadProcessor {
        HttpDownloadProcessor::new(
            Arc::new(FakeMainControllerRef),
            source,
            "/tmp/rubato-test-downloads".to_string(),
        )
    }

    /// Submitting the same md5 twice should be deduplicated on the calling thread
    /// (the second call should not spawn a background resolve).
    #[test]
    fn duplicate_md5_is_rejected_on_calling_thread() {
        let unblock = Arc::new(AtomicBool::new(false));
        let source = Arc::new(FakeHttpDownloadSource::new_blocking(
            "https://example.com/song.7z",
            unblock.clone(),
        ));
        let processor = make_processor(source.clone());

        // First submit should go through (spawn background thread).
        processor.submit_md5_task("abc123", "Song A");
        // Give the background thread a moment to start but it will block on unblock.
        std::thread::sleep(Duration::from_millis(50));

        // Second submit with same md5 should be rejected immediately.
        processor.submit_md5_task("abc123", "Song A duplicate");

        // The source should have been called at most once (background thread for first submit).
        // The second submit should NOT have spawned another thread or called the source again.
        let calls = source.call_count.load(Ordering::SeqCst);
        assert!(
            calls <= 1,
            "Expected at most 1 call to get_download_url_based_on_md5, got {}",
            calls,
        );

        // Unblock the background thread so it can finish.
        unblock.store(true, Ordering::Release);
        // Wait for the background thread to complete and clean up.
        std::thread::sleep(Duration::from_millis(200));
    }

    /// After the background resolve completes (and the download inevitably errors
    /// because there's no real server), the md5 should be cleaned up so a retry works.
    #[test]
    fn md5_is_cleaned_up_after_resolve_completes() {
        let source = Arc::new(FakeHttpDownloadSource::new("https://example.com/song.7z"));
        let processor = make_processor(source.clone());

        processor.submit_md5_task("def456", "Song B");

        // Wait for the background thread to resolve the URL, attempt the download
        // (which will fail since there's no real server), and clean up.
        std::thread::sleep(Duration::from_millis(500));

        // The md5 should be cleaned up now.
        let md5s = processor.submitted_md5s.lock().unwrap();
        assert!(
            !md5s.contains("def456"),
            "md5 should be cleaned up after resolve completes",
        );
    }

    /// submit_md5_task should return immediately without blocking (the URL resolve
    /// happens in a background thread).
    #[test]
    fn submit_does_not_block_caller() {
        let unblock = Arc::new(AtomicBool::new(false));
        let source = Arc::new(FakeHttpDownloadSource::new_blocking(
            "https://example.com/song.7z",
            unblock.clone(),
        ));
        let processor = make_processor(source);

        let start = std::time::Instant::now();
        processor.submit_md5_task("ghi789", "Song C");
        let elapsed = start.elapsed();

        // The call should return nearly instantly (well under 100ms).
        // The blocking source would take much longer if called synchronously.
        assert!(
            elapsed < Duration::from_millis(100),
            "submit_md5_task blocked for {:?}, expected < 100ms",
            elapsed,
        );

        // Clean up.
        unblock.store(true, Ordering::Release);
        std::thread::sleep(Duration::from_millis(200));
    }

    /// Different md5s should not be deduplicated.
    #[test]
    fn different_md5s_are_not_deduplicated() {
        let source = Arc::new(FakeHttpDownloadSource::new("https://example.com/song.7z"));
        let processor = make_processor(source.clone());

        processor.submit_md5_task("aaa111", "Song 1");
        processor.submit_md5_task("bbb222", "Song 2");

        // Wait for background threads to at least start.
        std::thread::sleep(Duration::from_millis(200));

        // Both should have called the source.
        let calls = source.call_count.load(Ordering::SeqCst);
        assert_eq!(
            calls, 2,
            "Expected 2 calls for different md5s, got {}",
            calls
        );
    }

    /// Helper: create a 7z archive with a single entry whose name is `entry_name`.
    fn create_7z_with_entry_name(archive_path: &Path, entry_name: &str) {
        let mut writer = sevenz_rust::SevenZWriter::create(archive_path).expect("create 7z writer");
        // Use new() + field assignment to avoid private field issue with struct literal syntax.
        let mut entry = sevenz_rust::SevenZArchiveEntry::new();
        entry.name = entry_name.to_string();
        entry.has_stream = true;
        let data: &[u8] = b"malicious content";
        writer
            .push_archive_entry(entry, Some(std::io::Cursor::new(data)))
            .expect("push entry");
        writer.finish().expect("finish 7z");
    }

    /// Archives with parent-directory traversal (`../`) in entry names must be
    /// rejected BEFORE extraction writes anything to disk.
    #[test]
    fn rejects_archive_with_parent_dir_traversal() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let archive_path = tmp.path().join("traversal.7z");
        create_7z_with_entry_name(&archive_path, "../../etc/evil.txt");

        let result = validate_archive_entry_names(&archive_path);
        assert!(result.is_err(), "expected error for parent-dir traversal");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("parent directory component"),
            "error should mention parent directory component, got: {}",
            err_msg
        );
    }

    /// Archives with absolute paths in entry names must be rejected.
    #[test]
    fn rejects_archive_with_absolute_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let archive_path = tmp.path().join("absolute.7z");
        create_7z_with_entry_name(&archive_path, "/etc/passwd");

        let result = validate_archive_entry_names(&archive_path);
        assert!(result.is_err(), "expected error for absolute path");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("absolute path"),
            "error should mention absolute path, got: {}",
            err_msg
        );
    }

    /// Archives with safe entry names should pass validation.
    #[test]
    fn accepts_archive_with_safe_entry_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let archive_path = tmp.path().join("safe.7z");
        create_7z_with_entry_name(&archive_path, "songs/test/chart.bms");

        let result = validate_archive_entry_names(&archive_path);
        assert!(result.is_ok(), "expected safe entry to pass validation");
    }

    /// Full extract_compressed_file should reject archives with path traversal
    /// entries before any files are written outside the staging directory.
    #[test]
    fn extract_rejects_path_traversal_archive() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let archive_path = tmp.path().join("evil.7z");
        create_7z_with_entry_name(&archive_path, "../escape.txt");

        let dest = tmp.path().join("output");
        fs::create_dir_all(&dest).expect("create dest");

        let result = extract_compressed_file(&archive_path, Some(&dest), &dest.to_string_lossy());
        assert!(
            result.is_err(),
            "extract should fail for path-traversal archive"
        );

        // Verify no escaped file was written.
        assert!(
            !tmp.path().join("escape.txt").exists(),
            "escaped file should not exist outside staging directory"
        );
    }
}
