//! Team/Organization storage module
//!
//! Provides storage operations for team management including:
//! - Team creation and management
//! - Team membership with role-based permissions
//! - Team invitations
//! - Team audit logs for compliance

use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Team member role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TeamRole {
    /// Full access to team management
    Admin = 0,
    /// Standard team member
    Member = 1,
}

impl TryFrom<i32> for TeamRole {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TeamRole::Admin),
            1 => Ok(TeamRole::Member),
            _ => Err(()),
        }
    }
}

impl From<TeamRole> for i32 {
    fn from(role: TeamRole) -> Self {
        role as i32
    }
}

/// Invitation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum InvitationStatus {
    Pending = 0,
    Accepted = 1,
    Declined = 2,
    Expired = 3,
    Revoked = 4,
}

impl TryFrom<i32> for InvitationStatus {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(InvitationStatus::Pending),
            1 => Ok(InvitationStatus::Accepted),
            2 => Ok(InvitationStatus::Declined),
            3 => Ok(InvitationStatus::Expired),
            4 => Ok(InvitationStatus::Revoked),
            _ => Err(()),
        }
    }
}

impl From<InvitationStatus> for i32 {
    fn from(status: InvitationStatus) -> Self {
        status as i32
    }
}

/// Audit action type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AuditAction {
    TeamCreated = 0,
    TeamUpdated = 1,
    TeamDeleted = 2,
    MemberAdded = 3,
    MemberRemoved = 4,
    MemberRoleChanged = 5,
    InvitationSent = 6,
    InvitationAccepted = 7,
    InvitationDeclined = 8,
    InvitationRevoked = 9,
    DeviceAddedToTeam = 10,
    DeviceRemovedFromTeam = 11,
    ClipboardBroadcast = 12,
    SettingsChanged = 13,
}

impl TryFrom<i32> for AuditAction {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AuditAction::TeamCreated),
            1 => Ok(AuditAction::TeamUpdated),
            2 => Ok(AuditAction::TeamDeleted),
            3 => Ok(AuditAction::MemberAdded),
            4 => Ok(AuditAction::MemberRemoved),
            5 => Ok(AuditAction::MemberRoleChanged),
            6 => Ok(AuditAction::InvitationSent),
            7 => Ok(AuditAction::InvitationAccepted),
            8 => Ok(AuditAction::InvitationDeclined),
            9 => Ok(AuditAction::InvitationRevoked),
            10 => Ok(AuditAction::DeviceAddedToTeam),
            11 => Ok(AuditAction::DeviceRemovedFromTeam),
            12 => Ok(AuditAction::ClipboardBroadcast),
            13 => Ok(AuditAction::SettingsChanged),
            _ => Err(()),
        }
    }
}

impl From<AuditAction> for i32 {
    fn from(action: AuditAction) -> Self {
        action as i32
    }
}

/// Stored team record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTeam {
    /// Unique team identifier
    pub id: String,
    /// Team name
    pub name: String,
    /// Team description (optional)
    pub description: Option<String>,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Last update timestamp (Unix seconds)
    pub updated_at: u64,
    /// Whether team-wide clipboard broadcast is enabled
    pub broadcast_enabled: bool,
    /// Maximum number of members allowed (0 = unlimited)
    pub max_members: u32,
}

/// Stored team membership record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTeamMember {
    /// Team ID
    pub team_id: String,
    /// Device ID of the member
    pub device_id: String,
    /// Member's display name
    pub display_name: String,
    /// Member's role
    pub role: TeamRole,
    /// When the member joined (Unix seconds)
    pub joined_at: u64,
    /// Who invited this member (device ID)
    pub invited_by: Option<String>,
}

/// Stored team invitation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTeamInvitation {
    /// Unique invitation ID
    pub id: String,
    /// Team ID
    pub team_id: String,
    /// Invitation code (6-character alphanumeric)
    pub code: String,
    /// Role to be assigned when accepted
    pub role: TeamRole,
    /// Who created the invitation (device ID)
    pub created_by: String,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Expiration timestamp (Unix seconds)
    pub expires_at: u64,
    /// Current status
    pub status: InvitationStatus,
    /// Maximum uses (0 = unlimited single use)
    pub max_uses: u32,
    /// Current use count
    pub use_count: u32,
}

