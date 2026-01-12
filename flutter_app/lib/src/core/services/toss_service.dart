import 'package:path_provider/path_provider.dart';
import 'dart:io';
import 'dart:async';

// TODO: Import generated FFI bindings when flutter_rust_bridge is set up
// import '../rust/api.dart' as api;

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

    // TODO: Call Rust FFI init_toss()
    // await api.initToss(_dataDir!, _deviceName);
    // _deviceId = await api.getDeviceId();

    // Mock device ID for now
    _deviceId = 'mock-device-${DateTime.now().millisecondsSinceEpoch}';

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
    // TODO: Call Rust FFI set_device_name()
    // await api.setDeviceName(name);
  }

  // ============================================================================
  // Pairing
  // ============================================================================

  /// Start a new pairing session
  static Future<PairingInfo> startPairing() async {
    // TODO: Call Rust FFI start_pairing()
    // final info = await api.startPairing();
    // return PairingInfo(
    //   code: info.code,
    //   qrData: info.qrData,
    //   expiresAt: info.expiresAt,
    //   publicKey: info.publicKey,
    // );

    // Mock pairing info
    final code = '${DateTime.now().millisecondsSinceEpoch % 1000000}'.padLeft(6, '0');
    return PairingInfo(
      code: code,
      qrData: '{"v":1,"code":"$code","pk":"mock-key","name":"$_deviceName"}',
      expiresAt: DateTime.now().add(const Duration(minutes: 5)).millisecondsSinceEpoch,
      publicKey: 'mock-public-key',
    );
  }

  /// Complete pairing with QR code data
  static Future<DeviceInfo> completePairingQR(String qrData) async {
    // TODO: Call Rust FFI complete_pairing_qr()
    // return await api.completePairingQr(qrData);

    return const DeviceInfo(
      id: 'mock-paired-device',
      name: 'Paired Device',
      isOnline: false,
    );
  }

  /// Complete pairing with manual code
  static Future<DeviceInfo> completePairingCode(String code, List<int> publicKey) async {
    // TODO: Call Rust FFI complete_pairing_code()
    // return await api.completePairingCode(code, publicKey);

    return const DeviceInfo(
      id: 'mock-paired-device',
      name: 'Paired Device',
      isOnline: false,
    );
  }

  /// Cancel active pairing session
  static void cancelPairing() {
    // TODO: Call Rust FFI cancel_pairing()
    // api.cancelPairing();
  }

  // ============================================================================
  // Device Management
  // ============================================================================

  /// Get list of paired devices
  static Future<List<DeviceInfo>> getPairedDevices() async {
    // TODO: Call Rust FFI get_paired_devices()
    // return await api.getPairedDevices();
    return [];
  }

  /// Get list of connected devices
  static Future<List<DeviceInfo>> getConnectedDevices() async {
    // TODO: Call Rust FFI get_connected_devices()
    // return await api.getConnectedDevices();
    return [];
  }

  /// Remove a paired device
  static Future<void> removeDevice(String deviceId) async {
    // TODO: Call Rust FFI remove_device()
    // await api.removeDevice(deviceId);
  }

  // ============================================================================
  // Clipboard Operations
  // ============================================================================

  /// Get current clipboard content
  static Future<ClipboardItemInfo?> getCurrentClipboard() async {
    // TODO: Call Rust FFI get_current_clipboard()
    // return await api.getCurrentClipboard();
    return null;
  }

  /// Send current clipboard to all devices
  static Future<void> sendClipboard() async {
    // TODO: Call Rust FFI send_clipboard()
    // await api.sendClipboard();
  }

  /// Send text to all devices
  static Future<void> sendText(String text) async {
    // TODO: Call Rust FFI send_text()
    // await api.sendText(text);
  }

  // ============================================================================
  // Network
  // ============================================================================

  /// Start networking (discovery + connections)
  static Future<void> startNetwork() async {
    // TODO: Call Rust FFI start_network()
    // await api.startNetwork();
  }

  /// Stop networking
  static Future<void> stopNetwork() async {
    // TODO: Call Rust FFI stop_network()
    // await api.stopNetwork();
  }

  // ============================================================================
  // Lifecycle
  // ============================================================================

  /// Shutdown the service
  static Future<void> shutdown() async {
    if (!_initialized) return;
    // TODO: Call Rust FFI shutdown_toss()
    // await api.shutdownToss();
    _initialized = false;
    _deviceId = null;
  }
}
