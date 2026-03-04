use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::ipfs_information::IpfsInformation;
use super::music_database_accessor::MusicDatabaseAccessor;

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
        let mut daemon_guard = self.daemon.lock().unwrap();
        let need_start = match &*daemon_guard {
            None => true,
            Some(d) => !d.alive.load(Ordering::SeqCst),
        };
        if need_start {
            {
                let mut ipfs = self.ipfs.lock().unwrap();
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
                download_daemon_thread_run(
                    commands,
                    ipfs,
                    message,
                    main,
                    alive_clone,
                    download_clone,
                    downloadpath_clone,
                    dispose_clone,
                );
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
            let mut cmds = self.commands.lock().unwrap();
            cmds.push_back(song);
        }
    }

    pub fn dispose(&self) {
        let mut daemon_guard = self.daemon.lock().unwrap();
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
        let daemon_guard = self.daemon.lock().unwrap();
        match &*daemon_guard {
            None => false,
            Some(d) => d.download.load(Ordering::SeqCst),
        }
    }

    pub fn is_alive(&self) -> bool {
        let daemon_guard = self.daemon.lock().unwrap();
        match &*daemon_guard {
            None => false,
            Some(d) => d.alive.load(Ordering::SeqCst),
        }
    }

    pub fn get_downloadpath(&self) -> Option<String> {
        let daemon_guard = self.daemon.lock().unwrap();
        match &*daemon_guard {
            None => None,
            Some(d) => d.downloadpath.lock().unwrap().clone(),
        }
    }

    pub fn set_downloadpath(&self, downloadpath: String) {
        let daemon_guard = self.daemon.lock().unwrap();
        if let Some(ref d) = *daemon_guard {
            *d.downloadpath.lock().unwrap() = Some(downloadpath);
        }
    }

    pub fn get_message(&self) -> String {
        self.message.lock().unwrap().clone()
    }

    pub fn set_message(&self, message: String) {
        *self.message.lock().unwrap() = message;
    }
}