/// Stored audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAuditEntry {
    /// Unique entry ID
    pub id: String,
    /// Team ID
    pub team_id: String,
    /// Action performed
    pub action: AuditAction,
    /// Device ID that performed the action
    pub actor_device_id: String,
    /// Target device ID (if applicable)
    pub target_device_id: Option<String>,
    /// Additional details (JSON)
    pub details: Option<String>,
    /// Timestamp (Unix seconds)
    pub timestamp: u64,
}

/// Team storage operations
pub struct TeamStorage<'a> {
    conn: &'a Mutex<Connection>,
}

impl<'a> TeamStorage<'a> {
    pub fn new(conn: &'a Mutex<Connection>) -> Self {
        Self { conn }
    }

    /// Execute a closure within a SQLite transaction.
    /// If the closure returns Ok, the transaction is committed.
    /// If it returns Err, the transaction is rolled back.
    pub fn with_transaction<F, T>(&self, f: F) -> SqliteResult<T>
    where
        F: FnOnce(&Connection) -> SqliteResult<T>,
    {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute("BEGIN IMMEDIATE", [])?;
        match f(&conn) {
            Ok(result) => {
                conn.execute("COMMIT", [])?;
                Ok(result)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    // Team CRUD operations

    /// Create a new team
    pub fn create_team(&self, team: &StoredTeam) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            r#"
            INSERT INTO teams (id, name, description, created_at, updated_at, broadcast_enabled, max_members)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            (
                &team.id,
                &team.name,
                &team.description,
                team.created_at as i64,
                team.updated_at as i64,
                team.broadcast_enabled,
                team.max_members,
            ),
        )?;
        Ok(())
    }

    /// Get a team by ID
    pub fn get_team(&self, id: &str) -> SqliteResult<Option<StoredTeam>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, description, created_at, updated_at, broadcast_enabled, max_members
            FROM teams
            WHERE id = ?
            "#,
        )?;

        let result = stmt.query_row([id], |row| {
            Ok(StoredTeam {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get::<_, i64>(3)? as u64,
                updated_at: row.get::<_, i64>(4)? as u64,
                broadcast_enabled: row.get(5)?,
                max_members: row.get(6)?,
            })
        });

