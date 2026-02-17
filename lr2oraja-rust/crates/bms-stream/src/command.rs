// Stream command definitions
//
// Defines the StreamCommand trait and StreamRequestCommand implementation.
// StreamRequestCommand handles "!!req <sha256>" messages from the named pipe.

use std::collections::VecDeque;

/// Trait for stream commands that process pipe messages.
pub trait StreamCommand: Send {
    /// The command prefix string (e.g., "!!req").
    fn command_string(&self) -> &str;

    /// Process the arguments after the command prefix.
    /// Returns an optional response string.
    fn run(&self, args: &str) -> anyhow::Result<Option<String>>;
}

/// Handles "!!req <sha256>" stream requests.
///
/// Stores incoming SHA256 hashes in a bounded queue.
/// The selector periodically drains pending requests via `poll_requests()`.
pub struct StreamRequestCommand {
    /// Pending SHA256 hashes waiting to be processed.
    pending: parking_lot::Mutex<VecDeque<String>>,
    /// Maximum number of pending requests.
    pub max_requests: usize,
}

impl Default for StreamRequestCommand {
    fn default() -> Self {
        Self::new(30)
    }
}

impl StreamRequestCommand {
    pub fn new(max_requests: usize) -> Self {
        Self {
            pending: parking_lot::Mutex::new(VecDeque::new()),
            max_requests,
        }
    }

    /// Drain and return all pending SHA256 request hashes.
    pub fn poll_requests(&self) -> Vec<String> {
        let mut pending = self.pending.lock();
        pending.drain(..).collect()
    }

    /// Return the current number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }
}

impl StreamCommand for StreamRequestCommand {
    fn command_string(&self) -> &str {
        "!!req"
    }

    fn run(&self, args: &str) -> anyhow::Result<Option<String>> {
        let hash = args.trim();

        // SHA256 hashes are exactly 64 hex characters
        if hash.len() != 64 {
            return Ok(None);
        }

        // Validate hex characters
        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(None);
        }

        let mut pending = self.pending.lock();

        // Evict oldest if at capacity
        if pending.len() >= self.max_requests {
            pending.pop_front();
        }

        pending.push_back(hash.to_string());
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_string() {
        let cmd = StreamRequestCommand::default();
        assert_eq!(cmd.command_string(), "!!req");
    }

    #[test]
    fn test_valid_sha256() {
        let cmd = StreamRequestCommand::default();
        let hash = "a".repeat(64);
        let result = cmd.run(&hash).unwrap();
        assert!(result.is_none());
        assert_eq!(cmd.pending_count(), 1);

        let requests = cmd.poll_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0], hash);
    }

    #[test]
    fn test_invalid_length() {
        let cmd = StreamRequestCommand::default();

        // Too short
        let result = cmd.run("abc123").unwrap();
        assert!(result.is_none());
        assert_eq!(cmd.pending_count(), 0);

        // Too long
        let hash = "a".repeat(65);
        let result = cmd.run(&hash).unwrap();
        assert!(result.is_none());
        assert_eq!(cmd.pending_count(), 0);
    }

    #[test]
    fn test_invalid_hex() {
        let cmd = StreamRequestCommand::default();

        // Contains non-hex characters
        let mut hash = "g".repeat(64);
        let result = cmd.run(&hash).unwrap();
        assert!(result.is_none());
        assert_eq!(cmd.pending_count(), 0);

        // Mixed valid/invalid
        hash = format!("{}x", "a".repeat(63));
        let result = cmd.run(&hash).unwrap();
        assert!(result.is_none());
        assert_eq!(cmd.pending_count(), 0);
    }

    #[test]
    fn test_whitespace_trimming() {
        let cmd = StreamRequestCommand::default();
        let hash = "a".repeat(64);
        let padded = format!("  {}  ", hash);
        cmd.run(&padded).unwrap();
        assert_eq!(cmd.pending_count(), 1);

        let requests = cmd.poll_requests();
        assert_eq!(requests[0], hash);
    }

    #[test]
    fn test_poll_drains() {
        let cmd = StreamRequestCommand::default();
        let hash1 = "a".repeat(64);
        let hash2 = "b".repeat(64);

        cmd.run(&hash1).unwrap();
        cmd.run(&hash2).unwrap();
        assert_eq!(cmd.pending_count(), 2);

        let requests = cmd.poll_requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], hash1);
        assert_eq!(requests[1], hash2);

        // Queue should be empty after polling
        assert_eq!(cmd.pending_count(), 0);
        let requests = cmd.poll_requests();
        assert!(requests.is_empty());
    }

    #[test]
    fn test_max_requests_eviction() {
        let cmd = StreamRequestCommand::new(3);

        // Fill to capacity
        for i in 0..3 {
            let hash = format!("{:0>64x}", i);
            cmd.run(&hash).unwrap();
        }
        assert_eq!(cmd.pending_count(), 3);

        // Adding one more should evict the oldest
        let new_hash = format!("{:0>64x}", 99);
        cmd.run(&new_hash).unwrap();
        assert_eq!(cmd.pending_count(), 3);

        let requests = cmd.poll_requests();
        assert_eq!(requests.len(), 3);
        // First entry (i=0) was evicted
        assert_eq!(requests[0], format!("{:0>64x}", 1));
        assert_eq!(requests[1], format!("{:0>64x}", 2));
        assert_eq!(requests[2], format!("{:0>64x}", 99));
    }

    #[test]
    fn test_empty_string() {
        let cmd = StreamRequestCommand::default();
        cmd.run("").unwrap();
        assert_eq!(cmd.pending_count(), 0);
    }

    #[test]
    fn test_mixed_case_hex() {
        let cmd = StreamRequestCommand::default();

        // Upper case hex
        let hash = "A".repeat(64);
        cmd.run(&hash).unwrap();
        assert_eq!(cmd.pending_count(), 1);

        // Mixed case
        let hash2 = format!("{}{}", "aB".repeat(16), "cD".repeat(16));
        cmd.run(&hash2).unwrap();
        assert_eq!(cmd.pending_count(), 2);
    }

    #[test]
    fn test_default_max_requests() {
        let cmd = StreamRequestCommand::default();
        assert_eq!(cmd.max_requests, 30);
    }
}
