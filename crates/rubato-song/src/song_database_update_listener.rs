use std::sync::atomic::{AtomicI32, Ordering};

/// Listen current songdata.db's update progress
pub struct SongDatabaseUpdateListener {
    bms_files: AtomicI32,
    processed_bms_files: AtomicI32,
    new_bms_files: AtomicI32,
}

impl Default for SongDatabaseUpdateListener {
    fn default() -> Self {
        Self::new()
    }
}

impl SongDatabaseUpdateListener {
    pub fn new() -> Self {
        Self {
            bms_files: AtomicI32::new(0),
            processed_bms_files: AtomicI32::new(0),
            new_bms_files: AtomicI32::new(0),
        }
    }

    pub fn add_bms_files_count(&self, count: i32) {
        self.bms_files.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_processed_bms_files_count(&self, count: i32) {
        self.processed_bms_files.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_new_bms_files_count(&self, count: i32) {
        self.new_bms_files.fetch_add(count, Ordering::Relaxed);
    }

    pub fn get_bms_files_count(&self) -> i32 {
        self.bms_files.load(Ordering::Relaxed)
    }

    pub fn get_processed_bms_files_count(&self) -> i32 {
        self.processed_bms_files.load(Ordering::Relaxed)
    }

    pub fn get_new_bms_files_count(&self) -> i32 {
        self.new_bms_files.load(Ordering::Relaxed)
    }
}
