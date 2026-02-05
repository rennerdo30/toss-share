//! Team/Organization API for Flutter integration
//!
//! Provides FFI functions for team management including:
//! - Team creation and management
//! - Team membership with role-based permissions
//! - Team invitations
//! - Team device discovery
//! - Team-wide clipboard broadcast
//! - Team audit logs

use flutter_rust_bridge::frb;
use uuid::Uuid;

use crate::storage::{
    AuditAction, InvitationStatus, StoredAuditEntry, StoredTeam, StoredTeamInvitation,
    StoredTeamMember, TeamRole,
};

use super::TOSS_INSTANCE;

/// Get current unix timestamp in seconds.
fn current_unix_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Generate a 6-character alphanumeric invitation code
fn generate_invitation_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Exclude confusing chars
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

// ============================================================================
// Data Transfer Objects (DTOs)
// ============================================================================

/// Team information for Flutter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
    pub broadcast_enabled: bool,
    pub max_members: u32,
    pub member_count: u32,
    pub is_admin: bool,
}

/// Team member information for Flutter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamMemberDto {
    pub device_id: String,
    pub display_name: String,
    pub role: String, // "admin" or "member"
    pub joined_at: u64,
    pub is_online: bool,
    pub platform: String,
}

/// Team invitation information for Flutter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TeamInvitationDto {
    pub id: String,
    pub team_id: String,
    pub team_name: String,
    pub code: String,
    pub role: String, // "admin" or "member"
    pub created_by: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: String, // "pending", "accepted", "declined", "expired", "revoked"
    pub max_uses: u32,
    pub use_count: u32,
}

/// Team audit log entry for Flutter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntryDto {
    pub id: String,
    pub action: String,
    pub actor_device_id: String,
    pub actor_display_name: Option<String>,
    pub target_device_id: Option<String>,
    pub target_display_name: Option<String>,
    pub details: Option<String>,
    pub timestamp: u64,
}

// ============================================================================
// Helper functions
// ============================================================================

fn role_to_string(role: TeamRole) -> String {
    match role {
        TeamRole::Admin => "admin".to_string(),
        TeamRole::Member => "member".to_string(),
    }
}

fn string_to_role(s: &str) -> TeamRole {
    match s.to_lowercase().as_str() {
        "admin" => TeamRole::Admin,
        _ => TeamRole::Member,
    }
}

fn status_to_string(status: InvitationStatus) -> String {
    match status {
        InvitationStatus::Pending => "pending".to_string(),
        InvitationStatus::Accepted => "accepted".to_string(),
        InvitationStatus::Declined => "declined".to_string(),
        InvitationStatus::Expired => "expired".to_string(),
        InvitationStatus::Revoked => "revoked".to_string(),
    }
}

fn action_to_string(action: AuditAction) -> String {
    match action {
        AuditAction::TeamCreated => "team_created".to_string(),
        AuditAction::TeamUpdated => "team_updated".to_string(),
        AuditAction::TeamDeleted => "team_deleted".to_string(),
        AuditAction::MemberAdded => "member_added".to_string(),
        AuditAction::MemberRemoved => "member_removed".to_string(),
        AuditAction::MemberRoleChanged => "member_role_changed".to_string(),
        AuditAction::InvitationSent => "invitation_sent".to_string(),
        AuditAction::InvitationAccepted => "invitation_accepted".to_string(),
        AuditAction::InvitationDeclined => "invitation_declined".to_string(),
        AuditAction::InvitationRevoked => "invitation_revoked".to_string(),
        AuditAction::DeviceAddedToTeam => "device_added".to_string(),
        AuditAction::DeviceRemovedFromTeam => "device_removed".to_string(),
        AuditAction::ClipboardBroadcast => "clipboard_broadcast".to_string(),
        AuditAction::SettingsChanged => "settings_changed".to_string(),
    }
}

// ============================================================================
// Validation helpers
// ============================================================================

fn validate_team_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Team name cannot be empty".to_string());
    }
    if trimmed.len() > 50 {
        return Err("Team name cannot exceed 50 characters".to_string());
    }
    Ok(())
}

