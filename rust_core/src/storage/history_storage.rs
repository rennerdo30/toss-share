//! Clipboard history storage operations

use rusqlite::Result as SqliteResult;
use std::sync::Mutex;

/// Stored clipboard history item
#[derive(Debug, Clone)]
pub struct StoredHistoryItem {
    pub id: String,
    pub content_type: u8, // ContentType as u8
    pub content_hash: String,
    pub encrypted_content: Vec<u8>,
    pub preview: String,
    pub source_device: Option<String>,
    pub created_at: u64,
}

/// Clipboard history storage operations
pub struct HistoryStorage<'conn> {
    conn: &'conn Mutex<rusqlite::Connection>,
}

impl<'conn> HistoryStorage<'conn> {
    pub fn new(conn: &'conn Mutex<rusqlite::Connection>) -> Self {
        Self { conn }
    }

    /// Store a clipboard history item
    pub fn store_item(&self, item: &StoredHistoryItem) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO clipboard_history 
            (id, content_type, content_hash, encrypted_content, preview, source_device, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            rusqlite::params![
                item.id,
                item.content_type,
                item.content_hash,
                item.encrypted_content,
                item.preview,
                item.source_device,
                item.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get a history item by ID
    pub fn get_item(&self, item_id: &str) -> SqliteResult<Option<StoredHistoryItem>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, content_type, content_hash, encrypted_content, preview, source_device, created_at FROM clipboard_history WHERE id = ?1"
        )?;

        let item = stmt.query_row([item_id], |row| {
            Ok(StoredHistoryItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                content_hash: row.get(2)?,
                encrypted_content: row.get(3)?,
                preview: row.get(4)?,
                source_device: row.get(5)?,
                created_at: row.get(6)?,
            })
        });

        match item {
            Ok(i) => Ok(Some(i)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all history items, ordered by creation time (newest first)
    pub fn get_all_items(&self, limit: Option<u32>) -> SqliteResult<Vec<StoredHistoryItem>> {
        let query = if let Some(limit) = limit {
            format!(
                "SELECT id, content_type, content_hash, encrypted_content, preview, source_device, created_at FROM clipboard_history ORDER BY created_at DESC LIMIT {}",
                limit
            )
        } else {
            "SELECT id, content_type, content_hash, encrypted_content, preview, source_device, created_at FROM clipboard_history ORDER BY created_at DESC".to_string()
        };

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&query)?;

        let items = stmt.query_map([], |row| {
            Ok(StoredHistoryItem {
                id: row.get(0)?,
                content_type: row.get(1)?,
                content_hash: row.get(2)?,
                encrypted_content: row.get(3)?,
                preview: row.get(4)?,
                source_device: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(items)
    }

    /// Remove a history item
    pub fn remove_item(&self, item_id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [item_id])?;
        Ok(())
    }

    /// Clear all history
    pub fn clear_history(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM clipboard_history", [])?;
        Ok(())
    }

    /// Prune old history items (keep only items newer than the given timestamp)
    pub fn prune_old_items(&self, before_timestamp: u64) -> SqliteResult<usize> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute(
            "DELETE FROM clipboard_history WHERE created_at < ?1",
            [before_timestamp],
        )?;
        Ok(count)
    }

    /// Prune history to keep only the most recent N items
    pub fn prune_to_limit(&self, max_items: u32) -> SqliteResult<usize> {
        let conn = self.conn.lock().unwrap();
        // Get count of items
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_history",
            [],
            |row| row.get(0),
        )?;

        if count <= max_items as i64 {
            return Ok(0);
        }

        // Get the timestamp of the Nth item
        let mut stmt = conn.prepare(
            "SELECT created_at FROM clipboard_history ORDER BY created_at DESC LIMIT 1 OFFSET ?1"
        )?;
        let cutoff_timestamp: Option<u64> = stmt.query_row([max_items], |row| row.get(0)).ok();

        if let Some(timestamp) = cutoff_timestamp {
            self.prune_old_items(timestamp)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::storage::Storage;

    #[test]
    fn test_store_and_retrieve_history_item() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let history_storage = storage.history();

        let item = StoredHistoryItem {
            id: "test-item-1".to_string(),
            content_type: 0, // PlainText
            content_hash: "abc123".to_string(),
            encrypted_content: vec![1, 2, 3],
            preview: "Test content".to_string(),
            source_device: None,
            created_at: 1234567890,
        };

        history_storage.store_item(&item).unwrap();
        let retrieved = history_storage.get_item("test-item-1").unwrap();

        assert!(retrieved.is_some());
        let i = retrieved.unwrap();
        assert_eq!(i.id, item.id);
        assert_eq!(i.preview, item.preview);
    }

    #[test]
    fn test_prune_history() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let history_storage = storage.history();

        // Add multiple items
        for i in 0..10 {
            let item = StoredHistoryItem {
                id: format!("item-{}", i),
                content_type: 0,
                content_hash: format!("hash-{}", i),
                encrypted_content: vec![],
                preview: format!("Item {}", i),
                source_device: None,
                created_at: 1000 + i as u64,
            };
            history_storage.store_item(&item).unwrap();
        }

        // Prune to keep only 5 items
        history_storage.prune_to_limit(5).unwrap();

        let all = history_storage.get_all_items(None).unwrap();
        assert_eq!(all.len(), 5);
    }
}
