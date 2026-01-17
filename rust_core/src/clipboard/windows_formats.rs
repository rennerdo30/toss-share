//! Windows clipboard format handling
//!
//! Windows supports multiple clipboard formats that need special handling:
//! - CF_TEXT: ANSI text
//! - CF_UNICODETEXT: Unicode text
//! - CF_HDROP: File list (HDROP)
//! - CF_DIB: Device-independent bitmap
//! - CF_BITMAP: Bitmap handle
//!
//! This module provides utilities for handling these formats.

use crate::error::ClipboardError;
use std::path::PathBuf;

/// Windows clipboard format identifiers
#[cfg(target_os = "windows")]
pub mod formats {
    pub const CF_TEXT: u32 = 1;
    pub const CF_BITMAP: u32 = 2;
    pub const CF_METAFILEPICT: u32 = 3;
    pub const CF_SYLK: u32 = 4;
    pub const CF_DIF: u32 = 5;
    pub const CF_TIFF: u32 = 6;
    pub const CF_OEMTEXT: u32 = 7;
    pub const CF_DIB: u32 = 8;
    pub const CF_PALETTE: u32 = 9;
    pub const CF_PENDATA: u32 = 10;
    pub const CF_RIFF: u32 = 11;
    pub const CF_WAVE: u32 = 12;
    pub const CF_UNICODETEXT: u32 = 13;
    pub const CF_ENHMETAFILE: u32 = 14;
    pub const CF_HDROP: u32 = 15;
    pub const CF_LOCALE: u32 = 16;
    pub const CF_DIBV5: u32 = 17;
    pub const CF_MAX: u32 = 18;
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;
    use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        SetClipboardData,
    };
    use windows::Win32::System::Memory::{
        GlobalAlloc, GlobalFree, GlobalLock, GlobalSize, GlobalUnlock, GHND,
    };
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    /// RAII guard for clipboard access
    struct ClipboardGuard;

    impl ClipboardGuard {
        /// Open the clipboard for the current thread
        fn open() -> Result<Self, ClipboardError> {
            unsafe {
                if OpenClipboard(HWND::default()).is_ok() {
                    Ok(ClipboardGuard)
                } else {
                    Err(ClipboardError::OperationFailed(
                        "Failed to open clipboard".to_string(),
                    ))
                }
            }
        }
    }

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseClipboard();
            }
        }
    }

    /// Check if a clipboard format is available
    pub fn is_format_available(format: u32) -> bool {
        unsafe { IsClipboardFormatAvailable(format).is_ok() }
    }

    /// Read Unicode text (CF_UNICODETEXT) from clipboard
    pub fn read_unicode_text() -> Result<Option<String>, ClipboardError> {
        let _guard = ClipboardGuard::open()?;

        unsafe {
            if !IsClipboardFormatAvailable(super::formats::CF_UNICODETEXT).is_ok() {
                return Ok(None);
            }

            let handle = GetClipboardData(super::formats::CF_UNICODETEXT);
            if handle.is_err() {
                return Ok(None);
            }

            let handle = handle.unwrap();
            if handle.is_invalid() {
                return Ok(None);
            }

            let hglobal = HGLOBAL(handle.0);
            let data_ptr = GlobalLock(hglobal) as *const u16;
            if data_ptr.is_null() {
                return Ok(None);
            }

            // Find the null terminator
            let mut len = 0;
            while *data_ptr.add(len) != 0 {
                len += 1;
            }

            let slice = std::slice::from_raw_parts(data_ptr, len);
            let text = OsString::from_wide(slice).to_string_lossy().into_owned();

            let _ = GlobalUnlock(hglobal);

            Ok(Some(text))
        }
    }

    /// Read file list (CF_HDROP) from clipboard
    pub fn read_file_list() -> Result<Option<Vec<PathBuf>>, ClipboardError> {
        let _guard = ClipboardGuard::open()?;

        unsafe {
            if !IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_ok() {
                return Ok(None);
            }

            let handle = GetClipboardData(CF_HDROP.0 as u32);
            if handle.is_err() {
                return Ok(None);
            }

            let handle = handle.unwrap();
            if handle.is_invalid() {
                return Ok(None);
            }

            let hdrop = HDROP(handle.0 as *mut _);

            // Get the number of files
            let file_count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
            if file_count == 0 {
                return Ok(None);
            }

            let mut files = Vec::with_capacity(file_count as usize);

            for i in 0..file_count {
                // Get the required buffer size
                let len = DragQueryFileW(hdrop, i, None);
                if len == 0 {
                    continue;
                }

                // Allocate buffer and get the file path
                let mut buffer: Vec<u16> = vec![0; (len + 1) as usize];
                DragQueryFileW(hdrop, i, Some(&mut buffer));

                // Convert to PathBuf
                let path_str = OsString::from_wide(&buffer[..len as usize]);
                files.push(PathBuf::from(path_str));
            }

            Ok(Some(files))
        }
    }

    /// Write file list (CF_HDROP) to clipboard
    pub fn write_file_list(files: &[PathBuf]) -> Result<(), ClipboardError> {
        if files.is_empty() {
            return Ok(());
        }

        // Build DROPFILES structure
        let hdrop_data = build_dropfiles_structure(files);

        let _guard = ClipboardGuard::open()?;

        unsafe {
            // Allocate global memory for the DROPFILES structure
            let mem_handle = GlobalAlloc(GHND, hdrop_data.len());
            if mem_handle.is_err() {
                return Err(ClipboardError::OperationFailed(
                    "Failed to allocate clipboard memory".to_string(),
                ));
            }

            let mem_handle = mem_handle.unwrap();
            let mem_ptr = GlobalLock(mem_handle);
            if mem_ptr.is_null() {
                let _ = GlobalFree(mem_handle);
                return Err(ClipboardError::OperationFailed(
                    "Failed to lock clipboard memory".to_string(),
                ));
            }

            // Copy data to global memory
            ptr::copy_nonoverlapping(hdrop_data.as_ptr(), mem_ptr as *mut u8, hdrop_data.len());
            let _ = GlobalUnlock(mem_handle);

            // Set clipboard data
            let result = SetClipboardData(CF_HDROP.0 as u32, HANDLE(mem_handle.0));
            if result.is_err() {
                GlobalFree(mem_handle);
                return Err(ClipboardError::OperationFailed(
                    "Failed to set clipboard data".to_string(),
                ));
            }

            // Don't free the memory - the clipboard now owns it
            Ok(())
        }
    }

    /// Build a DROPFILES structure from file paths
    fn build_dropfiles_structure(files: &[PathBuf]) -> Vec<u8> {
        let mut data = Vec::new();

        // DROPFILES header (20 bytes on 32-bit, but we use 20 for compatibility)
        // pFiles: offset to file list
        data.extend_from_slice(&20u32.to_le_bytes());
        // pt.x (LONG)
        data.extend_from_slice(&0i32.to_le_bytes());
        // pt.y (LONG)
        data.extend_from_slice(&0i32.to_le_bytes());
        // fNC (BOOL)
        data.extend_from_slice(&0u32.to_le_bytes());
        // fWide (BOOL) - use Unicode
        data.extend_from_slice(&1u32.to_le_bytes());

        // File paths as null-terminated UTF-16LE strings
        for path in files {
            let path_str = path.to_string_lossy();
            for c in path_str.encode_utf16() {
                data.extend_from_slice(&c.to_le_bytes());
            }
            // Null terminator
            data.extend_from_slice(&0u16.to_le_bytes());
        }

        // Double null terminator to end list
        data.extend_from_slice(&0u16.to_le_bytes());

        data
    }

    /// Read DIB (device-independent bitmap) from clipboard
    pub fn read_dib() -> Result<Option<Vec<u8>>, ClipboardError> {
        let _guard = ClipboardGuard::open()?;

        unsafe {
            if !IsClipboardFormatAvailable(super::formats::CF_DIB).is_ok() {
                return Ok(None);
            }

            let handle = GetClipboardData(super::formats::CF_DIB);
            if handle.is_err() {
                return Ok(None);
            }

            let handle = handle.unwrap();
            if handle.is_invalid() {
                return Ok(None);
            }

            let hglobal = HGLOBAL(handle.0);
            let size = GlobalSize(hglobal);
            if size == 0 {
                return Ok(None);
            }

            let data_ptr = GlobalLock(hglobal) as *const u8;
            if data_ptr.is_null() {
                return Ok(None);
            }

            let data = std::slice::from_raw_parts(data_ptr, size).to_vec();
            let _ = GlobalUnlock(hglobal);

            Ok(Some(data))
        }
    }

    /// Get available clipboard formats
    pub fn get_available_formats() -> Vec<u32> {
        let mut formats = Vec::new();

        // Check common formats
        let check_formats = [
            super::formats::CF_TEXT,
            super::formats::CF_UNICODETEXT,
            super::formats::CF_DIB,
            super::formats::CF_BITMAP,
            super::formats::CF_HDROP,
        ];

        for &format in &check_formats {
            if is_format_available(format) {
                formats.push(format);
            }
        }

        formats
    }
}

