//! Clipboard read/write operations using arboard

use arboard::Clipboard;
use parking_lot::Mutex;

use super::formats::{decode_image, encode_image_to_png};
use crate::error::ClipboardError;
use crate::protocol::{ClipboardContent, ContentType};

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

/// Clipboard handler using arboard
pub struct ClipboardHandler {
    clipboard: Mutex<Clipboard>,
}

impl ClipboardHandler {
    /// Create a new clipboard handler
    pub fn new() -> Result<Self, ClipboardError> {
        let clipboard =
            Clipboard::new().map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;

        Ok(Self {
            clipboard: Mutex::new(clipboard),
        })
    }
}

impl ClipboardProvider for ClipboardHandler {
    fn read(&self) -> Result<Option<ClipboardContent>, ClipboardError> {
        let mut clipboard = self.clipboard.lock();

        // Try to read text first
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
                // For rich text, we write as plain text (arboard doesn't support HTML directly)
                let text = String::from_utf8(content.data.clone())
                    .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
                clipboard
                    .set_text(text)
                    .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
            }
            ContentType::Image => {
                let image = decode_image(&content.data)?;
                clipboard
                    .set_image(image)
                    .map_err(|e| ClipboardError::OperationFailed(e.to_string()))?;
            }
            ContentType::File => {
                // Files are not directly supported - would need platform-specific handling
                return Err(ClipboardError::UnsupportedFormat(
                    "File clipboard not supported".to_string(),
                ));
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
            ContentType::File => false, // Not directly supported
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_creation() {
        let handler = ClipboardHandler::new();
        assert!(handler.is_ok());
    }

    #[test]
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
