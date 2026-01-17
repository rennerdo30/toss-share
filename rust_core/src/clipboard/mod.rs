//! Clipboard operations for Toss
//!
//! This module provides cross-platform clipboard access with:
//! - Read/write operations for text, images, and files
//! - Change detection via polling
//! - Content type detection

// Desktop-only modules (require arboard and image crates)
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod file_handler;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod formats;

mod handler;
mod monitor;

// Desktop-only modules
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod rich_text;

#[cfg(target_os = "windows")]
pub mod windows_formats;

#[cfg(target_os = "linux")]
pub mod linux_display;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub use formats::{decode_image, encode_image_to_png};
pub use handler::{ClipboardHandler, ClipboardProvider};
pub use monitor::ClipboardMonitor;

use crate::error::ClipboardError;
use crate::protocol::{ClipboardContent, ContentType};

/// Clipboard manager combining handler and monitor
pub struct ClipboardManager {
    handler: ClipboardHandler,
    monitor: ClipboardMonitor,
}

impl ClipboardManager {
    /// Create a new clipboard manager
    pub fn new() -> Result<Self, ClipboardError> {
        let handler = ClipboardHandler::new()?;
        let monitor = ClipboardMonitor::new();

        Ok(Self { handler, monitor })
    }

    /// Read current clipboard content
    pub fn read(&self) -> Result<Option<ClipboardContent>, ClipboardError> {
        self.handler.read()
    }

    /// Write content to clipboard
    pub fn write(&self, content: &ClipboardContent) -> Result<(), ClipboardError> {
        self.handler.write(content)
    }

    /// Clear the clipboard
    pub fn clear(&self) -> Result<(), ClipboardError> {
        self.handler.clear()
    }

    /// Check if clipboard supports a content type
    pub fn supports_type(&self, content_type: ContentType) -> bool {
        self.handler.supports_type(content_type)
    }

    /// Get the monitor for change detection
    pub fn monitor(&self) -> &ClipboardMonitor {
        &self.monitor
    }

    /// Get mutable monitor
    pub fn monitor_mut(&mut self) -> &mut ClipboardMonitor {
        &mut self.monitor
    }

    /// Check if clipboard has changed since last check
    pub fn has_changed(&mut self) -> bool {
        if let Ok(Some(content)) = self.read() {
            self.monitor.check_change(&content)
        } else {
            false
        }
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new().expect("Failed to create clipboard manager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests interact with the real system clipboard.
    // They are ignored by default to avoid interference with parallel tests.
    // Run with: cargo test -- --ignored --test-threads=1

    #[test]
    #[ignore = "Uses real system clipboard - run with --test-threads=1"]
    fn test_clipboard_text_roundtrip() {
        let manager = ClipboardManager::new().unwrap();

        // Write text
        let content = ClipboardContent::text("Test clipboard content");
        manager.write(&content).unwrap();

        // Read back
        let read_content = manager.read().unwrap().unwrap();
        assert_eq!(read_content.content_type, ContentType::PlainText);
        assert_eq!(read_content.as_text().unwrap(), "Test clipboard content");
    }

    #[test]
    #[ignore = "Uses real system clipboard - run with --test-threads=1"]
    fn test_change_detection() {
        let mut manager = ClipboardManager::new().unwrap();

        // Write initial content
        let content1 = ClipboardContent::text("Content 1");
        manager.write(&content1).unwrap();
        manager.has_changed(); // Reset

        // Same content should not trigger change
        manager.write(&content1).unwrap();
        assert!(!manager.has_changed());

        // Different content should trigger change
        let content2 = ClipboardContent::text("Content 2");
        manager.write(&content2).unwrap();
        assert!(manager.has_changed());
    }
}
