//! File clipboard handling
//!
//! Provides platform-specific file clipboard operations:
//! - Windows: CF_HDROP format
//! - macOS: File URLs in pasteboard
//! - Linux: File URI lists

use crate::error::ClipboardError;
use crate::protocol::ClipboardContent;
use std::path::PathBuf;

/// File list for clipboard operations
#[derive(Debug, Clone)]
pub struct FileList {
    pub files: Vec<PathBuf>,
}

impl FileList {
    /// Create a new file list
    #[allow(dead_code)]
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self { files }
    }

    /// Parse file list from clipboard content
    pub fn from_content(content: &ClipboardContent) -> Result<Self, ClipboardError> {
        if content.content_type != crate::protocol::ContentType::File {
            return Err(ClipboardError::UnsupportedFormat(
                "Content is not a file".to_string(),
            ));
        }

        // Parse file paths from content data
        // Format: newline-separated file paths (UTF-8)
        let text = String::from_utf8(content.data.clone())
            .map_err(|e| ClipboardError::OperationFailed(format!("Invalid file list: {}", e)))?;

        let files: Vec<PathBuf> = text
            .lines()
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .collect();

        Ok(Self { files })
    }

    /// Convert file list to clipboard content
    pub fn to_content(&self) -> ClipboardContent {
        // Serialize file paths as newline-separated UTF-8
        let data = self
            .files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes();

        ClipboardContent::new(crate::protocol::ContentType::File, data)
    }

    /// Get file count
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Platform-specific file clipboard operations
pub trait FileClipboardProvider: Send + Sync {
    /// Read file list from clipboard
    fn read_files(&self) -> Result<Option<FileList>, ClipboardError>;

    /// Write file list to clipboard
    fn write_files(&self, files: &FileList) -> Result<(), ClipboardError>;
}

/// Default file clipboard provider (uses file path serialization)
pub struct DefaultFileClipboardProvider;

impl FileClipboardProvider for DefaultFileClipboardProvider {
    fn read_files(&self) -> Result<Option<FileList>, ClipboardError> {
        // Default implementation: not supported
        // Platform-specific implementations needed
        Err(ClipboardError::UnsupportedFormat(
            "File clipboard not supported on this platform".to_string(),
        ))
    }

