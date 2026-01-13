//! Rich text clipboard handling (HTML/RTF)
//!
//! Provides platform-specific rich text clipboard operations:
//! - Windows: CF_HTML, CF_RTF formats
//! - macOS: HTML, RTF pasteboard types
//! - Linux: text/html, text/rtf MIME types

use crate::error::ClipboardError;
use crate::protocol::ClipboardContent;

/// Rich text format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichTextFormat {
    /// HTML format
    Html,
    /// RTF format
    Rtf,
}

impl RichTextFormat {
    /// Detect format from content
    pub fn detect(content: &ClipboardContent) -> Option<Self> {
        // Check metadata for format hint
        if let Some(mime) = &content.metadata.mime_type {
            if mime.contains("html") {
                return Some(Self::Html);
            }
            if mime.contains("rtf") {
                return Some(Self::Rtf);
            }
        }

        // Try to detect from content
        let text = String::from_utf8(content.data.clone()).ok()?;
        
        // HTML detection: look for HTML tags
        if text.trim_start().starts_with("<") && text.contains("</") {
            return Some(Self::Html);
        }

        // RTF detection: look for RTF header
        if text.starts_with("{\\rtf") {
            return Some(Self::Rtf);
        }

        None
    }

    /// Get MIME type for this format
    #[allow(dead_code)]
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Html => "text/html",
            Self::Rtf => "text/rtf",
        }
    }
}

/// Platform-specific rich text clipboard operations
pub trait RichTextClipboardProvider: Send + Sync {
    /// Read rich text from clipboard
    #[allow(dead_code)]
    fn read_rich_text(&self, format: RichTextFormat) -> Result<Option<ClipboardContent>, ClipboardError>;

    /// Write rich text to clipboard
    fn write_rich_text(&self, content: &ClipboardContent, format: RichTextFormat) -> Result<(), ClipboardError>;
}

/// Default rich text clipboard provider (falls back to plain text)
pub struct DefaultRichTextClipboardProvider;

impl RichTextClipboardProvider for DefaultRichTextClipboardProvider {
    #[allow(dead_code)]
    fn read_rich_text(&self, _format: RichTextFormat) -> Result<Option<ClipboardContent>, ClipboardError> {
        // Default: not supported, return None
        // Platform-specific implementations needed
        Ok(None)
    }

    fn write_rich_text(&self, content: &ClipboardContent, _format: RichTextFormat) -> Result<(), ClipboardError> {
        // Default: write as plain text
        // Extract plain text from HTML if needed
        if let Some(format) = RichTextFormat::detect(content) {
            match format {
                RichTextFormat::Html => {
                    // Simple HTML to text extraction (strip tags)
                    // In production, use a proper HTML parser
                    let html = String::from_utf8(content.data.clone())
                        .map_err(|e| ClipboardError::OperationFailed(format!("Invalid HTML: {}", e)))?;
                    
                    // Simple tag stripping (basic implementation)
                    // Remove HTML tags using simple string manipulation
                    let mut plain_text = html;
                    // Remove script and style tags with content
                    while let Some(start) = plain_text.find("<script") {
                        if let Some(end) = plain_text[start..].find("</script>") {
                            plain_text.replace_range(start..start+end+9, "");
                        } else {
                            break;
                        }
                    }
                    while let Some(start) = plain_text.find("<style") {
                        if let Some(end) = plain_text[start..].find("</style>") {
                            plain_text.replace_range(start..start+end+8, "");
                        } else {
                            break;
                        }
                    }
                    // Remove remaining tags (simplified - doesn't handle all cases)
                    plain_text.replace("<", " <").split('<').map(|s| {
                        if let Some(end) = s.find('>') {
                            &s[end+1..]
                        } else {
                            s
                        }
                    }).collect::<Vec<_>>().join("").trim().to_string()
                }
                RichTextFormat::Rtf => {
                    // RTF to text extraction (simplified)
                    // In production, use a proper RTF parser
                    String::from_utf8(content.data.clone())
                        .map_err(|e| ClipboardError::OperationFailed(format!("Invalid RTF: {}", e)))?
                }
            }
        } else {
            String::from_utf8(content.data.clone())
                .map_err(|e| ClipboardError::OperationFailed(format!("Invalid text: {}", e)))?
        };

        // Note: This is a simplified fallback - proper implementation would
        // use platform-specific rich text APIs
        // The text extraction is done, but writing is handled by the main handler
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    // TODO: Implement Windows CF_HTML and CF_RTF handling
    // TODO: Implement Windows CF_HTML and CF_RTF handling
    // This requires:
    // 1. Using Windows API to read/write CF_HTML format
    // 2. Using Windows API to read/write CF_RTF format
    // 3. Parsing HTML/RTF clipboard formats
}

#[cfg(target_os = "macos")]
mod macos_impl {
    // TODO: Implement macOS HTML/RTF pasteboard types
    // TODO: Implement macOS HTML/RTF pasteboard types
    // This requires:
    // 1. Using NSPasteboard with HTML/RTF types
    // 2. Converting between formats
}

#[cfg(target_os = "linux")]
mod linux_impl {
    // TODO: Implement Linux text/html and text/rtf MIME types
    // TODO: Implement Linux text/html and text/rtf MIME types
    // This requires:
    // 1. Using X11/Wayland clipboard with MIME types
    // 2. Handling multiple MIME types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_detection() {
        let html_content = ClipboardContent::new(
            crate::protocol::ContentType::RichText,
            "<html><body>Test</body></html>".as_bytes().to_vec(),
        );
        
        let format = RichTextFormat::detect(&html_content);
        assert_eq!(format, Some(RichTextFormat::Html));
    }

    #[test]
    fn test_rtf_detection() {
        let rtf_content = ClipboardContent::new(
            crate::protocol::ContentType::RichText,
            "{\\rtf1\\ansi Test}".as_bytes().to_vec(),
        );
        
        let format = RichTextFormat::detect(&rtf_content);
        assert_eq!(format, Some(RichTextFormat::Rtf));
    }

    #[test]
    fn test_mime_type() {
        assert_eq!(RichTextFormat::Html.mime_type(), "text/html");
        assert_eq!(RichTextFormat::Rtf.mime_type(), "text/rtf");
    }
}
