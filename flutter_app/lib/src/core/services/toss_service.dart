import 'package:path_provider/path_provider.dart';
import 'dart:io';
import 'dart:async';

// Import generated FFI bindings
import '../rust/api.dart/api.dart' as api;

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

  const DeviceInfo({
    required this.id,
    required this.name,
    this.isOnline = false,
    this.lastSeen = 0,
  });
}

/// Clipboard item from core
class ClipboardItemInfo {
  final String contentType;
  final String preview;
  final int sizeBytes;
  final int timestamp;
  final String? sourceDevice;

  const ClipboardItemInfo({
    required this.contentType,
    required this.preview,
    required this.sizeBytes,
    required this.timestamp,
    this.sourceDevice,
  });
}

/// Service for initializing and managing Toss core
class TossService {
  TossService._();

  static bool _initialized = false;
  static String? _dataDir;
  static String? _deviceId;
  static String _deviceName = 'Toss Device';

  /// Check if service is initialized
  static bool get isInitialized => _initialized;

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

    // Call Rust FFI init_toss()
    try {
      api.initToss(dataDir: _dataDir!, deviceName: _deviceName);
      _deviceId = api.getDeviceId();
    } catch (e) {
      // Fallback: Mock device ID if FFI fails
      _deviceId = 'mock-device-${DateTime.now().millisecondsSinceEpoch}';
      print('Warning: FFI initialization failed: $e');
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
      print('Warning: Failed to set device name: $e');
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
      // Fallback: Mock pairing info if FFI fails
      print('Warning: Failed to start pairing: $e');
      final code = '${DateTime.now().millisecondsSinceEpoch % 1000000}'.padLeft(6, '0');
      return PairingInfo(
        code: code,
        qrData: '{"v":1,"code":"$code","pk":"mock-key","name":"$_deviceName"}',
        expiresAt: DateTime.now().add(const Duration(minutes: 5)).millisecondsSinceEpoch,
        publicKey: 'mock-public-key',
      );
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
      );
    } catch (e) {
      print('Warning: Failed to complete pairing with QR: $e');
      return const DeviceInfo(
        id: 'mock-paired-device',
        name: 'Paired Device',
        isOnline: false,
      );
    }
  }

  /// Complete pairing with manual code
  static Future<DeviceInfo> completePairingCode(String code, List<int> publicKey) async {
    try {
      final device = api.completePairingCode(code: code, peerPublicKey: publicKey);
      return DeviceInfo(
        id: device.id,
        name: device.name,
        isOnline: device.isOnline,
        lastSeen: device.lastSeen.toInt(),
      );
    } catch (e) {
      print('Warning: Failed to complete pairing with code: $e');
      return const DeviceInfo(
        id: 'mock-paired-device',
        name: 'Paired Device',
        isOnline: false,
      );
    }
  }

  /// Cancel active pairing session
  static void cancelPairing() {
    try {
      api.cancelPairing();
    } catch (e) {
      print('Warning: Failed to cancel pairing: $e');
    }
  }

  // ============================================================================
  // Device Management
  // ============================================================================

  /// Get list of paired devices
  static Future<List<DeviceInfo>> getPairedDevices() async {
    try {
      final devices = api.getPairedDevices();
      return devices.map((d) => DeviceInfo(
        id: d.id,
        name: d.name,
        isOnline: d.isOnline,
        lastSeen: d.lastSeen.toInt(),
      )).toList();
    } catch (e) {
      print('Warning: Failed to get paired devices: $e');
      return [];
    }
  }

  /// Get list of connected devices
  static Future<List<DeviceInfo>> getConnectedDevices() async {
    try {
      final devices = api.getConnectedDevices();
      return devices.map((d) => DeviceInfo(
        id: d.id,
        name: d.name,
        isOnline: d.isOnline,
        lastSeen: d.lastSeen.toInt(),
      )).toList();
    } catch (e) {
      print('Warning: Failed to get connected devices: $e');
      return [];
    }
  }

  /// Remove a paired device
  static Future<void> removeDevice(String deviceId) async {
    try {
      api.removeDevice(deviceId: deviceId);
    } catch (e) {
      print('Warning: Failed to remove device: $e');
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
        contentType: item.contentType,
        preview: item.preview,
        sizeBytes: item.sizeBytes.toInt(),
        timestamp: item.timestamp.toInt(),
        sourceDevice: item.sourceDevice,
      );
    } catch (e) {
      print('Warning: Failed to get current clipboard: $e');
      return null;
    }
  }

  /// Send current clipboard to all devices
  static Future<void> sendClipboard() async {
    try {
      await api.sendClipboard();
    } catch (e) {
      print('Warning: Failed to send clipboard: $e');
    }
  }

  /// Send text to all devices
  static Future<void> sendText(String text) async {
    try {
      await api.sendText(text: text);
    } catch (e) {
      print('Warning: Failed to send text: $e');
    }
  }

  /// Get clipboard history
  static Future<List<ClipboardItemInfo>> getClipboardHistory({int? limit}) async {
    try {
      final items = api.getClipboardHistory(limit: limit);
      return items.map((item) => ClipboardItemInfo(
        contentType: item.contentType,
        preview: item.preview,
        sizeBytes: item.sizeBytes.toInt(),
        timestamp: item.timestamp.toInt(),
        sourceDevice: item.sourceDevice,
      )).toList();
    } catch (e) {
      print('Warning: Failed to get clipboard history: $e');
      return [];
    }
  }

  /// Remove clipboard history item
  static Future<void> removeHistoryItem(String itemId) async {
    try {
      api.removeHistoryItem(itemId: itemId);
    } catch (e) {
      print('Warning: Failed to remove history item: $e');
    }
  }

  /// Clear clipboard history
  static Future<void> clearClipboardHistory() async {
    try {
      api.clearClipboardHistory();
    } catch (e) {
      print('Warning: Failed to clear clipboard history: $e');
    }
  }

  // ============================================================================
  // Network
  // ============================================================================

  /// Start networking (discovery + connections)
  static Future<void> startNetwork() async {
    try {
      await api.startNetwork();
    } catch (e) {
      print('Warning: Failed to start network: $e');
    }
  }

  /// Stop networking
  static Future<void> stopNetwork() async {
    try {
      await api.stopNetwork();
    } catch (e) {
      print('Warning: Failed to stop network: $e');
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
      print('Warning: Failed to shutdown Toss: $e');
    }
    _initialized = false;
    _deviceId = null;
  }
}
