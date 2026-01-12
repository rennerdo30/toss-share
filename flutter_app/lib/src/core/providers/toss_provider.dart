import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'toss_provider.g.dart';

/// Toss state
class TossState {
  final String deviceId;
  final String deviceName;
  final bool isInitialized;
  final bool isSyncing;
  final int connectedDevices;

  const TossState({
    required this.deviceId,
    required this.deviceName,
    required this.isInitialized,
    this.isSyncing = false,
    this.connectedDevices = 0,
  });

  TossState copyWith({
    String? deviceId,
    String? deviceName,
    bool? isInitialized,
    bool? isSyncing,
    int? connectedDevices,
  }) {
    return TossState(
      deviceId: deviceId ?? this.deviceId,
      deviceName: deviceName ?? this.deviceName,
      isInitialized: isInitialized ?? this.isInitialized,
      isSyncing: isSyncing ?? this.isSyncing,
      connectedDevices: connectedDevices ?? this.connectedDevices,
    );
  }
}

/// Main Toss state provider
@Riverpod(keepAlive: true)
class Toss extends _$Toss {
  @override
  TossState build() {
    // TODO: Initialize from Rust FFI
    return const TossState(
      deviceId: '',
      deviceName: 'My Device',
      isInitialized: false,
    );
  }

  Future<void> initialize() async {
    // TODO: Call Rust FFI to get actual device info
    state = state.copyWith(
      deviceId: 'mock-device-id',
      isInitialized: true,
    );
  }

  void setDeviceName(String name) {
    state = state.copyWith(deviceName: name);
    // TODO: Call Rust FFI to update device name
  }

  void setSyncing(bool syncing) {
    state = state.copyWith(isSyncing: syncing);
  }

  void updateConnectedDevices(int count) {
    state = state.copyWith(connectedDevices: count);
  }
}