fn validate_team_description(description: &Option<String>) -> Result<(), String> {
    if let Some(desc) = description {
        if desc.len() > 500 {
            return Err("Team description cannot exceed 500 characters".to_string());
        }
    }
    Ok(())
}

// ============================================================================
// Team CRUD Operations
// ============================================================================

/// Create a new team
///
/// The current device becomes the team admin automatically.
#[frb(sync)]
pub fn create_team(name: String, description: Option<String>) -> Result<TeamDto, String> {
    validate_team_name(&name)?;
    validate_team_description(&description)?;

    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();
    let device_name = core.device_name.clone();

    let now = current_unix_timestamp_secs();
    let team_id = Uuid::new_v4().to_string();

    let team = StoredTeam {
        id: team_id.clone(),
        name: name.clone(),
        description: description.clone(),
        created_at: now,
        updated_at: now,
        broadcast_enabled: false,
        max_members: 0, // unlimited
    };

    // Add current device as admin atomically
    let member = StoredTeamMember {
        team_id: team_id.clone(),
        device_id: device_id.clone(),
        display_name: device_name,
        role: TeamRole::Admin,
        joined_at: now,
        invited_by: None,
    };

    core.storage
        .teams()
        .create_team_with_admin(&team, &member)
        .map_err(|e| format!("Failed to create team: {}", e))?;

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.clone(),
        action: AuditAction::TeamCreated,
        actor_device_id: device_id,
        target_device_id: None,
        details: Some(serde_json::json!({ "name": name }).to_string()),
        timestamp: now,
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log for team creation: {}", e);
    }

    Ok(TeamDto {
        id: team_id,
        name,
        description,
        created_at: now,
        broadcast_enabled: false,
        max_members: 0,
        member_count: 1,
        is_admin: true,
    })
}

/// Get all teams the current device belongs to
#[frb(sync)]
pub fn get_my_teams() -> Result<Vec<TeamDto>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    let teams = core
        .storage
        .teams()
        .get_teams_for_device(&device_id)
        .map_err(|e| format!("Failed to get teams: {}", e))?;

    let mut result = Vec::new();
    for team in teams {
        let member_count = core.storage.teams().count_members(&team.id).unwrap_or(0);
        let is_admin = core
            .storage
            .teams()
            .is_admin(&team.id, &device_id)
            .unwrap_or(false);

        result.push(TeamDto {
            id: team.id,
            name: team.name,
            description: team.description,
            created_at: team.created_at,
            broadcast_enabled: team.broadcast_enabled,
            max_members: team.max_members,
            member_count,
            is_admin,
        });
    }

    Ok(result)
}

/// Get team details by ID
#[frb(sync)]
pub fn get_team(team_id: String) -> Result<Option<TeamDto>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    let team = core
        .storage
        .teams()
        .get_team(&team_id)
        .map_err(|e| format!("Failed to get team: {}", e))?;

    match team {
        Some(team) => {
            let member_count = core.storage.teams().count_members(&team.id).unwrap_or(0);
            let is_admin = core
                .storage
                .teams()
                .is_admin(&team.id, &device_id)
                .unwrap_or(false);

            Ok(Some(TeamDto {
                id: team.id,
                name: team.name,
                description: team.description,
                created_at: team.created_at,
                broadcast_enabled: team.broadcast_enabled,
                max_members: team.max_members,
                member_count,
                is_admin,
            }))
        }
        None => Ok(None),
    }
}