        match result {
            Ok(team) => Ok(Some(team)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all teams for a device
    pub fn get_teams_for_device(&self, device_id: &str) -> SqliteResult<Vec<StoredTeam>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.name, t.description, t.created_at, t.updated_at, t.broadcast_enabled, t.max_members
            FROM teams t
            INNER JOIN team_members tm ON t.id = tm.team_id
            WHERE tm.device_id = ?
            ORDER BY t.name
            "#,
        )?;

        let teams = stmt
            .query_map([device_id], |row| {
                Ok(StoredTeam {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get::<_, i64>(3)? as u64,
                    updated_at: row.get::<_, i64>(4)? as u64,
                    broadcast_enabled: row.get(5)?,
                    max_members: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(teams)
    }

    /// Update a team
    pub fn update_team(&self, team: &StoredTeam) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            r#"
            UPDATE teams
            SET name = ?, description = ?, updated_at = ?, broadcast_enabled = ?, max_members = ?
            WHERE id = ?
            "#,
            (
                &team.name,
                &team.description,
                team.updated_at as i64,
                team.broadcast_enabled,
                team.max_members,
                &team.id,
            ),
        )?;
        Ok(())
    }

    /// Delete a team (within a transaction for atomicity)
    pub fn delete_team(&self, id: &str) -> SqliteResult<()> {
        self.with_transaction(|conn| {
            // Delete in order: audit logs, invitations, members, then team
            conn.execute("DELETE FROM team_audit_log WHERE team_id = ?", [id])?;
            conn.execute("DELETE FROM team_invitations WHERE team_id = ?", [id])?;
            conn.execute("DELETE FROM team_members WHERE team_id = ?", [id])?;
            conn.execute("DELETE FROM teams WHERE id = ?", [id])?;
            Ok(())
        })
    }

    /// Create a team and add the creator as admin atomically
    pub fn create_team_with_admin(
        &self,
        team: &StoredTeam,
        admin_member: &StoredTeamMember,
    ) -> SqliteResult<()> {
        self.with_transaction(|conn| {
            conn.execute(
                r#"
                INSERT INTO teams (id, name, description, created_at, updated_at, broadcast_enabled, max_members)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                (
                    &team.id,
                    &team.name,
                    &team.description,
                    team.created_at as i64,
                    team.updated_at as i64,
                    team.broadcast_enabled,
                    team.max_members,
                ),
            )?;

            conn.execute(
                r#"
                INSERT OR REPLACE INTO team_members (team_id, device_id, display_name, role, joined_at, invited_by)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                (
                    &admin_member.team_id,
                    &admin_member.device_id,
                    &admin_member.display_name,
                    i32::from(admin_member.role),
                    admin_member.joined_at as i64,
                    &admin_member.invited_by,
                ),
            )?;

            Ok(())
        })
    }

    /// Accept an invitation atomically: check limits, add member, update invitation
    pub fn accept_invitation_atomic(
        &self,
        invitation_id: &str,
        team_id: &str,
        max_members: u32,
        max_uses: u32,
        member: &StoredTeamMember,
    ) -> SqliteResult<()> {
        self.with_transaction(|conn| {
            // Re-check member count within transaction
            if max_members > 0 {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM team_members WHERE team_id = ?",
                    [team_id],
                    |row| row.get(0),
                )?;
                if count as u32 >= max_members {
                    return Err(rusqlite::Error::QueryReturnedNoRows); // Signal limit exceeded
                }
            }

            // Re-check max_uses within transaction
            if max_uses > 0 {
                let use_count: i64 = conn.query_row(
                    "SELECT use_count FROM team_invitations WHERE id = ?",
                    [invitation_id],
                    |row| row.get(0),
                )?;
                if use_count as u32 >= max_uses {
                    return Err(rusqlite::Error::QueryReturnedNoRows); // Signal limit exceeded
                }
            }

            // Add member
            conn.execute(
                r#"
                INSERT OR REPLACE INTO team_members (team_id, device_id, display_name, role, joined_at, invited_by)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                (
                    &member.team_id,
                    &member.device_id,
                    &member.display_name,
                    i32::from(member.role),
                    member.joined_at as i64,
                    &member.invited_by,
                ),
            )?;

            // Increment use count
            conn.execute(
                "UPDATE team_invitations SET use_count = use_count + 1 WHERE id = ?",
                [invitation_id],
            )?;

            // If single-use, mark as accepted
            if max_uses == 1 {
                conn.execute(
                    "UPDATE team_invitations SET status = ? WHERE id = ?",
                    (i32::from(InvitationStatus::Accepted), invitation_id),
                )?;
            }

            Ok(())
        })
    }

    /// Update member role with admin count check (atomic)
    pub fn update_member_role_checked(
        &self,
        team_id: &str,
        device_id: &str,
        role: TeamRole,
    ) -> SqliteResult<bool> {
        self.with_transaction(|conn| {
            // If demoting to member, verify at least one other admin remains
            if role == TeamRole::Member {
                let admin_count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM team_members WHERE team_id = ? AND role = 0 AND device_id != ?",
                    [team_id, device_id],
                    |row| row.get(0),
                )?;
                if admin_count == 0 {
                    return Ok(false); // Cannot demote: would leave team with no admin
                }
            }

            conn.execute(
                r#"
                UPDATE team_members
                SET role = ?
                WHERE team_id = ? AND device_id = ?
                "#,
                (i32::from(role), team_id, device_id),
            )?;

            Ok(true)
        })
    }

    // Team membership operations

