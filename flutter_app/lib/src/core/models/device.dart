/// Represents a paired device
class Device {
  final String id;
  final String name;
  final bool isOnline;
  final DateTime? lastSeen;
  final DevicePlatform platform;
  final bool syncEnabled; // Per-device sync setting

  const Device({
    required this.id,
    required this.name,
    this.isOnline = false,
    this.lastSeen,
    this.platform = DevicePlatform.unknown,
    this.syncEnabled = true, // Enabled by default
  });

  Device copyWith({
    String? id,
    String? name,
    bool? isOnline,
    DateTime? lastSeen,
    DevicePlatform? platform,
    bool? syncEnabled,
  }) {
    return Device(
      id: id ?? this.id,
      name: name ?? this.name,
      isOnline: isOnline ?? this.isOnline,
      lastSeen: lastSeen ?? this.lastSeen,
      platform: platform ?? this.platform,
      syncEnabled: syncEnabled ?? this.syncEnabled,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is Device && runtimeType == other.runtimeType && id == other.id;

  @override
  int get hashCode => id.hashCode;
}

/// Device platform types
enum DevicePlatform {
  unknown,
  macos,
  windows,
  linux,
  ios,
  android,
}