/// Update team settings (admin only)
#[frb(sync)]
pub fn update_team(
    team_id: String,
    name: Option<String>,
    description: Option<String>,
    broadcast_enabled: Option<bool>,
    max_members: Option<u32>,
) -> Result<(), String> {
    if let Some(ref n) = name {
        validate_team_name(n)?;
    }
    if description.is_some() {
        validate_team_description(&description)?;
    }

    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can update team settings".to_string());
    }

    let mut team = core
        .storage
        .teams()
        .get_team(&team_id)
        .map_err(|e| format!("Failed to get team: {}", e))?
        .ok_or("Team not found")?;

    if let Some(n) = name {
        team.name = n;
    }
    if let Some(d) = description {
        team.description = Some(d);
    }
    if let Some(b) = broadcast_enabled {
        team.broadcast_enabled = b;
    }
    if let Some(m) = max_members {
        if m > 0 {
            let current_count = core.storage.teams().count_members(&team_id).unwrap_or(0);
            if m < current_count {
                return Err(format!(
                    "Cannot set max_members to {} — team currently has {} members",
                    m, current_count
                ));
            }
        }
        team.max_members = m;
    }
    team.updated_at = current_unix_timestamp_secs();

    core.storage
        .teams()
        .update_team(&team)
        .map_err(|e| format!("Failed to update team: {}", e))?;

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.clone(),
        action: AuditAction::TeamUpdated,
        actor_device_id: device_id,
        target_device_id: None,
        details: None,
        timestamp: current_unix_timestamp_secs(),
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    Ok(())
}

/// Delete a team (admin only)
#[frb(sync)]
pub fn delete_team(team_id: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can delete a team".to_string());
    }

    core.storage
        .teams()
        .delete_team(&team_id)
        .map_err(|e| format!("Failed to delete team: {}", e))?;

    tracing::info!("Team {} deleted by device {}", team_id, device_id);

    Ok(())
}

/// Leave a team
#[frb(sync)]
pub fn leave_team(team_id: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check if this is the only admin
    let members = core
        .storage
        .teams()
        .get_team_members(&team_id)
        .map_err(|e| format!("Failed to get members: {}", e))?;

    // Check if this is the last member — suggest deletion instead
    if members.len() == 1 {
        return Err(
            "Cannot leave team: you are the only member. Delete the team instead.".to_string(),
        );
    }

    let admin_count = members.iter().filter(|m| m.role == TeamRole::Admin).count();
    let is_admin = core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .unwrap_or(false);

    if is_admin && admin_count == 1 {
        return Err(
            "Cannot leave team: you are the only admin. Transfer admin role or delete the team."
                .to_string(),
        );
    }

    core.storage
        .teams()
        .remove_member(&team_id, &device_id)
        .map_err(|e| format!("Failed to leave team: {}", e))?;

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.clone(),
        action: AuditAction::MemberRemoved,
        actor_device_id: device_id.clone(),
        target_device_id: Some(device_id),
        details: Some(r#"{"reason": "left"}"#.to_string()),
        timestamp: current_unix_timestamp_secs(),
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    Ok(())
}

// ============================================================================
// Team Member Operations
// ============================================================================

/// Get all members of a team
#[frb(sync)]
pub fn get_team_members(team_id: String) -> Result<Vec<TeamMemberDto>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check if device is a member
    let member = core
        .storage
        .teams()
        .get_member(&team_id, &device_id)
        .map_err(|e| format!("Failed to check membership: {}", e))?;

    if member.is_none() {
        return Err("You are not a member of this team".to_string());
    }

    let members = core
        .storage
        .teams()
        .get_team_members(&team_id)
        .map_err(|e| format!("Failed to get members: {}", e))?;

    // Get device info for online status and platform
    let devices = core.storage.devices().get_all_devices().unwrap_or_default();

    let result = members
        .into_iter()
        .map(|m| {
            let device_info = devices.iter().find(|d| d.id == m.device_id);
            TeamMemberDto {
                device_id: m.device_id,
                display_name: m.display_name,
                role: role_to_string(m.role),
                joined_at: m.joined_at,
                is_online: device_info.map(|d| d.is_active).unwrap_or(false),
                platform: device_info
                    .and_then(|d| d.platform.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        })
        .collect();

    Ok(result)
}

/// Update a member's role (admin only)
#[frb(sync)]
pub fn update_member_role(
    team_id: String,
    target_device_id: String,
    role: String,
) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can change member roles".to_string());
    }

    let new_role = string_to_role(&role);

    // Use transactional check to prevent race condition with concurrent demotions
    let success = core
        .storage
        .teams()
        .update_member_role_checked(&team_id, &target_device_id, new_role)
        .map_err(|e| format!("Failed to update role: {}", e))?;

    if !success {
        return Err("Cannot demote: team must have at least one admin".to_string());
    }

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.clone(),
        action: AuditAction::MemberRoleChanged,
        actor_device_id: device_id,
        target_device_id: Some(target_device_id),
        details: Some(serde_json::json!({ "new_role": role }).to_string()),
        timestamp: current_unix_timestamp_secs(),
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    Ok(())
}

