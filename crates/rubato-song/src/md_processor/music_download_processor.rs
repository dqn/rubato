use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::http_download_processor::copy_dir_recursive;
use super::ipfs_information::IpfsInformation;
use super::music_database_accessor::MusicDatabaseAccessor;
use rubato_types::sync_utils::lock_or_recover;

/// Corresponds to MusicDownloadProcessor in Java
/// IPFS-based song download processor
///
/// # Lock ordering
///
/// When acquiring multiple locks, always follow this order to prevent deadlock:
///   1. `daemon`    (outermost)
///   2. `ipfs`
///   3. `commands`
///   4. `message`   (innermost)
///
/// The `DaemonHandle.downloadpath` mutex is only accessed while holding `daemon`.
/// The daemon thread receives cloned Arcs and acquires them independently (no nesting).
pub struct MusicDownloadProcessor {
    commands: Arc<Mutex<VecDeque<Box<dyn IpfsInformation>>>>,
    ipfs: Arc<Mutex<String>>,
    daemon: Arc<Mutex<Option<DaemonHandle>>>,
    message: Arc<Mutex<String>>,
    pub main: Arc<dyn MusicDatabaseAccessor>,
}

struct DaemonHandle {
    alive: Arc<AtomicBool>,
    download: Arc<AtomicBool>,
    downloadpath: Arc<Mutex<Option<String>>>,
    dispose: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl MusicDownloadProcessor {
    pub fn new(ipfs: String, main: Arc<dyn MusicDatabaseAccessor>) -> Self {
        MusicDownloadProcessor {
            commands: Arc::new(Mutex::new(VecDeque::new())),
            ipfs: Arc::new(Mutex::new(ipfs)),
            daemon: Arc::new(Mutex::new(None)),
            message: Arc::new(Mutex::new(String::new())),
            main,
        }
    }

    pub fn start(&self, song: Option<Box<dyn IpfsInformation>>) {
        let mut daemon_guard = lock_or_recover(&self.daemon);
        let need_start = match &*daemon_guard {
            None => true,
            Some(d) => !d.alive.load(Ordering::SeqCst),
        };
        if need_start {
            {
                let mut ipfs = lock_or_recover(&self.ipfs);
                if ipfs.is_empty() {
                    *ipfs = "https://gateway.ipfs.io/".to_string();
                }
                if !ipfs.ends_with('/') {
                    ipfs.push('/');
                }
            }
            let commands = self.commands.clone();
            let ipfs = self.ipfs.clone();
            let message = self.message.clone();
            let main = self.main.clone();
            let alive = Arc::new(AtomicBool::new(true));
            let download = Arc::new(AtomicBool::new(false));
            let downloadpath: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let dispose = Arc::new(AtomicBool::new(false));

            let alive_clone = alive.clone();
            let download_clone = download.clone();
            let downloadpath_clone = downloadpath.clone();
            let dispose_clone = dispose.clone();

            let join_handle = thread::spawn(move || {
                download_daemon_thread_run(DownloadDaemonState {
                    commands,
                    ipfs,
                    message,
                    main,
                    alive: alive_clone,
                    download: download_clone,
                    downloadpath: downloadpath_clone,
                    dispose: dispose_clone,
                });
            });

            *daemon_guard = Some(DaemonHandle {
                alive,
                download,
                downloadpath,
                dispose,
                join_handle: Some(join_handle),
            });
        }
        if let Some(song) = song {
            let mut cmds = lock_or_recover(&self.commands);
            cmds.push_back(song);
        }
    }

    pub fn dispose(&self) {
        let mut daemon_guard = lock_or_recover(&self.daemon);
        if let Some(ref mut d) = *daemon_guard
            && d.alive.load(Ordering::SeqCst)
        {
            d.dispose.store(true, Ordering::SeqCst);
            if let Some(handle) = d.join_handle.take() {
                let _ = handle.join();
            }
        }
    }

    pub fn is_download(&self) -> bool {
        let daemon_guard = lock_or_recover(&self.daemon);
        match &*daemon_guard {
            None => false,
            Some(d) => d.download.load(Ordering::SeqCst),
        }
    }

    pub fn is_alive(&self) -> bool {
        let daemon_guard = lock_or_recover(&self.daemon);
        match &*daemon_guard {
            None => false,
            Some(d) => d.alive.load(Ordering::SeqCst),
        }
    }

    pub fn downloadpath(&self) -> Option<String> {
        let daemon_guard = lock_or_recover(&self.daemon);
        match &*daemon_guard {
            None => None,
            Some(d) => lock_or_recover(&d.downloadpath).clone(),
        }
    }

    pub fn set_downloadpath(&self, downloadpath: String) {
        let daemon_guard = lock_or_recover(&self.daemon);
        if let Some(ref d) = *daemon_guard {
            *lock_or_recover(&d.downloadpath) = Some(downloadpath);
        }
    }

    pub fn message(&self) -> String {
        lock_or_recover(&self.message).clone()
    }

    pub fn set_message(&self, message: String) {
        *lock_or_recover(&self.message) = message;
    }
}

/// Returns `true` if the path contains `..` components or is absolute,
/// indicating a potential path traversal attack.
fn has_path_traversal(s: &str) -> bool {
    let p = Path::new(s);
    if p.is_absolute() {
        return true;
    }
    p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
}

/// Recursively validate that all paths under `root` are within `root` (no symlink escapes).
fn validate_staging_paths(root: &Path) -> Result<(), String> {
    validate_staging_paths_recursive(root, root)
}

fn validate_staging_paths_recursive(root: &Path, dir: &Path) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read dir {:?}: {}", dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry in {:?}: {}", dir, e))?;
        let canonical = entry
            .path()
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize {:?}: {}", entry.path(), e))?;
        if !canonical.starts_with(root) {
            return Err(format!(
                "Path traversal detected: {} escapes {}",
                entry.path().display(),
                root.display()
            ));
        }
        if entry.path().is_dir() {
            validate_staging_paths_recursive(root, &entry.path())?;
        }
    }
    Ok(())
}

