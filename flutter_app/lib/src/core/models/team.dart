/// Sentinel value for distinguishing "not provided" from null in copyWith
const _descriptionSentinel = Object();

/// Team model for organization/team management
class Team {
  final String id;
  final String name;
  final String? description;
  final DateTime createdAt;
  final bool broadcastEnabled;
  final int maxMembers;
  final int memberCount;
  final bool isAdmin;

  const Team({
    required this.id,
    required this.name,
    this.description,
    required this.createdAt,
    required this.broadcastEnabled,
    required this.maxMembers,
    required this.memberCount,
    required this.isAdmin,
  });

  factory Team.fromJson(Map<String, dynamic> json) {
    return Team(
      id: json['id'] as String,
      name: json['name'] as String,
      description: json['description'] as String?,
      createdAt: DateTime.fromMillisecondsSinceEpoch(
        (json['created_at'] as int) * 1000,
      ),
      broadcastEnabled: json['broadcast_enabled'] as bool? ?? false,
      maxMembers: json['max_members'] as int? ?? 0,
      memberCount: json['member_count'] as int? ?? 0,
      isAdmin: json['is_admin'] as bool? ?? false,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'description': description,
      'created_at': createdAt.millisecondsSinceEpoch ~/ 1000,
      'broadcast_enabled': broadcastEnabled,
      'max_members': maxMembers,
      'member_count': memberCount,
      'is_admin': isAdmin,
    };
  }

  Team copyWith({
    String? id,
    String? name,
    Object? description = _descriptionSentinel,
    DateTime? createdAt,
    bool? broadcastEnabled,
    int? maxMembers,
    int? memberCount,
    bool? isAdmin,
  }) {
    return Team(
      id: id ?? this.id,
      name: name ?? this.name,
      description: identical(description, _descriptionSentinel)
          ? this.description
          : description as String?,
      createdAt: createdAt ?? this.createdAt,
      broadcastEnabled: broadcastEnabled ?? this.broadcastEnabled,
      maxMembers: maxMembers ?? this.maxMembers,
      memberCount: memberCount ?? this.memberCount,
      isAdmin: isAdmin ?? this.isAdmin,
    );
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is Team && other.id == id;
  }

  @override
  int get hashCode => id.hashCode;
}

/// Team member role
enum TeamMemberRole {
  admin,
  member;

  static TeamMemberRole fromString(String value) {
    switch (value.toLowerCase()) {
      case 'admin':
        return TeamMemberRole.admin;
      default:
        return TeamMemberRole.member;
    }
  }

  String get displayName {
    switch (this) {
      case TeamMemberRole.admin:
        return 'Admin';
      case TeamMemberRole.member:
        return 'Member';
    }
  }
}

/// Team member model
class TeamMember {
  final String deviceId;
  final String displayName;
  final TeamMemberRole role;
  final DateTime joinedAt;
  final bool isOnline;
  final String platform;

  const TeamMember({
    required this.deviceId,
    required this.displayName,
    required this.role,
    required this.joinedAt,
    required this.isOnline,
    required this.platform,
  });

  factory TeamMember.fromJson(Map<String, dynamic> json) {
    return TeamMember(
      deviceId: json['device_id'] as String,
      displayName: json['display_name'] as String,
      role: TeamMemberRole.fromString(json['role'] as String),
      joinedAt: DateTime.fromMillisecondsSinceEpoch(
        (json['joined_at'] as int) * 1000,
      ),
      isOnline: json['is_online'] as bool? ?? false,
      platform: json['platform'] as String? ?? 'unknown',
    );
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is TeamMember && other.deviceId == deviceId;
  }

  @override
  int get hashCode => deviceId.hashCode;
}

/// Team invitation status
enum InvitationStatus {
  pending,
  accepted,
  declined,
  expired,
  revoked;

  static InvitationStatus fromString(String value) {
    switch (value.toLowerCase()) {
      case 'pending':
        return InvitationStatus.pending;
      case 'accepted':
        return InvitationStatus.accepted;
      case 'declined':
        return InvitationStatus.declined;
      case 'expired':
        return InvitationStatus.expired;
      case 'revoked':
        return InvitationStatus.revoked;
      default:
        return InvitationStatus.pending;
    }
  }

