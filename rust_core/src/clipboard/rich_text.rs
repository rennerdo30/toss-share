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
pub mod windows_impl {
    use super::*;

    /// Windows rich text clipboard provider
    ///
    /// Windows uses CF_HTML and CF_RTF clipboard formats:
    /// - CF_HTML: HTML Format with header describing fragment location
    /// - CF_RTF: Rich Text Format
    ///
    /// CF_HTML header format:
    /// ```text
    /// Version:0.9
    /// StartHTML:XXXX
    /// EndHTML:XXXX
    /// StartFragment:XXXX
    /// EndFragment:XXXX
    /// <html>...<!--StartFragment-->content<!--EndFragment-->...</html>
    /// ```
    pub struct WindowsRichTextClipboardProvider;

    impl RichTextClipboardProvider for WindowsRichTextClipboardProvider {
        fn read_rich_text(&self, _format: RichTextFormat) -> Result<Option<ClipboardContent>, ClipboardError> {
            // Full implementation would use Windows clipboard APIs
            // to read CF_HTML or CF_RTF format
            Ok(None)
        }

        fn write_rich_text(&self, _content: &ClipboardContent, _format: RichTextFormat) -> Result<(), ClipboardError> {
            // Full implementation would:
            // 1. For HTML: Create CF_HTML header and set clipboard
            // 2. For RTF: Set CF_RTF format
            Err(ClipboardError::UnsupportedFormat(
                "Windows rich text writing requires native implementation".to_string(),
            ))
        }
    }

    impl WindowsRichTextClipboardProvider {
        /// Create a new Windows rich text clipboard provider
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self
        }

        /// Create CF_HTML format from HTML content
        /// Returns the complete CF_HTML clipboard data with header
        #[allow(dead_code)]
        pub fn create_cf_html(html: &str) -> String {
            // Calculate positions
            let header_template = "Version:0.9\r\nStartHTML:0000000000\r\nEndHTML:0000000000\r\nStartFragment:0000000000\r\nEndFragment:0000000000\r\n";
            let header_len = header_template.len();

            // Wrap HTML fragment with markers
            let html_start = "<!DOCTYPE html><html><body><!--StartFragment-->";
            let html_end = "<!--EndFragment--></body></html>";
            let full_html = format!("{}{}{}", html_start, html, html_end);

            let start_html = header_len;
            let end_html = start_html + full_html.len();
            let start_fragment = start_html + html_start.len();
            let end_fragment = end_html - html_end.len();

            // Format header with actual positions
            let header = format!(
                "Version:0.9\r\nStartHTML:{:010}\r\nEndHTML:{:010}\r\nStartFragment:{:010}\r\nEndFragment:{:010}\r\n",
                start_html, end_html, start_fragment, end_fragment
            );

            format!("{}{}", header, full_html)
        }