fn normalize_ipfs_path(path: &str) -> String {
    // Case-insensitive prefix match: "/ipfs/" is 6 characters, strip to get bare CID.
    // Use str::get() instead of direct slicing to avoid panics on non-ASCII char boundaries.
    match path.get(..6) {
        Some(prefix) if prefix.eq_ignore_ascii_case("/ipfs/") => path[6..].to_string(),
        _ => path.to_string(),
    }
}

/// Shared state for the download daemon thread.
struct DownloadDaemonState {
    pub commands: Arc<Mutex<VecDeque<Box<dyn IpfsInformation>>>>,
    pub ipfs: Arc<Mutex<String>>,
    pub message: Arc<Mutex<String>>,
    pub main: Arc<dyn MusicDatabaseAccessor>,
    pub alive: Arc<AtomicBool>,
    pub download: Arc<AtomicBool>,
    pub downloadpath: Arc<Mutex<Option<String>>>,
    pub dispose: Arc<AtomicBool>,
}

fn download_daemon_thread_run(state: DownloadDaemonState) {
    let commands = state.commands;
    let ipfs = state.ipfs;
    let message = state.message;
    let main = state.main;
    let alive = state.alive;
    let download = state.download;
    let downloadpath = state.downloadpath;
    let dispose = state.dispose;
    let mut download_ipfs_handle: Option<thread::JoinHandle<()>> = None;
    let download_ipfs_alive: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    dispose.store(false, Ordering::SeqCst);

    let mut ipfspath = String::new();
    let mut path = String::new();
    let mut diffpath = String::new();
    #[allow(unused_assignments)]
    let mut orgbms: Option<PathBuf> = None;
    let path_sanitize_re = regex::Regex::new(r#"[(\\/|:*?"<>|)]"#).expect("valid regex");

    let result: Result<(), Box<dyn std::error::Error>> = {
        loop {
            if dispose.load(Ordering::SeqCst) {
                break;
            }

            let is_download = download.load(Ordering::SeqCst);

            // Single lock acquisition: check + pop atomically (avoids TOCTOU race)
            let song = if !is_download {
                lock_or_recover(&commands).pop_front()
            } else {
                None
            };
            if let Some(song) = song {
                ipfspath = song.ipfs();
                diffpath = song.append_ipfs();

                ipfspath = normalize_ipfs_path(&ipfspath);
                path = format!("[{}]{}", song.artist(), song.title());
                // path = "ipfs/" + path.replaceAll("[(\\\\|/|:|\\*|\\?|\"|<|>|\\|)]", "");
                path = format!("ipfs/{}", path_sanitize_re.replace_all(&path, ""));

                if !diffpath.is_empty() {
                    diffpath = normalize_ipfs_path(&diffpath);
                }

                // Reject paths with traversal components to prevent escaping
                // the ipfs/ directory (e.g. "../../sensitive/file").
                if has_path_traversal(&ipfspath) {
                    log::warn!("Rejecting ipfspath with path traversal: {}", ipfspath);
                    ipfspath = String::new();
                }
                if has_path_traversal(&diffpath) {
                    log::warn!("Rejecting diffpath with path traversal: {}", diffpath);
                    diffpath = String::new();
                }

                let orgmd5 = song.org_md5();
                orgbms = None;
                if !orgmd5.is_empty() {
                    let s = main.get_music_paths(&orgmd5);
                    if !s.is_empty() {
                        for bms in &s {
                            let bmspath = PathBuf::from(bms);
                            if bmspath.exists() {
                                orgbms = Some(bmspath.clone());
                                if let Some(parent) = bmspath.parent() {
                                    path = parent.to_string_lossy().to_string();
                                }
                                break;
                            }
                        }
                    }
                }
                if !ipfspath.is_empty() && orgbms.is_none() {
                    let ipfs_url = lock_or_recover(&ipfs).clone();
                    let ipfspath_clone = ipfspath.clone();
                    let path_clone = path.clone();
                    let message_clone = message.clone();
                    let alive_flag = download_ipfs_alive.clone();
                    alive_flag.store(true, Ordering::SeqCst);
                    download_ipfs_handle = Some(thread::spawn(move || {
                        download_ipfs_thread_run(
                            &ipfs_url,
                            &ipfspath_clone,
                            &path_clone,
                            message_clone,
                        );
                        alive_flag.store(false, Ordering::SeqCst);
                    }));
                    download.store(true, Ordering::SeqCst);
                    log::info!("BMS本体取得開始");
                } else if !ipfspath.is_empty() && !diffpath.is_empty() {
                    log::info!("{}は既に存在します（差分取得のみ）", path);
                    download.store(true, Ordering::SeqCst);
                }
            }

            let is_download = download.load(Ordering::SeqCst);
            let ipfs_thread_alive = download_ipfs_alive.load(Ordering::SeqCst);

            if is_download && (download_ipfs_handle.is_none() || !ipfs_thread_alive) {
                if !diffpath.is_empty() {
                    let f = PathBuf::from(format!("ipfs/{}", diffpath));
                    if ipfspath.is_empty() {
                        if f.exists() && f.is_dir() {
                            if let Ok(entries) = fs::read_dir(&f) {
                                for entry in entries.flatten() {
                                    let src = entry.path();
                                    let dest = PathBuf::from(&path).join(entry.file_name());
                                    move_path_with_fallback(&src, &dest);
                                }
                            }
                            let _ = fs::remove_dir(&f);
                        } else if f.exists() {
                            let dest = PathBuf::from(&path).join(format!("{}.bms", diffpath));
                            move_path_with_fallback(&f, &dest);
                        }
                        diffpath = String::new();
                    } else {
                        let ipfs_url = lock_or_recover(&ipfs).clone();
                        let diffpath_clone = diffpath.clone();
                        let diff_dest = format!("ipfs{}{}", std::path::MAIN_SEPARATOR, &diffpath);
                        let message_clone = message.clone();
                        let alive_flag = download_ipfs_alive.clone();
                        alive_flag.store(true, Ordering::SeqCst);
                        download_ipfs_handle = Some(thread::spawn(move || {
                            download_ipfs_thread_run(
                                &ipfs_url,
                                &diffpath_clone,
                                &diff_dest,
                                message_clone,
                            );
                            alive_flag.store(false, Ordering::SeqCst);
                        }));
                        ipfspath = String::new();
                        log::info!("差分取得開始");
                    }
                } else {
                    let dp = {
                        let p = Path::new(&path)
                            .canonicalize()
                            .unwrap_or_else(|_| PathBuf::from(&path));
                        if p.exists() {
                            Some(p.to_string_lossy().to_string())
                        } else {
                            None
                        }
                    };
                    *lock_or_recover(&downloadpath) = dp;
                    download.store(false, Ordering::SeqCst);
                    ipfspath = String::new();
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        Ok(())
    };

    if let Err(e) = result {
        log::error!("{}", e);
    }

    log::info!("IPFS Thread終了");
    // Join the IPFS download thread if it exists, to avoid detaching it
    if let Some(handle) = download_ipfs_handle.take() {
        let _ = handle.join();
    }
    dispose.store(false, Ordering::SeqCst);
    download.store(false, Ordering::SeqCst);
    alive.store(false, Ordering::SeqCst);
}

/// Build the IPFS gateway download URL, encoding the `ipfspath` argument
/// so that metacharacters (`&`, `=`, `#`, etc.) cannot inject extra query
/// parameters or truncate the URL.
fn build_ipfs_download_url(gateway_base: &str, ipfspath: &str) -> String {
    format!(
        "{}api/v0/get?arg={}&archive=true&compress=true",
        gateway_base,
        urlencoding::encode(ipfspath)
    )
}

fn download_ipfs_thread_run(ipfs: &str, ipfspath: &str, path: &str, message: Arc<Mutex<String>>) {
    *lock_or_recover(&message) = format!("downloading:{}", path);

    // Download tar.gz from IPFS gateway
    let url_str = build_ipfs_download_url(ipfs, ipfspath);
    let _ = fs::remove_file("ipfs/bms.tar.gz");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("failed to build HTTP client");
    let download_ok = match client.get(&url_str).send() {
        Ok(mut response) => {
            if !response.status().is_success() {
                log::error!(
                    "IPFS download failed with status {}: {}",
                    response.status(),
                    url_str
                );
                false
            } else {
                let _ = fs::create_dir_all("ipfs");
                match fs::File::create("ipfs/bms.tar.gz") {
                    Ok(mut out) => {
                        let chunk_size = 1024 * 512;
                        let mut total: i64 = 0;
                        let mut buf = vec![0u8; chunk_size];
                        let mut write_ok = true;
                        loop {
                            use std::io::Read;
                            match response.read(&mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    total += n as i64;
                                    *lock_or_recover(&message) =
                                        format!("downloading:{} {}MB", path, total / 1024 / 1024);
                                    if out.write_all(&buf[..n]).is_err() {
                                        log::error!(
                                            "Failed to write download data at offset {}",
                                            total
                                        );
                                        write_ok = false;
                                        break;
                                    }
                                }
                                Err(e) => {
                                    log::error!("Download read error: {}", e);
                                    write_ok = false;
                                    break;
                                }
                            }
                        }
                        if write_ok && out.flush().is_err() {
                            log::error!("Failed to flush output");
                            write_ok = false;
                        }
                        write_ok
                    }
                    Err(_) => false,
                }
            }
        }
        Err(_) => {
            log::info!("URL:{}に接続失敗。", url_str);
            false
        }
    };

    if download_ok {
        // Extract tar.gz via staging directory pattern: extract to a temporary
        // staging directory first, validate contents, then move to final destination.
        let gz_path = std::path::Path::new("ipfs/bms.tar.gz");
        let mut extraction_ok = true;
        if gz_path.exists() {
            let gz_file = match fs::File::open(gz_path) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to open tar.gz: {}", e);
                    let _ = fs::remove_file(gz_path);
                    return;
                }
            };
            let dest = std::path::Path::new("ipfs");
            let _ = fs::create_dir_all(dest);

            // Create a staging directory within the destination to avoid
            // cross-filesystem move issues and ensure atomic placement.
            let staging_dir = match tempfile::tempdir_in(dest) {
                Ok(d) => d,
                Err(e) => {
                    log::error!("Failed to create staging directory: {}", e);
                    let _ = fs::remove_file(gz_path);
                    return;
                }
            };
            let canonical_staging = staging_dir
                .path()
                .canonicalize()
                .unwrap_or_else(|_| staging_dir.path().to_path_buf());

            let decoder = flate2::read::GzDecoder::new(gz_file);
            let mut archive = tar::Archive::new(decoder);
            match archive.entries() {
                Ok(entries) => {
                    for entry_result in entries {
                        match entry_result {
                            Ok(mut entry) => {
                                let entry_path = match entry.path() {
                                    Ok(p) => p.into_owned(),
                                    Err(e) => {
                                        log::warn!("Skipping tar entry with invalid path: {}", e);
                                        continue;
                                    }
                                };
                                // Reject symlink entries to prevent symlink-based
                                // directory escape attacks.
                                let entry_type = entry.header().entry_type();
                                if entry_type.is_symlink() || entry_type.is_hard_link() {
                                    log::warn!(
                                        "Skipping tar symlink/hardlink entry: {:?}",
                                        entry_path
                                    );
                                    continue;
                                }
                                // Reject entries with path traversal components
                                let has_traversal = entry_path
                                    .components()
                                    .any(|c| matches!(c, std::path::Component::ParentDir))
                                    || entry_path.is_absolute();
                                if has_traversal {
                                    log::warn!(
                                        "Skipping tar entry with unsafe path: {:?}",
                                        entry_path
                                    );
                                    continue;
                                }
                                let full_path = canonical_staging.join(&entry_path);
                                if !full_path.starts_with(&canonical_staging) {
                                    log::warn!(
                                        "Skipping tar entry escaping destination: {:?}",
                                        entry_path
                                    );
                                    continue;
                                }
                                if let Err(e) = entry.unpack_in(&canonical_staging) {
                                    log::warn!(
                                        "Failed to extract tar entry {:?}: {}",
                                        entry_path,
                                        e
                                    );
                                    extraction_ok = false;
                                }
                            }
                            Err(e) => {
                                log::warn!("Skipping malformed tar entry: {}", e);
                                extraction_ok = false;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to read tar.gz entries: {}", e);
                    extraction_ok = false;
                }
            }
            let _ = fs::remove_file(gz_path);
            if !extraction_ok {
                log::error!("Extraction failed; skipping move phase");
                // staging_dir is dropped here, cleaning up partial extraction
                return;
            }

            // Defense-in-depth: validate all extracted paths stay within the
            // staging directory (catches symlink escapes that entry-name checks
            // cannot detect).
            if let Err(e) = validate_staging_paths(&canonical_staging) {
                log::error!(
                    "Post-extraction path validation failed: {}; aborting",
                    e
                );
                return;
            }

            // Move validated contents from staging into final destination.
            let canonical_dest = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
            if let Ok(entries) = fs::read_dir(staging_dir.path()) {
                for entry in entries.flatten() {
                    let target = canonical_dest.join(entry.file_name());
                    if !move_path_with_fallback(&entry.path(), &target) {
                        log::error!(
                            "Failed to move {:?} from staging to {:?}",
                            entry.file_name(),
                            target
                        );
                    }
                }
            }
            // staging_dir is dropped here, cleaning up the now-empty temp directory
        }
    }

    // File move logic (post-extraction)
    let dir = PathBuf::from(format!("ipfs{}{}", std::path::MAIN_SEPARATOR, ipfspath));
    if !ipfspath.is_empty() && dir.to_string_lossy() != path && dir.exists() {
        if dir.is_dir() {
            let dest_path = PathBuf::from(path);
            if !dest_path.exists() {
                let _ = fs::create_dir_all(&dest_path);
            }
            let mut all_moved = false;
            if let Ok(entries) = fs::read_dir(&dir) {
                all_moved = true;
                for entry in entries.flatten() {
                    let src = entry.path();
                    let dest =
                        PathBuf::from(format!("{}/{}", path, entry.file_name().to_string_lossy()));
                    if !move_path_with_fallback(&src, &dest) {
                        all_moved = false;
                    }
                }
            } else {
                log::error!("Failed to read directory {:?}; skipping cleanup", dir);
            }
            if all_moved {
                let _ = fs::remove_dir_all(&dir).or_else(|_| fs::remove_file(&dir));
            } else {
                log::error!("Skipping cleanup of {:?} due to move failures", dir);
            }
        } else if !PathBuf::from(path).exists() {
            if !move_path_with_fallback(&dir, &PathBuf::from(path)) {
                log::error!("Failed to move {:?} to {}", dir, path);
            }
        } else {
            let _ = fs::remove_dir_all(&dir).or_else(|_| fs::remove_file(&dir));
        }
    }
}

/// Move a file or directory from `src` to `dest`, falling back to copy-and-remove
/// when `fs::rename()` fails (e.g., EXDEV on cross-filesystem moves).
/// Returns `true` on success, `false` on failure.
fn move_path_with_fallback(src: &Path, dest: &Path) -> bool {
    if fs::rename(src, dest).is_ok() {
        return true;
    }
    // rename failed (possibly cross-filesystem); fall back to copy + remove
    if src.is_dir() {
        if let Err(e) = copy_dir_recursive(src, dest) {
            log::error!("Failed to copy dir {:?} to {:?}: {}", src, dest, e);
            return false;
        }
        if let Err(e) = fs::remove_dir_all(src) {
            log::error!("Failed to remove source dir {:?} after copy: {}", src, e);
        }
    } else if src.is_file() {
        if let Err(e) = fs::copy(src, dest) {
            log::error!("Failed to copy file {:?} to {:?}: {}", src, dest, e);
            return false;
        }
        if let Err(e) = fs::remove_file(src) {
            log::error!("Failed to remove source file {:?} after copy: {}", src, e);
        }
    } else {
        // src doesn't exist or is a special file type
        return false;
    }
    true
}

impl rubato_types::music_download_access::MusicDownloadAccess for MusicDownloadProcessor {
    fn start_download(&self, song: &rubato_types::song_data::SongData) {
        // SongData implements IpfsInformation, so we can clone and box it.
        let song_clone = song.clone();
        self.start(Some(Box::new(song_clone)));
    }

    fn dispose(&self) {
        MusicDownloadProcessor::dispose(self);
    }

    fn is_alive(&self) -> bool {
        MusicDownloadProcessor::is_alive(self)
    }

    fn is_download(&self) -> bool {
        MusicDownloadProcessor::is_download(self)
    }

    fn message(&self) -> String {
        MusicDownloadProcessor::message(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ipfs_path_strips_ipfs_prefix() {
        assert_eq!(normalize_ipfs_path("/ipfs/QmMain"), "QmMain");
        assert_eq!(normalize_ipfs_path("/IPFS/QmUpper"), "QmUpper");
    }

    #[test]
    fn normalize_ipfs_path_keeps_non_prefixed_paths() {
        assert_eq!(normalize_ipfs_path("QmMain"), "QmMain");
        assert_eq!(normalize_ipfs_path(""), String::new());
    }

    #[test]
    fn normalize_ipfs_path_bare_prefix_only() {
        assert_eq!(normalize_ipfs_path("/ipfs/"), "");
    }

    #[test]
    fn move_path_with_fallback_moves_file() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("source.bms");
        let dest = tmp.path().join("dest.bms");
        fs::write(&src, "hello").unwrap();

        move_path_with_fallback(&src, &dest);

        assert!(!src.exists());
        assert_eq!(fs::read_to_string(&dest).unwrap(), "hello");
    }

    #[test]
    fn move_path_with_fallback_moves_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src_dir");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("a.txt"), "aaa").unwrap();
        fs::create_dir_all(src_dir.join("sub")).unwrap();
        fs::write(src_dir.join("sub/b.txt"), "bbb").unwrap();

        let dest_dir = tmp.path().join("dest_dir");

        move_path_with_fallback(&src_dir, &dest_dir);

        assert!(!src_dir.exists());
        assert!(dest_dir.exists());
        assert_eq!(fs::read_to_string(dest_dir.join("a.txt")).unwrap(), "aaa");
        assert_eq!(
            fs::read_to_string(dest_dir.join("sub/b.txt")).unwrap(),
            "bbb"
        );
    }

    #[test]
    fn move_path_with_fallback_returns_false_on_missing_src() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("nonexistent");
        let dest = tmp.path().join("dest");

        let result = move_path_with_fallback(&src, &dest);

        assert!(!result);
        assert!(!dest.exists());
    }

    #[test]
    fn has_path_traversal_detects_parent_dir() {
        assert!(has_path_traversal("../../etc/passwd"));
        assert!(has_path_traversal("foo/../../../bar"));
        assert!(has_path_traversal("../secret"));
    }

    #[test]
    fn has_path_traversal_detects_absolute_paths() {
        assert!(has_path_traversal("/etc/passwd"));
        assert!(has_path_traversal("/tmp/evil"));
    }

    #[test]
    fn has_path_traversal_allows_safe_paths() {
        assert!(!has_path_traversal("QmSomeCID"));
        assert!(!has_path_traversal("subdir/file.bms"));
        assert!(!has_path_traversal(""));
        assert!(!has_path_traversal("normal_cid_hash"));
    }

    #[test]
    fn build_ipfs_download_url_encodes_plain_cid() {
        let url = build_ipfs_download_url("https://gateway.ipfs.io/", "QmAbc123");
        assert_eq!(
            url,
            "https://gateway.ipfs.io/api/v0/get?arg=QmAbc123&archive=true&compress=true"
        );
    }

    #[test]
    fn build_ipfs_download_url_encodes_metacharacters() {
        // An IPFS path containing URL metacharacters must be percent-encoded
        // so they do not inject extra query parameters or truncate the URL.
        let url = build_ipfs_download_url("https://gateway.ipfs.io/", "Qm&evil=1#frag");
        assert!(
            !url.contains("&evil=1"),
            "raw '&' in ipfspath must be encoded"
        );
        assert!(
            !url.contains("#frag"),
            "raw '#' in ipfspath must be encoded"
        );
        assert_eq!(
            url,
            "https://gateway.ipfs.io/api/v0/get?arg=Qm%26evil%3D1%23frag&archive=true&compress=true"
        );
    }

    #[test]
    fn build_ipfs_download_url_encodes_spaces_and_unicode() {
        let url = build_ipfs_download_url("https://gw.example.com/", "Qm path/to file");
        assert!(
            !url.contains(' '),
            "spaces in ipfspath must be percent-encoded"
        );
        assert!(url.contains("Qm%20path%2Fto%20file"));
    }
}
