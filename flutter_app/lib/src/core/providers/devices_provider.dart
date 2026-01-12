import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/device.dart';

part 'devices_provider.g.dart';

/// Provider for paired devices
@Riverpod(keepAlive: true)
class Devices extends _$Devices {
  @override
  List<Device> build() {
    // TODO: Load from Rust FFI
    return [];
  }

  Future<void> refresh() async {
    // TODO: Call Rust FFI to get paired devices
    state = [];
  }

  void addDevice(Device device) {
    state = [...state, device];
  }

  void removeDevice(String deviceId) {
    state = state.where((d) => d.id != deviceId).toList();
    // TODO: Call Rust FFI to remove device
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
