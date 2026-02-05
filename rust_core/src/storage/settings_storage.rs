//! Settings storage operations

use rusqlite::Result as SqliteResult;
use std::sync::Mutex;

/// Settings storage operations
pub struct SettingsStorage<'conn> {
    conn: &'conn Mutex<rusqlite::Connection>,
}

impl<'conn> SettingsStorage<'conn> {
    pub fn new(conn: &'conn Mutex<rusqlite::Connection>) -> Self {
        Self { conn }
    }

    /// Save a setting key-value pair
    pub fn save_setting(&self, key: &str, value: &str) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute(
            r#"
            INSERT OR REPLACE INTO settings (key, value)
            VALUES (?1, ?2)
            "#,
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// Load a setting by key
    pub fn load_setting(&self, key: &str) -> SqliteResult<Option<String>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;

        let result = stmt.query_row([key], |row| row.get(0));

        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Load all settings as key-value pairs
    pub fn load_all_settings(&self) -> SqliteResult<Vec<(String, String)>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;

        let settings = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(settings)
    }

    /// Delete a setting by key
    pub fn delete_setting(&self, key: &str) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }

    /// Clear all settings
    pub fn clear_settings(&self) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute("DELETE FROM settings", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::Storage;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_setting() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let settings_storage = storage.settings();

        settings_storage
            .save_setting("test_key", "test_value")
            .unwrap();
        let loaded = settings_storage.load_setting("test_key").unwrap();

        assert_eq!(loaded, Some("test_value".to_string()));
    }

    #[test]
    fn test_load_nonexistent_setting() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let settings_storage = storage.settings();

        let loaded = settings_storage.load_setting("nonexistent").unwrap();
        assert_eq!(loaded, None);
    }

    #[test]
    fn test_update_setting() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let settings_storage = storage.settings();

        settings_storage.save_setting("key", "value1").unwrap();
        settings_storage.save_setting("key", "value2").unwrap();
        let loaded = settings_storage.load_setting("key").unwrap();

        assert_eq!(loaded, Some("value2".to_string()));
    }

    #[test]
    fn test_load_all_settings() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let settings_storage = storage.settings();

        settings_storage.save_setting("key1", "value1").unwrap();
        settings_storage.save_setting("key2", "value2").unwrap();

        let all = settings_storage.load_all_settings().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_delete_setting() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let settings_storage = storage.settings();

        settings_storage.save_setting("to_delete", "value").unwrap();
        settings_storage.delete_setting("to_delete").unwrap();
        let loaded = settings_storage.load_setting("to_delete").unwrap();

        assert_eq!(loaded, None);
    }

    #[test]
    fn test_clear_settings() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let settings_storage = storage.settings();

        settings_storage.save_setting("key1", "value1").unwrap();
        settings_storage.save_setting("key2", "value2").unwrap();
        settings_storage.clear_settings().unwrap();

        let all = settings_storage.load_all_settings().unwrap();
        assert!(all.is_empty());
    }
}
