import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/device.dart';
import '../services/toss_service.dart';
import '../services/storage_service.dart';
import '../services/logging_service.dart';

part 'devices_provider.g.dart';

/// Polling interval for device status updates (30 seconds)
const Duration _statusPollingInterval = Duration(seconds: 30);

/// Provider for paired devices
@Riverpod(keepAlive: true)
class Devices extends _$Devices {
  Timer? _statusPollingTimer;

  @override
  List<Device> build() {
    // Clean up timer when provider is disposed
    ref.onDispose(() {
      stopStatusPolling();
    });
    // Load from TossService (which calls Rust FFI)
    return [];
  }

  Future<void> refresh() async {
    // Call Rust FFI to get paired devices
    // The syncEnabled field now comes directly from the Rust backend
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
              syncEnabled: d.syncEnabled, // Use value from Rust backend
            ))
        .toList();
  }

  /// Start periodic status polling from the relay server
  ///
  /// [relayUrl] - The relay server URL to poll status from
  /// [authToken] - Optional authentication token for the relay server
  void startStatusPolling({required String? relayUrl, String? authToken}) {
    // Don't start if already polling
    if (_statusPollingTimer != null && _statusPollingTimer!.isActive) {
      LoggingService.debug('DevicesProvider: Status polling already active');
      return;
    }

    if (relayUrl == null || relayUrl.isEmpty) {
      LoggingService.debug(
          'DevicesProvider: No relay URL configured, status polling disabled');
      return;
    }

    LoggingService.info(
        'DevicesProvider: Starting status polling every ${_statusPollingInterval.inSeconds}s');

    // Do an immediate status refresh
    refreshDeviceStatuses(relayUrl: relayUrl, authToken: authToken);

    // Start periodic polling
    _statusPollingTimer = Timer.periodic(_statusPollingInterval, (_) {
      refreshDeviceStatuses(relayUrl: relayUrl, authToken: authToken);
    });
  }

  /// Stop periodic status polling
  void stopStatusPolling() {
    if (_statusPollingTimer != null) {
      _statusPollingTimer!.cancel();
      _statusPollingTimer = null;
      LoggingService.debug('DevicesProvider: Status polling stopped');
    }
  }

  /// Refresh device statuses from the relay server API
  ///
  /// This calls the /api/v1/devices/{device_id}/status endpoint for each device
  Future<void> refreshDeviceStatuses({
    required String? relayUrl,
    String? authToken,
  }) async {
    if (relayUrl == null || relayUrl.isEmpty) {
      return;
    }

    if (state.isEmpty) {
      return;
    }

    LoggingService.debug(
        'DevicesProvider: Refreshing status for ${state.length} devices');

    final deviceIds = state.map((d) => d.id).toList();
    final statuses = await TossService.getMultipleDeviceStatuses(
      deviceIds,
      relayUrl: relayUrl,
      authToken: authToken,
    );

    // Update device states with new status information
    bool hasChanges = false;
    final updatedDevices = state.map((device) {
      final status = statuses[device.id];
      if (status != null) {
        final wasOnline = device.isOnline;
        final isNowOnline = status.isOnline;

        if (wasOnline != isNowOnline || device.lastSeen != status.lastSeen) {
          hasChanges = true;
          LoggingService.debug(
              'DevicesProvider: Device ${device.name} status changed: '
              '${wasOnline ? "online" : "offline"} -> ${isNowOnline ? "online" : "offline"}');
          return device.copyWith(
            isOnline: status.isOnline,
            lastSeen: status.lastSeen,
          );
        }
      }
      return device;
    }).toList();

    if (hasChanges) {
      state = updatedDevices;
    }
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

  /// Update device status immediately (for real-time events)
  void updateDeviceStatus(String deviceId, bool isOnline, {DateTime? lastSeen}) {
    state = state.map((d) {
      if (d.id == deviceId) {
        return d.copyWith(
          isOnline: isOnline,
          lastSeen: lastSeen ?? (isOnline ? null : DateTime.now()),
        );
      }
      return d;
    }).toList();
  }

  /// Toggle sync enabled for a device
  Future<void> toggleDeviceSync(String deviceId, bool enabled) async {
    // Update in Rust backend for persistence
    await TossService.setDeviceSyncEnabled(deviceId, enabled);
    // Also update local storage for backward compatibility
    _setDeviceSyncEnabled(deviceId, enabled);
    // Update local state
    state = state.map((d) {
      if (d.id == deviceId) {
        return d.copyWith(syncEnabled: enabled);
      }
      return d;
    }).toList();
    // Broadcast sync preference to other devices (meta-sync)
    await TossService.broadcastSyncPreference(deviceId, enabled);
  }

  /// Enable sync for all devices (Sync to All action)
  Future<void> enableSyncAllDevices() async {
    // Update in Rust backend
    await TossService.enableSyncAllDevices();
    // Update local state
    state = state.map((d) => d.copyWith(syncEnabled: true)).toList();
    // Update local storage
    for (final device in state) {
      _setDeviceSyncEnabled(device.id, true);
    }
  }

  /// Get list of sync-enabled device IDs
  List<String> getSyncEnabledDeviceIds() {
    return state.where((d) => d.syncEnabled).map((d) => d.id).toList();
  }

  /// Get list of online devices with sync enabled
  List<Device> getOnlineSyncEnabledDevices() {
    return state.where((d) => d.isOnline && d.syncEnabled).toList();
  }

  /// Update device sync status from meta-sync (without re-broadcasting)
  void updateFromMetaSync(String deviceId, bool syncEnabled) {
    // Update local storage without broadcasting
    _setDeviceSyncEnabled(deviceId, syncEnabled);
    // Update local state
    state = state.map((d) {
      if (d.id == deviceId) {
        return d.copyWith(syncEnabled: syncEnabled);
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
