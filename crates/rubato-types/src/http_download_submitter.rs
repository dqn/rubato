/// Trait for submitting HTTP download tasks.
/// Used by ContextMenuBar::fillMissingCharts to submit download tasks
/// without importing HttpDownloadProcessor from md-processor (avoids circular dep).
pub trait HttpDownloadSubmitter: Send + Sync {
    /// Submit a download task for a missing chart by MD5 hash.
    fn submit_md5_task(&self, md5: &str, task_name: &str);
}
