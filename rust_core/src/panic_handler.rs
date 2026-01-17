//! Panic handler with Windows MessageBox support
//!
//! This module provides a custom panic hook that:
//! - Writes panic information to a log file
//! - Shows a MessageBox on Windows so users see crash details
//! - Prints to stderr for console debugging

use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::sync::OnceLock;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Install the custom panic hook
///
/// This should be called early in initialization, before any other code
/// that might panic. The log directory should already exist.
pub fn install_panic_hook(log_dir: &std::path::Path) {
    LOG_DIR.set(log_dir.to_path_buf()).ok();

    panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let msg = format!(
            "Toss crashed!\n\n{}\n\nLocation: {}",
            info.payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("Unknown panic"),
            location
        );

        // Write to panic.log
        if let Some(dir) = LOG_DIR.get() {
            let log_path = dir.join("panic.log");
            if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(f, "[{}] {}", timestamp, msg.replace('\n', "\n    "));
                let _ = writeln!(f);
            }
        }

        // Show message box on Windows
        #[cfg(target_os = "windows")]
        {
            show_message_box("Toss Crashed", &msg);
        }

        // Also print to stderr
        eprintln!("PANIC: {}", msg);
    }));
}

#[cfg(target_os = "windows")]
fn show_message_box(title: &str, message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;

    let title: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
    let msg: Vec<u16> = OsStr::new(message).encode_wide().chain(Some(0)).collect();

    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            null_mut(),
            msg.as_ptr(),
            title.as_ptr(),
            0x10, // MB_ICONERROR
        );
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_log_dir_initialization() {
        // This test just verifies the module compiles correctly
        // Actual panic testing would require a separate process
        let dir = tempdir().unwrap();
        let log_dir = dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();

        // Note: We can't actually test install_panic_hook because
        // panic hooks are global and would interfere with test harness
        assert!(log_dir.exists());
    }
}
