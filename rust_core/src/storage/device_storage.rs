//! Device storage operations

use super::secure_storage::{decrypt_from_storage, encrypt_for_storage};
use super::{timestamp_from_sql, timestamp_to_sql};
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
    pub sync_enabled: bool,       // Whether sync is enabled for this device
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

        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute(
            r#"
            INSERT OR REPLACE INTO devices
            (id, name, public_key, session_key, last_seen, created_at, is_active, platform, sync_enabled)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            rusqlite::params![
                device.id,
                device.name,
                device.public_key,
                encrypted_session_key,
                device.last_seen.map(timestamp_to_sql),
                timestamp_to_sql(device.created_at),
                device.is_active as i32,
                device.platform,
                device.sync_enabled as i32,
            ],
        )?;
        Ok(())
    }

    /// Get a device by ID
    /// Session keys are decrypted after retrieval
    pub fn get_device(&self, device_id: &str) -> SqliteResult<Option<StoredDevice>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let mut stmt = conn.prepare(
            "SELECT id, name, public_key, session_key, last_seen, created_at, is_active, platform, sync_enabled FROM devices WHERE id = ?1"
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
                last_seen: row.get::<_, Option<i64>>(4)?.map(timestamp_from_sql),
                created_at: timestamp_from_sql(row.get(5)?),
                is_active: row.get::<_, i32>(6)? != 0,
                platform: row.get(7).ok(), // Platform is optional, may not exist in old databases
                sync_enabled: row.get::<_, Option<i32>>(8).ok().flatten().unwrap_or(1) != 0, // Default to true for old databases
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
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let mut stmt = conn.prepare(
            "SELECT id, name, public_key, session_key, last_seen, created_at, is_active, platform, sync_enabled FROM devices WHERE is_active = 1 ORDER BY created_at DESC"
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
                    last_seen: row.get::<_, Option<i64>>(4)?.map(timestamp_from_sql),
                    created_at: timestamp_from_sql(row.get(5)?),
                    is_active: row.get::<_, i32>(6)? != 0,
                    platform: row.get(7).ok(), // Platform is optional, may not exist in old databases
                    sync_enabled: row.get::<_, Option<i32>>(8).ok().flatten().unwrap_or(1) != 0, // Default to true for old databases
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(devices)
    }

    /// Update device last seen timestamp
    pub fn update_last_seen(&self, device_id: &str) -> SqliteResult<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_else(|e| {
                tracing::error!("System time before UNIX_EPOCH: {}", e);
                0
            });

        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute(
            "UPDATE devices SET last_seen = ?1 WHERE id = ?2",
            rusqlite::params![timestamp_to_sql(now), device_id],
        )?;
        Ok(())
    }

    /// Remove a device (mark as inactive)
    pub fn remove_device(&self, device_id: &str) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute(
            "UPDATE devices SET is_active = 0 WHERE id = ?1",
            [device_id],
        )?;
        Ok(())
    }

    /// Permanently delete a device
    pub fn delete_device(&self, device_id: &str) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute("DELETE FROM devices WHERE id = ?1", [device_id])?;
        Ok(())
    }

    /// Update device name
    pub fn update_device_name(&self, device_id: &str, new_name: &str) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute(
            "UPDATE devices SET name = ?1 WHERE id = ?2",
            rusqlite::params![new_name, device_id],
        )?;
        Ok(())
    }

    /// Update device sync enabled setting
    pub fn set_device_sync_enabled(&self, device_id: &str, enabled: bool) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute(
            "UPDATE devices SET sync_enabled = ?1 WHERE id = ?2",
            rusqlite::params![enabled as i32, device_id],
        )?;
        Ok(())
    }

    /// Get device sync enabled setting
    pub fn get_device_sync_enabled(&self, device_id: &str) -> SqliteResult<bool> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let result: Option<i32> = conn.query_row(
            "SELECT sync_enabled FROM devices WHERE id = ?1 AND is_active = 1",
            [device_id],
            |row| row.get(0),
        )?;
        // Default to true if column doesn't exist (migration case) or null
        Ok(result.unwrap_or(1) != 0)
    }

    /// Get all devices with sync enabled
    pub fn get_sync_enabled_device_ids(&self) -> SqliteResult<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let mut stmt = conn.prepare(
            "SELECT id FROM devices WHERE is_active = 1 AND (sync_enabled = 1 OR sync_enabled IS NULL)"
        )?;

        let device_ids = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;

        Ok(device_ids)
    }

    /// Enable sync for all devices
    pub fn enable_sync_all_devices(&self) -> SqliteResult<usize> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let count = conn.execute(
            "UPDATE devices SET sync_enabled = 1 WHERE is_active = 1",
            [],
        )?;
        Ok(count)
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
            sync_enabled: true,
        };

        device_storage.store_device(&device).unwrap();
        let retrieved = device_storage.get_device("test-device-1").unwrap();

        assert!(retrieved.is_some());
        let d = retrieved.unwrap();
        assert_eq!(d.id, device.id);
        assert_eq!(d.name, device.name);
        assert_eq!(d.public_key, device.public_key);
        assert!(d.sync_enabled);
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
            sync_enabled: true,
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
            sync_enabled: true,
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
            sync_enabled: true,
        };

        device_storage.store_device(&device).unwrap();
        device_storage.remove_device("device-to-remove").unwrap();

        let all = device_storage.get_all_devices().unwrap();
        assert_eq!(all.len(), 0); // Should be filtered out
    }

    #[test]
    fn test_device_sync_enabled() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let device_storage = storage.devices();

        let device = StoredDevice {
            id: "sync-device".to_string(),
            name: "Sync Device".to_string(),
            public_key: vec![1],
            session_key: None,
            last_seen: None,
            created_at: 1000,
            is_active: true,
            platform: None,
            sync_enabled: true,
        };

        device_storage.store_device(&device).unwrap();

        // Should be enabled by default
        assert!(device_storage
            .get_device_sync_enabled("sync-device")
            .unwrap());

        // Disable sync
        device_storage
            .set_device_sync_enabled("sync-device", false)
            .unwrap();
        assert!(!device_storage
            .get_device_sync_enabled("sync-device")
            .unwrap());

        // Re-enable sync
        device_storage
            .set_device_sync_enabled("sync-device", true)
            .unwrap();
        assert!(device_storage
            .get_device_sync_enabled("sync-device")
            .unwrap());
    }

    #[test]
    fn test_get_sync_enabled_device_ids() {
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
            sync_enabled: true,
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
            sync_enabled: false,
        };

        let device3 = StoredDevice {
            id: "device-3".to_string(),
            name: "Device 3".to_string(),
            public_key: vec![3],
            session_key: None,
            last_seen: None,
            created_at: 3000,
            is_active: true,
            platform: None,
            sync_enabled: true,
        };

        device_storage.store_device(&device1).unwrap();
        device_storage.store_device(&device2).unwrap();
        device_storage.store_device(&device3).unwrap();

        let sync_enabled_ids = device_storage.get_sync_enabled_device_ids().unwrap();
        assert_eq!(sync_enabled_ids.len(), 2);
        assert!(sync_enabled_ids.contains(&"device-1".to_string()));
        assert!(!sync_enabled_ids.contains(&"device-2".to_string()));
        assert!(sync_enabled_ids.contains(&"device-3".to_string()));
    }

    #[test]
    fn test_enable_sync_all_devices() {
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
            sync_enabled: false,
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
            sync_enabled: false,
        };

        device_storage.store_device(&device1).unwrap();
        device_storage.store_device(&device2).unwrap();

        // Initially both disabled
        assert_eq!(
            device_storage.get_sync_enabled_device_ids().unwrap().len(),
            0
        );

        // Enable all
        let count = device_storage.enable_sync_all_devices().unwrap();
        assert_eq!(count, 2);

        // Both should now be enabled
        assert_eq!(
            device_storage.get_sync_enabled_device_ids().unwrap().len(),
            2
        );
    }
}
