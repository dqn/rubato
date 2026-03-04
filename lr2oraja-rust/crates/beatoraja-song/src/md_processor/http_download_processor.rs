use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
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
        map.insert(wriggle_meta.get_name().to_string(), wriggle_meta);
        // Konmai
        let konmai_meta: &HttpDownloadSourceMeta = &konmai_download_source::META;
        map.insert(konmai_meta.get_name().to_string(), konmai_meta);
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
    submitted_urls: Mutex<HashSet<String>>,
    // In-memory self-add id generator
    id_generator: AtomicI32,
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
            submitted_urls: Mutex::new(HashSet::new()),
            id_generator: AtomicI32::new(0),
            main,
            http_download_source,
        }
    }

    pub fn get_default_download_source() -> &'static HttpDownloadSourceMeta {
        &wriggle_download_source::META
    }

    #[allow(dead_code)]
    fn get_task_by_id(&self, task_id: i32) -> Option<Arc<Mutex<DownloadTask>>> {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(&task_id).cloned()
    }

    // Would be best if this returned an immutable view over the tasks,
    // without creating a copy, in the interest of efficiency,
    // however I'm not sure if that is possible in java
    pub fn get_all_tasks(&self) -> Arc<Mutex<HashMap<i32, Arc<Mutex<DownloadTask>>>>> {
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
        let source_name = self.http_download_source.get_name().to_string();
        let download_url = match self.http_download_source.get_download_url_based_on_md5(md5) {
            Ok(url) => url,
            Err(e) => {
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
            let mut urls = self.submitted_urls.lock().unwrap();
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
            let mut tasks = self.tasks.lock().unwrap();
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
        let download_directory = self.download_directory.clone();
        let main = self.main.clone();
        let source_name = self.http_download_source.get_name().to_string();

        // Java uses ExecutorService.submit() with a fixed thread pool.
        // Here we simply spawn a thread per task. A bounded thread pool could be added later.
        thread::spawn(move || {
            let (task_name, download_url, hash) = {
                let task = download_task.lock().unwrap();
                (
                    task.get_name().to_string(),
                    task.get_url().to_string(),
                    task.get_hash().to_string(),
                )
            };
            log::info!(
                "[HttpDownloadProcessor] Trying to kick new download task[{}]({})",
                task_name,
                download_url
            );
            {
                let mut task = download_task.lock().unwrap();
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
                let mut task = download_task.lock().unwrap();
                task.set_download_task_status(DownloadTaskStatus::Error);
                return;
            }
            let result = result.unwrap();
            // 2) Extract the compressed archive & update download directory automatically
            let mut successfully_extracted = false;
            let mut bms_directory: Option<String> = None;
            match extract_compressed_file(&result, None, &download_directory) {
                Ok(dir) => {
                    bms_directory = dir;
                    successfully_extracted = true;
                    let mut task = download_task.lock().unwrap();
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
                // TODO: Directory update is protected, this might cause some uncovered situation. Personally speaking,
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
            let mut task = download_task.lock().unwrap();
            task.set_download_task_status(DownloadTaskStatus::Prepare);
        }
        self.execute_download_task(download_task);
    }
}

impl beatoraja_types::http_download_submitter::HttpDownloadSubmitter for HttpDownloadProcessor {
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
        let t = task.lock().unwrap();
        t.get_url().to_string()
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
        let re = Regex::new(r#"filename="?([^"]+)"?"#).unwrap();
        if let Some(caps) = re.captures(&content_disposition)
            && let Some(m) = caps.get(1)
        {
            candidate_file_name = m.as_str().to_string();
        }
    }
    if !candidate_file_name.is_empty() {
        file_name = candidate_file_name;
    }

    let content_length = response.content_length().map(|l| l as i64).unwrap_or(-1);

    let result = Path::new(download_directory).join(&file_name);

    // Read body in chunks
    let bytes = response.bytes()?;
    let total = bytes.len() as i64;

    // Write to file
    let mut fos = fs::File::create(&result)?;
    // TODO: We can bind the buffer to the worker thread instead of creating & releasing it repeatedly
    let chunk_size = 8192;
    let mut download_bytes: i64 = 0;
    let data = bytes.as_ref();
    let mut offset = 0;
    while offset < data.len() {
        let end = std::cmp::min(offset + chunk_size, data.len());
        let read = end - offset;
        fos.write_all(&data[offset..end])?;
        download_bytes += read as i64;
        offset = end;
        {
            let mut t = task.lock().unwrap();
            t.set_download_size(download_bytes);
            t.set_content_length(content_length);
        }
    }
    let _ = total;
    log::info!(
        "[HttpDownloadProcessor] Download successfully to {}",
        result.display()
    );
    {
        let mut t = task.lock().unwrap();
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

    sevenz_rust::decompress_file(file, &dest)
        .map_err(|e| anyhow::anyhow!("7z extraction failed: {}", e))?;

    // Find the extracted directory (first subdirectory in dest)
    let mut extracted_dir = None;
    if let Ok(entries) = fs::read_dir(&dest) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                extracted_dir = Some(entry.path().to_string_lossy().to_string());
                break;
            }
        }
    }

    Ok(extracted_dir)
}
