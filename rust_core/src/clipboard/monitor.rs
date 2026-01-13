//! Clipboard change monitoring
//!
//! Monitors clipboard for changes using polling and hash comparison.

#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::protocol::ClipboardContent;

/// Default polling interval in milliseconds
const DEFAULT_POLL_INTERVAL_MS: u64 = 250;

/// Clipboard monitor for detecting changes
pub struct ClipboardMonitor {
    /// Hash of last seen clipboard content
    last_hash: Option<[u8; 32]>,
    /// Whether monitoring is active
    running: Arc<AtomicBool>,
    /// Polling interval
    poll_interval: Duration,
}

impl ClipboardMonitor {
    /// Create a new clipboard monitor
    pub fn new() -> Self {
        Self {
            last_hash: None,
            running: Arc::new(AtomicBool::new(false)),
            poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
        }
    }

    /// Create with custom polling interval
    pub fn with_interval(interval_ms: u64) -> Self {
        Self {
            last_hash: None,
            running: Arc::new(AtomicBool::new(false)),
            poll_interval: Duration::from_millis(interval_ms),
        }
    }

    /// Check if content has changed since last check
    pub fn check_change(&mut self, content: &ClipboardContent) -> bool {
        let new_hash = Self::hash_content(content);

        let changed = match self.last_hash {
            Some(old_hash) => old_hash != new_hash,
            None => true,
        };

        self.last_hash = Some(new_hash);
        changed
    }

    /// Reset the last hash (useful when writing to clipboard)
    pub fn reset(&mut self) {
        self.last_hash = None;
    }

    /// Update the last hash without checking for change
    pub fn update_hash(&mut self, content: &ClipboardContent) {
        self.last_hash = Some(Self::hash_content(content));
    }

    /// Get the polling interval
    pub fn poll_interval(&self) -> Duration {
        self.poll_interval
    }

    /// Set the polling interval
    pub fn set_poll_interval(&mut self, interval_ms: u64) {
        self.poll_interval = Duration::from_millis(interval_ms);
    }

    /// Check if monitoring is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Stop monitoring
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Get a handle to the running flag for async monitoring
    pub fn running_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Start monitoring - sets the running flag to true
    pub fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
    }

    /// Hash clipboard content for comparison
    fn hash_content(content: &ClipboardContent) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update([content.content_type as u8]);
        hasher.update(&content.data);
        hasher.finalize().into()
    }
}

impl Default for ClipboardMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Async clipboard monitoring task
pub async fn run_monitor<F>(
    running: Arc<AtomicBool>,
    poll_interval: Duration,
    mut read_fn: F,
    change_tx: broadcast::Sender<ClipboardContent>,
) where
    F: FnMut() -> Option<ClipboardContent>,
{
    let mut last_hash: Option<[u8; 32]> = None;

    while running.load(Ordering::Relaxed) {
        if let Some(content) = read_fn() {
            let new_hash = {
                let mut hasher = Sha256::new();
                hasher.update([content.content_type as u8]);
                hasher.update(&content.data);
                let result: [u8; 32] = hasher.finalize().into();
                result
            };

            let changed = match last_hash {
                Some(old_hash) => old_hash != new_hash,
                None => true,
            };

            if changed {
                last_hash = Some(new_hash);
                // Send might fail if no receivers, that's ok
                let _ = change_tx.send(content);
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_detection() {
        let mut monitor = ClipboardMonitor::new();

        let content1 = ClipboardContent::text("Hello");
        let content2 = ClipboardContent::text("Hello");
        let content3 = ClipboardContent::text("World");

        // First content is always a change
        assert!(monitor.check_change(&content1));

        // Same content should not be a change
        assert!(!monitor.check_change(&content2));

        // Different content should be a change
        assert!(monitor.check_change(&content3));
    }

    #[test]
    fn test_reset() {
        let mut monitor = ClipboardMonitor::new();

        let content = ClipboardContent::text("Test");
        monitor.check_change(&content);

        // Reset should make next check a change
        monitor.reset();
        assert!(monitor.check_change(&content));
    }

    #[test]
    fn test_running_flag() {
        let monitor = ClipboardMonitor::new();

        assert!(!monitor.is_running());
        monitor.start();
        assert!(monitor.is_running());
        monitor.stop();
        assert!(!monitor.is_running());
    }

    #[test]
    fn test_custom_interval() {
        let monitor = ClipboardMonitor::with_interval(500);
        assert_eq!(monitor.poll_interval(), Duration::from_millis(500));
    }
}
