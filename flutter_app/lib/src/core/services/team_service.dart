import '../models/team.dart';
import '../../rust/api.dart' as api;

/// Service for team/organization management operations
class TeamService {
  // ============================================================================
  // Team CRUD Operations
  // ============================================================================

  /// Create a new team
  static Team createTeam(String name, {String? description}) {
    final result = api.createTeam(
      name: name,
      description: description,
    );
    return Team(
      id: result.id,
      name: result.name,
      description: result.description,
      createdAt:
          DateTime.fromMillisecondsSinceEpoch(result.createdAt.toInt() * 1000),
      broadcastEnabled: result.broadcastEnabled,
      maxMembers: result.maxMembers,
      memberCount: result.memberCount,
      isAdmin: result.isAdmin,
    );
  }

  /// Get all teams the current device belongs to
  static List<Team> getMyTeams() {
    final results = api.getMyTeams();
    return results
        .map((dto) => Team(
              id: dto.id,
              name: dto.name,
              description: dto.description,
              createdAt: DateTime.fromMillisecondsSinceEpoch(
                  dto.createdAt.toInt() * 1000),
              broadcastEnabled: dto.broadcastEnabled,
              maxMembers: dto.maxMembers,
              memberCount: dto.memberCount,
              isAdmin: dto.isAdmin,
            ))
        .toList();
  }

  /// Get team details by ID
  static Team? getTeam(String teamId) {
    final result = api.getTeam(teamId: teamId);
    if (result == null) return null;
    return Team(
      id: result.id,
      name: result.name,
      description: result.description,
      createdAt:
          DateTime.fromMillisecondsSinceEpoch(result.createdAt.toInt() * 1000),
      broadcastEnabled: result.broadcastEnabled,
      maxMembers: result.maxMembers,
      memberCount: result.memberCount,
      isAdmin: result.isAdmin,
    );
  }

  /// Update team settings (admin only)
  static void updateTeam({
    required String teamId,
    String? name,
    String? description,
    bool? broadcastEnabled,
    int? maxMembers,
  }) {
    api.updateTeam(
      teamId: teamId,
      name: name,
      description: description,
      broadcastEnabled: broadcastEnabled,
      maxMembers: maxMembers,
    );
  }

  /// Delete a team (admin only)
  static void deleteTeam(String teamId) {
    api.deleteTeam(teamId: teamId);
  }

  /// Leave a team
  static void leaveTeam(String teamId) {
    api.leaveTeam(teamId: teamId);
  }

  // ============================================================================
  // Team Member Operations
  // ============================================================================

  /// Get all members of a team
  static List<TeamMember> getTeamMembers(String teamId) {
    final results = api.getTeamMembers(teamId: teamId);
    return results
        .map((dto) => TeamMember(
              deviceId: dto.deviceId,
              displayName: dto.displayName,
              role: TeamMemberRole.fromString(dto.role),
              joinedAt: DateTime.fromMillisecondsSinceEpoch(
                  dto.joinedAt.toInt() * 1000),
              isOnline: dto.isOnline,
              platform: dto.platform,
            ))
        .toList();
  }

  /// Update a member's role (admin only)
  static void updateMemberRole(
    String teamId,
    String targetDeviceId,
    TeamMemberRole role,
  ) {
    api.updateMemberRole(
      teamId: teamId,
      targetDeviceId: targetDeviceId,
      role: role == TeamMemberRole.admin ? 'admin' : 'member',
    );
  }

  /// Remove a member from team (admin only)
  static void removeTeamMember(
    String teamId,
    String targetDeviceId,
  ) {
    api.removeTeamMember(
      teamId: teamId,
      targetDeviceId: targetDeviceId,
    );
  }

  // ============================================================================
  // Team Invitation Operations
  // ============================================================================

  /// Create a team invitation (admin only)
  static TeamInvitation createTeamInvitation({
    required String teamId,
    required TeamMemberRole role,
    required int expiresInHours,
    int maxUses = 1,
  }) {
    final result = api.createTeamInvitation(
      teamId: teamId,
      role: role == TeamMemberRole.admin ? 'admin' : 'member',
      expiresInHours: expiresInHours,
      maxUses: maxUses,
    );
    return TeamInvitation(
      id: result.id,
      teamId: result.teamId,
      teamName: result.teamName,
      code: result.code,
      role: TeamMemberRole.fromString(result.role),
      createdBy: result.createdBy,
      createdAt:
          DateTime.fromMillisecondsSinceEpoch(result.createdAt.toInt() * 1000),
      expiresAt:
          DateTime.fromMillisecondsSinceEpoch(result.expiresAt.toInt() * 1000),
      status: InvitationStatus.fromString(result.status),
      maxUses: result.maxUses,
      useCount: result.useCount,
    );
  }

