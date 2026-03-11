use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;

use regex::Regex;

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
    // In-memory self-add id generator
    id_generator: AtomicI32,
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
            id_generator: AtomicI32::new(0),
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
        let source_name = self.http_download_source.name().to_string();
        let download_url = match self.http_download_source.get_download_url_based_on_md5(md5) {
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
                    ImGuiNotify::error(&format!("Cannot find specified song from {}", source_name));
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

        // NOTE: The reason of using executor instead of using 'synchronized' on tasks directly is forcing
        // it to run the submit step on an different thread to get rid of the re-entrant feature of 'synchronized'.
        let download_task = {
            // Check for duplicate URLs via submitted_urls set (O(1), no nested locking)
            let mut urls = self
                .submitted_urls
                .lock()
                .expect("submitted_urls lock poisoned");
            if urls.contains(&download_url) {
                log::error!(
                    "[HttpDownloadProcessor] Rejecting download task[{}] because duplication has been found",
                    download_url
                );
                ImGuiNotify::warning("Already submitted");
                return;
            }
            let task_id = self.id_generator.fetch_add(1, Ordering::SeqCst) + 1;
            let download_task = Arc::new(Mutex::new(DownloadTask::new(
                task_id,
                download_url.clone(),
                task_name.to_string(),
                md5.to_string(),
            )));
            urls.insert(download_url);
            drop(urls);
            let mut tasks = self.tasks.lock().expect("tasks lock poisoned");
            tasks.insert(task_id, download_task.clone());
            ImGuiNotify::info(&format!("New download task[{}] submitted", task_name));
            download_task
        };

        self.execute_download_task(download_task);
    }

    /// Execute the download task, which are chained steps:
    /// 1. Download the archive file from url
    /// 2. Extract the package
    /// 3. Update download directory
    /// 4. Delete the archive file
    pub fn execute_download_task(&self, download_task: Arc<Mutex<DownloadTask>>) {
        // Reserve a download slot atomically using compare_exchange to prevent
        // concurrent threads from exceeding MAXIMUM_DOWNLOAD_COUNT.
        loop {
            let current = self.active_downloads.load(Ordering::Acquire);
            if current >= MAXIMUM_DOWNLOAD_COUNT {
                log::warn!(
                    "[HttpDownloadProcessor] Maximum concurrent downloads ({}) reached, rejecting task",
                    MAXIMUM_DOWNLOAD_COUNT
                );
                ImGuiNotify::warning("Download queue is full, try again later");
                let mut task = download_task.lock().expect("download_task lock poisoned");
                // Release the URL from submitted_urls so the user can retry later
                if let Ok(mut urls) = self.submitted_urls.lock() {
                    urls.remove(task.url());
                }
                task.set_download_task_status(DownloadTaskStatus::Error);
                return;
            }
            if self
                .active_downloads
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }

        let download_directory = self.download_directory.clone();
        let main = self.main.clone();
        let source_name = self.http_download_source.name().to_string();
        let active_downloads = self.active_downloads.clone();
        let submitted_urls = self.submitted_urls.clone();

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
                let task = download_task.lock().expect("download_task lock poisoned");
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
                let mut task = download_task.lock().expect("download_task lock poisoned");
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
                let mut task = download_task.lock().expect("download_task lock poisoned");
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
                    let mut task = download_task.lock().expect("download_task lock poisoned");
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

    /// Retry a download task
    pub fn retry_download_task(&self, download_task: Arc<Mutex<DownloadTask>>) {
        {
            let mut task = download_task.lock().expect("download_task lock poisoned");
            task.set_download_task_status(DownloadTaskStatus::Prepare);
        }
        self.execute_download_task(download_task);
    }
}

impl rubato_types::http_download_submitter::HttpDownloadSubmitter for HttpDownloadProcessor {
    fn submit_md5_task(&self, md5: &str, task_name: &str) {
        HttpDownloadProcessor::submit_md5_task(self, md5, task_name);
    }
}

/// Download a file from url (no intermediate file protection)
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
        let t = task.lock().expect("task lock poisoned");
        t.url().to_string()
    };

    let response = reqwest::blocking::get(&url)?;
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

    // Stream body in chunks to avoid buffering entire archive in memory
    let mut fos = fs::File::create(&result)?;
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
            let mut t = task.lock().expect("mutex poisoned");
            t.download_size = download_bytes;
            t.content_length = content_length;
        }
    }
    log::info!(
        "[HttpDownloadProcessor] Download successfully to {}",
        result.display()
    );
    {
        let mut t = task.lock().expect("task lock poisoned");
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

    // Extract into a temp directory first, then validate paths before moving.
    // This prevents path-traversal attacks via ../ entries in the archive.
    let staging_dir = tempfile::tempdir_in(&dest)?;

    sevenz_rust::decompress_file(file, staging_dir.path())
        .map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;

    // Validate all extracted paths stay within the staging directory.
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
