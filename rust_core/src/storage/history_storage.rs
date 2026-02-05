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
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
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
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
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
        self.get_items_by_date_range(None, None, limit)
    }

    /// Get history items filtered by date range, ordered by creation time (newest first)
    ///
    /// # Arguments
    /// * `start_timestamp` - Optional start timestamp (Unix seconds, inclusive)
    /// * `end_timestamp` - Optional end timestamp (Unix seconds, inclusive)
    /// * `limit` - Optional maximum number of items to return
    pub fn get_items_by_date_range(
        &self,
        start_timestamp: Option<u64>,
        end_timestamp: Option<u64>,
        limit: Option<u32>,
    ) -> SqliteResult<Vec<StoredHistoryItem>> {
        let base_query = "SELECT id, content_type, content_hash, encrypted_content, preview, source_device, created_at FROM clipboard_history";

        // Build WHERE clause based on date range
        let mut conditions = Vec::new();
        if start_timestamp.is_some() {
            conditions.push("created_at >= ?");
        }
        if end_timestamp.is_some() {
            conditions.push("created_at <= ?");
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let order_clause = " ORDER BY created_at DESC";
        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

        let query = format!(
            "{}{}{}{}",
            base_query, where_clause, order_clause, limit_clause
        );

        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let mut stmt = conn.prepare(&query)?;

        // Build parameter list based on which filters are present
        let params: Vec<u64> = [start_timestamp, end_timestamp]
            .into_iter()
            .flatten()
            .collect();

        let items = stmt
            .query_map(rusqlite::params_from_iter(params), |row| {
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
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute("DELETE FROM clipboard_history WHERE id = ?1", [item_id])?;
        Ok(())
    }

    /// Clear all history
    pub fn clear_history(&self) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        conn.execute("DELETE FROM clipboard_history", [])?;
        Ok(())
    }

    /// Prune old history items (keep only items newer than the given timestamp)
    pub fn prune_old_items(&self, before_timestamp: u64) -> SqliteResult<usize> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        let count = conn.execute(
            "DELETE FROM clipboard_history WHERE created_at < ?1",
            [before_timestamp],
        )?;
        Ok(count)
    }

    /// Prune history to keep only the most recent N items
    pub fn prune_to_limit(&self, max_items: u32) -> SqliteResult<usize> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");
        // Get count of items
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM clipboard_history", [], |row| {
            row.get(0)
        })?;

        if count <= max_items as i64 {
            return Ok(0);
        }

        // Get the timestamp of the Nth item
        let mut stmt = conn.prepare(
            "SELECT created_at FROM clipboard_history ORDER BY created_at DESC LIMIT 1 OFFSET ?1",
        )?;
        let cutoff_timestamp: Option<u64> = stmt.query_row([max_items], |row| row.get(0)).ok();
        drop(stmt);

        if let Some(timestamp) = cutoff_timestamp {
            // Delete directly instead of calling prune_old_items to avoid deadlock
            // Use <= to include the cutoff item in deletion (we want to keep max_items, not max_items+1)
            let deleted = conn.execute(
                "DELETE FROM clipboard_history WHERE created_at <= ?1",
                [timestamp],
            )?;
            Ok(deleted)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use tempfile::TempDir;

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

    #[test]
    fn test_get_items_by_date_range() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let history_storage = storage.history();

        // Add items with different timestamps
        for i in 0..10 {
            let item = StoredHistoryItem {
                id: format!("item-{}", i),
                content_type: 0,
                content_hash: format!("hash-{}", i),
                encrypted_content: vec![],
                preview: format!("Item {}", i),
                source_device: None,
                created_at: 1000 + i as u64, // timestamps: 1000, 1001, ..., 1009
            };
            history_storage.store_item(&item).unwrap();
        }

        // Test with no filters (should return all)
        let all = history_storage
            .get_items_by_date_range(None, None, None)
            .unwrap();
        assert_eq!(all.len(), 10);

        // Test with start filter only (items >= 1005)
        let filtered = history_storage
            .get_items_by_date_range(Some(1005), None, None)
            .unwrap();
        assert_eq!(filtered.len(), 5);
        assert!(filtered.iter().all(|item| item.created_at >= 1005));

        // Test with end filter only (items <= 1003)
        let filtered = history_storage
            .get_items_by_date_range(None, Some(1003), None)
            .unwrap();
        assert_eq!(filtered.len(), 4);
        assert!(filtered.iter().all(|item| item.created_at <= 1003));

        // Test with both start and end (items between 1003 and 1006 inclusive)
        let filtered = history_storage
            .get_items_by_date_range(Some(1003), Some(1006), None)
            .unwrap();
        assert_eq!(filtered.len(), 4);
        assert!(filtered
            .iter()
            .all(|item| item.created_at >= 1003 && item.created_at <= 1006));

        // Test with limit
        let filtered = history_storage
            .get_items_by_date_range(Some(1003), Some(1006), Some(2))
            .unwrap();
        assert_eq!(filtered.len(), 2);
        // Should return newest first (1006, 1005)
        assert_eq!(filtered[0].created_at, 1006);
        assert_eq!(filtered[1].created_at, 1005);
    }

    #[test]
    fn test_get_items_by_date_range_empty_result() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        let history_storage = storage.history();

        // Add items with timestamps 1000-1009
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

        // Test range with no matching items
        let filtered = history_storage
            .get_items_by_date_range(Some(2000), Some(3000), None)
            .unwrap();
        assert_eq!(filtered.len(), 0);
    }
}
