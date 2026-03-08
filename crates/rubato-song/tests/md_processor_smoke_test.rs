//! Smoke tests for md-processor crate: zero-coverage API surface.
//!
//! These tests verify that core data types can be constructed and accessed
//! without panicking.

use rubato_song::md_processor::download_task::{DownloadTask, DownloadTaskStatus};

// ---------------------------------------------------------------------------
// DownloadTask
// ---------------------------------------------------------------------------

#[test]
fn download_task_construction() {
    let task = DownloadTask::new(
        1,
        "https://example.com/chart.zip".to_string(),
        "Test Chart Pack".to_string(),
        "abc123def456".to_string(),
    );

    assert_eq!(task.id(), 1);
    assert_eq!(task.url(), "https://example.com/chart.zip");
    assert_eq!(task.name(), "Test Chart Pack");
    assert_eq!(task.hash(), "abc123def456");
    assert_eq!(task.download_task_status(), DownloadTaskStatus::Prepare);
    assert_eq!(task.download_size, 0);
    assert_eq!(task.content_length, 0);
    assert!(task.get_error_message().is_none());
    assert_eq!(task.time_finished(), 0);
}

#[test]
fn download_task_status_transitions() {
    let mut task = DownloadTask::new(
        2,
        "https://example.com/song.7z".to_string(),
        "Song Pack".to_string(),
        "deadbeef".to_string(),
    );

    // Prepare -> Downloading
    task.set_download_task_status(DownloadTaskStatus::Downloading);
    assert_eq!(task.download_task_status(), DownloadTaskStatus::Downloading);
    assert_eq!(task.time_finished(), 0); // Not finished yet

    // Downloading -> Downloaded
    task.set_download_task_status(DownloadTaskStatus::Downloaded);
    assert_eq!(task.download_task_status(), DownloadTaskStatus::Downloaded);

    // Downloaded -> Extracted (sets time_finished)
    task.set_download_task_status(DownloadTaskStatus::Extracted);
    assert_eq!(task.download_task_status(), DownloadTaskStatus::Extracted);
    assert!(
        task.time_finished() > 0,
        "time_finished should be set after Extracted"
    );
}

#[test]
fn download_task_error_message() {
    let mut task = DownloadTask::new(
        3,
        "https://example.com/broken.zip".to_string(),
        "Broken Pack".to_string(),
        "000000".to_string(),
    );

    task.set_download_task_status(DownloadTaskStatus::Error);
    task.set_error_message("connection timed out".to_string());

    assert_eq!(task.download_task_status(), DownloadTaskStatus::Error);
    assert_eq!(task.get_error_message(), Some("connection timed out"));
}

#[test]
fn download_task_size_tracking() {
    let mut task = DownloadTask::new(
        4,
        "https://example.com/large.zip".to_string(),
        "Large Pack".to_string(),
        "ffffff".to_string(),
    );

    task.content_length = 1_000_000;
    task.download_size = 500_000;

    assert_eq!(task.content_length, 1_000_000);
    assert_eq!(task.download_size, 500_000);
}

// ---------------------------------------------------------------------------
// DownloadTaskStatus
// ---------------------------------------------------------------------------

#[test]
fn download_task_status_values_and_names() {
    assert_eq!(DownloadTaskStatus::Prepare.value(), 0);
    assert_eq!(DownloadTaskStatus::Prepare.name(), "Prepare");

    assert_eq!(DownloadTaskStatus::Downloading.value(), 1);
    assert_eq!(DownloadTaskStatus::Downloading.name(), "Downloading");

    assert_eq!(DownloadTaskStatus::Downloaded.value(), 2);
    assert_eq!(DownloadTaskStatus::Downloaded.name(), "Downloaded");

    assert_eq!(DownloadTaskStatus::Extracted.value(), 3);
    assert_eq!(DownloadTaskStatus::Extracted.name(), "Finished");

    assert_eq!(DownloadTaskStatus::Error.value(), 4);
    assert_eq!(DownloadTaskStatus::Error.name(), "Error");

    assert_eq!(DownloadTaskStatus::Cancel.value(), 5);
    assert_eq!(DownloadTaskStatus::Cancel.name(), "Cancel");
}
