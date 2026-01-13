import 'package:hive_flutter/hive_flutter.dart';

/// Service for local storage using Hive
class StorageService {
  StorageService._();

  static const String _settingsBoxName = 'settings';
  static const String _devicesBoxName = 'devices';
  static const String _historyBoxName = 'clipboard_history';

  static Box? _settingsBox;
  static Box? _devicesBox;
  static Box? _historyBox;

  /// Initialize the storage service
  static Future<void> initialize() async {
    await Hive.initFlutter();

    _settingsBox = await Hive.openBox(_settingsBoxName);
    _devicesBox = await Hive.openBox(_devicesBoxName);
    _historyBox = await Hive.openBox(_historyBoxName);
  }

  /// Close all boxes
  static Future<void> close() async {
    await _settingsBox?.close();
    await _devicesBox?.close();
    await _historyBox?.close();
  }

  // ============================================================================
  // Settings Storage
  // ============================================================================

  /// Get a setting value
  static T? getSetting<T>(String key, {T? defaultValue}) {
    return _settingsBox?.get(key, defaultValue: defaultValue) as T?;
  }

  /// Set a setting value
  static Future<void> setSetting<T>(String key, T value) async {
    await _settingsBox?.put(key, value);
  }

  /// Remove a setting
  static Future<void> removeSetting(String key) async {
    await _settingsBox?.delete(key);
  }

  /// Get all settings as a map
  static Map<String, dynamic> getAllSettings() {
    if (_settingsBox == null) return {};
    return Map<String, dynamic>.from(_settingsBox!.toMap());
  }

  /// Save all settings at once
  static Future<void> saveAllSettings(Map<String, dynamic> settings) async {
    await _settingsBox?.putAll(settings);
  }

  // ============================================================================
  // Device Storage
  // ============================================================================

  /// Get a stored device
  static Map<String, dynamic>? getDevice(String deviceId) {
    final data = _devicesBox?.get(deviceId);
    if (data == null) return null;
    return Map<String, dynamic>.from(data);
  }

  /// Save a device
  static Future<void> saveDevice(String deviceId, Map<String, dynamic> device) async {
    await _devicesBox?.put(deviceId, device);
  }

  /// Remove a device
  static Future<void> removeDevice(String deviceId) async {
    await _devicesBox?.delete(deviceId);
  }

  /// Get all stored devices
  static List<Map<String, dynamic>> getAllDevices() {
    if (_devicesBox == null) return [];
    return _devicesBox!.values
        .map((v) => Map<String, dynamic>.from(v))
        .toList();
  }

  /// Clear all devices
  static Future<void> clearDevices() async {
    await _devicesBox?.clear();
  }

  // ============================================================================
  // Clipboard History Storage
  // ============================================================================

  /// Add an item to clipboard history
  static Future<void> addHistoryItem(String id, Map<String, dynamic> item) async {
    await _historyBox?.put(id, item);
  }

  /// Get a history item
  static Map<String, dynamic>? getHistoryItem(String id) {
    final data = _historyBox?.get(id);
    if (data == null) return null;
    return Map<String, dynamic>.from(data);
  }

  /// Remove a history item
  static Future<void> removeHistoryItem(String id) async {
    await _historyBox?.delete(id);
  }

  /// Get all history items
  static List<Map<String, dynamic>> getAllHistoryItems() {
    if (_historyBox == null) return [];
    return _historyBox!.values
        .map((v) => Map<String, dynamic>.from(v))
        .toList();
  }

  /// Clear clipboard history
  static Future<void> clearHistory() async {
    await _historyBox?.clear();
  }

  /// Prune old history items (keep only last N items)
  static Future<void> pruneHistory(int maxItems) async {
    if (_historyBox == null) return;

    final keys = _historyBox!.keys.toList();
    if (keys.length <= maxItems) return;

    // Sort by timestamp if available, otherwise by insertion order
    final items = <MapEntry<dynamic, Map<String, dynamic>>>[];
    for (final key in keys) {
      final data = _historyBox!.get(key);
      if (data != null) {
        items.add(MapEntry(key, Map<String, dynamic>.from(data)));
      }
    }

    // Sort by timestamp descending (newest first)
    items.sort((a, b) {
      final aTime = a.value['timestamp'] as int? ?? 0;
      final bTime = b.value['timestamp'] as int? ?? 0;
      return bTime.compareTo(aTime);
    });

    // Remove oldest items
    final toRemove = items.skip(maxItems).map((e) => e.key).toList();
    for (final key in toRemove) {
      await _historyBox!.delete(key);
    }
  }

  // ============================================================================
  // Utility Methods
  // ============================================================================

  /// Clear all stored data
  static Future<void> clearAll() async {
    await _settingsBox?.clear();
    await _devicesBox?.clear();
    await _historyBox?.clear();
  }

  /// Get storage statistics
  static Map<String, int> getStats() {
    return {
      'settings': _settingsBox?.length ?? 0,
      'devices': _devicesBox?.length ?? 0,
      'history': _historyBox?.length ?? 0,
    };
  }
}

/// Settings keys
class SettingsKeys {
  SettingsKeys._();

  static const String autoSync = 'auto_sync';
  static const String syncText = 'sync_text';
  static const String syncRichText = 'sync_rich_text';
  static const String syncImages = 'sync_images';
  static const String syncFiles = 'sync_files';
  static const String maxFileSizeMb = 'max_file_size_mb';
  static const String historyEnabled = 'history_enabled';
  static const String historyDays = 'history_days';
  static const String relayUrl = 'relay_url';
  static const String showNotifications = 'show_notifications';
  static const String themeMode = 'theme_mode';
  static const String deviceName = 'device_name';

  // Auto-updater keys
  static const String pendingUpdatePath = 'pending_update_path';
  static const String pendingUpdateSha = 'pending_update_sha';
  static const String currentBuildSha = 'current_build_sha';
  static const String lastUpdateCheck = 'last_update_check';
}
