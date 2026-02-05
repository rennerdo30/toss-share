import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/toss_service.dart';

part 'toss_provider.g.dart';

/// Progress information for large content transfers
class TransferProgress {
  final int totalChunks;
  final int chunksCompleted;
  final int bytesTransferred;
  final int totalBytes;
  final double progress; // 0.0 - 1.0

  const TransferProgress({
    this.totalChunks = 0,
    this.chunksCompleted = 0,
    this.bytesTransferred = 0,
    this.totalBytes = 0,
    this.progress = 0.0,
  });

  /// Whether this is a chunked transfer (content > 1MB)
  bool get isChunked => totalChunks > 1;

  /// Formatted progress percentage string
  String get progressPercent => '${(progress * 100).toStringAsFixed(0)}%';

  /// Formatted bytes transferred string
  String get bytesProgressString {
    final transferred = _formatBytes(bytesTransferred);
    final total = _formatBytes(totalBytes);
    return '$transferred / $total';
  }

  static String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
}

/// Toss state
class TossState {
  final String deviceId;
  final String deviceName;
  final bool isInitialized;
  final bool isSyncing;
  final int connectedDevices;
  final TransferProgress? transferProgress;

  const TossState({
    required this.deviceId,
    required this.deviceName,
    required this.isInitialized,
    this.isSyncing = false,
    this.connectedDevices = 0,
    this.transferProgress,
  });

  TossState copyWith({
    String? deviceId,
    String? deviceName,
    bool? isInitialized,
    bool? isSyncing,
    int? connectedDevices,
    TransferProgress? transferProgress,
    bool clearTransferProgress = false,
  }) {
    return TossState(
      deviceId: deviceId ?? this.deviceId,
      deviceName: deviceName ?? this.deviceName,
      isInitialized: isInitialized ?? this.isInitialized,
      isSyncing: isSyncing ?? this.isSyncing,
      connectedDevices: connectedDevices ?? this.connectedDevices,
      transferProgress: clearTransferProgress ? null : (transferProgress ?? this.transferProgress),
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

  /// Update transfer progress for chunked transfers
  void updateTransferProgress(TransferProgress? progress) {
    state = state.copyWith(
      transferProgress: progress,
      clearTransferProgress: progress == null,
    );
  }

  /// Send clipboard to all connected devices
  /// For large content (> 1MB), this uses chunked transfer with progress tracking
  Future<void> sendClipboard() async {
    if (state.isSyncing) return; // Prevent multiple sends

    state = state.copyWith(isSyncing: true, clearTransferProgress: true);
    try {
      await TossService.sendClipboard();
    } finally {
      state = state.copyWith(isSyncing: false, clearTransferProgress: true);
    }
  }
}
