import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'dart:io';
import 'dart:async';

// Import generated FFI bindings
import 'package:toss/src/rust/api.dart' as api;
import 'package:toss/src/rust/frb_generated.dart';

/// Pairing information returned from start_pairing
class PairingInfo {
  final String code;
  final String qrData;
  final int expiresAt;
  final String publicKey;

  const PairingInfo({
    required this.code,
    required this.qrData,
    required this.expiresAt,
    required this.publicKey,
  });
}

/// Device information
class DeviceInfo {
  final String id;
  final String name;
  final bool isOnline;
  final int lastSeen;
  final String platform;

  const DeviceInfo({
    required this.id,
    required this.name,
    this.isOnline = false,
    this.lastSeen = 0,
    this.platform = 'unknown',
  });
}

/// Clipboard item from core
class ClipboardItemInfo {
  final String id;
  final String contentType;
  final String preview;
  final int sizeBytes;
  final int timestamp;
  final String? sourceDevice;

  const ClipboardItemInfo({
    required this.id,
    required this.contentType,
    required this.preview,
    required this.sizeBytes,
    required this.timestamp,
    this.sourceDevice,
  });
}

/// Decrypted clipboard content from history
class ClipboardContent {
  final String contentType;
  final List<int> data;

  const ClipboardContent({
    required this.contentType,
    required this.data,
  });
}

/// Network event from Rust core
class TossEvent {
  final String type;
  final Map<String, dynamic>? data;

  const TossEvent({
    required this.type,
    this.data,
  });

  factory TossEvent.fromApi(api.TossEvent event) {
    // Convert API event to Dart event using pattern matching
    return event.when(
      clipboardReceived: (item) => TossEvent(
        type: 'clipboard_received',
        data: {
          'item': ClipboardItemInfo(
            id: item.id,
            contentType: item.contentType,
            preview: item.preview,
            sizeBytes: item.sizeBytes.toInt(),
            timestamp: item.timestamp.toInt(),
            sourceDevice: item.sourceDevice,
          ),
        },
      ),
      deviceConnected: (device) => TossEvent(
        type: 'device_connected',
        data: {
          'device': DeviceInfo(
            id: device.id,
            name: device.name,
            isOnline: device.isOnline,
            lastSeen: device.lastSeen.toInt(),
            platform: device.platform,
          ),
        },
      ),
      deviceDisconnected: (deviceId) => TossEvent(
        type: 'device_disconnected',
        data: {'device_id': deviceId},
      ),
      pairingRequest: (device) => TossEvent(
        type: 'pairing_request',
        data: {
          'device': DeviceInfo(
            id: device.id,
            name: device.name,
            isOnline: device.isOnline,
            lastSeen: device.lastSeen.toInt(),
            platform: device.platform,
          ),
        },
      ),
      error: (message) => TossEvent(
        type: 'error',
        data: {'message': message},
      ),
    );
  }
}

/// Service for initializing and managing Toss core
class TossService {
  TossService._();

  static bool _initialized = false;
  static bool _ffiAvailable = false;
  static String? _dataDir;
  static String? _deviceId;
  static String _deviceName = 'Toss Device';

  /// Check if service is initialized
  static bool get isInitialized => _initialized;

  /// Check if FFI is available (native library loaded)
  static bool get isFfiAvailable => _ffiAvailable;

  /// Get current device ID
  static String? get deviceId => _deviceId;

  /// Get current device name
  static String get deviceName => _deviceName;

  /// Initialize the Toss service
  static Future<void> initialize() async {
    if (_initialized) return;

    final appDir = await getApplicationDocumentsDirectory();
    final dataDir = Directory('${appDir.path}/toss');

    if (!await dataDir.exists()) {
      await dataDir.create(recursive: true);
    }

    _dataDir = dataDir.path;
    _deviceName = await _getDeviceName();

    // Initialize flutter_rust_bridge first
    try {
      await RustLib.init();
      _ffiAvailable = true;
    } catch (e) {
      _ffiAvailable = false;
      debugPrint('Warning: flutter_rust_bridge initialization failed: $e');
    }

    // Call Rust FFI init_toss()
    if (_ffiAvailable) {
      try {
        api.initToss(dataDir: _dataDir!, deviceName: _deviceName);
        _deviceId = api.getDeviceId();
      } catch (e) {
        // Fallback: Mock device ID if FFI fails
        _ffiAvailable = false;
        _deviceId = 'mock-device-${DateTime.now().millisecondsSinceEpoch}';
        debugPrint('Warning: FFI initialization failed: $e');
      }
    } else {
      // Fallback: Mock device ID when FFI not available
      _deviceId = 'mock-device-${DateTime.now().millisecondsSinceEpoch}';
    }

    _initialized = true;
  }

