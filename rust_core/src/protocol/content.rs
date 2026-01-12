//! Content types for clipboard data
//!
//! Defines clipboard content types and metadata structures.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Type of clipboard content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ContentType {
    /// Plain UTF-8 text
    PlainText = 0,
    /// Rich text (HTML or RTF)
    RichText = 1,
    /// Image data (PNG, JPEG, etc.)
    Image = 2,
    /// File(s)
    File = 3,
    /// URL (detected from text)
    Url = 4,
}

impl ContentType {
    /// Get MIME type for this content type
    pub fn mime_type(&self) -> &'static str {
        match self {
            ContentType::PlainText => "text/plain",
            ContentType::RichText => "text/html",
            ContentType::Image => "image/png",
            ContentType::File => "application/octet-stream",
            ContentType::Url => "text/uri-list",
        }
    }
}

impl TryFrom<u8> for ContentType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ContentType::PlainText),
            1 => Ok(ContentType::RichText),
            2 => Ok(ContentType::Image),
            3 => Ok(ContentType::File),
            4 => Ok(ContentType::Url),
            _ => Err(()),
        }
    }
}

/// Metadata about clipboard content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentMetadata {
    /// Original filename (for files)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Image dimensions (width, height)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<(u32, u32)>,

    /// Preview/thumbnail data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<Vec<u8>>,

    /// Size in bytes
    pub size_bytes: u64,

    /// Text preview (first N characters for text content)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_preview: Option<String>,
}

/// Clipboard content with type and data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardContent {
    /// Type of content
    pub content_type: ContentType,

    /// Raw content data
    pub data: Vec<u8>,

    /// Content metadata
    pub metadata: ContentMetadata,
}

impl ClipboardContent {
    /// Create new clipboard content
    pub fn new(content_type: ContentType, data: Vec<u8>) -> Self {
        let metadata = ContentMetadata {
            size_bytes: data.len() as u64,
            ..Default::default()
        };

        Self {
            content_type,
            data,
            metadata,
        }
    }

    /// Create text content
    pub fn text(text: &str) -> Self {
        let data = text.as_bytes().to_vec();
        let preview = if text.len() > 200 {
            Some(text[..200].to_string())
        } else {
            Some(text.to_string())
        };

        Self {
            content_type: if is_url(text) { ContentType::Url } else { ContentType::PlainText },
            data,
            metadata: ContentMetadata {
                size_bytes: text.len() as u64,
                text_preview: preview,
                mime_type: Some("text/plain".to_string()),
                ..Default::default()
            },
        }
    }

    /// Create image content
    pub fn image(data: Vec<u8>, dimensions: Option<(u32, u32)>, mime_type: Option<String>) -> Self {
        Self {
            content_type: ContentType::Image,
            metadata: ContentMetadata {
                size_bytes: data.len() as u64,
                dimensions,
                mime_type,
                ..Default::default()
            },
            data,
        }
    }

    /// Calculate SHA-256 hash of content
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        hasher.finalize().into()
    }

    /// Get content as string (for text types)
    pub fn as_text(&self) -> Option<String> {
        match self.content_type {
            ContentType::PlainText | ContentType::Url | ContentType::RichText => {
                String::from_utf8(self.data.clone()).ok()
            }
            _ => None,
        }
    }

    /// Check if content is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Check if a string is a URL
pub fn is_url(text: &str) -> bool {
    let trimmed = text.trim();
    trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("ftp://")
        || trimmed.starts_with("file://")
}

/// Detect content type from raw data
pub fn detect_content_type(data: &[u8]) -> ContentType {
    // Check for image magic bytes
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        // PNG
        return ContentType::Image;
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        // JPEG
        return ContentType::Image;
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        // GIF
        return ContentType::Image;
    }
    if data.starts_with(b"RIFF") && data.len() > 12 && &data[8..12] == b"WEBP" {
        // WebP
        return ContentType::Image;
    }

    // Check for HTML
    if let Ok(text) = std::str::from_utf8(data) {
        let lower = text.to_lowercase();
        if lower.contains("<!doctype html") || lower.contains("<html") {
            return ContentType::RichText;
        }
        if is_url(text) {
            return ContentType::Url;
        }
        // Valid UTF-8 text
        return ContentType::PlainText;
    }

    // Default to file for binary data
    ContentType::File
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_from_u8() {
        assert_eq!(ContentType::try_from(0).unwrap(), ContentType::PlainText);
        assert_eq!(ContentType::try_from(2).unwrap(), ContentType::Image);
        assert!(ContentType::try_from(255).is_err());
    }

    #[test]
    fn test_is_url() {
        assert!(is_url("https://example.com"));
        assert!(is_url("http://test.org/path"));
        assert!(is_url("  https://example.com  "));
        assert!(!is_url("not a url"));
        assert!(!is_url("example.com"));
    }

    #[test]
    fn test_detect_content_type() {
        // PNG magic bytes
        assert_eq!(
            detect_content_type(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
            ContentType::Image
        );

        // Plain text
        assert_eq!(detect_content_type(b"Hello, World!"), ContentType::PlainText);

        // URL
        assert_eq!(detect_content_type(b"https://example.com"), ContentType::Url);
    }

    #[test]
    fn test_clipboard_content_hash() {
        let content1 = ClipboardContent::text("Hello");
        let content2 = ClipboardContent::text("Hello");
        let content3 = ClipboardContent::text("World");

        assert_eq!(content1.hash(), content2.hash());
        assert_ne!(content1.hash(), content3.hash());
    }

    #[test]
    fn test_text_content() {
        let content = ClipboardContent::text("Hello, World!");
        assert_eq!(content.content_type, ContentType::PlainText);
        assert_eq!(content.as_text().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_url_detection_in_text() {
        let content = ClipboardContent::text("https://github.com");
        assert_eq!(content.content_type, ContentType::Url);
    }
}
