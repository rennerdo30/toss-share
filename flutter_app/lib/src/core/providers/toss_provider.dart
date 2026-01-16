import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/toss_service.dart';

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
    // Initialize from TossService
    final deviceId = TossService.deviceId ?? '';
    final deviceName = TossService.deviceName;
    final isInitialized = TossService.isInitialized;

    return TossState(
      deviceId: deviceId,
      deviceName: deviceName,
      isInitialized: isInitialized,
    );
  }

  Future<void> initialize() async {
    // Initialize TossService (which calls Rust FFI)
    await TossService.initialize();
    
    // Update state with actual device info
    state = state.copyWith(
      deviceId: TossService.deviceId ?? '',
      deviceName: TossService.deviceName,
      isInitialized: TossService.isInitialized,
    );
  }

  Future<void> setDeviceName(String name) async {
    // Update via TossService (which calls Rust FFI)
    await TossService.setDeviceName(name);
    state = state.copyWith(deviceName: TossService.deviceName);
  }

  void setSyncing(bool syncing) {
    state = state.copyWith(isSyncing: syncing);
  }

  void updateConnectedDevices(int count) {
    state = state.copyWith(connectedDevices: count);
  }

  /// Send clipboard to all connected devices
  Future<void> sendClipboard() async {
    if (state.isSyncing) return; // Prevent multiple sends

    state = state.copyWith(isSyncing: true);
    try {
      await TossService.sendClipboard();
    } finally {
      state = state.copyWith(isSyncing: false);
    }
  }
}