  /// Get a friendly device name based on platform
  static Future<String> _getDeviceName() async {
    if (Platform.isMacOS) {
      return 'Mac';
    } else if (Platform.isWindows) {
      return 'Windows PC';
    } else if (Platform.isLinux) {
      return 'Linux';
    } else if (Platform.isIOS) {
      return 'iPhone';
    } else if (Platform.isAndroid) {
      return 'Android';
    }
    return 'Toss Device';
  }

  /// Set device name
  static Future<void> setDeviceName(String name) async {
    _deviceName = name;
    try {
      api.setDeviceName(name: name);
    } catch (e) {
      debugPrint('Warning: Failed to set device name: $e');
    }
  }

  // ============================================================================
  // Pairing
  // ============================================================================

  /// Start a new pairing session
  static Future<PairingInfo> startPairing() async {
    try {
      final info = api.startPairing();
      return PairingInfo(
        code: info.code,
        qrData: info.qrData,
        expiresAt: info.expiresAt.toInt(),
        publicKey: info.publicKey,
      );
    } catch (e) {
      throw Exception('Failed to start pairing: $e');
    }
  }

  /// Complete pairing with QR code data
  static Future<DeviceInfo> completePairingQR(String qrData) async {
    try {
      final device = api.completePairingQr(qrData: qrData);
      return DeviceInfo(
        id: device.id,
        name: device.name,
        isOnline: device.isOnline,
        lastSeen: device.lastSeen.toInt(),
        platform: device.platform,
      );
    } catch (e) {
      throw Exception('Failed to complete pairing: $e');
    }
  }

  /// Complete pairing with manual code
  static Future<DeviceInfo> completePairingCode(
      String code, List<int> publicKey) async {
    try {
      final device =
          api.completePairingCode(code: code, peerPublicKey: publicKey);
      return DeviceInfo(
        id: device.id,
        name: device.name,
        isOnline: device.isOnline,
        lastSeen: device.lastSeen.toInt(),
        platform: device.platform,
      );
    } catch (e) {
      throw Exception('Failed to complete pairing with code: $e');
    }
  }

  /// Cancel active pairing session
  static void cancelPairing() {
    try {
      api.cancelPairing();
    } catch (e) {
      debugPrint('Warning: Failed to cancel pairing: $e');
    }
  }

  // ============================================================================
  // Device Management
  // ============================================================================

  /// Get list of paired devices
  static Future<List<DeviceInfo>> getPairedDevices() async {
    if (!_ffiAvailable) return [];
    try {
      final devices = api.getPairedDevices();
      return devices
          .map((d) => DeviceInfo(
                id: d.id,
                name: d.name,
                isOnline: d.isOnline,
                lastSeen: d.lastSeen.toInt(),
                platform: d.platform,
              ))
          .toList();
    } catch (e) {
      debugPrint('Warning: Failed to get paired devices: $e');
      return [];
    }
  }

  /// Get list of connected devices
  static Future<List<DeviceInfo>> getConnectedDevices() async {
    if (!_ffiAvailable) return [];
    try {
      final devices = api.getConnectedDevices();
      return devices
          .map((d) => DeviceInfo(
                id: d.id,
                name: d.name,
                isOnline: d.isOnline,
                lastSeen: d.lastSeen.toInt(),
                platform: d.platform,
              ))
          .toList();
    } catch (e) {
      debugPrint('Warning: Failed to get connected devices: $e');
      return [];
    }
  }

  /// Remove a paired device
  static Future<void> removeDevice(String deviceId) async {
    try {
      api.removeDevice(deviceId: deviceId);
    } catch (e) {
      debugPrint('Warning: Failed to remove device: $e');
    }
  }

  /// Rename a paired device
  static Future<void> renameDevice(String deviceId, String newName) async {
    try {
      api.renameDevice(deviceId: deviceId, newName: newName);
    } catch (e) {
      debugPrint('Warning: Failed to rename device: $e');
      rethrow;
    }
  }

  // ============================================================================
  // Clipboard Operations
  // ============================================================================

  /// Get current clipboard content
  static Future<ClipboardItemInfo?> getCurrentClipboard() async {
    try {
      final item = api.getCurrentClipboard();
      if (item == null) return null;
      return ClipboardItemInfo(
        id: item.id,
        contentType: item.contentType,
        preview: item.preview,
        sizeBytes: item.sizeBytes.toInt(),
        timestamp: item.timestamp.toInt(),
        sourceDevice: item.sourceDevice,
      );
    } catch (e) {
      debugPrint('Warning: Failed to get current clipboard: $e');
      return null;
    }
  }

  /// Send current clipboard to all devices with retry logic
  static Future<void> sendClipboard({int maxRetries = 3}) async {
    await _retryOperation(
      () => api.sendClipboard(),
      'send clipboard',
      maxRetries: maxRetries,
    );
  }

  /// Send text to all devices with retry logic
  static Future<void> sendText(String text, {int maxRetries = 3}) async {
    await _retryOperation(
      () => api.sendText(text: text),
      'send text',
      maxRetries: maxRetries,
    );
  }

