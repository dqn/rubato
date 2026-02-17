//! Song package downloader for BMS archives from external sources.
//!
//! Provides [`processor::DownloadProcessor`] for managing download queues,
//! [`source`] for site-specific scrapers (Konmai, Wriggle, etc.),
//! [`task::DownloadTask`] for individual download state tracking, and
//! [`extract`] for archive extraction (zip, lzh, tar.gz) with path traversal protection.

pub mod extract;
pub mod processor;
pub mod source;
pub mod task;
