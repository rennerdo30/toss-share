//! Clipboard monitoring service for auto-sync
//!
//! Monitors clipboard changes and automatically syncs when auto-sync is enabled.

import 'dart:async';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'toss_service.dart';
import 'notification_service.dart';
import 'tray_service.dart';
import '../providers/settings_provider.dart' show settingsProvider, ConflictResolutionMode;
import '../providers/devices_provider.dart';
import '../providers/clipboard_provider.dart';
import '../models/clipboard_item.dart';

/// Service for monitoring clipboard changes and auto-syncing
class ClipboardMonitorService {
  static final ClipboardMonitorService _instance = ClipboardMonitorService._internal();
  factory ClipboardMonitorService() => _instance;
  ClipboardMonitorService._internal();

  Timer? _monitorTimer;
  Timer? _eventPollTimer;
  bool _isMonitoring = false;

  // Rate limiting: minimum time between clipboard syncs
  static const Duration _minSyncInterval = Duration(milliseconds: 500);
  DateTime? _lastSyncTime;
  bool _pendingSync = false;
  Timer? _rateLimitTimer;

  /// Start monitoring clipboard changes
  void startMonitoring(WidgetRef ref) {
    if (_isMonitoring) return;
    _isMonitoring = true;

    // Poll for clipboard changes every 250ms
    _monitorTimer = Timer.periodic(const Duration(milliseconds: 250), (_) {
      final settings = ref.read(settingsProvider);
      if (!settings.autoSync) {
        stopMonitoring();
        return;
      }

      // Check if clipboard changed
      if (TossService.checkClipboardChanged()) {
        _scheduleSync();
      }
    });

    // Poll for network events every 500ms
    _eventPollTimer = Timer.periodic(const Duration(milliseconds: 500), (_) {
      final event = TossService.pollEvent();
      if (event != null) {
        _handleEvent(event, ref);
      }
    });
  }

  /// Stop monitoring clipboard changes
  void stopMonitoring() {
    _isMonitoring = false;
    _monitorTimer?.cancel();
    _monitorTimer = null;
    _eventPollTimer?.cancel();
    _eventPollTimer = null;
    _rateLimitTimer?.cancel();
    _rateLimitTimer = null;
    _pendingSync = false;
  }

  /// Schedule a clipboard sync with rate limiting
  void _scheduleSync() {
    final now = DateTime.now();

    // Check if we can sync immediately
    if (_lastSyncTime == null ||
        now.difference(_lastSyncTime!) >= _minSyncInterval) {
      _performSync();
      return;
    }

    // Rate limited - schedule for later if not already pending
    if (!_pendingSync) {
      _pendingSync = true;
      final delay = _minSyncInterval - now.difference(_lastSyncTime!);
      _rateLimitTimer?.cancel();
      _rateLimitTimer = Timer(delay, () {
        if (_isMonitoring && _pendingSync) {
          _performSync();
        }
        _pendingSync = false;
      });
    }
  }

  /// Perform the actual clipboard sync
  void _performSync() {
    _lastSyncTime = DateTime.now();
    _pendingSync = false;

    TossService.sendClipboard().catchError((e) {
      // Only log non-critical errors (like sync disabled for content type)
      if (!e.toString().contains('sync disabled') &&
          !e.toString().contains('too large')) {
        debugPrint('Warning: Failed to auto-sync clipboard: $e');
      }
    });
  }