/// Remove a member from team (admin only)
#[frb(sync)]
pub fn remove_team_member(team_id: String, target_device_id: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can remove members".to_string());
    }

    // Cannot remove yourself this way
    if target_device_id == device_id {
        return Err("Use leave_team to leave the team yourself".to_string());
    }

    core.storage
        .teams()
        .remove_member(&team_id, &target_device_id)
        .map_err(|e| format!("Failed to remove member: {}", e))?;

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.clone(),
        action: AuditAction::MemberRemoved,
        actor_device_id: device_id,
        target_device_id: Some(target_device_id),
        details: Some(r#"{"reason": "removed"}"#.to_string()),
        timestamp: current_unix_timestamp_secs(),
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    Ok(())
}

// ============================================================================
// Team Invitation Operations
// ============================================================================

/// Create a team invitation (admin only)
#[frb(sync)]
pub fn create_team_invitation(
    team_id: String,
    role: String,
    expires_in_hours: u32,
    max_uses: u32,
) -> Result<TeamInvitationDto, String> {
    if expires_in_hours == 0 {
        return Err("Expiration must be at least 1 hour".to_string());
    }
    if expires_in_hours > 720 {
        // 30 days max
        return Err("Expiration cannot exceed 30 days (720 hours)".to_string());
    }

    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can create invitations".to_string());
    }

    let team = core
        .storage
        .teams()
        .get_team(&team_id)
        .map_err(|e| format!("Failed to get team: {}", e))?
        .ok_or("Team not found")?;

    let now = current_unix_timestamp_secs();
    let expires_at = now + (expires_in_hours as u64 * 3600);

    // Generate invitation code with retry on collision
    let code = {
        let mut attempts = 0;
        loop {
            let candidate = generate_invitation_code();
            let existing = core.storage.teams().get_invitation_by_code(&candidate);
            match existing {
                Ok(None) => break candidate,
                Ok(Some(_)) => {
                    attempts += 1;
                    if attempts >= 10 {
                        return Err("Failed to generate unique invitation code".to_string());
                    }
                }
                Err(e) => return Err(format!("Failed to check code uniqueness: {}", e)),
            }
        }
    };

    let invitation = StoredTeamInvitation {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.clone(),
        code: code.clone(),
        role: string_to_role(&role),
        created_by: device_id.clone(),
        created_at: now,
        expires_at,
        status: InvitationStatus::Pending,
        max_uses,
        use_count: 0,
    };

    core.storage
        .teams()
        .create_invitation(&invitation)
        .map_err(|e| format!("Failed to create invitation: {}", e))?;

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: team_id.clone(),
        action: AuditAction::InvitationSent,
        actor_device_id: device_id,
        target_device_id: None,
        details: Some(serde_json::json!({ "code": code, "role": role }).to_string()),
        timestamp: now,
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    Ok(TeamInvitationDto {
        id: invitation.id,
        team_id,
        team_name: team.name,
        code,
        role,
        created_by: invitation.created_by,
        created_at: now,
        expires_at,
        status: "pending".to_string(),
        max_uses,
        use_count: 0,
    })
}

/// Get invitations for a team (admin only)
#[frb(sync)]
pub fn get_team_invitations(team_id: String) -> Result<Vec<TeamInvitationDto>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can view invitations".to_string());
    }

    let team = core
        .storage
        .teams()
        .get_team(&team_id)
        .map_err(|e| format!("Failed to get team: {}", e))?
        .ok_or("Team not found")?;

    let invitations = core
        .storage
        .teams()
        .get_team_invitations(&team_id)
        .map_err(|e| format!("Failed to get invitations: {}", e))?;

    let result = invitations
        .into_iter()
        .map(|i| TeamInvitationDto {
            id: i.id,
            team_id: i.team_id,
            team_name: team.name.clone(),
            code: i.code,
            role: role_to_string(i.role),
            created_by: i.created_by,
            created_at: i.created_at,
            expires_at: i.expires_at,
            status: status_to_string(i.status),
            max_uses: i.max_uses,
            use_count: i.use_count,
        })
        .collect();

    Ok(result)
}

