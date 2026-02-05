//! Storage module for persisting paired devices and settings
//!
//! Uses SQLite for local storage with encrypted session keys.

mod device_storage;
mod history_storage;
mod secure_storage;
mod settings_storage;

pub use device_storage::{DeviceStorage, StoredDevice};
pub use history_storage::{HistoryStorage, StoredHistoryItem};
pub use secure_storage::{
    decrypt_from_storage, delete_identity_key, encrypt_for_storage,
    get_or_create_storage_encryption_key, retrieve_identity_key, store_identity_key,
};
pub use settings_storage::SettingsStorage;

use rusqlite::{Connection, Result as SqliteResult};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Storage manager
/// Note: rusqlite::Connection is not Sync, so we wrap operations in Mutex
/// when needed for thread-safe access
pub struct Storage {
    conn: Mutex<Connection>,
    db_path: PathBuf,
}

// Safety: We ensure all access to Connection is through the Mutex,
// which provides synchronization. The Connection itself is not Sync,
// but we only access it through locked methods.
unsafe impl Sync for Storage {}

impl Storage {
    /// Create or open storage at the given path
    pub fn new<P: AsRef<Path>>(db_path: P) -> SqliteResult<Self> {
        let path = db_path.as_ref().to_path_buf();
        let conn = Connection::open(&path)?;
        let storage = Self {
            conn: Mutex::new(conn),
            db_path: path,
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Get the database path
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Initialize database schema
    fn init_schema(&self) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        // Create devices table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                public_key BLOB NOT NULL,
                session_key BLOB,
                last_seen INTEGER,
                created_at INTEGER NOT NULL,
                is_active INTEGER DEFAULT 1,
                platform TEXT
            )
            "#,
            [],
        )?;

        // Add platform column if it doesn't exist (migration for existing databases)
        let _ = conn.execute("ALTER TABLE devices ADD COLUMN platform TEXT", []);

