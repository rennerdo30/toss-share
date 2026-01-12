//! Database operations

use chrono::Utc;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

use crate::error::ApiError;

mod models;

pub use models::{Device, QueuedMessage};

/// Database wrapper
pub struct Database {
    pool: Pool<Sqlite>,
}

impl Database {
    /// Create a new database connection
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await?;

        Ok(Self { pool })
    }

    /// Run migrations
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                public_key BLOB NOT NULL,
                device_name TEXT NOT NULL,
                is_online INTEGER DEFAULT 0,
                last_seen INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS message_queue (
                id TEXT PRIMARY KEY,
                from_device TEXT NOT NULL,
                to_device TEXT NOT NULL,
                encrypted_payload TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (from_device) REFERENCES devices(id),
                FOREIGN KEY (to_device) REFERENCES devices(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_message_queue_to_device
            ON message_queue(to_device)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Device operations

    /// Register or update a device
    pub async fn upsert_device(
        &self,
        id: &str,
        public_key: &[u8],
        device_name: &str,
    ) -> Result<Device, ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO devices (id, public_key, device_name, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                device_name = excluded.device_name,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(id)
        .bind(public_key)
        .bind(device_name)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_device(id)
            .await?
            .ok_or_else(|| ApiError::Internal("Failed to create device".to_string()))
    }

    /// Get a device by ID
    pub async fn get_device(&self, id: &str) -> Result<Option<Device>, ApiError> {
        let device = sqlx::query_as::<_, Device>(
            r#"
            SELECT id, public_key, device_name, is_online, last_seen, created_at, updated_at
            FROM devices
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(device)
    }

    /// Update device online status
    pub async fn update_device_status(&self, id: &str, is_online: bool) -> Result<(), ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE devices
            SET is_online = ?, last_seen = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(is_online)
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a device
    pub async fn delete_device(&self, id: &str) -> Result<(), ApiError> {
        // First delete queued messages
        sqlx::query("DELETE FROM message_queue WHERE from_device = ? OR to_device = ?")
            .bind(id)
            .bind(id)
            .execute(&self.pool)
            .await?;

        // Then delete device
        sqlx::query("DELETE FROM devices WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Message queue operations

    /// Queue a message for later delivery
    pub async fn queue_message(
        &self,
        id: &str,
        from_device: &str,
        to_device: &str,
        encrypted_payload: &str,
    ) -> Result<(), ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO message_queue (id, from_device, to_device, encrypted_payload, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(from_device)
        .bind(to_device)
        .bind(encrypted_payload)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get queued messages for a device
    pub async fn get_queued_messages(&self, device_id: &str) -> Result<Vec<QueuedMessage>, ApiError> {
        let messages = sqlx::query_as::<_, QueuedMessage>(
            r#"
            SELECT id, from_device, to_device, encrypted_payload, created_at
            FROM message_queue
            WHERE to_device = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(device_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    /// Delete queued messages for a device
    pub async fn delete_queued_messages(&self, device_id: &str) -> Result<u64, ApiError> {
        let result = sqlx::query("DELETE FROM message_queue WHERE to_device = ?")
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Delete old queued messages (cleanup)
    pub async fn cleanup_old_messages(&self, older_than_secs: i64) -> Result<u64, ApiError> {
        let cutoff = Utc::now().timestamp() - older_than_secs;

        let result = sqlx::query("DELETE FROM message_queue WHERE created_at < ?")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}