  /// Get invitations for a team (admin only)
  static List<TeamInvitation> getTeamInvitations(String teamId) {
    final results = api.getTeamInvitations(teamId: teamId);
    return results
        .map((dto) => TeamInvitation(
              id: dto.id,
              teamId: dto.teamId,
              teamName: dto.teamName,
              code: dto.code,
              role: TeamMemberRole.fromString(dto.role),
              createdBy: dto.createdBy,
              createdAt: DateTime.fromMillisecondsSinceEpoch(
                  dto.createdAt.toInt() * 1000),
              expiresAt: DateTime.fromMillisecondsSinceEpoch(
                  dto.expiresAt.toInt() * 1000),
              status: InvitationStatus.fromString(dto.status),
              maxUses: dto.maxUses,
              useCount: dto.useCount,
            ))
        .toList();
  }

  /// Revoke an invitation (admin only)
  static void revokeTeamInvitation(
    String teamId,
    String invitationId,
  ) {
    api.revokeTeamInvitation(
      teamId: teamId,
      invitationId: invitationId,
    );
  }

  /// Look up invitation by code
  static TeamInvitation? getInvitationByCode(String code) {
    final result = api.getInvitationByCode(code: code);
    if (result == null) return null;
    return TeamInvitation(
      id: result.id,
      teamId: result.teamId,
      teamName: result.teamName,
      code: result.code,
      role: TeamMemberRole.fromString(result.role),
      createdBy: result.createdBy,
      createdAt:
          DateTime.fromMillisecondsSinceEpoch(result.createdAt.toInt() * 1000),
      expiresAt:
          DateTime.fromMillisecondsSinceEpoch(result.expiresAt.toInt() * 1000),
      status: InvitationStatus.fromString(result.status),
      maxUses: result.maxUses,
      useCount: result.useCount,
    );
  }

  /// Accept a team invitation
  static Team acceptTeamInvitation(String code) {
    final result = api.acceptTeamInvitation(code: code);
    return Team(
      id: result.id,
      name: result.name,
      description: result.description,
      createdAt:
          DateTime.fromMillisecondsSinceEpoch(result.createdAt.toInt() * 1000),
      broadcastEnabled: result.broadcastEnabled,
      maxMembers: result.maxMembers,
      memberCount: result.memberCount,
      isAdmin: result.isAdmin,
    );
  }

  /// Decline a team invitation
  static void declineTeamInvitation(String code) {
    api.declineTeamInvitation(code: code);
  }

  // ============================================================================
  // Team Audit Log Operations
  // ============================================================================

  /// Get team audit log (admin only)
  static List<AuditEntry> getTeamAuditLog(
    String teamId, {
    int? limit,
  }) {
    final results = api.getTeamAuditLog(
      teamId: teamId,
      limit: limit,
    );
    return results
        .map((dto) => AuditEntry(
              id: dto.id,
              action: dto.action,
              actorDeviceId: dto.actorDeviceId,
              actorDisplayName: dto.actorDisplayName,
              targetDeviceId: dto.targetDeviceId,
              targetDisplayName: dto.targetDisplayName,
              details: dto.details,
              timestamp: DateTime.fromMillisecondsSinceEpoch(
                  dto.timestamp.toInt() * 1000),
            ))
        .toList();
  }

  // ============================================================================
  // Team Broadcast Operations
  // ============================================================================

  /// Check if current device can broadcast to a team
  static bool canBroadcastToTeam(String teamId) {
    return api.canBroadcastToTeam(teamId: teamId);
  }

  /// Get teams that have broadcast enabled
  static List<Team> getBroadcastEnabledTeams() {
    final results = api.getBroadcastEnabledTeams();
    return results
        .map((dto) => Team(
              id: dto.id,
              name: dto.name,
              description: dto.description,
              createdAt: DateTime.fromMillisecondsSinceEpoch(
                  dto.createdAt.toInt() * 1000),
              broadcastEnabled: dto.broadcastEnabled,
              maxMembers: dto.maxMembers,
              memberCount: dto.memberCount,
              isAdmin: dto.isAdmin,
            ))
        .toList();
  }

  /// Get all device IDs in a team for broadcast
  static List<String> getTeamDeviceIds(String teamId) {
    return api.getTeamDeviceIds(teamId: teamId);
  }
}