/// Revoke an invitation (admin only)
#[frb(sync)]
pub fn revoke_team_invitation(team_id: String, invitation_id: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can revoke invitations".to_string());
    }

    core.storage
        .teams()
        .update_invitation_status(&invitation_id, InvitationStatus::Revoked)
        .map_err(|e| format!("Failed to revoke invitation: {}", e))?;

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id,
        action: AuditAction::InvitationRevoked,
        actor_device_id: device_id,
        target_device_id: None,
        details: Some(serde_json::json!({ "invitation_id": invitation_id }).to_string()),
        timestamp: current_unix_timestamp_secs(),
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    Ok(())
}

/// Look up invitation by code
#[frb(sync)]
pub fn get_invitation_by_code(code: String) -> Result<Option<TeamInvitationDto>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let invitation = core
        .storage
        .teams()
        .get_invitation_by_code(&code)
        .map_err(|e| format!("Failed to get invitation: {}", e))?;

    match invitation {
        Some(inv) => {
            let now = current_unix_timestamp_secs();

            // Check if expired
            if inv.expires_at < now && inv.status == InvitationStatus::Pending {
                // Mark as expired
                let _ = core
                    .storage
                    .teams()
                    .update_invitation_status(&inv.id, InvitationStatus::Expired);
                return Ok(Some(TeamInvitationDto {
                    id: inv.id,
                    team_id: inv.team_id.clone(),
                    team_name: core
                        .storage
                        .teams()
                        .get_team(&inv.team_id)
                        .ok()
                        .flatten()
                        .map(|t| t.name)
                        .unwrap_or_default(),
                    code: inv.code,
                    role: role_to_string(inv.role),
                    created_by: inv.created_by,
                    created_at: inv.created_at,
                    expires_at: inv.expires_at,
                    status: "expired".to_string(),
                    max_uses: inv.max_uses,
                    use_count: inv.use_count,
                }));
            }

            let team_name = core
                .storage
                .teams()
                .get_team(&inv.team_id)
                .ok()
                .flatten()
                .map(|t| t.name)
                .unwrap_or_default();

            Ok(Some(TeamInvitationDto {
                id: inv.id,
                team_id: inv.team_id,
                team_name,
                code: inv.code,
                role: role_to_string(inv.role),
                created_by: inv.created_by,
                created_at: inv.created_at,
                expires_at: inv.expires_at,
                status: status_to_string(inv.status),
                max_uses: inv.max_uses,
                use_count: inv.use_count,
            }))
        }
        None => Ok(None),
    }
}