/// Read text from Windows clipboard, preferring Unicode over ANSI
#[cfg(target_os = "windows")]
pub fn read_text() -> Result<Option<String>, ClipboardError> {
    windows_impl::read_unicode_text()
}

/// Read file list from Windows clipboard (CF_HDROP format)
#[cfg(target_os = "windows")]
pub fn read_files() -> Result<Option<Vec<PathBuf>>, ClipboardError> {
    windows_impl::read_file_list()
}

/// Write file list to Windows clipboard (CF_HDROP format)
#[cfg(target_os = "windows")]
pub fn write_files(files: &[PathBuf]) -> Result<(), ClipboardError> {
    windows_impl::write_file_list(files)
}

/// Read DIB image from Windows clipboard
#[cfg(target_os = "windows")]
pub fn read_dib_image() -> Result<Option<Vec<u8>>, ClipboardError> {
    windows_impl::read_dib()
}

/// Get available clipboard formats
#[cfg(target_os = "windows")]
pub fn get_available_formats() -> Vec<u32> {
    windows_impl::get_available_formats()
}

/// Check if a specific format is available
#[cfg(target_os = "windows")]
pub fn is_format_available(format: u32) -> bool {
    windows_impl::is_format_available(format)
}

// Stub implementations for non-Windows platforms
#[cfg(not(target_os = "windows"))]
pub fn read_text() -> Result<Option<String>, ClipboardError> {
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
pub fn read_files() -> Result<Option<Vec<PathBuf>>, ClipboardError> {
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
pub fn write_files(_files: &[PathBuf]) -> Result<(), ClipboardError> {
    Err(ClipboardError::UnsupportedFormat(
        "CF_HDROP only available on Windows".to_string(),
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn read_dib_image() -> Result<Option<Vec<u8>>, ClipboardError> {
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
pub fn get_available_formats() -> Vec<u32> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
pub fn is_format_available(_format: u32) -> bool {
    false
}

/// Test Windows clipboard format conversion
#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_constants() {
        assert_eq!(formats::CF_TEXT, 1);
        assert_eq!(formats::CF_UNICODETEXT, 13);
        assert_eq!(formats::CF_HDROP, 15);
        assert_eq!(formats::CF_DIB, 8);
    }

    #[test]
    #[ignore = "Requires Windows clipboard access"]
    fn test_get_available_formats() {
        let formats = get_available_formats();
        // Just verify it doesn't crash
        println!("Available formats: {:?}", formats);
    }

    #[test]
    fn test_dropfiles_structure() {
        let files = vec![
            PathBuf::from("C:\\test\\file1.txt"),
            PathBuf::from("C:\\test\\file2.txt"),
        ];
        let data = windows_impl::build_dropfiles_structure(&files);

        // Verify header
        assert_eq!(u32::from_le_bytes([data[0], data[1], data[2], data[3]]), 20); // pFiles offset
        assert_eq!(
            u32::from_le_bytes([data[16], data[17], data[18], data[19]]),
            1
        ); // fWide = true
    }
}
