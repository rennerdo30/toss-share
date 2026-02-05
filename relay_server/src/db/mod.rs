//! Database operations

use chrono::Utc;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};

use crate::error::ApiError;

mod models;

pub use models::{
    Device, InvitationStatus, PairingSession, QueuedMessage, Team, TeamAuditEntry, TeamInvitation,
    TeamMember, TeamRole,
};

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

        // Pairing sessions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pairing_sessions (
                code TEXT PRIMARY KEY,
                public_key BLOB NOT NULL,
                device_name TEXT NOT NULL,
                expires_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Teams table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS teams (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                broadcast_enabled INTEGER DEFAULT 0,
                max_members INTEGER DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Team members table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS team_members (
                team_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'member',
                joined_at INTEGER NOT NULL,
                PRIMARY KEY (team_id, device_id),
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_team_members_device
            ON team_members(device_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Team invitations table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS team_invitations (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                code TEXT NOT NULL UNIQUE,
                created_by TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                used_by TEXT,
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_team_invitations_code
            ON team_invitations(code)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Team audit log table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS team_audit_log (
                id TEXT PRIMARY KEY,
                team_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                action TEXT NOT NULL,
                details TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_team_audit_log_team
            ON team_audit_log(team_id, created_at DESC)
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
    pub async fn get_queued_messages(
        &self,
        device_id: &str,
    ) -> Result<Vec<QueuedMessage>, ApiError> {
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

    // Pairing session operations

    /// Register a pairing session
    pub async fn register_pairing(
        &self,
        code: &str,
        public_key: &[u8],
        device_name: &str,
        expires_at: i64,
    ) -> Result<(), ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO pairing_sessions (code, public_key, device_name, expires_at, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(code)
        .bind(public_key)
        .bind(device_name)
        .bind(expires_at)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Find a pairing session by code
    pub async fn find_pairing(&self, code: &str) -> Result<Option<PairingSession>, ApiError> {
        let now = Utc::now().timestamp();

        // Find non-expired session
        let session = sqlx::query_as::<_, PairingSession>(
            r#"
            SELECT code, public_key, device_name, expires_at, created_at
            FROM pairing_sessions
            WHERE code = ? AND expires_at > ?
            "#,
        )
        .bind(code)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        Ok(session)
    }

    /// Cancel/delete a pairing session
    pub async fn cancel_pairing(&self, code: &str) -> Result<bool, ApiError> {
        let result = sqlx::query("DELETE FROM pairing_sessions WHERE code = ?")
            .bind(code)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Cleanup expired pairing sessions
    pub async fn cleanup_expired_pairings(&self) -> Result<u64, ApiError> {
        let now = Utc::now().timestamp();

        let result = sqlx::query("DELETE FROM pairing_sessions WHERE expires_at < ?")
            .bind(now)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // Admin operations

    /// List all devices (for admin dashboard)
    pub async fn list_all_devices(&self) -> Result<Vec<Device>, ApiError> {
        let devices = sqlx::query_as::<_, Device>(
            r#"
            SELECT id, public_key, device_name, is_online, last_seen, created_at, updated_at
            FROM devices
            ORDER BY last_seen DESC NULLS LAST
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(devices)
    }

    /// Count devices by online status (for admin dashboard)
    pub async fn count_devices_by_status(&self) -> Result<(i64, i64), ApiError> {
        let online: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices WHERE is_online = 1")
            .fetch_one(&self.pool)
            .await?;

        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM devices")
            .fetch_one(&self.pool)
            .await?;

        Ok((online.0, total.0))
    }

    /// Count queued messages (for admin dashboard)
    pub async fn count_queued_messages(&self) -> Result<i64, ApiError> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM message_queue")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Cleanup stale devices (not seen for specified days)
    pub async fn cleanup_stale_devices(&self, days: i64) -> Result<u64, ApiError> {
        let cutoff = Utc::now().timestamp() - (days * 24 * 60 * 60);

        // First delete related queued messages
        sqlx::query(
            r#"
            DELETE FROM message_queue
            WHERE from_device IN (SELECT id FROM devices WHERE last_seen < ? OR last_seen IS NULL)
               OR to_device IN (SELECT id FROM devices WHERE last_seen < ? OR last_seen IS NULL)
            "#,
        )
        .bind(cutoff)
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        // Then delete stale devices
        let result = sqlx::query("DELETE FROM devices WHERE last_seen < ? OR last_seen IS NULL")
            .bind(cutoff)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Count pairing sessions (for admin dashboard)
    pub async fn count_pairing_sessions(&self) -> Result<i64, ApiError> {
        let now = Utc::now().timestamp();
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pairing_sessions WHERE expires_at > ?")
                .bind(now)
                .fetch_one(&self.pool)
                .await?;

        Ok(count.0)
    }

    /// List all active pairing sessions (for admin dashboard)
    pub async fn list_all_pairing_sessions(&self) -> Result<Vec<PairingSession>, ApiError> {
        let now = Utc::now().timestamp();
        let sessions = sqlx::query_as::<_, PairingSession>(
            r#"
            SELECT code, public_key, device_name, expires_at, created_at
            FROM pairing_sessions
            WHERE expires_at > ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions)
    }

    // Team operations

    /// Create a new team
    pub async fn create_team(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Team, ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO teams (id, name, description, created_at, updated_at, broadcast_enabled, max_members)
            VALUES (?, ?, ?, ?, ?, 0, 0)
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get_team(id)
            .await?
            .ok_or_else(|| ApiError::Internal("Failed to create team".to_string()))
    }

    /// Get a team by ID
    pub async fn get_team(&self, id: &str) -> Result<Option<Team>, ApiError> {
        let team = sqlx::query_as::<_, Team>(
            r#"
            SELECT id, name, description, created_at, updated_at, broadcast_enabled, max_members
            FROM teams
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(team)
    }

    /// List all teams (for admin dashboard)
    pub async fn list_all_teams(&self) -> Result<Vec<Team>, ApiError> {
        let teams = sqlx::query_as::<_, Team>(
            r#"
            SELECT id, name, description, created_at, updated_at, broadcast_enabled, max_members
            FROM teams
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(teams)
    }

    /// Update a team
    pub async fn update_team(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        broadcast_enabled: bool,
    ) -> Result<(), ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            UPDATE teams
            SET name = ?, description = ?, broadcast_enabled = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(broadcast_enabled)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete a team
    pub async fn delete_team(&self, id: &str) -> Result<(), ApiError> {
        sqlx::query("DELETE FROM teams WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Count teams (for admin dashboard)
    pub async fn count_teams(&self) -> Result<i64, ApiError> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM teams")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Add a member to a team
    pub async fn add_team_member(
        &self,
        team_id: &str,
        device_id: &str,
        role: TeamRole,
    ) -> Result<(), ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO team_members (team_id, device_id, role, joined_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(team_id)
        .bind(device_id)
        .bind(role.as_str())
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a member from a team
    pub async fn remove_team_member(&self, team_id: &str, device_id: &str) -> Result<(), ApiError> {
        sqlx::query("DELETE FROM team_members WHERE team_id = ? AND device_id = ?")
            .bind(team_id)
            .bind(device_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get team members
    pub async fn get_team_members(&self, team_id: &str) -> Result<Vec<TeamMember>, ApiError> {
        let members = sqlx::query_as::<_, TeamMember>(
            r#"
            SELECT team_id, device_id, role, joined_at
            FROM team_members
            WHERE team_id = ?
            ORDER BY joined_at ASC
            "#,
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(members)
    }

    /// Count team members
    pub async fn count_team_members(&self, team_id: &str) -> Result<i64, ApiError> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM team_members WHERE team_id = ?")
            .bind(team_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0)
    }

    /// Get team invitations
    pub async fn get_team_invitations(
        &self,
        team_id: &str,
    ) -> Result<Vec<TeamInvitation>, ApiError> {
        let invitations = sqlx::query_as::<_, TeamInvitation>(
            r#"
            SELECT id, team_id, code, created_by, created_at, expires_at, status, used_by
            FROM team_invitations
            WHERE team_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(invitations)
    }

    /// Count pending invitations for a team
    pub async fn count_pending_invitations(&self, team_id: &str) -> Result<i64, ApiError> {
        let now = Utc::now().timestamp();
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM team_invitations WHERE team_id = ? AND status = 'pending' AND expires_at > ?",
        )
        .bind(team_id)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0)
    }

    /// Get team audit log entries
    pub async fn get_team_audit_log(&self, team_id: &str) -> Result<Vec<TeamAuditEntry>, ApiError> {
        let entries = sqlx::query_as::<_, TeamAuditEntry>(
            r#"
            SELECT id, team_id, device_id, action, details, created_at
            FROM team_audit_log
            WHERE team_id = ?
            ORDER BY created_at DESC
            LIMIT 100
            "#,
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Add a team audit log entry
    pub async fn add_team_audit_entry(
        &self,
        id: &str,
        team_id: &str,
        device_id: &str,
        action: &str,
        details: Option<&str>,
    ) -> Result<(), ApiError> {
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO team_audit_log (id, team_id, device_id, action, details, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(team_id)
        .bind(device_id)
        .bind(action)
        .bind(details)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