/// Accept a team invitation
#[frb(sync)]
pub fn accept_team_invitation(code: String) -> Result<TeamDto, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();
    let device_name = core.device_name.clone();

    let invitation = core
        .storage
        .teams()
        .get_invitation_by_code(&code)
        .map_err(|e| format!("Failed to get invitation: {}", e))?
        .ok_or("Invitation not found")?;

    let now = current_unix_timestamp_secs();

    // Validate invitation
    if invitation.status != InvitationStatus::Pending {
        return Err(format!(
            "Invitation is {}",
            status_to_string(invitation.status)
        ));
    }
    if invitation.expires_at < now {
        core.storage
            .teams()
            .update_invitation_status(&invitation.id, InvitationStatus::Expired)
            .ok();
        return Err("Invitation has expired".to_string());
    }
    if invitation.max_uses > 0 && invitation.use_count >= invitation.max_uses {
        return Err("Invitation has reached maximum uses".to_string());
    }

    // Check if already a member
    if core
        .storage
        .teams()
        .get_member(&invitation.team_id, &device_id)
        .map_err(|e| format!("Failed to check membership: {}", e))?
        .is_some()
    {
        return Err("You are already a member of this team".to_string());
    }

    // Check max members limit
    let team = core
        .storage
        .teams()
        .get_team(&invitation.team_id)
        .map_err(|e| format!("Failed to get team: {}", e))?
        .ok_or("Team not found")?;

    if team.max_members > 0 {
        let member_count = core.storage.teams().count_members(&team.id).unwrap_or(0);
        if member_count >= team.max_members {
            return Err("Team has reached maximum member limit".to_string());
        }
    }

    // Add member and update invitation atomically to prevent race conditions
    let member = StoredTeamMember {
        team_id: invitation.team_id.clone(),
        device_id: device_id.clone(),
        display_name: device_name,
        role: invitation.role,
        joined_at: now,
        invited_by: Some(invitation.created_by.clone()),
    };

    core.storage
        .teams()
        .accept_invitation_atomic(
            &invitation.id,
            &invitation.team_id,
            team.max_members,
            invitation.max_uses,
            &member,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                "Team has reached maximum member limit or invitation has reached maximum uses"
                    .to_string()
            }
            other => format!("Failed to join team: {}", other),
        })?;

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: invitation.team_id.clone(),
        action: AuditAction::InvitationAccepted,
        actor_device_id: device_id.clone(),
        target_device_id: None,
        details: Some(serde_json::json!({ "code": code }).to_string()),
        timestamp: now,
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    let member_count = core.storage.teams().count_members(&team.id).unwrap_or(0);
    let is_admin = invitation.role == TeamRole::Admin;

    Ok(TeamDto {
        id: team.id,
        name: team.name,
        description: team.description,
        created_at: team.created_at,
        broadcast_enabled: team.broadcast_enabled,
        max_members: team.max_members,
        member_count,
        is_admin,
    })
}

/// Decline a team invitation
#[frb(sync)]
pub fn decline_team_invitation(code: String) -> Result<(), String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    let invitation = core
        .storage
        .teams()
        .get_invitation_by_code(&code)
        .map_err(|e| format!("Failed to get invitation: {}", e))?
        .ok_or("Invitation not found")?;

    if invitation.status != InvitationStatus::Pending {
        return Err(format!(
            "Invitation is already {}",
            status_to_string(invitation.status)
        ));
    }

    // Only mark as declined if it's a single-use invitation
    if invitation.max_uses == 1 {
        core.storage
            .teams()
            .update_invitation_status(&invitation.id, InvitationStatus::Declined)
            .map_err(|e| format!("Failed to decline invitation: {}", e))?;
    }

    // Add audit log entry
    let audit_entry = StoredAuditEntry {
        id: Uuid::new_v4().to_string(),
        team_id: invitation.team_id,
        action: AuditAction::InvitationDeclined,
        actor_device_id: device_id,
        target_device_id: None,
        details: Some(serde_json::json!({ "code": code }).to_string()),
        timestamp: current_unix_timestamp_secs(),
    };
    if let Err(e) = core.storage.teams().add_audit_entry(&audit_entry) {
        tracing::error!("Failed to write audit log: {}", e);
    }

    Ok(())
}

// ============================================================================
// Team Audit Log Operations
// ============================================================================

/// Get team audit log (admin only)
#[frb(sync)]
pub fn get_team_audit_log(
    team_id: String,
    limit: Option<u32>,
) -> Result<Vec<AuditEntryDto>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check admin permission
    if !core
        .storage
        .teams()
        .is_admin(&team_id, &device_id)
        .map_err(|e| format!("Failed to check permission: {}", e))?
    {
        return Err("Only team admins can view audit logs".to_string());
    }

    let entries = core
        .storage
        .teams()
        .get_audit_log(&team_id, limit)
        .map_err(|e| format!("Failed to get audit log: {}", e))?;

    // Get member names for display
    let members = core
        .storage
        .teams()
        .get_team_members(&team_id)
        .unwrap_or_default();

    let result = entries
        .into_iter()
        .map(|e| {
            let actor_name = members
                .iter()
                .find(|m| m.device_id == e.actor_device_id)
                .map(|m| m.display_name.clone());

            let target_name = e.target_device_id.as_ref().and_then(|tid| {
                members
                    .iter()
                    .find(|m| &m.device_id == tid)
                    .map(|m| m.display_name.clone())
            });

            AuditEntryDto {
                id: e.id,
                action: action_to_string(e.action),
                actor_device_id: e.actor_device_id,
                actor_display_name: actor_name,
                target_device_id: e.target_device_id,
                target_display_name: target_name,
                details: e.details,
                timestamp: e.timestamp,
            }
        })
        .collect();

    Ok(result)
}

