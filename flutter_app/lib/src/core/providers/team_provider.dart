import 'dart:developer' as developer;

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/team.dart';
import '../services/team_service.dart';

part 'team_provider.g.dart';

/// Provider for the list of teams the current device belongs to
@Riverpod(keepAlive: true)
class Teams extends _$Teams {
  @override
  List<Team> build() {
    // Load teams on initialization
    _loadTeams();
    return [];
  }

  void _loadTeams() {
    try {
      final teams = TeamService.getMyTeams();
      state = teams;
    } catch (e) {
      developer.log('Failed to load teams: $e', name: 'TeamProvider', error: e);
    }
  }

  /// Refresh the teams list
  void refresh() {
    _loadTeams();
  }

  /// Create a new team
  Team createTeam(String name, {String? description}) {
    final team = TeamService.createTeam(name, description: description);
    state = [...state, team];
    return team;
  }

  /// Update a team
  void updateTeam({
    required String teamId,
    String? name,
    String? description,
    bool? broadcastEnabled,
    int? maxMembers,
  }) {
    TeamService.updateTeam(
      teamId: teamId,
      name: name,
      description: description,
      broadcastEnabled: broadcastEnabled,
      maxMembers: maxMembers,
    );
    refresh();
  }

  /// Delete a team
  void deleteTeam(String teamId) {
    final previousState = state;
    state = state.where((t) => t.id != teamId).toList();
    try {
      TeamService.deleteTeam(teamId);
    } catch (e) {
      state = previousState; // Rollback on error
      rethrow;
    }
  }

  /// Leave a team
  void leaveTeam(String teamId) {
    final previousState = state;
    state = state.where((t) => t.id != teamId).toList();
    try {
      TeamService.leaveTeam(teamId);
    } catch (e) {
      state = previousState; // Rollback on error
      rethrow;
    }
  }

  /// Accept a team invitation
  Team acceptInvitation(String code) {
    final team = TeamService.acceptTeamInvitation(code);
    state = [...state, team];
    return team;
  }
}

/// Provider for a single team's details
@riverpod
Team? teamDetails(Ref ref, String teamId) {
  return TeamService.getTeam(teamId);
}

/// Provider for team members
@riverpod
List<TeamMember> teamMembers(Ref ref, String teamId) {
  return TeamService.getTeamMembers(teamId);
}

/// Provider for team invitations (admin only)
@riverpod
List<TeamInvitation> teamInvitations(Ref ref, String teamId) {
  return TeamService.getTeamInvitations(teamId);
}

/// Provider for team audit log (admin only)
@riverpod
List<AuditEntry> teamAuditLog(Ref ref, String teamId, {int? limit}) {
  return TeamService.getTeamAuditLog(teamId, limit: limit);
}

/// Provider for looking up an invitation by code
@riverpod
TeamInvitation? invitationByCode(Ref ref, String code) {
  return TeamService.getInvitationByCode(code);
}

/// Provider for teams that have broadcast enabled
@riverpod
List<Team> broadcastEnabledTeams(Ref ref) {
  return TeamService.getBroadcastEnabledTeams();
}

/// State class for managing team member operations
class TeamMemberNotifier extends Notifier<List<TeamMember>> {
  final String teamId;

  TeamMemberNotifier(this.teamId);

  @override
  List<TeamMember> build() {
    _loadMembers();
    return [];
  }

  void _loadMembers() {
    try {
      final members = TeamService.getTeamMembers(teamId);
      state = members;
    } catch (e) {
      developer.log('Failed to load members: $e',
          name: 'TeamProvider', error: e);
    }
  }

  void refresh() {
    _loadMembers();
  }

  void updateMemberRole(
    String targetDeviceId,
    TeamMemberRole role,
  ) {
    TeamService.updateMemberRole(teamId, targetDeviceId, role);
    refresh();
  }

  void removeMember(String targetDeviceId) {
    TeamService.removeTeamMember(teamId, targetDeviceId);
    state = state.where((m) => m.deviceId != targetDeviceId).toList();
  }
}

/// State class for managing team invitations
class TeamInvitationNotifier extends Notifier<List<TeamInvitation>> {
  final String teamId;

  TeamInvitationNotifier(this.teamId);

  @override
  List<TeamInvitation> build() {
    _loadInvitations();
    return [];
  }

  void _loadInvitations() {
    try {
      final invitations = TeamService.getTeamInvitations(teamId);
      state = invitations;
    } catch (e) {
      developer.log('Failed to load invitations: $e',
          name: 'TeamProvider', error: e);
    }
  }

  void refresh() {
    _loadInvitations();
  }

  TeamInvitation createInvitation({
    required TeamMemberRole role,
    required int expiresInHours,
    int maxUses = 1,
  }) {
    final invitation = TeamService.createTeamInvitation(
      teamId: teamId,
      role: role,
      expiresInHours: expiresInHours,
      maxUses: maxUses,
    );
    state = [...state, invitation];
    return invitation;
  }

  void revokeInvitation(String invitationId) {
    TeamService.revokeTeamInvitation(teamId, invitationId);
    refresh();
  }
}
