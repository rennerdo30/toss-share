import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/models/team.dart';
import '../../core/providers/team_provider.dart';
import '../../core/services/team_service.dart';
import '../../shared/utils/platform_utils.dart';
import '../../shared/utils/timestamp_utils.dart';

class TeamDetailsScreen extends ConsumerStatefulWidget {
  final String teamId;

  const TeamDetailsScreen({super.key, required this.teamId});

  @override
  ConsumerState<TeamDetailsScreen> createState() => _TeamDetailsScreenState();
}

class _TeamDetailsScreenState extends ConsumerState<TeamDetailsScreen>
    with SingleTickerProviderStateMixin {
  TabController? _tabController;
  Team? _team;
  List<TeamMember> _members = [];
  List<TeamInvitation> _invitations = [];
  List<AuditEntry> _auditLog = [];
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  void _initTabController(bool isAdmin) {
    final length = isAdmin ? 3 : 1;
    if (_tabController?.length != length) {
      _tabController?.dispose();
      _tabController = TabController(length: length, vsync: this);
    }
  }

  @override
  void dispose() {
    _tabController?.dispose();
    super.dispose();
  }

  void _loadData() {
    setState(() => _isLoading = true);
    try {
      final team = TeamService.getTeam(widget.teamId);
      final members = TeamService.getTeamMembers(widget.teamId);

      List<TeamInvitation> invitations = [];
      List<AuditEntry> auditLog = [];

      if (team?.isAdmin == true) {
        try {
          invitations = TeamService.getTeamInvitations(widget.teamId);
          auditLog = TeamService.getTeamAuditLog(widget.teamId, limit: 50);
        } catch (e) {
          // Admin-only operations may fail for non-admins
        }
      }

      if (team != null) {
        _initTabController(team.isAdmin);
      }

      setState(() {
        _team = team;
        _members = members;
        _invitations = invitations;
        _auditLog = auditLog;
        _isLoading = false;
      });
    } catch (e) {
      setState(() => _isLoading = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Error loading team: $e')),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isLoading) {
      return Scaffold(
        appBar: AppBar(title: const Text('Team')),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    if (_team == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Team')),
        body: const Center(child: Text('Team not found')),
      );
    }

    final tabController = _tabController;
    if (tabController == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Team')),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: Text(_team!.name),
        actions: [
          if (_team!.isAdmin)
            PopupMenuButton<String>(
              onSelected: (value) {
                switch (value) {
                  case 'settings':
                    _showTeamSettingsDialog();
                    break;
                  case 'delete':
                    _showDeleteTeamDialog();
                    break;
                }
              },
              itemBuilder: (context) => [
                const PopupMenuItem(
                  value: 'settings',
                  child: ListTile(
                    leading: Icon(Icons.settings),
                    title: Text('Settings'),
                    contentPadding: EdgeInsets.zero,
                  ),
                ),
                const PopupMenuItem(
                  value: 'delete',
                  child: ListTile(
                    leading: Icon(Icons.delete, color: Colors.red),
                    title: Text('Delete Team',
                        style: TextStyle(color: Colors.red)),
                    contentPadding: EdgeInsets.zero,
                  ),
                ),
              ],
            )
          else
            IconButton(
              icon: const Icon(Icons.exit_to_app),
              tooltip: 'Leave Team',
              onPressed: _showLeaveTeamDialog,
            ),
        ],
        bottom: TabBar(
          controller: tabController,
          tabs: [
            const Tab(text: 'Members'),
            if (_team!.isAdmin) const Tab(text: 'Invitations'),
            if (_team!.isAdmin) const Tab(text: 'Audit Log'),
          ].take(_team!.isAdmin ? 3 : 1).toList(),
        ),
      ),
      body: TabBarView(
        controller: tabController,
        children: [
          _MembersTab(
            members: _members,
            team: _team!,
            onRefresh: _loadData,
          ),
          if (_team!.isAdmin)
            _InvitationsTab(
              invitations: _invitations,
              teamId: widget.teamId,
              onRefresh: _loadData,
            ),
          if (_team!.isAdmin) _AuditLogTab(auditLog: _auditLog),
        ].take(_team!.isAdmin ? 3 : 1).toList(),
      ),
      floatingActionButton: _team!.isAdmin
          ? FloatingActionButton.extended(
              onPressed: _showCreateInvitationDialog,
              icon: const Icon(Icons.person_add),
              label: const Text('Invite'),
            )
          : null,
    );
  }

  void _showTeamSettingsDialog() {
    final nameController = TextEditingController(text: _team!.name);
    final descriptionController =
        TextEditingController(text: _team!.description ?? '');
    bool broadcastEnabled = _team!.broadcastEnabled;

    showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: const Text('Team Settings'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: nameController,
                decoration: const InputDecoration(labelText: 'Team Name'),
                maxLength: 50,
              ),
              const SizedBox(height: 8),
              TextField(
                controller: descriptionController,
                decoration: const InputDecoration(labelText: 'Description'),
                maxLength: 200,
                maxLines: 2,
              ),
              const SizedBox(height: 16),
              SwitchListTile(
                title: const Text('Team Broadcast'),
                subtitle:
                    const Text('Allow members to broadcast clipboard to all'),
                value: broadcastEnabled,
                onChanged: (value) {
                  setDialogState(() => broadcastEnabled = value);
                },
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            TextButton(
              onPressed: () {
                try {
                  ref.read(teamsProvider.notifier).updateTeam(
                        teamId: widget.teamId,
                        name: nameController.text.trim(),
                        description: descriptionController.text.trim().isEmpty
                            ? null
                            : descriptionController.text.trim(),
                        broadcastEnabled: broadcastEnabled,
                      );
                  Navigator.pop(context);
                  _loadData();
                } catch (e) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Error: $e')),
                  );
                }
              },
              child: const Text('Save'),
            ),
          ],
        ),
      ),
    );
  }

  void _showDeleteTeamDialog() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete Team?'),
        content: Text(
          'Are you sure you want to delete "${_team!.name}"? '
          'This will remove all members and cannot be undone.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              try {
                ref.read(teamsProvider.notifier).deleteTeam(widget.teamId);
                Navigator.pop(context);
                context.pop();
              } catch (e) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('Error: $e')),
                );
              }
            },
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
  }

  void _showLeaveTeamDialog() {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Leave Team?'),
        content: Text('Are you sure you want to leave "${_team!.name}"?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              try {
                ref.read(teamsProvider.notifier).leaveTeam(widget.teamId);
                Navigator.pop(context);
                context.pop();
              } catch (e) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('Error: $e')),
                );
              }
            },
            child: const Text('Leave'),
          ),
        ],
      ),
    );
  }

  void _showCreateInvitationDialog() {
    TeamMemberRole selectedRole = TeamMemberRole.member;
    int expiresInHours = 24;

    showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: const Text('Create Invitation'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text('Role:'),
              const SizedBox(height: 8),
              SegmentedButton<TeamMemberRole>(
                segments: const [
                  ButtonSegment(
                    value: TeamMemberRole.member,
                    label: Text('Member'),
                  ),
                  ButtonSegment(
                    value: TeamMemberRole.admin,
                    label: Text('Admin'),
                  ),
                ],
                selected: {selectedRole},
                onSelectionChanged: (value) {
                  setDialogState(() => selectedRole = value.first);
                },
              ),
              const SizedBox(height: 16),
              const Text('Expires in:'),
              const SizedBox(height: 8),
              DropdownButton<int>(
                value: expiresInHours,
                isExpanded: true,
                items: const [
                  DropdownMenuItem(value: 1, child: Text('1 hour')),
                  DropdownMenuItem(value: 6, child: Text('6 hours')),
                  DropdownMenuItem(value: 24, child: Text('24 hours')),
                  DropdownMenuItem(value: 72, child: Text('3 days')),
                  DropdownMenuItem(value: 168, child: Text('7 days')),
                ],
                onChanged: (value) {
                  setDialogState(() => expiresInHours = value!);
                },
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            TextButton(
              onPressed: () {
                try {
                  final invitation = TeamService.createTeamInvitation(
                    teamId: widget.teamId,
                    role: selectedRole,
                    expiresInHours: expiresInHours,
                  );
                  Navigator.pop(context);
                  _showInvitationCodeDialog(invitation);
                  _loadData();
                } catch (e) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Error: $e')),
                  );
                }
              },
              child: const Text('Create'),
            ),
          ],
        ),
      ),
    );
  }

  void _showInvitationCodeDialog(TeamInvitation invitation) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Invitation Created'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('Share this code with the person you want to invite:'),
            const SizedBox(height: 16),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: SelectableText(
                invitation.code,
                style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                      letterSpacing: 4,
                      fontWeight: FontWeight.bold,
                    ),
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'Expires: ${formatTimestamp(invitation.expiresAt)}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () {
              Clipboard.setData(ClipboardData(text: invitation.code));
              ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('Code copied to clipboard')),
              );
            },
            child: const Text('Copy'),
          ),
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Done'),
          ),
        ],
      ),
    );
  }
}

