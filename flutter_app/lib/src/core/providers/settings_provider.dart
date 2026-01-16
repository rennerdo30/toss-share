import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/storage_service.dart';
import '../services/toss_service.dart';

part 'settings_provider.g.dart';

/// Conflict resolution modes for clipboard sync
enum ConflictResolutionMode {
  /// Use the clipboard with the newest timestamp (default)
  newest,

  /// Always prefer local clipboard (ignore incoming)
  local,

  /// Always accept incoming clipboard
  remote,
}

/// App settings
class AppSettings {
  final bool autoSync;
  final bool syncText;
  final bool syncRichText;
  final bool syncImages;
  final bool syncFiles;
  final int maxFileSizeMb;
  final bool historyEnabled;
  final int historyDays;
  final String? relayUrl;
  final bool showNotifications;
  final bool notifyOnPairing;
  final bool notifyOnClipboard;
  final bool notifyOnConnection;
  final ConflictResolutionMode conflictResolution;

  const AppSettings({
    this.autoSync = true,
    this.syncText = true,
    this.syncRichText = true,
    this.syncImages = true,
    this.syncFiles = true,
    this.maxFileSizeMb = 50,
    this.historyEnabled = true,
    this.historyDays = 7,
    this.relayUrl,
    this.showNotifications = true,
    this.notifyOnPairing = true,
    this.notifyOnClipboard = true,
    this.notifyOnConnection = false,
    this.conflictResolution = ConflictResolutionMode.newest,
  });

  AppSettings copyWith({
    bool? autoSync,
    bool? syncText,
    bool? syncRichText,
    bool? syncImages,
    bool? syncFiles,
    int? maxFileSizeMb,
    bool? historyEnabled,
    int? historyDays,
    String? relayUrl,
    bool? showNotifications,
    bool? notifyOnPairing,
    bool? notifyOnClipboard,
    bool? notifyOnConnection,
    ConflictResolutionMode? conflictResolution,
  }) {
    return AppSettings(
      autoSync: autoSync ?? this.autoSync,
      syncText: syncText ?? this.syncText,
      syncRichText: syncRichText ?? this.syncRichText,
      syncImages: syncImages ?? this.syncImages,
      syncFiles: syncFiles ?? this.syncFiles,
      maxFileSizeMb: maxFileSizeMb ?? this.maxFileSizeMb,
      historyEnabled: historyEnabled ?? this.historyEnabled,
      historyDays: historyDays ?? this.historyDays,
      relayUrl: relayUrl ?? this.relayUrl,
      showNotifications: showNotifications ?? this.showNotifications,
      notifyOnPairing: notifyOnPairing ?? this.notifyOnPairing,
      notifyOnClipboard: notifyOnClipboard ?? this.notifyOnClipboard,
      notifyOnConnection: notifyOnConnection ?? this.notifyOnConnection,
      conflictResolution: conflictResolution ?? this.conflictResolution,
    );
  }
}

/// Settings provider
@Riverpod(keepAlive: true)
class Settings extends _$Settings {
  @override
  AppSettings build() {
    // Load settings from storage
    return AppSettings(
      autoSync: StorageService.getSetting<bool>(SettingsKeys.autoSync,
              defaultValue: true) ??
          true,
      syncText: StorageService.getSetting<bool>(SettingsKeys.syncText,
              defaultValue: true) ??
          true,
      syncRichText: StorageService.getSetting<bool>(SettingsKeys.syncRichText,
              defaultValue: true) ??
          true,
      syncImages: StorageService.getSetting<bool>(SettingsKeys.syncImages,
              defaultValue: true) ??
          true,
      syncFiles: StorageService.getSetting<bool>(SettingsKeys.syncFiles,
              defaultValue: true) ??
          true,
      maxFileSizeMb: StorageService.getSetting<int>(SettingsKeys.maxFileSizeMb,
              defaultValue: 50) ??
          50,
      historyEnabled: StorageService.getSetting<bool>(
              SettingsKeys.historyEnabled,
              defaultValue: true) ??
          true,
      historyDays: StorageService.getSetting<int>(SettingsKeys.historyDays,
              defaultValue: 7) ??
          7,
      relayUrl: StorageService.getSetting<String?>(SettingsKeys.relayUrl),
      showNotifications: StorageService.getSetting<bool>(
              SettingsKeys.showNotifications,
              defaultValue: true) ??
          true,
      notifyOnPairing: StorageService.getSetting<bool>(
              SettingsKeys.notifyOnPairing,
              defaultValue: true) ??
          true,
      notifyOnClipboard: StorageService.getSetting<bool>(
              SettingsKeys.notifyOnClipboard,
              defaultValue: true) ??
          true,
      notifyOnConnection: StorageService.getSetting<bool>(
              SettingsKeys.notifyOnConnection,
              defaultValue: false) ??
          false,
      conflictResolution: _parseConflictResolution(
        StorageService.getSetting<String>(SettingsKeys.conflictResolution,
            defaultValue: 'newest'),
      ),
    );
  }

