use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::download_task::{DownloadTask, DownloadTaskStatus};
use super::http_download_processor::HttpDownloadProcessor;

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
        let inner = Self::get_inner().lock().unwrap();
        inner.running_download_tasks.clone()
    }

    pub fn get_expired_tasks() -> HashMap<i32, Arc<Mutex<DownloadTask>>> {
        let inner = Self::get_inner().lock().unwrap();
        inner.expired_tasks.clone()
    }

    pub fn initialize() {
        let _ = Self::get_inner();
    }

    pub fn update(processor: &HttpDownloadProcessor) {
        let mut inner = Self::get_inner().lock().unwrap();
        let now = Instant::now();
        // no reason to check very often (1s)
        if now.duration_since(inner.last_snapshot).as_nanos() < 1_000_000_000 {
            return;
        }
        inner.last_snapshot = now;

        let tasks_arc = processor.get_all_tasks();
        let tasks = tasks_arc.lock().unwrap();
        if tasks.len() == inner.expired_tasks.len() {
            return;
        }

        for (id, task_arc) in tasks.iter() {
            let id = *id;
            if inner.expired_tasks.contains_key(&id) {
                continue;
            }

            let task = task_arc.lock().unwrap();
            let _state = task.get_download_task_status();
            let finished =
                task.get_download_task_status().value() >= DownloadTaskStatus::Extracted.value();
            let elapsed_nanos = now.elapsed().as_nanos() as i64;
            let expired = finished && (5_000_000_000i64 < elapsed_nanos - task.get_time_finished());

            if expired {
                inner.running_download_tasks.remove(&id);
                inner.expired_tasks.insert(id, task_arc.clone());
            } else {
                inner.running_download_tasks.insert(id, task_arc.clone());
            }
        }
    }
}
