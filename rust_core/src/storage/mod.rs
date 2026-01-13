//! Storage module for persisting paired devices and settings
//!
//! Uses SQLite for local storage with encrypted session keys.

mod device_storage;
mod history_storage;

pub use device_storage::{DeviceStorage, StoredDevice};
pub use history_storage::{HistoryStorage, StoredHistoryItem};

use std::path::Path;
use std::sync::Mutex;
use rusqlite::{Connection, Result as SqliteResult};

/// Storage manager
/// Note: rusqlite::Connection is not Sync, so we wrap operations in Mutex
/// when needed for thread-safe access
pub struct Storage {
    conn: Mutex<Connection>,
}

// Safety: We ensure all access to Connection is through the Mutex,
// which provides synchronization. The Connection itself is not Sync,
// but we only access it through locked methods.
unsafe impl Sync for Storage {}

impl Storage {
    /// Create or open storage at the given path
    pub fn new<P: AsRef<Path>>(db_path: P) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize database schema
    fn init_schema(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
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
                is_active INTEGER DEFAULT 1
            )
            "#,
            [],
        )?;

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
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let tables: Vec<String> = stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"devices".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }
}
