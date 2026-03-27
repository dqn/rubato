use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::download_task::{DownloadTask, DownloadTaskStatus};
use super::http_download_processor::HttpDownloadProcessor;

use rubato_types::sync_utils::lock_or_recover;
use std::sync::OnceLock;

/// Global state for running and expired download tasks.
/// Corresponds to DownloadTaskState.java
///
/// Note: Because DownloadTask does not impl Clone and is held behind Arc<Mutex<>>,
/// we store references (Arc) rather than owned copies.
static STATE: OnceLock<Mutex<DownloadTaskStateInner>> = OnceLock::new();

struct DownloadTaskStateInner {
    pub running_download_tasks: HashMap<i32, Arc<Mutex<DownloadTask>>>,
    pub expired_tasks: HashMap<i32, Arc<Mutex<DownloadTask>>>,
    pub last_snapshot: Instant,
}

pub struct DownloadTaskState;

impl DownloadTaskState {
    fn get_inner() -> &'static Mutex<DownloadTaskStateInner> {
        STATE.get_or_init(|| {
            Mutex::new(DownloadTaskStateInner {
                running_download_tasks: HashMap::new(),
                expired_tasks: HashMap::new(),
                last_snapshot: Instant::now(),
            })
        })
    }

    pub fn get_running_download_tasks() -> HashMap<i32, Arc<Mutex<DownloadTask>>> {
        let inner = lock_or_recover(Self::get_inner());
        inner.running_download_tasks.clone()
    }

    pub fn get_expired_tasks() -> HashMap<i32, Arc<Mutex<DownloadTask>>> {
        let inner = lock_or_recover(Self::get_inner());
        inner.expired_tasks.clone()
    }

    pub fn initialize() {
        let _ = Self::get_inner();
    }

    pub fn update(processor: &HttpDownloadProcessor) {
        let mut inner = lock_or_recover(Self::get_inner());
        let now = Instant::now();
        // no reason to check very often (1s)
        if now.duration_since(inner.last_snapshot).as_nanos() < 1_000_000_000 {
            return;
        }
        inner.last_snapshot = now;

        let tasks_arc = processor.all_tasks();
        let tasks = lock_or_recover(&tasks_arc);
        if tasks.len() == inner.expired_tasks.len() {
            return;
        }

        for (id, task_arc) in tasks.iter() {
            let id = *id;
            if inner.expired_tasks.contains_key(&id) {
                continue;
            }

            let task = lock_or_recover(task_arc);
            let finished =
                task.download_task_status().value() >= DownloadTaskStatus::Extracted.value();
            let now_nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as i64;
            let expired = finished && (now_nanos - task.time_finished() > 5_000_000_000i64);

            if expired {
                inner.running_download_tasks.remove(&id);
                inner.expired_tasks.insert(id, task_arc.clone());
            } else {
                inner.running_download_tasks.insert(id, task_arc.clone());
            }
        }
    }
}
