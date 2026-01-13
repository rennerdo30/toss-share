import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/device.dart';
import '../services/toss_service.dart';

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
    state = devices.map((d) => Device(
      id: d.id,
      name: d.name,
      isOnline: d.isOnline,
      lastSeen: d.lastSeen > 0 
          ? DateTime.fromMillisecondsSinceEpoch(d.lastSeen)
          : null,
    )).toList();
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

  void updateDeviceStatus(String deviceId, bool isOnline) {
    state = state.map((d) {
      if (d.id == deviceId) {
        return d.copyWith(isOnline: isOnline);
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