        /// Parse CF_HTML format and extract the HTML fragment
        #[allow(dead_code)]
        pub fn parse_cf_html(cf_html: &str) -> Option<String> {
            // Parse header to get fragment positions
            let mut start_fragment: Option<usize> = None;
            let mut end_fragment: Option<usize> = None;

            for line in cf_html.lines() {
                if line.starts_with("StartFragment:") {
                    start_fragment = line[14..].parse().ok();
                } else if line.starts_with("EndFragment:") {
                    end_fragment = line[12..].parse().ok();
                }
            }

            match (start_fragment, end_fragment) {
                (Some(start), Some(end)) if start < end && end <= cf_html.len() => {
                    Some(cf_html[start..end].to_string())
                }
                _ => None,
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub mod macos_impl {
    use super::*;

    /// macOS rich text clipboard provider
    ///
    /// macOS uses pasteboard types:
    /// - public.html: HTML content
    /// - public.rtf: RTF content
    /// - com.apple.webarchive: Web archive (HTML with resources)
    pub struct MacOSRichTextClipboardProvider;

    impl RichTextClipboardProvider for MacOSRichTextClipboardProvider {
        fn read_rich_text(&self, _format: RichTextFormat) -> Result<Option<ClipboardContent>, ClipboardError> {
            // Full implementation would use NSPasteboard APIs
            Ok(None)
        }

        fn write_rich_text(&self, _content: &ClipboardContent, _format: RichTextFormat) -> Result<(), ClipboardError> {
            // Full implementation would set appropriate pasteboard types
            Err(ClipboardError::UnsupportedFormat(
                "macOS rich text writing requires native implementation".to_string(),
            ))
        }
    }

    impl MacOSRichTextClipboardProvider {
        /// Create a new macOS rich text clipboard provider
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self
        }

        /// Get the pasteboard type string for a format
        #[allow(dead_code)]
        pub fn pasteboard_type(format: RichTextFormat) -> &'static str {
            match format {
                RichTextFormat::Html => "public.html",
                RichTextFormat::Rtf => "public.rtf",
            }
        }

        /// Convert HTML to a simple RTF representation
        /// This is a basic conversion for interoperability
        #[allow(dead_code)]
        pub fn html_to_basic_rtf(html: &str) -> String {
            // Very basic conversion - just extract text
            // A full implementation would preserve formatting
            let text = Self::strip_html_tags(html);
            format!("{{\\rtf1\\ansi {}\\par}}", Self::escape_rtf(&text))
        }

        /// Strip HTML tags (basic implementation)
        #[allow(dead_code)]
        fn strip_html_tags(html: &str) -> String {
            let mut result = String::new();
            let mut in_tag = false;

            for c in html.chars() {
                match c {
                    '<' => in_tag = true,
                    '>' => in_tag = false,
                    _ if !in_tag => result.push(c),
                    _ => {}
                }
            }

            result
        }

        /// Escape special RTF characters
        #[allow(dead_code)]
        fn escape_rtf(text: &str) -> String {
            text.chars()
                .map(|c| match c {
                    '\\' => "\\\\".to_string(),
                    '{' => "\\{".to_string(),
                    '}' => "\\}".to_string(),
                    '\n' => "\\par ".to_string(),
                    _ if c as u32 > 127 => format!("\\u{}?", c as i32),
                    _ => c.to_string(),
                })
                .collect()
        }
    }
}

#[cfg(target_os = "linux")]
pub mod linux_impl {
    use super::*;

    /// Linux rich text clipboard provider
    ///
    /// Linux clipboards use MIME types:
    /// - text/html: HTML content
    /// - text/rtf or application/rtf: RTF content
    /// - text/plain: Plain text fallback
    ///
    /// Clipboard typically returns in order of preference,
    /// so we check for rich formats first
    pub struct LinuxRichTextClipboardProvider;

    impl RichTextClipboardProvider for LinuxRichTextClipboardProvider {
        fn read_rich_text(&self, _format: RichTextFormat) -> Result<Option<ClipboardContent>, ClipboardError> {
            // Full implementation would query clipboard for specific MIME types
            Ok(None)
        }

        fn write_rich_text(&self, _content: &ClipboardContent, _format: RichTextFormat) -> Result<(), ClipboardError> {
            // Full implementation would set clipboard with appropriate MIME type
            Err(ClipboardError::UnsupportedFormat(
                "Linux rich text writing requires native implementation".to_string(),
            ))
        }
    }

    impl LinuxRichTextClipboardProvider {
        /// Create a new Linux rich text clipboard provider
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self
        }

        /// Get MIME types for a format (in preference order)
        #[allow(dead_code)]
        pub fn mime_types(format: RichTextFormat) -> &'static [&'static str] {
            match format {
                RichTextFormat::Html => &["text/html", "application/xhtml+xml"],
                RichTextFormat::Rtf => &["text/rtf", "application/rtf"],
            }
        }

        /// Detect format from MIME type string
        #[allow(dead_code)]
        pub fn format_from_mime(mime: &str) -> Option<RichTextFormat> {
            let mime_lower = mime.to_lowercase();
            if mime_lower.contains("html") || mime_lower.contains("xhtml") {
                Some(RichTextFormat::Html)
            } else if mime_lower.contains("rtf") {
                Some(RichTextFormat::Rtf)
            } else {
                None
            }
        }

        /// Create targets list for rich text (for X11 selection)
        /// Returns list of MIME types that should be offered
        #[allow(dead_code)]
        pub fn create_targets(format: RichTextFormat) -> Vec<&'static str> {
            let mut targets = Vec::new();

            // Add format-specific types
            targets.extend_from_slice(Self::mime_types(format));

            // Always add plain text fallback
            targets.push("text/plain");
            targets.push("UTF8_STRING");
            targets.push("STRING");

            targets
        }
    }
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
