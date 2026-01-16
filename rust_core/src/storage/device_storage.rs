//! Device storage operations

use super::secure_storage::{decrypt_from_storage, encrypt_for_storage};
use rusqlite::Result as SqliteResult;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Stored device information
#[derive(Debug, Clone)]
pub struct StoredDevice {
    pub id: String,
    pub name: String,
    pub public_key: Vec<u8>,
    pub session_key: Option<Vec<u8>>,
    pub last_seen: Option<u64>,
    pub created_at: u64,
    pub is_active: bool,
    pub platform: Option<String>, // Platform: "macos", "windows", "linux", "ios", "android", "unknown"
}

/// Device storage operations
pub struct DeviceStorage<'conn> {
    conn: &'conn Mutex<rusqlite::Connection>,
}

impl<'conn> DeviceStorage<'conn> {
    pub fn new(conn: &'conn Mutex<rusqlite::Connection>) -> Self {
        Self { conn }
    }

    /// Store a paired device
    /// Session keys are encrypted before storage for security
    pub fn store_device(&self, device: &StoredDevice) -> SqliteResult<()> {
        // Encrypt session key if present
        let encrypted_session_key = device
            .session_key
            .as_ref()
            .and_then(|key| encrypt_for_storage(key).ok());

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO devices
            (id, name, public_key, session_key, last_seen, created_at, is_active, platform)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            rusqlite::params![
                device.id,
                device.name,
                device.public_key,
                encrypted_session_key,
                device.last_seen,
                device.created_at,
                device.is_active as i32,
                device.platform,
            ],
        )?;
        Ok(())
    }

    /// Get a device by ID
    /// Session keys are decrypted after retrieval
    pub fn get_device(&self, device_id: &str) -> SqliteResult<Option<StoredDevice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, public_key, session_key, last_seen, created_at, is_active, platform FROM devices WHERE id = ?1"
        )?;

        let device = stmt.query_row([device_id], |row| {
            let encrypted_session_key: Option<Vec<u8>> = row.get(3)?;
            // Decrypt session key if present
            let session_key =
                encrypted_session_key.and_then(|encrypted| decrypt_from_storage(&encrypted).ok());

            Ok(StoredDevice {
                id: row.get(0)?,
                name: row.get(1)?,
                public_key: row.get(2)?,
                session_key,
                last_seen: row.get(4)?,
                created_at: row.get(5)?,
                is_active: row.get::<_, i32>(6)? != 0,
                platform: row.get(7).ok(), // Platform is optional, may not exist in old databases
            })
        });

        match device {
            Ok(d) => Ok(Some(d)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all active devices
    /// Session keys are decrypted after retrieval
    pub fn get_all_devices(&self) -> SqliteResult<Vec<StoredDevice>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, public_key, session_key, last_seen, created_at, is_active, platform FROM devices WHERE is_active = 1 ORDER BY created_at DESC"
        )?;

        let devices = stmt
            .query_map([], |row| {
                let encrypted_session_key: Option<Vec<u8>> = row.get(3)?;
                // Decrypt session key if present
                let session_key = encrypted_session_key
                    .and_then(|encrypted| decrypt_from_storage(&encrypted).ok());

                Ok(StoredDevice {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    public_key: row.get(2)?,
                    session_key,
                    last_seen: row.get(4)?,
                    created_at: row.get(5)?,
                    is_active: row.get::<_, i32>(6)? != 0,
                    platform: row.get(7).ok(), // Platform is optional, may not exist in old databases
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(devices)
    }

    /// Update device last seen timestamp
    pub fn update_last_seen(&self, device_id: &str) -> SqliteResult<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE devices SET last_seen = ?1 WHERE id = ?2",
            rusqlite::params![now, device_id],
        )?;
        Ok(())
    }

    /// Remove a device (mark as inactive)
    pub fn remove_device(&self, device_id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE devices SET is_active = 0 WHERE id = ?1",
            [device_id],
        )?;
        Ok(())
    }

    /// Permanently delete a device
    pub fn delete_device(&self, device_id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM devices WHERE id = ?1", [device_id])?;
        Ok(())
    }

    /// Update device name
    pub fn update_device_name(&self, device_id: &str, new_name: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE devices SET name = ?1 WHERE id = ?2",
            rusqlite::params![new_name, device_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use tempfile::TempDir;

    #[test]
    fn test_store_and_retrieve_device() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let device_storage = storage.devices();

        let device = StoredDevice {
            id: "test-device-1".to_string(),
            name: "Test Device".to_string(),
            public_key: vec![1, 2, 3, 4],
            session_key: Some(vec![5, 6, 7, 8]),
            last_seen: None,
            created_at: 1234567890,
            is_active: true,
            platform: None,
        };

        device_storage.store_device(&device).unwrap();
        let retrieved = device_storage.get_device("test-device-1").unwrap();

        assert!(retrieved.is_some());
        let d = retrieved.unwrap();
        assert_eq!(d.id, device.id);
        assert_eq!(d.name, device.name);
        assert_eq!(d.public_key, device.public_key);
    }

    #[test]
    fn test_get_all_devices() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let device_storage = storage.devices();

        let device1 = StoredDevice {
            id: "device-1".to_string(),
            name: "Device 1".to_string(),
            public_key: vec![1],
            session_key: None,
            last_seen: None,
            created_at: 1000,
            is_active: true,
            platform: None,
        };

        let device2 = StoredDevice {
            id: "device-2".to_string(),
            name: "Device 2".to_string(),
            public_key: vec![2],
            session_key: None,
            last_seen: None,
            created_at: 2000,
            is_active: true,
            platform: None,
        };

        device_storage.store_device(&device1).unwrap();
        device_storage.store_device(&device2).unwrap();

        let all = device_storage.get_all_devices().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_remove_device() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let device_storage = storage.devices();

        let device = StoredDevice {
            id: "device-to-remove".to_string(),
            name: "To Remove".to_string(),
            public_key: vec![1],
            session_key: None,
            last_seen: None,
            created_at: 1000,
            is_active: true,
            platform: None,
        };

        device_storage.store_device(&device).unwrap();
        device_storage.remove_device("device-to-remove").unwrap();

        let all = device_storage.get_all_devices().unwrap();
        assert_eq!(all.len(), 0); // Should be filtered out
    }
}