  static ConflictResolutionMode _parseConflictResolution(String? value) {
    switch (value) {
      case 'local':
        return ConflictResolutionMode.local;
      case 'remote':
        return ConflictResolutionMode.remote;
      default:
        return ConflictResolutionMode.newest;
    }
  }

  void updateAutoSync(bool value) {
    state = state.copyWith(autoSync: value);
    _save();
  }

  void updateSyncText(bool value) {
    state = state.copyWith(syncText: value);
    _save();
  }

  void updateSyncRichText(bool value) {
    state = state.copyWith(syncRichText: value);
    _save();
  }

  void updateSyncImages(bool value) {
    state = state.copyWith(syncImages: value);
    _save();
  }

  void updateSyncFiles(bool value) {
    state = state.copyWith(syncFiles: value);
    _save();
  }

  void updateMaxFileSize(int mb) {
    state = state.copyWith(maxFileSizeMb: mb);
    _save();
  }

  void updateHistoryEnabled(bool value) {
    state = state.copyWith(historyEnabled: value);
    _save();
  }

  void updateHistoryDays(int days) {
    state = state.copyWith(historyDays: days);
    _save();
  }

  void updateRelayUrl(String? url) {
    state = state.copyWith(relayUrl: url);
    _save();
  }

  void updateShowNotifications(bool value) {
    state = state.copyWith(showNotifications: value);
    _save();
  }

  void updateNotifyOnPairing(bool value) {
    state = state.copyWith(notifyOnPairing: value);
    _save();
  }

  void updateNotifyOnClipboard(bool value) {
    state = state.copyWith(notifyOnClipboard: value);
    _save();
  }

  void updateNotifyOnConnection(bool value) {
    state = state.copyWith(notifyOnConnection: value);
    _save();
  }

  void updateConflictResolution(ConflictResolutionMode mode) {
    state = state.copyWith(conflictResolution: mode);
    _save();
  }

  void _save() {
    // Persist all settings to storage
    StorageService.setSetting(SettingsKeys.autoSync, state.autoSync);
    StorageService.setSetting(SettingsKeys.syncText, state.syncText);
    StorageService.setSetting(SettingsKeys.syncRichText, state.syncRichText);
    StorageService.setSetting(SettingsKeys.syncImages, state.syncImages);
    StorageService.setSetting(SettingsKeys.syncFiles, state.syncFiles);
    StorageService.setSetting(SettingsKeys.maxFileSizeMb, state.maxFileSizeMb);
    StorageService.setSetting(
        SettingsKeys.historyEnabled, state.historyEnabled);
    StorageService.setSetting(SettingsKeys.historyDays, state.historyDays);
    StorageService.setSetting(SettingsKeys.relayUrl, state.relayUrl);
    StorageService.setSetting(
        SettingsKeys.showNotifications, state.showNotifications);
    StorageService.setSetting(
        SettingsKeys.notifyOnPairing, state.notifyOnPairing);
    StorageService.setSetting(
        SettingsKeys.notifyOnClipboard, state.notifyOnClipboard);
    StorageService.setSetting(
        SettingsKeys.notifyOnConnection, state.notifyOnConnection);
    StorageService.setSetting(
        SettingsKeys.conflictResolution, state.conflictResolution.name);

    // Update Rust FFI settings
    TossService.updateSettings(
      autoSync: state.autoSync,
      syncText: state.syncText,
      syncRichText: state.syncRichText,
      syncImages: state.syncImages,
      syncFiles: state.syncFiles,
      maxFileSizeMb: state.maxFileSizeMb,
      historyEnabled: state.historyEnabled,
      historyDays: state.historyDays,
      relayUrl: state.relayUrl,
    );
  }
}