#[allow(clippy::too_many_arguments)]
fn download_daemon_thread_run(
    commands: Arc<Mutex<VecDeque<Box<dyn IpfsInformation>>>>,
    ipfs: Arc<Mutex<String>>,
    message: Arc<Mutex<String>>,
    main: Arc<dyn MusicDatabaseAccessor>,
    alive: Arc<AtomicBool>,
    download: Arc<AtomicBool>,
    downloadpath: Arc<Mutex<Option<String>>>,
    dispose: Arc<AtomicBool>,
) {
    let mut download_ipfs_handle: Option<thread::JoinHandle<()>> = None;
    let download_ipfs_alive: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    dispose.store(false, Ordering::SeqCst);

    let mut ipfspath = String::new();
    let mut path = String::new();
    let mut diffpath = String::new();
    #[allow(unused_assignments)]
    let mut orgbms: Option<PathBuf> = None;
    let path_sanitize_re = regex::Regex::new(r#"[(\\/|:*?"<>|)]"#).unwrap();

    let result: Result<(), Box<dyn std::error::Error>> = {
        loop {
            if dispose.load(Ordering::SeqCst) {
                break;
            }

            let is_download = download.load(Ordering::SeqCst);

            // Single lock acquisition: check + pop atomically (avoids TOCTOU race)
            let song = if !is_download {
                commands.lock().unwrap().pop_front()
            } else {
                None
            };
            if let Some(song) = song {
                ipfspath = song.get_ipfs();
                diffpath = song.get_append_ipfs();

                if ipfspath.to_lowercase().starts_with("/ipfs/") {
                    // Java: ipfspath = path.substring(5);
                    // NOTE: This is a bug in the Java code - it uses `path` instead of `ipfspath`
                    // Translating as-is
                    ipfspath = path[5..].to_string();
                }
                path = format!("[{}]{}", song.get_artist(), song.get_title());
                // path = "ipfs/" + path.replaceAll("[(\\\\|/|:|\\*|\\?|\"|<|>|\\|)]", "");
                path = format!("ipfs/{}", path_sanitize_re.replace_all(&path, ""));

                if !diffpath.is_empty() && diffpath.to_lowercase().starts_with("/ipfs/") {
                    diffpath = diffpath[5..].to_string();
                }

                let orgmd5 = song.get_org_md5();
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
                    let ipfs_url = ipfs.lock().unwrap().clone();
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
                                    let _ = fs::rename(&src, &dest);
                                }
                            }
                            let _ = fs::remove_dir(&f);
                        } else if f.exists() {
                            let dest = PathBuf::from(&path).join(format!("{}.bms", diffpath));
                            let _ = fs::rename(&f, &dest);
                        }
                        diffpath = String::new();
                    } else {
                        let ipfs_url = ipfs.lock().unwrap().clone();
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
                    *downloadpath.lock().unwrap() = dp;
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
    // If download ipfs thread is still alive, we can't force-kill it in Rust
    // but we can drop the handle
    drop(download_ipfs_handle);
    dispose.store(false, Ordering::SeqCst);
    download.store(false, Ordering::SeqCst);
    alive.store(false, Ordering::SeqCst);
}

fn download_ipfs_thread_run(ipfs: &str, ipfspath: &str, path: &str, message: Arc<Mutex<String>>) {
    *message.lock().unwrap() = format!("downloading:{}", path);

    // Download tar.gz from IPFS gateway
    let url_str = format!(
        "{}api/v0/get?arg={}&archive=true&compress=true",
        ipfs, ipfspath
    );
    let _ = fs::remove_file("ipfs/bms.tar.gz");

    let download_ok = match reqwest::blocking::get(&url_str) {
        Ok(response) => match response.bytes() {
            Ok(bytes) => {
                let _ = fs::create_dir_all("ipfs");
                match fs::File::create("ipfs/bms.tar.gz") {
                    Ok(mut out) => {
                        let data = bytes.as_ref();
                        let chunk_size = 1024 * 512;
                        let mut total: i64 = 0;
                        let mut offset = 0;
                        while offset < data.len() {
                            let end = std::cmp::min(offset + chunk_size, data.len());
                            let count = end - offset;
                            total += count as i64;
                            *message.lock().unwrap() =
                                format!("downloading:{} {}MB", path, total / 1024 / 1024);
                            if out.write_all(&data[offset..end]).is_err() {
                                break;
                            }
                            offset = end;
                        }
                        if out.flush().is_err() {
                            log::error!("Failed to flush output");
                        }
                        true
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        },
        Err(_) => {
            log::info!("URL:{}に接続失敗。", url_str);
            false
        }
    };

    if download_ok {
        // Extract tar.gz
        let gz_path = std::path::Path::new("ipfs/bms.tar.gz");
        if gz_path.exists() {
            let gz_file = match fs::File::open(gz_path) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to open tar.gz: {}", e);
                    return;
                }
            };
            let decoder = flate2::read::GzDecoder::new(gz_file);
            let mut archive = tar::Archive::new(decoder);
            if let Err(e) = archive.unpack("ipfs") {
                log::error!("Failed to extract tar.gz: {}", e);
            }
            let _ = fs::remove_file(gz_path);
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
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let src = entry.path();
                    let dest =
                        PathBuf::from(format!("{}/{}", path, entry.file_name().to_string_lossy()));
                    let _ = fs::rename(&src, &dest);
                }
            }
        } else if !PathBuf::from(path).exists() {
            let _ = fs::rename(&dir, PathBuf::from(path));
        }
        let _ = fs::remove_file(&dir);
    }
}

impl beatoraja_types::music_download_access::MusicDownloadAccess for MusicDownloadProcessor {
    fn start_download(&self, song: &beatoraja_types::song_data::SongData) {
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

    fn get_message(&self) -> String {
        MusicDownloadProcessor::get_message(self)
    }
}
