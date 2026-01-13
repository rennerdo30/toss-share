//! Clipboard monitoring service for auto-sync
//!
//! Monitors clipboard changes and automatically syncs when auto-sync is enabled.

import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'toss_service.dart';
import 'notification_service.dart';
import '../providers/settings_provider.dart';
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
        // Send clipboard to all devices
        TossService.sendClipboard().catchError((e) {
          // Only log non-critical errors (like sync disabled for content type)
          if (!e.toString().contains('sync disabled') && 
              !e.toString().contains('too large')) {
            print('Warning: Failed to auto-sync clipboard: $e');
          }
        });
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
  }

  /// Handle network events
  void _handleEvent(TossEvent event, WidgetRef ref) {
    final settings = ref.read(settingsProvider);
    
    switch (event.type) {
      case 'clipboard_received':
        // Update clipboard history provider
        final itemData = event.data?['item'] as ClipboardItemInfo?;
        if (itemData != null) {
          // Add to history provider
          ref.read(clipboardHistoryProvider.notifier).addItem(
            _convertToClipboardItem(itemData),
          );
          
          // Show notification if enabled
          if (settings.showNotifications) {
            NotificationService().showClipboardReceived(itemData.preview);
          }
        }
        break;
      case 'device_connected':
        // Update devices provider
        ref.read(devicesProvider.notifier).refresh();
        break;
      case 'device_disconnected':
        // Update devices provider
        ref.read(devicesProvider.notifier).refresh();
        break;
      case 'pairing_request':
        // Show notification
        final deviceData = event.data?['device'] as DeviceInfo?;
        if (deviceData != null && settings.showNotifications) {
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
            print('Toss error: $message');
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