// ============================================================================
// Team Broadcast Operations
// ============================================================================

/// Check if current device can broadcast to a team
#[frb(sync)]
pub fn can_broadcast_to_team(team_id: String) -> Result<bool, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check if device is a member
    let member = core
        .storage
        .teams()
        .get_member(&team_id, &device_id)
        .map_err(|e| format!("Failed to check membership: {}", e))?;

    if member.is_none() {
        return Ok(false);
    }

    // Check if team has broadcast enabled
    let team = core
        .storage
        .teams()
        .get_team(&team_id)
        .map_err(|e| format!("Failed to get team: {}", e))?;

    Ok(team.map(|t| t.broadcast_enabled).unwrap_or(false))
}

/// Get teams that have broadcast enabled and the device is a member of
#[frb(sync)]
pub fn get_broadcast_enabled_teams() -> Result<Vec<TeamDto>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    let teams = core
        .storage
        .teams()
        .get_teams_for_device(&device_id)
        .map_err(|e| format!("Failed to get teams: {}", e))?;

    let result: Vec<TeamDto> = teams
        .into_iter()
        .filter(|t| t.broadcast_enabled)
        .map(|team| {
            let member_count = core.storage.teams().count_members(&team.id).unwrap_or(0);
            let is_admin = core
                .storage
                .teams()
                .is_admin(&team.id, &device_id)
                .unwrap_or(false);

            TeamDto {
                id: team.id,
                name: team.name,
                description: team.description,
                created_at: team.created_at,
                broadcast_enabled: team.broadcast_enabled,
                max_members: team.max_members,
                member_count,
                is_admin,
            }
        })
        .collect();

    Ok(result)
}

/// Get all device IDs in a team for broadcast
#[frb(sync)]
pub fn get_team_device_ids(team_id: String) -> Result<Vec<String>, String> {
    let guard = TOSS_INSTANCE.read();
    let core = guard.as_ref().ok_or("Toss not initialized")?;

    let device_id = core.identity.device_id_hex();

    // Check if device is a member
    let member = core
        .storage
        .teams()
        .get_member(&team_id, &device_id)
        .map_err(|e| format!("Failed to check membership: {}", e))?;

    if member.is_none() {
        return Err("You are not a member of this team".to_string());
    }

    let members = core
        .storage
        .teams()
        .get_team_members(&team_id)
        .map_err(|e| format!("Failed to get members: {}", e))?;

    // Return all device IDs except the current device
    let device_ids: Vec<String> = members
        .into_iter()
        .filter(|m| m.device_id != device_id)
        .map(|m| m.device_id)
        .collect();

    Ok(device_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_invitation_code() {
        let code = generate_invitation_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_role_conversion() {
        assert_eq!(role_to_string(TeamRole::Admin), "admin");
        assert_eq!(role_to_string(TeamRole::Member), "member");
        assert_eq!(string_to_role("admin"), TeamRole::Admin);
        assert_eq!(string_to_role("member"), TeamRole::Member);
        assert_eq!(string_to_role("unknown"), TeamRole::Member);
    }

    #[test]
    fn test_status_conversion() {
        assert_eq!(status_to_string(InvitationStatus::Pending), "pending");
        assert_eq!(status_to_string(InvitationStatus::Accepted), "accepted");
        assert_eq!(status_to_string(InvitationStatus::Declined), "declined");
        assert_eq!(status_to_string(InvitationStatus::Expired), "expired");
        assert_eq!(status_to_string(InvitationStatus::Revoked), "revoked");
    }
}
