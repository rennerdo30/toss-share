import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/device.dart';
import '../services/toss_service.dart';
import '../services/storage_service.dart';

part 'devices_provider.g.dart';

/// Provider for paired devices
@Riverpod(keepAlive: true)
class Devices extends _$Devices {
  @override
  List<Device> build() {
    // Load from TossService (which calls Rust FFI)
    return [];
  }

  Future<void> refresh() async {
    // Call Rust FFI to get paired devices
    final devices = await TossService.getPairedDevices();
    state = devices
        .map((d) => Device(
              id: d.id,
              name: d.name,
              isOnline: d.isOnline,
              lastSeen: d.lastSeen > 0
                  ? DateTime.fromMillisecondsSinceEpoch(d.lastSeen)
                  : null,
              platform: _parsePlatform(d.platform),
              syncEnabled: _getDeviceSyncEnabled(d.id),
            ))
        .toList();
  }

  /// Get per-device sync setting from storage
  bool _getDeviceSyncEnabled(String deviceId) {
    final key = 'device_sync_enabled_$deviceId';
    return StorageService.getSetting<bool>(key, defaultValue: true) ?? true;
  }

  /// Set per-device sync setting
  void _setDeviceSyncEnabled(String deviceId, bool enabled) {
    final key = 'device_sync_enabled_$deviceId';
    StorageService.setSetting(key, enabled);
  }

  DevicePlatform _parsePlatform(String platform) {
    switch (platform.toLowerCase()) {
      case 'macos':
        return DevicePlatform.macos;
      case 'windows':
        return DevicePlatform.windows;
      case 'linux':
        return DevicePlatform.linux;
      case 'ios':
        return DevicePlatform.ios;
      case 'android':
        return DevicePlatform.android;
      default:
        return DevicePlatform.unknown;
    }
  }

  void addDevice(Device device) {
    state = [...state, device];
  }

  Future<void> removeDevice(String deviceId) async {
    // Call Rust FFI to remove device
    await TossService.removeDevice(deviceId);
    // Update local state
    state = state.where((d) => d.id != deviceId).toList();
  }

  Future<void> renameDevice(String deviceId, String newName) async {
    // Call Rust FFI to rename device
    await TossService.renameDevice(deviceId, newName);
    // Update local state
    state = state.map((d) {
      if (d.id == deviceId) {
        return d.copyWith(name: newName);
      }
      return d;
    }).toList();
  }

  void updateDeviceStatus(String deviceId, bool isOnline) {
    state = state.map((d) {
      if (d.id == deviceId) {
        return d.copyWith(isOnline: isOnline);
      }
      return d;
    }).toList();
  }

  /// Toggle sync enabled for a device
  void toggleDeviceSync(String deviceId, bool enabled) {
    _setDeviceSyncEnabled(deviceId, enabled);
    state = state.map((d) {
      if (d.id == deviceId) {
        return d.copyWith(syncEnabled: enabled);
      }
      return d;
    }).toList();
  }
}

/// Provider for online devices count
@riverpod
int onlineDevicesCount(OnlineDevicesCountRef ref) {
  final devices = ref.watch(devicesProvider);
  return devices.where((d) => d.isOnline).length;
}