  /// Handle network events
  void _handleEvent(TossEvent event, WidgetRef ref) {
    final settings = ref.read(settingsProvider);
    
    switch (event.type) {
      case 'clipboard_received':
        // Update clipboard history provider
        final itemData = event.data?['item'] as ClipboardItemInfo?;
        if (itemData != null) {
          final newItem = _convertToClipboardItem(itemData);

          // Check per-device sync setting
          if (newItem.sourceDeviceId != null) {
            final devices = ref.read(devicesProvider);
            final sourceDevice = devices.where((d) => d.id == newItem.sourceDeviceId).firstOrNull;
            if (sourceDevice != null && !sourceDevice.syncEnabled) {
              debugPrint('Per-device sync disabled for ${sourceDevice.name}, ignoring clipboard');
              break;
            }
          }

          // Conflict resolution based on user preference
          final currentClipboard = ref.read(currentClipboardProvider);
          bool shouldUpdate;

          switch (settings.conflictResolution) {
            case ConflictResolutionMode.local:
              // Never update from remote - prefer local clipboard
              shouldUpdate = false;
              break;
            case ConflictResolutionMode.remote:
              // Always accept incoming clipboard
              shouldUpdate = true;
              break;
            case ConflictResolutionMode.newest:
            default:
              // Use timestamp-based resolution (default)
              shouldUpdate = currentClipboard == null ||
                  newItem.timestamp.isAfter(currentClipboard.timestamp);
              break;
          }

          if (shouldUpdate) {
            // Update current clipboard
            ref.read(currentClipboardProvider.notifier).update(newItem);

            // Add to history provider
            ref.read(clipboardHistoryProvider.notifier).addItem(newItem);

            // Show notification if enabled
            if (settings.showNotifications && settings.notifyOnClipboard) {
              NotificationService().showClipboardReceived(itemData.preview);
            }
          } else {
            debugPrint(
                'Conflict resolution (${settings.conflictResolution.name}): Ignoring clipboard');
            // Show conflict notification if enabled
            if (settings.showNotifications) {
              final sourceDevice = newItem.sourceDeviceName ?? 'Unknown device';
              NotificationService().showConflictDetected(sourceDevice);
            }
          }
        }
        break;
      case 'device_connected':
        // Update devices provider
        ref.read(devicesProvider.notifier).refresh();
        final connectedCount = ref.read(devicesProvider).where((d) => d.isOnline).length;
        // Update tray icon status on desktop
        if (Platform.isWindows || Platform.isMacOS || Platform.isLinux) {
          TrayService().updateConnectionStatus(true, connectedCount);
        }
        // Show connection notification if enabled
        if (settings.showNotifications && settings.notifyOnConnection) {
          NotificationService().showConnectionStatus(true, connectedCount);
        }
        break;
      case 'device_disconnected':
        // Update devices provider
        ref.read(devicesProvider.notifier).refresh();
        final remainingCount = ref.read(devicesProvider).where((d) => d.isOnline).length;
        // Update tray icon status on desktop
        if (Platform.isWindows || Platform.isMacOS || Platform.isLinux) {
          TrayService().updateConnectionStatus(remainingCount > 0, remainingCount);
        }
        // Show disconnection notification if enabled
        if (settings.showNotifications && settings.notifyOnConnection) {
          NotificationService().showConnectionStatus(remainingCount > 0, remainingCount);
        }
        break;
      case 'pairing_request':
        // Show notification
        final deviceData = event.data?['device'] as DeviceInfo?;
        if (deviceData != null && settings.showNotifications && settings.notifyOnPairing) {
          NotificationService().showPairingRequest(deviceData.name);
        }
        break;
      case 'error':
        // Show error notification
        final message = event.data?['message'] as String?;
        if (message != null) {
          if (settings.showNotifications) {
            NotificationService().showError(message);
          } else {
            debugPrint('Toss error: $message');
          }
        }
        break;
    }
  }

  ClipboardItem _convertToClipboardItem(ClipboardItemInfo info) {
    return ClipboardItem(
      id: info.id,
      contentType: _parseContentType(info.contentType),
      preview: info.preview,
      sizeBytes: info.sizeBytes,
      timestamp: DateTime.fromMillisecondsSinceEpoch(info.timestamp),
      sourceDeviceId: info.sourceDevice,
    );
  }

  ClipboardContentType _parseContentType(String type) {
    final lower = type.toLowerCase();
    if (lower.contains('text') && !lower.contains('rich')) {
      return ClipboardContentType.text;
    } else if (lower.contains('rich')) {
      return ClipboardContentType.richText;
    } else if (lower.contains('image')) {
      return ClipboardContentType.image;
    } else if (lower.contains('file')) {
      return ClipboardContentType.file;
    } else if (lower.contains('url')) {
      return ClipboardContentType.url;
    }
    return ClipboardContentType.text;
  }

  /// Check if monitoring is active
  bool get isMonitoring => _isMonitoring;
}