class _MembersTab extends StatelessWidget {
  final List<TeamMember> members;
  final Team team;
  final VoidCallback onRefresh;

  const _MembersTab({
    required this.members,
    required this.team,
    required this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: members.length,
      itemBuilder: (context, index) {
        final member = members[index];
        return Card(
          child: ListTile(
            leading: Stack(
              children: [
                CircleAvatar(
                  child: Icon(getPlatformIcon(
                    platformFromString(member.platform),
                  )),
                ),
                Positioned(
                  right: 0,
                  bottom: 0,
                  child: Container(
                    width: 12,
                    height: 12,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: member.isOnline ? Colors.green : Colors.grey,
                      border: Border.all(
                        color: Theme.of(context).cardColor,
                        width: 2,
                      ),
                    ),
                  ),
                ),
              ],
            ),
            title: Row(
              children: [
                Expanded(child: Text(member.displayName)),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                  decoration: BoxDecoration(
                    color: member.role == TeamMemberRole.admin
                        ? Theme.of(context).colorScheme.primaryContainer
                        : Theme.of(context).colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    member.role.displayName,
                    style: TextStyle(
                      fontSize: 12,
                      color: member.role == TeamMemberRole.admin
                          ? Theme.of(context).colorScheme.onPrimaryContainer
                          : Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ),
              ],
            ),
            subtitle: Text(
              member.isOnline
                  ? 'Online'
                  : 'Joined ${formatLastSeen(member.joinedAt)}',
            ),
            trailing: team.isAdmin
                ? PopupMenuButton<String>(
                    onSelected: (value) async {
                      switch (value) {
                        case 'promote':
                          TeamService.updateMemberRole(
                            team.id,
                            member.deviceId,
                            TeamMemberRole.admin,
                          );
                          onRefresh();
                          break;
                        case 'demote':
                          TeamService.updateMemberRole(
                            team.id,
                            member.deviceId,
                            TeamMemberRole.member,
                          );
                          onRefresh();
                          break;
                        case 'remove':
                          TeamService.removeTeamMember(
                            team.id,
                            member.deviceId,
                          );
                          onRefresh();
                          break;
                      }
                    },
                    itemBuilder: (context) => [
                      if (member.role == TeamMemberRole.member)
                        const PopupMenuItem(
                          value: 'promote',
                          child: ListTile(
                            leading: Icon(Icons.arrow_upward),
                            title: Text('Make Admin'),
                            contentPadding: EdgeInsets.zero,
                          ),
                        ),
                      if (member.role == TeamMemberRole.admin)
                        const PopupMenuItem(
                          value: 'demote',
                          child: ListTile(
                            leading: Icon(Icons.arrow_downward),
                            title: Text('Remove Admin'),
                            contentPadding: EdgeInsets.zero,
                          ),
                        ),
                      const PopupMenuItem(
                        value: 'remove',
                        child: ListTile(
                          leading: Icon(Icons.remove_circle, color: Colors.red),
                          title: Text('Remove',
                              style: TextStyle(color: Colors.red)),
                          contentPadding: EdgeInsets.zero,
                        ),
                      ),
                    ],
                  )
                : null,
          ),
        );
      },
    );
  }
}