  /// Retry an operation with exponential backoff
  static Future<void> _retryOperation(
    Future<void> Function() operation,
    String operationName, {
    int maxRetries = 3,
  }) async {
    int attempt = 0;
    Duration delay = const Duration(milliseconds: 500);

    while (attempt < maxRetries) {
      try {
        await operation();
        return; // Success
      } catch (e) {
        attempt++;
        if (attempt >= maxRetries) {
          debugPrint(
              'Warning: Failed to $operationName after $maxRetries attempts: $e');
          rethrow; // Propagate error after all retries exhausted
        }
        debugPrint(
            'Warning: $operationName failed (attempt $attempt/$maxRetries), retrying in ${delay.inMilliseconds}ms: $e');
        await Future.delayed(delay);
        delay *= 2; // Exponential backoff
      }
    }
  }

  /// Get clipboard history
  static Future<List<ClipboardItemInfo>> getClipboardHistory(
      {int? limit}) async {
    if (!_ffiAvailable) return [];
    try {
      final items = api.getClipboardHistory(limit: limit);
      return items
          .map((item) => ClipboardItemInfo(
                id: item.id,
                contentType: item.contentType,
                preview: item.preview,
                sizeBytes: item.sizeBytes.toInt(),
                timestamp: item.timestamp.toInt(),
                sourceDevice: item.sourceDevice,
              ))
          .toList();
    } catch (e) {
      debugPrint('Warning: Failed to get clipboard history: $e');
      return [];
    }
  }

  /// Remove clipboard history item
  static Future<void> removeHistoryItem(String itemId) async {
    try {
      api.removeHistoryItem(itemId: itemId);
    } catch (e) {
      debugPrint('Warning: Failed to remove history item: $e');
    }
  }

  /// Clear clipboard history
  static Future<void> clearClipboardHistory() async {
    try {
      api.clearClipboardHistory();
    } catch (e) {
      debugPrint('Warning: Failed to clear clipboard history: $e');
    }
  }

  /// Get decrypted content from clipboard history item
  static Future<ClipboardContent?> getHistoryItemContent(String itemId) async {
    try {
      final content = api.getClipboardHistoryContent(itemId: itemId);
      return ClipboardContent(
        contentType: content.contentType,
        data: content.data,
      );
    } catch (e) {
      debugPrint('Warning: Failed to get history item content: $e');
      return null;
    }
  }

  // ============================================================================
  // Settings
  // ============================================================================

  /// Update settings in Rust core
  static Future<void> updateSettings({
    required bool autoSync,
    required bool syncText,
    required bool syncRichText,
    required bool syncImages,
    required bool syncFiles,
    required int maxFileSizeMb,
    required bool historyEnabled,
    required int historyDays,
    String? relayUrl,
  }) async {
    try {
      final settings = api.TossSettings(
        autoSync: autoSync,
        syncText: syncText,
        syncRichText: syncRichText,
        syncImages: syncImages,
        syncFiles: syncFiles,
        maxFileSizeMb: maxFileSizeMb,
        historyEnabled: historyEnabled,
        historyDays: historyDays,
        relayUrl: relayUrl,
      );
      api.updateSettings(settings: settings);
    } catch (e) {
      debugPrint('Warning: Failed to update settings: $e');
    }
  }

  // ============================================================================
  // Network
  // ============================================================================

  /// Start networking (discovery + connections)
  static Future<void> startNetwork() async {
    if (!_ffiAvailable) return;
    try {
      await api.startNetwork();
    } catch (e) {
      debugPrint('Warning: Failed to start network: $e');
    }
  }

  /// Poll for network events (non-blocking)
  static TossEvent? pollEvent() {
    if (!_ffiAvailable) return null;
    try {
      final event = api.pollEvent();
      if (event == null) return null;
      return TossEvent.fromApi(event);
    } catch (e) {
      debugPrint('Warning: Failed to poll event: $e');
      return null;
    }
  }

  /// Check if clipboard has changed since last check
  static bool checkClipboardChanged() {
    if (!_ffiAvailable) return false;
    try {
      return api.checkClipboardChanged();
    } catch (e) {
      debugPrint('Warning: Failed to check clipboard: $e');
      return false;
    }
  }

  /// Stop networking
  static Future<void> stopNetwork() async {
    try {
      await api.stopNetwork();
    } catch (e) {
      debugPrint('Warning: Failed to stop network: $e');
    }
  }

  // ============================================================================
  // Lifecycle
  // ============================================================================

  /// Shutdown the service
  static Future<void> shutdown() async {
    if (!_initialized) return;
    try {
      await api.shutdownToss();
    } catch (e) {
      debugPrint('Warning: Failed to shutdown Toss: $e');
    }
    _initialized = false;
    _deviceId = null;
  }
}