  String get displayName {
    switch (this) {
      case InvitationStatus.pending:
        return 'Pending';
      case InvitationStatus.accepted:
        return 'Accepted';
      case InvitationStatus.declined:
        return 'Declined';
      case InvitationStatus.expired:
        return 'Expired';
      case InvitationStatus.revoked:
        return 'Revoked';
    }
  }
}

/// Team invitation model
class TeamInvitation {
  final String id;
  final String teamId;
  final String teamName;
  final String code;
  final TeamMemberRole role;
  final String createdBy;
  final DateTime createdAt;
  final DateTime expiresAt;
  final InvitationStatus status;
  final int maxUses;
  final int useCount;

  const TeamInvitation({
    required this.id,
    required this.teamId,
    required this.teamName,
    required this.code,
    required this.role,
    required this.createdBy,
    required this.createdAt,
    required this.expiresAt,
    required this.status,
    required this.maxUses,
    required this.useCount,
  });

  factory TeamInvitation.fromJson(Map<String, dynamic> json) {
    return TeamInvitation(
      id: json['id'] as String,
      teamId: json['team_id'] as String,
      teamName: json['team_name'] as String,
      code: json['code'] as String,
      role: TeamMemberRole.fromString(json['role'] as String),
      createdBy: json['created_by'] as String,
      createdAt: DateTime.fromMillisecondsSinceEpoch(
        (json['created_at'] as int) * 1000,
      ),
      expiresAt: DateTime.fromMillisecondsSinceEpoch(
        (json['expires_at'] as int) * 1000,
      ),
      status: InvitationStatus.fromString(json['status'] as String),
      maxUses: json['max_uses'] as int? ?? 1,
      useCount: json['use_count'] as int? ?? 0,
    );
  }

  bool get isExpired => DateTime.now().isAfter(expiresAt);
  bool get isValid =>
      status == InvitationStatus.pending &&
      !isExpired &&
      (maxUses == 0 || useCount < maxUses);

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is TeamInvitation && other.id == id;
  }

  @override
  int get hashCode => id.hashCode;
}

/// Team audit log entry
class AuditEntry {
  final String id;
  final String action;
  final String actorDeviceId;
  final String? actorDisplayName;
  final String? targetDeviceId;
  final String? targetDisplayName;
  final String? details;
  final DateTime timestamp;

  const AuditEntry({
    required this.id,
    required this.action,
    required this.actorDeviceId,
    this.actorDisplayName,
    this.targetDeviceId,
    this.targetDisplayName,
    this.details,
    required this.timestamp,
  });

  factory AuditEntry.fromJson(Map<String, dynamic> json) {
    return AuditEntry(
      id: json['id'] as String,
      action: json['action'] as String,
      actorDeviceId: json['actor_device_id'] as String,
      actorDisplayName: json['actor_display_name'] as String?,
      targetDeviceId: json['target_device_id'] as String?,
      targetDisplayName: json['target_display_name'] as String?,
      details: json['details'] as String?,
      timestamp: DateTime.fromMillisecondsSinceEpoch(
        (json['timestamp'] as int) * 1000,
      ),
    );
  }

  String get actionDisplayName {
    switch (action) {
      case 'team_created':
        return 'Team created';
      case 'team_updated':
        return 'Team updated';
      case 'team_deleted':
        return 'Team deleted';
      case 'member_added':
        return 'Member added';
      case 'member_removed':
        return 'Member removed';
      case 'member_role_changed':
        return 'Role changed';
      case 'invitation_sent':
        return 'Invitation sent';
      case 'invitation_accepted':
        return 'Invitation accepted';
      case 'invitation_declined':
        return 'Invitation declined';
      case 'invitation_revoked':
        return 'Invitation revoked';
      case 'device_added':
        return 'Device added';
      case 'device_removed':
        return 'Device removed';
      case 'clipboard_broadcast':
        return 'Clipboard broadcast';
      case 'settings_changed':
        return 'Settings changed';
      default:
        return action;
    }
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is AuditEntry && other.id == id;
  }

  @override
  int get hashCode => id.hashCode;
}