class _InvitationsTab extends StatelessWidget {
  final List<TeamInvitation> invitations;
  final String teamId;
  final VoidCallback onRefresh;

  const _InvitationsTab({
    required this.invitations,
    required this.teamId,
    required this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    if (invitations.isEmpty) {
      return const Center(
        child: Text('No invitations'),
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: invitations.length,
      itemBuilder: (context, index) {
        final invitation = invitations[index];
        return Card(
          child: ListTile(
            leading: CircleAvatar(
              backgroundColor: invitation.isValid
                  ? Theme.of(context).colorScheme.primaryContainer
                  : Theme.of(context).colorScheme.surfaceContainerHighest,
              child: Text(
                invitation.code.substring(0, 2),
                style: TextStyle(
                  color: invitation.isValid
                      ? Theme.of(context).colorScheme.onPrimaryContainer
                      : Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            title: Text(invitation.code),
            subtitle: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Role: ${invitation.role.displayName}'),
                Text(
                  invitation.isValid
                      ? 'Expires: ${formatTimestamp(invitation.expiresAt)}'
                      : invitation.status.displayName,
                  style: TextStyle(
                    color: invitation.isValid ? null : Colors.red,
                  ),
                ),
              ],
            ),
            trailing: invitation.isValid
                ? IconButton(
                    icon: const Icon(Icons.cancel),
                    tooltip: 'Revoke',
                    onPressed: () {
                      TeamService.revokeTeamInvitation(
                        teamId,
                        invitation.id,
                      );
                      onRefresh();
                    },
                  )
                : null,
          ),
        );
      },
    );
  }
}

class _AuditLogTab extends StatelessWidget {
  final List<AuditEntry> auditLog;

  const _AuditLogTab({required this.auditLog});

  @override
  Widget build(BuildContext context) {
    if (auditLog.isEmpty) {
      return const Center(
        child: Text('No audit entries'),
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: auditLog.length,
      itemBuilder: (context, index) {
        final entry = auditLog[index];
        return Card(
          child: ListTile(
            leading: CircleAvatar(
              child: Icon(_getActionIcon(entry.action)),
            ),
            title: Text(entry.actionDisplayName),
            subtitle: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('By: ${entry.actorDisplayName ?? entry.actorDeviceId}'),
                if (entry.targetDisplayName != null ||
                    entry.targetDeviceId != null)
                  Text(
                    'Target: ${entry.targetDisplayName ?? entry.targetDeviceId}',
                  ),
                Text(formatTimestamp(entry.timestamp)),
              ],
            ),
          ),
        );
      },
    );
  }

  IconData _getActionIcon(String action) {
    switch (action) {
      case 'team_created':
        return Icons.add_circle;
      case 'team_updated':
        return Icons.edit;
      case 'team_deleted':
        return Icons.delete;
      case 'member_added':
        return Icons.person_add;
      case 'member_removed':
        return Icons.person_remove;
      case 'member_role_changed':
        return Icons.admin_panel_settings;
      case 'invitation_sent':
        return Icons.send;
      case 'invitation_accepted':
        return Icons.check_circle;
      case 'invitation_declined':
        return Icons.cancel;
      case 'invitation_revoked':
        return Icons.block;
      case 'clipboard_broadcast':
        return Icons.broadcast_on_home;
      default:
        return Icons.info;
    }
  }
}

// Use formatSmartTimestamp from shared utilities for timestamp formatting
// (imported via timestamp_utils.dart)
String formatTimestamp(DateTime dateTime) => formatSmartTimestamp(dateTime);