    fn write_files(&self, _files: &FileList) -> Result<(), ClipboardError> {
        Err(ClipboardError::UnsupportedFormat(
            "File clipboard not supported on this platform".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
pub mod windows_impl {
    use super::*;
    use crate::clipboard::windows_formats;

    /// Windows CF_HDROP file clipboard provider
    ///
    /// Uses the Windows clipboard API to read and write file lists
    /// in the CF_HDROP format.
    #[allow(dead_code)]
    pub struct WindowsFileClipboardProvider;

    impl FileClipboardProvider for WindowsFileClipboardProvider {
        fn read_files(&self) -> Result<Option<FileList>, ClipboardError> {
            match windows_formats::read_files()? {
                Some(files) => Ok(Some(FileList { files })),
                None => Ok(None),
            }
        }

        fn write_files(&self, files: &FileList) -> Result<(), ClipboardError> {
            windows_formats::write_files(&files.files)
        }
    }

    impl WindowsFileClipboardProvider {
        /// Create a new Windows file clipboard provider
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for WindowsFileClipboardProvider {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(target_os = "macos")]
pub mod macos_impl {
    use super::*;

    /// macOS file clipboard provider using file URLs
    ///
    /// macOS uses file:// URLs in the pasteboard for file references.
    /// The standard types are:
    /// - public.file-url: Individual file URL
    /// - NSFilenamesPboardType: Array of file paths (deprecated but still used)
    pub struct MacOSFileClipboardProvider;

    impl FileClipboardProvider for MacOSFileClipboardProvider {
        fn read_files(&self) -> Result<Option<FileList>, ClipboardError> {
            // Use arboard for basic clipboard access
            // Note: arboard doesn't directly support file URLs on macOS
            // A full implementation would use NSPasteboard APIs

            // For now, return not available - file transfer works via serialized paths
            Ok(None)
        }

        fn write_files(&self, _files: &FileList) -> Result<(), ClipboardError> {
            // A full implementation would:
            // 1. Create NSArray of NSURL objects
            // 2. Write to NSPasteboard with file URL types

            Err(ClipboardError::UnsupportedFormat(
                "macOS file URL writing requires native implementation".to_string(),
            ))
        }
    }

    impl MacOSFileClipboardProvider {
        /// Create a new macOS file clipboard provider
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self
        }

        /// Convert a file path to a file:// URL
        #[allow(dead_code)]
        pub fn path_to_file_url(path: &std::path::Path) -> String {
            // Encode path for URL
            let path_str = path.to_string_lossy();
            let encoded: String = path_str
                .chars()
                .map(|c| match c {
                    ' ' => "%20".to_string(),
                    '#' => "%23".to_string(),
                    '%' => "%25".to_string(),
                    '?' => "%3F".to_string(),
                    _ => c.to_string(),
                })
                .collect();

            format!("file://{}", encoded)
        }

        /// Convert a file:// URL to a file path
        #[allow(dead_code)]
        pub fn file_url_to_path(url: &str) -> Option<PathBuf> {
            if !url.starts_with("file://") {
                return None;
            }

            let path_part = &url[7..]; // Remove "file://"

            // Handle localhost prefix
            let path_part = path_part.strip_prefix("localhost").unwrap_or(path_part);

            // Decode URL encoding
            let decoded = Self::url_decode(path_part)?;
            Some(PathBuf::from(decoded))
        }

        /// Decode URL percent-encoding
        #[allow(dead_code)]
        fn url_decode(s: &str) -> Option<String> {
            let mut result = String::new();
            let mut chars = s.chars().peekable();

            while let Some(c) = chars.next() {
                if c == '%' {
                    // Get next two hex digits
                    let hex1 = chars.next()?;
                    let hex2 = chars.next()?;
                    let hex_str: String = [hex1, hex2].iter().collect();
                    let byte = u8::from_str_radix(&hex_str, 16).ok()?;
                    result.push(byte as char);
                } else {
                    result.push(c);
                }
            }

            Some(result)
        }

        /// Parse file URLs from pasteboard data
        /// The data may be a newline-separated list of file:// URLs
        #[allow(dead_code)]
        pub fn parse_file_urls(data: &str) -> Vec<PathBuf> {
            data.lines()
                .filter(|line| line.starts_with("file://"))
                .filter_map(Self::file_url_to_path)
                .collect()
        }

        /// Serialize file paths as file:// URLs
        #[allow(dead_code)]
        pub fn serialize_file_urls(files: &[PathBuf]) -> String {
            files
                .iter()
                .map(|p| Self::path_to_file_url(p))
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

#[cfg(target_os = "linux")]
pub mod linux_impl {
    use super::*;

    /// Linux file clipboard provider using text/uri-list
    ///
    /// Linux clipboards (both X11 and Wayland) use the text/uri-list MIME type
    /// for file references. The format is:
    /// - One file:// URL per line
    /// - Lines starting with # are comments
    /// - URLs are percent-encoded
    pub struct LinuxFileClipboardProvider;

    impl FileClipboardProvider for LinuxFileClipboardProvider {
        fn read_files(&self) -> Result<Option<FileList>, ClipboardError> {
            // Use arboard for basic clipboard access
            // Note: arboard supports text but not directly text/uri-list MIME type
            // A full implementation would query the clipboard directly

            // For now, return not available - file transfer works via serialized paths
            Ok(None)
        }

        fn write_files(&self, _files: &FileList) -> Result<(), ClipboardError> {
            // A full implementation would:
            // 1. Serialize paths as file:// URLs
            // 2. Set clipboard with text/uri-list MIME type

            Err(ClipboardError::UnsupportedFormat(
                "Linux file URI list writing requires native implementation".to_string(),
            ))
        }
    }

    impl LinuxFileClipboardProvider {
        /// Create a new Linux file clipboard provider
        #[allow(dead_code)]
        pub fn new() -> Self {
            Self
        }

        /// Convert a file path to a file:// URI
        /// Uses RFC 8089 file URI format
        #[allow(dead_code)]
        pub fn path_to_file_uri(path: &std::path::Path) -> String {
            // Encode path for URI
            let path_str = path.to_string_lossy();
            let encoded: String = path_str
                .bytes()
                .map(|b| {
                    // Characters that don't need encoding in file URIs
                    if b.is_ascii_alphanumeric()
                        || matches!(b, b'-' | b'_' | b'.' | b'~' | b'/' | b':')
                    {
                        (b as char).to_string()
                    } else {
                        format!("%{:02X}", b)
                    }
                })
                .collect();

            format!("file://{}", encoded)
        }

        /// Convert a file:// URI to a file path
        #[allow(dead_code)]
        pub fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
            // Handle various file URI formats:
            // - file:///path (standard)
            // - file://localhost/path
            // - file://host/path (network paths)

            if !uri.starts_with("file://") {
                return None;
            }

            let after_scheme = &uri[7..];

            // Check for localhost or empty host
            let path_part = if after_scheme.starts_with('/') {
                after_scheme
            } else if after_scheme.starts_with("localhost/") {
                &after_scheme[9..]
            } else {
                // Network path - not supported
                return None;
            };

            // Decode percent-encoding
            let decoded = Self::percent_decode(path_part)?;
            Some(PathBuf::from(decoded))
        }

        /// Decode percent-encoded URI
        #[allow(dead_code)]
        fn percent_decode(s: &str) -> Option<String> {
            let mut result = Vec::new();
            let bytes = s.as_bytes();
            let mut i = 0;

            while i < bytes.len() {
                if bytes[i] == b'%' && i + 2 < bytes.len() {
                    // Decode hex pair
                    let hex_str = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                    let byte = u8::from_str_radix(hex_str, 16).ok()?;
                    result.push(byte);
                    i += 3;
                } else {
                    result.push(bytes[i]);
                    i += 1;
                }
            }

            String::from_utf8(result).ok()
        }

        /// Parse text/uri-list format
        /// Returns file paths from the URI list
        #[allow(dead_code)]
        pub fn parse_uri_list(data: &str) -> Vec<PathBuf> {
            data.lines()
                .filter(|line| !line.starts_with('#')) // Skip comments
                .filter(|line| !line.is_empty())
                .filter_map(|uri| Self::file_uri_to_path(uri.trim()))
                .collect()
        }

        /// Serialize file paths as text/uri-list
        #[allow(dead_code)]
        pub fn serialize_uri_list(files: &[PathBuf]) -> String {
            files
                .iter()
                .map(|p| Self::path_to_file_uri(p))
                .collect::<Vec<_>>()
                .join("\r\n") // URI lists use CRLF per RFC 2483
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_list_creation() {
        let files = vec![
            PathBuf::from("/path/to/file1.txt"),
            PathBuf::from("/path/to/file2.txt"),
        ];
        let file_list = FileList::new(files);
        assert_eq!(file_list.len(), 2);
    }

    #[test]
    fn test_file_list_to_content() {
        let files = vec![PathBuf::from("/test/file.txt")];
        let file_list = FileList::new(files);
        let content = file_list.to_content();

        assert_eq!(content.content_type, crate::protocol::ContentType::File);
        let text = String::from_utf8(content.data).unwrap();
        assert_eq!(text, "/test/file.txt");
    }

    #[test]
    fn test_file_list_from_content() {
        let content = ClipboardContent::new(
            crate::protocol::ContentType::File,
            "/file1.txt\n/file2.txt".as_bytes().to_vec(),
        );

        let file_list = FileList::from_content(&content).unwrap();
        assert_eq!(file_list.len(), 2);
        assert_eq!(file_list.files[0], PathBuf::from("/file1.txt"));
        assert_eq!(file_list.files[1], PathBuf::from("/file2.txt"));
    }
}
