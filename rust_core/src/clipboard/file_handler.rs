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
mod windows_impl {
    // TODO: Implement Windows CF_HDROP handling
    // TODO: Implement Windows CF_HDROP handling
    // This requires:
    // 1. Using Windows API to read/write CF_HDROP format
    // 2. Parsing HDROP structure
    // 3. Converting to/from file paths
}

#[cfg(target_os = "macos")]
mod macos_impl {
    // TODO: Implement macOS file URL handling
    // This requires:
    // 1. Using NSPasteboard with file URL types
    // 2. Converting between file URLs and paths
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    // TODO: Implement Linux file URI list handling
    // This requires:
    // 1. Using X11/Wayland clipboard with text/uri-list MIME type
    // 2. Parsing URI lists
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
