use std::sync::atomic::{AtomicI64, Ordering};

/// Corresponds to DownloadTask.DownloadTaskStatus in Java
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadTaskStatus {
    Prepare,
    Downloading,
    Downloaded,
    Extracted,
    Error,
    Cancel,
}

impl DownloadTaskStatus {
    pub fn value(&self) -> i32 {
        match self {
            DownloadTaskStatus::Prepare => 0,
            DownloadTaskStatus::Downloading => 1,
            DownloadTaskStatus::Downloaded => 2,
            DownloadTaskStatus::Extracted => 3,
            DownloadTaskStatus::Error => 4,
            DownloadTaskStatus::Cancel => 5,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            DownloadTaskStatus::Prepare => "Prepare",
            DownloadTaskStatus::Downloading => "Downloading",
            DownloadTaskStatus::Downloaded => "Downloaded",
            DownloadTaskStatus::Extracted => "Finished",
            DownloadTaskStatus::Error => "Error",
            DownloadTaskStatus::Cancel => "Cancel",
        }
    }
}

/// Corresponds to DownloadTask in Java
pub struct DownloadTask {
    id: i32,
    url: String,
    name: String,
    hash: String,
    download_task_status: DownloadTaskStatus,
    pub download_size: i64,
    pub content_length: i64,
    error_message: Option<String>,
    time_finished: AtomicI64,
}

impl DownloadTask {
    pub fn new(id: i32, url: String, name: String, hash: String) -> Self {
        DownloadTask {
            id,
            url,
            name,
            hash,
            download_task_status: DownloadTaskStatus::Prepare,
            download_size: 0,
            content_length: 0,
            error_message: None,
            time_finished: AtomicI64::new(0),
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn download_task_status(&self) -> DownloadTaskStatus {
        self.download_task_status
    }

    pub fn set_download_task_status(&mut self, status: DownloadTaskStatus) {
        if status.value() >= DownloadTaskStatus::Extracted.value() {
            // Java: System.nanoTime()
            // Use std::time::Instant elapsed as nanos approximation
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as i64;
            self.time_finished.store(now, Ordering::Release);
        }
        self.download_task_status = status;
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn set_error_message(&mut self, error_message: String) {
        self.error_message = Some(error_message);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn time_finished(&self) -> i64 {
        self.time_finished.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_value_mapping() {
        assert_eq!(DownloadTaskStatus::Prepare.value(), 0);
        assert_eq!(DownloadTaskStatus::Downloading.value(), 1);
        assert_eq!(DownloadTaskStatus::Downloaded.value(), 2);
        assert_eq!(DownloadTaskStatus::Extracted.value(), 3);
        assert_eq!(DownloadTaskStatus::Error.value(), 4);
        assert_eq!(DownloadTaskStatus::Cancel.value(), 5);
    }

    #[test]
    fn status_name_mapping() {
        assert_eq!(DownloadTaskStatus::Prepare.name(), "Prepare");
        assert_eq!(DownloadTaskStatus::Downloading.name(), "Downloading");
        assert_eq!(DownloadTaskStatus::Downloaded.name(), "Downloaded");
        // Extracted maps to "Finished" (matching Java behavior)
        assert_eq!(DownloadTaskStatus::Extracted.name(), "Finished");
        assert_eq!(DownloadTaskStatus::Error.name(), "Error");
        assert_eq!(DownloadTaskStatus::Cancel.name(), "Cancel");
    }

    #[test]
    fn new_task_defaults() {
        let task = DownloadTask::new(
            1,
            "https://example.com/song.7z".to_string(),
            "Test Song".to_string(),
            "abc123".to_string(),
        );
        assert_eq!(task.id(), 1);
        assert_eq!(task.url(), "https://example.com/song.7z");
        assert_eq!(task.name(), "Test Song");
        assert_eq!(task.hash(), "abc123");
        assert_eq!(task.download_task_status(), DownloadTaskStatus::Prepare);
        assert_eq!(task.download_size, 0);
        assert_eq!(task.content_length, 0);
        assert!(task.error_message().is_none());
        assert_eq!(task.time_finished(), 0);
    }

    #[test]
    fn set_status_below_extracted_does_not_set_time_finished() {
        let mut task = DownloadTask::new(
            2,
            "https://example.com/a.7z".to_string(),
            "Song A".to_string(),
            "hash_a".to_string(),
        );
        task.set_download_task_status(DownloadTaskStatus::Downloading);
        assert_eq!(task.download_task_status(), DownloadTaskStatus::Downloading);
        assert_eq!(task.time_finished(), 0);

        task.set_download_task_status(DownloadTaskStatus::Downloaded);
        assert_eq!(task.download_task_status(), DownloadTaskStatus::Downloaded);
        assert_eq!(task.time_finished(), 0);
    }

    #[test]
    fn set_status_extracted_sets_time_finished() {
        let mut task = DownloadTask::new(
            3,
            "https://example.com/b.7z".to_string(),
            "Song B".to_string(),
            "hash_b".to_string(),
        );
        task.set_download_task_status(DownloadTaskStatus::Extracted);
        assert_eq!(task.download_task_status(), DownloadTaskStatus::Extracted);
        assert_ne!(task.time_finished(), 0);
    }

    #[test]
    fn set_status_error_sets_time_finished() {
        let mut task = DownloadTask::new(
            4,
            "https://example.com/c.7z".to_string(),
            "Song C".to_string(),
            "hash_c".to_string(),
        );
        task.set_download_task_status(DownloadTaskStatus::Error);
        assert_eq!(task.download_task_status(), DownloadTaskStatus::Error);
        // Error (value 4) >= Extracted (value 3), so time_finished is set
        assert_ne!(task.time_finished(), 0);
    }

    #[test]
    fn download_size_and_content_length() {
        let mut task = DownloadTask::new(
            5,
            "https://example.com/d.7z".to_string(),
            "Song D".to_string(),
            "hash_d".to_string(),
        );
        task.download_size = 4096;
        task.content_length = 8192;
        assert_eq!(task.download_size, 4096);
        assert_eq!(task.content_length, 8192);
    }

    #[test]
    fn error_message() {
        let mut task = DownloadTask::new(
            6,
            "https://example.com/e.7z".to_string(),
            "Song E".to_string(),
            "hash_e".to_string(),
        );
        assert!(task.error_message().is_none());
        task.set_error_message("Connection timeout".to_string());
        assert_eq!(task.error_message(), Some("Connection timeout"));
    }

    #[test]
    fn empty_strings_in_constructor() {
        let task = DownloadTask::new(0, String::new(), String::new(), String::new());
        assert_eq!(task.id(), 0);
        assert_eq!(task.url(), "");
        assert_eq!(task.name(), "");
        assert_eq!(task.hash(), "");
    }

    #[test]
    fn special_characters_in_fields() {
        let task = DownloadTask::new(
            7,
            "https://example.com/path?q=a&b=c#frag".to_string(),
            "Song with spaces & symbols!".to_string(),
            "deadbeef".to_string(),
        );
        assert_eq!(task.url(), "https://example.com/path?q=a&b=c#frag");
        assert_eq!(task.name(), "Song with spaces & symbols!");
    }
}
