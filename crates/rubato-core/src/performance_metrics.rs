use rubato_types::sync_utils::lock_or_recover;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static INSTANCE: OnceLock<PerformanceMetrics> = OnceLock::new();

thread_local! {
    /// Per-thread active block stack for correct parent-child event relationships.
    static THREAD_ACTIVE_BLOCKS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

/// PerformanceMetrics - tracks performance events and watch measurements
pub struct PerformanceMetrics {
    /// Event results (thread-safe list)
    pub event_results: Mutex<Vec<EventResult>>,
    /// Watch records keyed by name
    watch_records: Mutex<HashMap<String, VecDeque<(i64, i64)>>>,
    /// Base instant for timing
    base_instant: Instant,
}

/// EventResult - records a single performance event
#[derive(Clone, Debug)]
pub struct EventResult {
    pub name: String,
    pub id: u64,
    pub parent: u64,
    pub start_time: i64,
    pub duration: i64,
    pub thread: String,
}

static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            event_results: Mutex::new(Vec::new()),
            watch_records: Mutex::new(HashMap::new()),
            base_instant: Instant::now(),
        }
    }

    pub fn get() -> &'static PerformanceMetrics {
        INSTANCE.get_or_init(PerformanceMetrics::new)
    }

    fn nanos(&self) -> i64 {
        self.base_instant.elapsed().as_nanos() as i64
    }

    /// Create a new EventBlock for scoped performance measurement
    pub fn event(&self, event_name: &str) -> EventBlock {
        let id = NEXT_EVENT_ID.fetch_add(1, Ordering::Relaxed);
        let parent = THREAD_ACTIVE_BLOCKS.with(|blocks| {
            let mut b = blocks.borrow_mut();
            let p = b.last().copied().unwrap_or(0);
            b.push(id);
            p
        });
        EventBlock {
            name: event_name.to_string(),
            id,
            parent,
            start_time: self.nanos(),
        }
    }

    /// Create a new WatchBlock for scoped performance measurement
    pub fn watch(&self, event_name: &str) -> WatchBlock {
        WatchBlock {
            name: event_name.to_string(),
            start_time: self.nanos(),
        }
    }

    pub fn submit_watch_result(&self, name: &str, time: i64, duration: i64) {
        let mut records = lock_or_recover(&self.watch_records);
        let deque = records.entry(name.to_string()).or_default();
        deque.push_back((time, duration));
    }

    /// Drop measurements older than 3 seconds
    pub fn commit(&self) {
        let now = self.nanos();
        let keep = now - 3_000_000_000;
        let mut records = lock_or_recover(&self.watch_records);
        for (_k, v) in records.iter_mut() {
            while let Some(&(time, _)) = v.front() {
                if time < keep {
                    v.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    pub fn watch_names(&self) -> Vec<String> {
        let records = lock_or_recover(&self.watch_records);
        records.keys().cloned().collect()
    }

    pub fn get_watch_records(&self, name: &str) -> Option<VecDeque<(i64, i64)>> {
        let records = lock_or_recover(&self.watch_records);
        records.get(name).cloned()
    }
}

/// EventBlock - RAII block for measuring event duration
pub struct EventBlock {
    name: String,
    id: u64,
    parent: u64,
    start_time: i64,
}

impl Drop for EventBlock {
    fn drop(&mut self) {
        let metrics = PerformanceMetrics::get();
        let end_time = metrics.nanos();
        THREAD_ACTIVE_BLOCKS.with(|blocks| {
            blocks.borrow_mut().pop();
        });
        let result = EventResult {
            name: self.name.clone(),
            id: self.id,
            parent: self.parent,
            start_time: self.start_time,
            duration: end_time - self.start_time,
            thread: std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string(),
        };
        // Use try_lock() instead of lock_or_recover() to avoid deadlock risk
        // during panic unwinding when the same Mutex may already be held.
        if let Ok(mut results) = metrics.event_results.try_lock() {
            results.push(result);
        }
    }
}

/// WatchBlock - RAII block for watch-style measurement
pub struct WatchBlock {
    name: String,
    start_time: i64,
}

impl Drop for WatchBlock {
    fn drop(&mut self) {
        let metrics = PerformanceMetrics::get();
        let end_time = metrics.nanos();
        metrics.submit_watch_result(&self.name, self.start_time, end_time - self.start_time);
    }
}
