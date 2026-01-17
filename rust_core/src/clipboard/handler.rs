//! Clipboard read/write operations
//!
//! On desktop platforms, uses arboard for clipboard access.
//! On mobile (Android/iOS), clipboard is handled by Flutter - this provides a stub.

use crate::error::ClipboardError;
use crate::protocol::{ClipboardContent, ContentType};

// Desktop-only imports
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use arboard::Clipboard;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use parking_lot::Mutex;

#[cfg(not(any(target_os = "android", target_os = "ios")))]
use super::file_handler::{DefaultFileClipboardProvider, FileClipboardProvider, FileList};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use super::formats::{decode_image, encode_image_to_png};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use super::rich_text::{
    DefaultRichTextClipboardProvider, RichTextClipboardProvider, RichTextFormat,
};

/// Trait for clipboard operations
pub trait ClipboardProvider: Send + Sync {
    /// Read current clipboard content
    fn read(&self) -> Result<Option<ClipboardContent>, ClipboardError>;

    /// Write content to clipboard
    fn write(&self, content: &ClipboardContent) -> Result<(), ClipboardError>;

    /// Clear the clipboard
    fn clear(&self) -> Result<(), ClipboardError>;

    /// Check if content type is supported
    fn supports_type(&self, content_type: ContentType) -> bool;
}

// ============================================================================
// Desktop Implementation (using arboard)
// ============================================================================

/// Clipboard handler using arboard (desktop platforms only)
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub struct ClipboardHandler {
    clipboard: Mutex<Clipboard>,
    file_provider: Box<dyn FileClipboardProvider>,
    rich_text_provider: Box<dyn RichTextClipboardProvider>,
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl ClipboardHandler {
    /// Create a new clipboard handler
    pub fn new() -> Result<Self, ClipboardError> {
        let clipboard =
            Clipboard::new().map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;

        Ok(Self {
            clipboard: Mutex::new(clipboard),
            file_provider: Box::new(DefaultFileClipboardProvider),
            rich_text_provider: Box::new(DefaultRichTextClipboardProvider),
        })
    }
}

// ============================================================================
// Mobile Stub Implementation (Android/iOS)
// ============================================================================

/// Clipboard handler stub for mobile platforms
/// On mobile, clipboard operations should be handled by Flutter/Dart
#[cfg(any(target_os = "android", target_os = "ios"))]
pub struct ClipboardHandler;

#[cfg(any(target_os = "android", target_os = "ios"))]
impl ClipboardHandler {
    /// Create a new clipboard handler (mobile stub)
    pub fn new() -> Result<Self, ClipboardError> {
        Ok(Self)
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
impl ClipboardProvider for ClipboardHandler {
    fn read(&self) -> Result<Option<ClipboardContent>, ClipboardError> {
        // On mobile, clipboard is handled by Flutter
        Ok(None)
    }

    fn write(&self, _content: &ClipboardContent) -> Result<(), ClipboardError> {
        // On mobile, clipboard is handled by Flutter
        Err(ClipboardError::OperationFailed(
            "Clipboard operations on mobile should use Flutter's clipboard API".to_string(),
        ))
    }

    fn clear(&self) -> Result<(), ClipboardError> {
        // On mobile, clipboard is handled by Flutter
        Err(ClipboardError::OperationFailed(
            "Clipboard operations on mobile should use Flutter's clipboard API".to_string(),
        ))
    }

    fn supports_type(&self, _content_type: ContentType) -> bool {
        // Mobile clipboard is handled by Flutter
        false
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl ClipboardProvider for ClipboardHandler {
    fn read(&self) -> Result<Option<ClipboardContent>, ClipboardError> {
        let mut clipboard = self.clipboard.lock();

        // Try to read text first (rich text detection happens after we have content)
        if let Ok(text) = clipboard.get_text() {
            if !text.is_empty() {
                return Ok(Some(ClipboardContent::text(&text)));
            }
        }

        // Try to read image
        if let Ok(image) = clipboard.get_image() {
            let png_data = encode_image_to_png(&image)?;
            return Ok(Some(ClipboardContent::image(
                png_data,
                Some((image.width as u32, image.height as u32)),
                Some("image/png".to_string()),
            )));
        }

        // Try to read files (platform-specific)
        if let Ok(Some(file_list)) = self.file_provider.read_files() {
            return Ok(Some(file_list.to_content()));
        }

        // Nothing readable
        Ok(None)
    }

    fn write(&self, content: &ClipboardContent) -> Result<(), ClipboardError> {
        let mut clipboard = self.clipboard.lock();

        match content.content_type {
            ContentType::PlainText | ContentType::Url => {
                let text = String::from_utf8(content.data.clone())
                    .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
                clipboard
                    .set_text(text)
                    .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
            }
            ContentType::RichText => {
                // Try to detect format and use platform-specific provider
                if let Some(format) = RichTextFormat::detect(content) {
                    // Try platform-specific rich text write
                    if self
                        .rich_text_provider
                        .write_rich_text(content, format)
                        .is_err()
                    {
                        // Fallback to plain text if rich text write fails
                        let text = String::from_utf8(content.data.clone())
                            .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
                        clipboard
                            .set_text(text)
                            .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
                    }
                } else {
                    // No format detected, write as plain text
                    let text = String::from_utf8(content.data.clone())
                        .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
                    clipboard
                        .set_text(text)
                        .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
                }
            }
            ContentType::Image => {
                let image = decode_image(&content.data)?;
                clipboard
                    .set_image(image)
                    .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
            }
            ContentType::File => {
                // Parse file list from content
                let file_list = FileList::from_content(content)?;

                // Write files using platform-specific provider
                self.file_provider.write_files(&file_list)?;
            }
        }

        Ok(())
    }

    fn clear(&self) -> Result<(), ClipboardError> {
        let mut clipboard = self.clipboard.lock();
        clipboard
            .clear()
            .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    fn supports_type(&self, content_type: ContentType) -> bool {
        match content_type {
            ContentType::PlainText => true,
            ContentType::Url => true,
            ContentType::RichText => true, // Written as plain text
            ContentType::Image => true,
            ContentType::File => {
                // Check if file provider supports files
                // For now, return true (will fail at runtime if not supported)
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires clipboard access (X11 server)
    fn test_handler_creation() {
        let handler = ClipboardHandler::new();
        assert!(handler.is_ok());
    }

    #[test]
    #[ignore] // Requires clipboard access (X11 server)
    fn test_supports_type() {
        let handler = ClipboardHandler::new().unwrap();
        assert!(handler.supports_type(ContentType::PlainText));
        assert!(handler.supports_type(ContentType::Image));
        assert!(!handler.supports_type(ContentType::File));
    }

    #[test]
    #[ignore = "Uses real system clipboard - run with --test-threads=1"]
    fn test_write_and_read_text() {
        let handler = ClipboardHandler::new().unwrap();
        let content = ClipboardContent::text("Test text");

        handler.write(&content).unwrap();

        let read = handler.read().unwrap();
        assert!(read.is_some());
        let read_content = read.unwrap();
        assert_eq!(read_content.as_text().unwrap(), "Test text");
    }
}