        // Create clipboard history table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS clipboard_history (
                id TEXT PRIMARY KEY,
                content_type INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                encrypted_content BLOB,
                preview TEXT,
                source_device TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (source_device) REFERENCES devices(id)
            )
            "#,
            [],
        )?;

        // Create index on created_at for efficient pruning
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_clipboard_history_created_at
            ON clipboard_history(created_at DESC)
            "#,
            [],
        )?;

        // Create settings table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
            "#,
            [],
        )?;

        Ok(())
    }

    /// Get device storage operations
    pub fn devices(&self) -> DeviceStorage<'_> {
        DeviceStorage::new(&self.conn)
    }

    /// Get history storage operations
    pub fn history(&self) -> HistoryStorage<'_> {
        HistoryStorage::new(&self.conn)
    }

    /// Get settings storage operations
    pub fn settings(&self) -> SettingsStorage<'_> {
        SettingsStorage::new(&self.conn)
    }

    /// Save TossSettings to the database
    /// Settings are stored as individual key-value pairs for flexibility
    pub fn save_settings(&self, settings: &crate::api::TossSettings) -> SqliteResult<()> {
        let settings_storage = self.settings();

        settings_storage.save_setting("auto_sync", &settings.auto_sync.to_string())?;
        settings_storage.save_setting("sync_text", &settings.sync_text.to_string())?;
        settings_storage.save_setting("sync_rich_text", &settings.sync_rich_text.to_string())?;
        settings_storage.save_setting("sync_images", &settings.sync_images.to_string())?;
        settings_storage.save_setting("sync_files", &settings.sync_files.to_string())?;
        settings_storage
            .save_setting("max_file_size_mb", &settings.max_file_size_mb.to_string())?;
        settings_storage.save_setting("history_enabled", &settings.history_enabled.to_string())?;
        settings_storage.save_setting("history_days", &settings.history_days.to_string())?;

        // Handle optional relay_url
        if let Some(ref url) = settings.relay_url {
            settings_storage.save_setting("relay_url", url)?;
        } else {
            settings_storage.delete_setting("relay_url")?;
        }

        Ok(())
    }

    /// Load TossSettings from the database
    /// Returns default settings if no settings are stored
    pub fn load_settings(&self) -> SqliteResult<crate::api::TossSettings> {
        let settings_storage = self.settings();
        let defaults = crate::api::TossSettings::default();

        let auto_sync = settings_storage
            .load_setting("auto_sync")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.auto_sync);

        let sync_text = settings_storage
            .load_setting("sync_text")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.sync_text);

        let sync_rich_text = settings_storage
            .load_setting("sync_rich_text")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.sync_rich_text);

        let sync_images = settings_storage
            .load_setting("sync_images")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.sync_images);

        let sync_files = settings_storage
            .load_setting("sync_files")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.sync_files);

        let max_file_size_mb = settings_storage
            .load_setting("max_file_size_mb")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.max_file_size_mb);

        let history_enabled = settings_storage
            .load_setting("history_enabled")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.history_enabled);

        let history_days = settings_storage
            .load_setting("history_days")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(defaults.history_days);

        let relay_url = settings_storage.load_setting("relay_url")?;

        Ok(crate::api::TossSettings {
            auto_sync,
            sync_text,
            sync_rich_text,
            sync_images,
            sync_files,
            max_file_size_mb,
            history_enabled,
            history_days,
            relay_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::TossSettings;
    use tempfile::TempDir;

    #[test]
    fn test_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path);
        assert!(storage.is_ok());
    }

    #[test]
    fn test_schema_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        // Verify tables exist
        let conn = storage.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('devices', 'settings')"
        ).unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"devices".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }

    #[test]
    fn test_save_and_load_settings() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        let settings = TossSettings {
            auto_sync: false,
            sync_text: true,
            sync_rich_text: false,
            sync_images: true,
            sync_files: false,
            max_file_size_mb: 100,
            history_enabled: false,
            history_days: 14,
            relay_url: Some("https://relay.example.com".to_string()),
        };

        storage.save_settings(&settings).unwrap();
        let loaded = storage.load_settings().unwrap();

        assert_eq!(loaded.auto_sync, settings.auto_sync);
        assert_eq!(loaded.sync_text, settings.sync_text);
        assert_eq!(loaded.sync_rich_text, settings.sync_rich_text);
        assert_eq!(loaded.sync_images, settings.sync_images);
        assert_eq!(loaded.sync_files, settings.sync_files);
        assert_eq!(loaded.max_file_size_mb, settings.max_file_size_mb);
        assert_eq!(loaded.history_enabled, settings.history_enabled);
        assert_eq!(loaded.history_days, settings.history_days);
        assert_eq!(loaded.relay_url, settings.relay_url);
    }

    #[test]
    fn test_load_settings_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        // Load settings without saving - should return defaults
        let loaded = storage.load_settings().unwrap();
        let defaults = TossSettings::default();

        assert_eq!(loaded.auto_sync, defaults.auto_sync);
        assert_eq!(loaded.sync_text, defaults.sync_text);
        assert_eq!(loaded.sync_rich_text, defaults.sync_rich_text);
        assert_eq!(loaded.sync_images, defaults.sync_images);
        assert_eq!(loaded.sync_files, defaults.sync_files);
        assert_eq!(loaded.max_file_size_mb, defaults.max_file_size_mb);
        assert_eq!(loaded.history_enabled, defaults.history_enabled);
        assert_eq!(loaded.history_days, defaults.history_days);
        assert_eq!(loaded.relay_url, defaults.relay_url);
    }

    #[test]
    fn test_update_settings() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        // Save initial settings
        let settings1 = TossSettings {
            auto_sync: true,
            sync_text: true,
            sync_rich_text: true,
            sync_images: true,
            sync_files: true,
            max_file_size_mb: 50,
            history_enabled: true,
            history_days: 7,
            relay_url: None,
        };
        storage.save_settings(&settings1).unwrap();

        // Save updated settings
        let settings2 = TossSettings {
            auto_sync: false,
            sync_text: false,
            sync_rich_text: false,
            sync_images: false,
            sync_files: false,
            max_file_size_mb: 100,
            history_enabled: false,
            history_days: 30,
            relay_url: Some("https://relay.test.com".to_string()),
        };
        storage.save_settings(&settings2).unwrap();

        let loaded = storage.load_settings().unwrap();

        assert_eq!(loaded.auto_sync, false);
        assert_eq!(loaded.max_file_size_mb, 100);
        assert_eq!(loaded.relay_url, Some("https://relay.test.com".to_string()));
    }

    #[test]
    fn test_settings_persist_across_storage_reopens() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Save settings with first storage instance
        {
            let storage = Storage::new(&db_path).unwrap();
            let settings = TossSettings {
                auto_sync: false,
                sync_text: true,
                sync_rich_text: false,
                sync_images: true,
                sync_files: false,
                max_file_size_mb: 75,
                history_enabled: true,
                history_days: 21,
                relay_url: Some("https://persistent.example.com".to_string()),
            };
            storage.save_settings(&settings).unwrap();
        }

        // Open new storage instance and verify settings persisted
        {
            let storage = Storage::new(&db_path).unwrap();
            let loaded = storage.load_settings().unwrap();

            assert_eq!(loaded.auto_sync, false);
            assert_eq!(loaded.sync_text, true);
            assert_eq!(loaded.sync_rich_text, false);
            assert_eq!(loaded.sync_images, true);
            assert_eq!(loaded.sync_files, false);
            assert_eq!(loaded.max_file_size_mb, 75);
            assert_eq!(loaded.history_enabled, true);
            assert_eq!(loaded.history_days, 21);
            assert_eq!(
                loaded.relay_url,
                Some("https://persistent.example.com".to_string())
            );
        }
    }

    #[test]
    fn test_relay_url_removal() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        // Save settings with relay_url
        let settings1 = TossSettings {
            relay_url: Some("https://relay.example.com".to_string()),
            ..TossSettings::default()
        };
        storage.save_settings(&settings1).unwrap();

        // Verify relay_url is saved
        let loaded = storage.load_settings().unwrap();
        assert_eq!(
            loaded.relay_url,
            Some("https://relay.example.com".to_string())
        );

        // Save settings without relay_url (should remove it)
        let settings2 = TossSettings {
            relay_url: None,
            ..TossSettings::default()
        };
        storage.save_settings(&settings2).unwrap();

        // Verify relay_url is removed
        let loaded = storage.load_settings().unwrap();
        assert_eq!(loaded.relay_url, None);
    }
}