    /// Add a member to a team
    pub fn add_member(&self, member: &StoredTeamMember) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            r#"
            INSERT OR REPLACE INTO team_members (team_id, device_id, display_name, role, joined_at, invited_by)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            (
                &member.team_id,
                &member.device_id,
                &member.display_name,
                i32::from(member.role),
                member.joined_at as i64,
                &member.invited_by,
            ),
        )?;
        Ok(())
    }

    /// Get a team member
    pub fn get_member(
        &self,
        team_id: &str,
        device_id: &str,
    ) -> SqliteResult<Option<StoredTeamMember>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let mut stmt = conn.prepare(
            r#"
            SELECT team_id, device_id, display_name, role, joined_at, invited_by
            FROM team_members
            WHERE team_id = ? AND device_id = ?
            "#,
        )?;

        let result = stmt.query_row([team_id, device_id], |row| {
            let role_int: i32 = row.get(3)?;
            Ok(StoredTeamMember {
                team_id: row.get(0)?,
                device_id: row.get(1)?,
                display_name: row.get(2)?,
                role: TeamRole::try_from(role_int).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Integer,
                        Box::from(format!("Invalid team role: {}", role_int)),
                    )
                })?,
                joined_at: row.get::<_, i64>(4)? as u64,
                invited_by: row.get(5)?,
            })
        });

        match result {
            Ok(member) => Ok(Some(member)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get all members of a team
    pub fn get_team_members(&self, team_id: &str) -> SqliteResult<Vec<StoredTeamMember>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let mut stmt = conn.prepare(
            r#"
            SELECT team_id, device_id, display_name, role, joined_at, invited_by
            FROM team_members
            WHERE team_id = ?
            ORDER BY role ASC, joined_at ASC
            "#,
        )?;

        let members = stmt
            .query_map([team_id], |row| {
                let role_int: i32 = row.get(3)?;
                Ok(StoredTeamMember {
                    team_id: row.get(0)?,
                    device_id: row.get(1)?,
                    display_name: row.get(2)?,
                    role: TeamRole::try_from(role_int).map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Integer,
                            Box::from(format!("Invalid team role: {}", role_int)),
                        )
                    })?,
                    joined_at: row.get::<_, i64>(4)? as u64,
                    invited_by: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(members)
    }

    /// Update member role
    pub fn update_member_role(
        &self,
        team_id: &str,
        device_id: &str,
        role: TeamRole,
    ) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            r#"
            UPDATE team_members
            SET role = ?
            WHERE team_id = ? AND device_id = ?
            "#,
            (i32::from(role), team_id, device_id),
        )?;
        Ok(())
    }

    /// Remove a member from a team
    pub fn remove_member(&self, team_id: &str, device_id: &str) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            "DELETE FROM team_members WHERE team_id = ? AND device_id = ?",
            [team_id, device_id],
        )?;
        Ok(())
    }

    /// Count team members
    pub fn count_members(&self, team_id: &str) -> SqliteResult<u32> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM team_members WHERE team_id = ?",
            [team_id],
            |row| row.get(0),
        )?;

        Ok(count as u32)
    }

    /// Check if device is admin of team
    pub fn is_admin(&self, team_id: &str, device_id: &str) -> SqliteResult<bool> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM team_members WHERE team_id = ? AND device_id = ? AND role = 0",
            [team_id, device_id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    // Invitation operations

    /// Create an invitation
    pub fn create_invitation(&self, invitation: &StoredTeamInvitation) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            r#"
            INSERT INTO team_invitations (id, team_id, code, role, created_by, created_at, expires_at, status, max_uses, use_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            (
                &invitation.id,
                &invitation.team_id,
                &invitation.code,
                i32::from(invitation.role),
                &invitation.created_by,
                invitation.created_at as i64,
                invitation.expires_at as i64,
                i32::from(invitation.status),
                invitation.max_uses,
                invitation.use_count,
            ),
        )?;
        Ok(())
    }

    /// Get invitation by code
    pub fn get_invitation_by_code(&self, code: &str) -> SqliteResult<Option<StoredTeamInvitation>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, team_id, code, role, created_by, created_at, expires_at, status, max_uses, use_count
            FROM team_invitations
            WHERE code = ?
            "#,
        )?;

        let result = stmt.query_row([code], |row| {
            let role_int: i32 = row.get(3)?;
            let status_int: i32 = row.get(7)?;
            Ok(StoredTeamInvitation {
                id: row.get(0)?,
                team_id: row.get(1)?,
                code: row.get(2)?,
                role: TeamRole::try_from(role_int).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Integer,
                        Box::from(format!("Invalid team role: {}", role_int)),
                    )
                })?,
                created_by: row.get(4)?,
                created_at: row.get::<_, i64>(5)? as u64,
                expires_at: row.get::<_, i64>(6)? as u64,
                status: InvitationStatus::try_from(status_int).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        7,
                        rusqlite::types::Type::Integer,
                        Box::from(format!("Invalid invitation status: {}", status_int)),
                    )
                })?,
                max_uses: row.get(8)?,
                use_count: row.get(9)?,
            })
        });

        match result {
            Ok(invitation) => Ok(Some(invitation)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get invitations for a team
    pub fn get_team_invitations(&self, team_id: &str) -> SqliteResult<Vec<StoredTeamInvitation>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, team_id, code, role, created_by, created_at, expires_at, status, max_uses, use_count
            FROM team_invitations
            WHERE team_id = ?
            ORDER BY created_at DESC
            "#,
        )?;

        let invitations = stmt
            .query_map([team_id], |row| {
                let role_int: i32 = row.get(3)?;
                let status_int: i32 = row.get(7)?;
                Ok(StoredTeamInvitation {
                    id: row.get(0)?,
                    team_id: row.get(1)?,
                    code: row.get(2)?,
                    role: TeamRole::try_from(role_int).map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Integer,
                            Box::from(format!("Invalid team role: {}", role_int)),
                        )
                    })?,
                    created_by: row.get(4)?,
                    created_at: row.get::<_, i64>(5)? as u64,
                    expires_at: row.get::<_, i64>(6)? as u64,
                    status: InvitationStatus::try_from(status_int).map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            7,
                            rusqlite::types::Type::Integer,
                            Box::from(format!("Invalid invitation status: {}", status_int)),
                        )
                    })?,
                    max_uses: row.get(8)?,
                    use_count: row.get(9)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(invitations)
    }

    /// Update invitation status
    pub fn update_invitation_status(&self, id: &str, status: InvitationStatus) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            "UPDATE team_invitations SET status = ? WHERE id = ?",
            (i32::from(status), id),
        )?;
        Ok(())
    }

    /// Increment invitation use count
    pub fn increment_invitation_use(&self, id: &str) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            "UPDATE team_invitations SET use_count = use_count + 1 WHERE id = ?",
            [id],
        )?;
        Ok(())
    }

    /// Delete expired invitations
    pub fn cleanup_expired_invitations(&self, current_time: u64) -> SqliteResult<usize> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let deleted = conn.execute(
            "UPDATE team_invitations SET status = ? WHERE expires_at < ? AND status = ?",
            (
                i32::from(InvitationStatus::Expired),
                current_time as i64,
                i32::from(InvitationStatus::Pending),
            ),
        )?;
        Ok(deleted)
    }

    // Audit log operations

    /// Add an audit log entry
    pub fn add_audit_entry(&self, entry: &StoredAuditEntry) -> SqliteResult<()> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        conn.execute(
            r#"
            INSERT INTO team_audit_log (id, team_id, action, actor_device_id, target_device_id, details, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            (
                &entry.id,
                &entry.team_id,
                i32::from(entry.action),
                &entry.actor_device_id,
                &entry.target_device_id,
                &entry.details,
                entry.timestamp as i64,
            ),
        )?;
        Ok(())
    }

    /// Get audit log for a team
    pub fn get_audit_log(
        &self,
        team_id: &str,
        limit: Option<u32>,
    ) -> SqliteResult<Vec<StoredAuditEntry>> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let mut stmt = conn.prepare(
            r#"
            SELECT id, team_id, action, actor_device_id, target_device_id, details, timestamp
            FROM team_audit_log
            WHERE team_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )?;

        let limit_val: i64 = limit.map_or(i64::MAX, |l| l as i64);

        let entries = stmt
            .query_map(rusqlite::params![team_id, limit_val], |row| {
                let action_int: i32 = row.get(2)?;
                Ok(StoredAuditEntry {
                    id: row.get(0)?,
                    team_id: row.get(1)?,
                    action: AuditAction::try_from(action_int).map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Integer,
                            Box::from(format!("Invalid audit action: {}", action_int)),
                        )
                    })?,
                    actor_device_id: row.get(3)?,
                    target_device_id: row.get(4)?,
                    details: row.get(5)?,
                    timestamp: row.get::<_, i64>(6)? as u64,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    /// Cleanup old audit entries
    pub fn cleanup_old_audit_entries(&self, older_than: u64) -> SqliteResult<usize> {
        let conn = self
            .conn
            .lock()
            .expect("storage mutex poisoned - this is a bug");

        let deleted = conn.execute(
            "DELETE FROM team_audit_log WHERE timestamp < ?",
            [older_than as i64],
        )?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use tempfile::TempDir;

    fn setup_storage() -> (TempDir, Storage) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();
        (temp_dir, storage)
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[test]
    fn test_create_and_get_team() {
        let (_temp_dir, storage) = setup_storage();
        let teams = storage.teams();

        let team = StoredTeam {
            id: "team-123".to_string(),
            name: "Test Team".to_string(),
            description: Some("A test team".to_string()),
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            broadcast_enabled: true,
            max_members: 10,
        };

        teams.create_team(&team).unwrap();
        let retrieved = teams.get_team("team-123").unwrap().unwrap();

        assert_eq!(retrieved.id, team.id);
        assert_eq!(retrieved.name, team.name);
        assert_eq!(retrieved.description, team.description);
        assert_eq!(retrieved.broadcast_enabled, team.broadcast_enabled);
    }

    #[test]
    fn test_team_membership() {
        let (_temp_dir, storage) = setup_storage();
        let teams = storage.teams();

        // Create team
        let team = StoredTeam {
            id: "team-456".to_string(),
            name: "Membership Test".to_string(),
            description: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            broadcast_enabled: false,
            max_members: 0,
        };
        teams.create_team(&team).unwrap();

        // Add admin member
        let admin = StoredTeamMember {
            team_id: "team-456".to_string(),
            device_id: "device-admin".to_string(),
            display_name: "Admin Device".to_string(),
            role: TeamRole::Admin,
            joined_at: current_timestamp(),
            invited_by: None,
        };
        teams.add_member(&admin).unwrap();

        // Add regular member
        let member = StoredTeamMember {
            team_id: "team-456".to_string(),
            device_id: "device-member".to_string(),
            display_name: "Member Device".to_string(),
            role: TeamRole::Member,
            joined_at: current_timestamp(),
            invited_by: Some("device-admin".to_string()),
        };
        teams.add_member(&member).unwrap();

        // Test retrieval
        let members = teams.get_team_members("team-456").unwrap();
        assert_eq!(members.len(), 2);

        // Test admin check
        assert!(teams.is_admin("team-456", "device-admin").unwrap());
        assert!(!teams.is_admin("team-456", "device-member").unwrap());

        // Test member count
        assert_eq!(teams.count_members("team-456").unwrap(), 2);

        // Test role update
        teams
            .update_member_role("team-456", "device-member", TeamRole::Admin)
            .unwrap();
        assert!(teams.is_admin("team-456", "device-member").unwrap());
    }

    #[test]
    fn test_team_invitations() {
        let (_temp_dir, storage) = setup_storage();
        let teams = storage.teams();

        // Create team
        let team = StoredTeam {
            id: "team-789".to_string(),
            name: "Invitation Test".to_string(),
            description: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            broadcast_enabled: false,
            max_members: 0,
        };
        teams.create_team(&team).unwrap();

        // Create invitation
        let invitation = StoredTeamInvitation {
            id: "inv-123".to_string(),
            team_id: "team-789".to_string(),
            code: "ABC123".to_string(),
            role: TeamRole::Member,
            created_by: "device-admin".to_string(),
            created_at: current_timestamp(),
            expires_at: current_timestamp() + 86400,
            status: InvitationStatus::Pending,
            max_uses: 1,
            use_count: 0,
        };
        teams.create_invitation(&invitation).unwrap();

        // Retrieve by code
        let retrieved = teams.get_invitation_by_code("ABC123").unwrap().unwrap();
        assert_eq!(retrieved.team_id, "team-789");
        assert_eq!(retrieved.status, InvitationStatus::Pending);

        // Update status
        teams
            .update_invitation_status("inv-123", InvitationStatus::Accepted)
            .unwrap();
        let updated = teams.get_invitation_by_code("ABC123").unwrap().unwrap();
        assert_eq!(updated.status, InvitationStatus::Accepted);
    }

    #[test]
    fn test_audit_log() {
        let (_temp_dir, storage) = setup_storage();
        let teams = storage.teams();

        // Create team
        let team = StoredTeam {
            id: "team-audit".to_string(),
            name: "Audit Test".to_string(),
            description: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            broadcast_enabled: false,
            max_members: 0,
        };
        teams.create_team(&team).unwrap();

        // Add audit entries
        let entry1 = StoredAuditEntry {
            id: "audit-1".to_string(),
            team_id: "team-audit".to_string(),
            action: AuditAction::TeamCreated,
            actor_device_id: "device-admin".to_string(),
            target_device_id: None,
            details: Some(r#"{"name": "Audit Test"}"#.to_string()),
            timestamp: current_timestamp(),
        };
        teams.add_audit_entry(&entry1).unwrap();

        let entry2 = StoredAuditEntry {
            id: "audit-2".to_string(),
            team_id: "team-audit".to_string(),
            action: AuditAction::MemberAdded,
            actor_device_id: "device-admin".to_string(),
            target_device_id: Some("device-member".to_string()),
            details: None,
            timestamp: current_timestamp(),
        };
        teams.add_audit_entry(&entry2).unwrap();

        // Retrieve audit log
        let log = teams.get_audit_log("team-audit", None).unwrap();
        assert_eq!(log.len(), 2);

        // Test with limit
        let limited_log = teams.get_audit_log("team-audit", Some(1)).unwrap();
        assert_eq!(limited_log.len(), 1);
    }

    #[test]
    fn test_get_teams_for_device() {
        let (_temp_dir, storage) = setup_storage();
        let teams = storage.teams();

        // Create teams
        for i in 1..=3 {
            let team = StoredTeam {
                id: format!("team-{}", i),
                name: format!("Team {}", i),
                description: None,
                created_at: current_timestamp(),
                updated_at: current_timestamp(),
                broadcast_enabled: false,
                max_members: 0,
            };
            teams.create_team(&team).unwrap();

            let member = StoredTeamMember {
                team_id: format!("team-{}", i),
                device_id: "device-1".to_string(),
                display_name: "Device 1".to_string(),
                role: TeamRole::Member,
                joined_at: current_timestamp(),
                invited_by: None,
            };
            teams.add_member(&member).unwrap();
        }

        let device_teams = teams.get_teams_for_device("device-1").unwrap();
        assert_eq!(device_teams.len(), 3);
    }

    #[test]
    fn test_delete_team_cascade() {
        let (_temp_dir, storage) = setup_storage();
        let teams = storage.teams();

        // Create team with members, invitations, and audit log
        let team = StoredTeam {
            id: "team-delete".to_string(),
            name: "Delete Test".to_string(),
            description: None,
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            broadcast_enabled: false,
            max_members: 0,
        };
        teams.create_team(&team).unwrap();

        let member = StoredTeamMember {
            team_id: "team-delete".to_string(),
            device_id: "device-1".to_string(),
            display_name: "Device 1".to_string(),
            role: TeamRole::Admin,
            joined_at: current_timestamp(),
            invited_by: None,
        };
        teams.add_member(&member).unwrap();

        let invitation = StoredTeamInvitation {
            id: "inv-delete".to_string(),
            team_id: "team-delete".to_string(),
            code: "DEL123".to_string(),
            role: TeamRole::Member,
            created_by: "device-1".to_string(),
            created_at: current_timestamp(),
            expires_at: current_timestamp() + 86400,
            status: InvitationStatus::Pending,
            max_uses: 1,
            use_count: 0,
        };
        teams.create_invitation(&invitation).unwrap();

        let entry = StoredAuditEntry {
            id: "audit-delete".to_string(),
            team_id: "team-delete".to_string(),
            action: AuditAction::TeamCreated,
            actor_device_id: "device-1".to_string(),
            target_device_id: None,
            details: None,
            timestamp: current_timestamp(),
        };
        teams.add_audit_entry(&entry).unwrap();

        // Delete team - should cascade delete everything
        teams.delete_team("team-delete").unwrap();

        assert!(teams.get_team("team-delete").unwrap().is_none());
        assert_eq!(teams.get_team_members("team-delete").unwrap().len(), 0);
        assert_eq!(teams.get_team_invitations("team-delete").unwrap().len(), 0);
        assert_eq!(teams.get_audit_log("team-delete", None).unwrap().len(), 0);
    }
}
