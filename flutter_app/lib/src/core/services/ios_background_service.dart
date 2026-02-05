//! iOS background clipboard service
//!
//! iOS has limited background clipboard access. This service provides:
//! - App extension for clipboard access
//! - Shortcuts integration (Siri Shortcuts)
//! - Widget for quick sync
//! - Foreground optimization with lifecycle handling
//!
//! iOS-specific limitations:
//! - iOS restricts clipboard access to foreground apps (iOS 14+)
//! - Background clipboard reading requires user interaction or Shortcuts
//! - Background fetch has limited time windows
//! - App extensions have separate memory space

import 'dart:async';
import 'dart:io';
import 'package:flutter/services.dart';

import 'toss_service.dart';
import 'logging_service.dart';

/// Callback type for clipboard sync operations
typedef ClipboardSyncCallback = Future<void> Function();

/// Callback type for handling received clipboard content
typedef ClipboardReceivedCallback = Future<void> Function(String preview);

/// iOS background service sync result
class IosSyncResult {
  final bool success;
  final String? error;
  final int? itemsSynced;

  const IosSyncResult({
    required this.success,
    this.error,
    this.itemsSynced,
  });
}

/// Service for handling iOS background limitations
class IosBackgroundService {
  static final IosBackgroundService _instance =
      IosBackgroundService._internal();
  factory IosBackgroundService() => _instance;
  IosBackgroundService._internal();

  static const MethodChannel _channel = MethodChannel('toss.ios.background');

  bool _initialized = false;
  bool _isInForeground = true;
  DateTime? _lastForegroundSync;
  ClipboardSyncCallback? _onSyncRequested;
  ClipboardReceivedCallback? _onClipboardReceived;

  /// Minimum interval between foreground syncs to avoid excessive syncing
  static const Duration _minForegroundSyncInterval = Duration(seconds: 2);

  /// Check if running on iOS
  bool get isIos => Platform.isIOS;

  /// Check if service is initialized
  bool get isInitialized => _initialized;

  /// Check if app is currently in foreground
  bool get isInForeground => _isInForeground;

  /// Set callback for sync requests
  void setOnSyncRequested(ClipboardSyncCallback? callback) {
    _onSyncRequested = callback;
  }

  /// Set callback for received clipboard content
  void setOnClipboardReceived(ClipboardReceivedCallback? callback) {
    _onClipboardReceived = callback;
  }

  /// Initialize iOS background service
  Future<bool> initialize() async {
    if (!isIos) return false;
    if (_initialized) return true;

    try {
      // Set up method call handler for native -> Dart calls
      _channel.setMethodCallHandler(_handleMethodCall);

      // Initialize iOS-specific background handlers
      // This would set up app extensions, shortcuts, and widgets
      final result = await _channel.invokeMethod<bool>('initialize');
      _initialized = result ?? false;

      if (_initialized) {
        LoggingService.info('iOS background service initialized');
        // Register default shortcuts
        await _registerDefaultShortcuts();
      }

      return _initialized;
    } catch (e) {
      // Service not available or not on iOS
      LoggingService.debug('iOS background service not available: $e');
      // Even if native init fails, mark as initialized for Dart-side functionality
      _initialized = true;
      return true;
    }
  }

  /// Handle method calls from native iOS code
  Future<dynamic> _handleMethodCall(MethodCall call) async {
    switch (call.method) {
      case 'onShortcutAction':
        final actionId = call.arguments['actionId'] as String?;
        if (actionId != null) {
          await handleShortcutAction(actionId);
        }
        return true;

      case 'onAppDidBecomeActive':
        await _handleAppDidBecomeActive();
        return true;

      case 'onAppWillResignActive':
        _handleAppWillResignActive();
        return true;

      case 'onBackgroundFetch':
        return await _handleBackgroundFetch();

      case 'onClipboardChanged':
        // Native code detected clipboard change
        await _onSyncRequested?.call();
        return true;

      case 'onClipboardReceived':
        // Native code received clipboard from network
        final preview = call.arguments['preview'] as String?;
        if (preview != null) {
          await _onClipboardReceived?.call(preview);
        }
        return true;

      default:
        return null;
    }
  }

  /// Register default Siri Shortcuts actions
  Future<void> _registerDefaultShortcuts() async {
    // Register common shortcuts
    await registerShortcutAction('sync_clipboard', 'Sync Clipboard');
    await registerShortcutAction('send_clipboard', 'Send Clipboard to Devices');
    await registerShortcutAction('receive_clipboard', 'Get Latest Clipboard');
  }

  /// Register iOS Shortcuts action
  /// This allows users to trigger clipboard sync via Siri Shortcuts
  Future<bool> registerShortcutAction(String actionId, String title) async {
    if (!isIos) return false;

    try {
      final result = await _channel.invokeMethod<bool>(
        'registerShortcut',
        {'actionId': actionId, 'title': title},
      );
      LoggingService.debug('Registered shortcut: $actionId');
      return result ?? false;
    } catch (e) {
      LoggingService.debug('Failed to register shortcut $actionId: $e');
      return false;
    }
  }

  /// Handle shortcut action invocation
  /// Called when user triggers a Siri Shortcut
  Future<void> handleShortcutAction(String actionId) async {
    if (!isIos) return;

    LoggingService.info('Handling iOS shortcut action: $actionId');

    try {
      switch (actionId) {
        case 'sync_clipboard':
        case 'send_clipboard':
          // Send current clipboard to all paired devices
          await TossService.sendClipboard();
          LoggingService.info('Shortcut: Clipboard sent to devices');
          // Update widget to reflect sync
          await updateWidget();
          break;

        case 'receive_clipboard':
          // Trigger sync to receive latest clipboard from devices
          // This calls the registered callback which should refresh from network
          await _onSyncRequested?.call();
          LoggingService.info('Shortcut: Requested clipboard from devices');
          break;

        default:
          // Handle custom shortcut actions
          LoggingService.debug('Unknown shortcut action: $actionId');
          // Still attempt to sync as a fallback
          await _onSyncRequested?.call();
      }

      // Notify native side that action completed
      await _channel.invokeMethod('shortcutActionCompleted', {
        'actionId': actionId,
        'success': true,
      });
    } catch (e) {
      LoggingService.warn('Shortcut action failed: $e');
      // Notify native side of failure
      await _channel.invokeMethod('shortcutActionCompleted', {
        'actionId': actionId,
        'success': false,
        'error': e.toString(),
      });
    }
  }

  /// Update widget with current clipboard status
  Future<void> updateWidget() async {
    if (!isIos) return;

    try {
      // Get current clipboard info for widget display
      final clipboard = await TossService.getCurrentClipboard();
      await _channel.invokeMethod('updateWidget', {
        'hasContent': clipboard != null,
        'preview': clipboard?.preview ?? '',
        'contentType': clipboard?.contentType ?? 'text',
        'timestamp': clipboard?.timestamp ?? 0,
      });
    } catch (e) {
      // Widget not available or update failed
      LoggingService.debug('Widget update failed: $e');
    }
  }

  /// Sync clipboard when app comes to foreground
  /// iOS allows clipboard access when app is in foreground
  Future<IosSyncResult> syncOnForeground() async {
    if (!isIos) {
      return const IosSyncResult(success: false, error: 'Not running on iOS');
    }

    // Rate limit foreground syncs
    final now = DateTime.now();
    if (_lastForegroundSync != null &&
        now.difference(_lastForegroundSync!) < _minForegroundSyncInterval) {
      LoggingService.debug('Foreground sync skipped - too soon');
      return const IosSyncResult(
        success: true,
        error: 'Sync skipped - rate limited',
        itemsSynced: 0,
      );
    }

    _lastForegroundSync = now;

    try {
      LoggingService.info('Syncing clipboard on foreground');

      // Check if clipboard has changed since last check
      final hasChanged = TossService.checkClipboardChanged();

      if (hasChanged) {
        // Send updated clipboard to devices
        await TossService.sendClipboard();
        LoggingService.info('Foreground sync: sent updated clipboard');

        // Update widget
        await updateWidget();

        return const IosSyncResult(success: true, itemsSynced: 1);
      } else {
        // No local changes, check for incoming content via callback
        await _onSyncRequested?.call();
        return const IosSyncResult(success: true, itemsSynced: 0);
      }
    } catch (e) {
      LoggingService.warn('Foreground sync failed: $e');
      return IosSyncResult(success: false, error: e.toString());
    }
  }

  /// Handle app becoming active (returned to foreground)
  Future<void> _handleAppDidBecomeActive() async {
    _isInForeground = true;
    LoggingService.debug('iOS app became active');

    // Sync clipboard when returning to foreground
    await syncOnForeground();
  }

  /// Handle app about to resign active (going to background)
  void _handleAppWillResignActive() {
    _isInForeground = false;
    LoggingService.debug('iOS app will resign active');

    // Update widget before going to background
    updateWidget();
  }

  /// Handle iOS background fetch
  /// iOS grants limited time for background fetch operations
  Future<Map<String, dynamic>> _handleBackgroundFetch() async {
    LoggingService.info('iOS background fetch triggered');

    try {
      // Check for pending clipboard content from devices
      // Note: We cannot read local clipboard in background on iOS 14+
      await _onSyncRequested?.call();

      // Update widget with latest status
      await updateWidget();

      return {'success': true, 'newData': true};
    } catch (e) {
      LoggingService.warn('Background fetch failed: $e');
      return {'success': false, 'error': e.toString()};
    }
  }

  /// Setup app extension for clipboard access
  /// App extensions can access clipboard even when main app is in background
  Future<bool> setupAppExtension() async {
    if (!isIos) return false;

    try {
      final result = await _channel.invokeMethod<bool>('setupExtension');
      if (result == true) {
        LoggingService.info('iOS app extension setup successful');
      }
      return result ?? false;
    } catch (e) {
      LoggingService.debug('App extension setup failed: $e');
      return false;
    }
  }

  /// Check iOS version for clipboard restrictions
  /// iOS 14+ restricts background clipboard access
  Future<bool> hasClipboardRestrictions() async {
    if (!isIos) return false;

    try {
      final version = await _channel.invokeMethod<int>('getIOSVersion');
      // iOS 14+ has clipboard access restrictions
      return version != null && version >= 14;
    } catch (e) {
      // Assume restrictions apply if we can't check
      return true;
    }
  }

  /// Request clipboard access permission
  /// On iOS 14+, reading clipboard shows a user notification
  Future<bool> requestClipboardAccess() async {
    if (!isIos) return true;

    try {
      // Attempt to read clipboard to trigger permission prompt
      final result = await _channel.invokeMethod<bool>('requestClipboardAccess');
      return result ?? true;
    } catch (e) {
      // Clipboard access is allowed by default, just notifies user
      return true;
    }
  }

  /// Notify user about iOS clipboard limitations
  /// Returns a user-friendly message explaining the restrictions
  String getClipboardLimitationsMessage() {
    return '''
iOS Clipboard Limitations:
• Clipboard can only be read when the app is in the foreground
• iOS shows a notification when apps read the clipboard
• Use Siri Shortcuts for quick clipboard sync
• Background sync receives content but cannot read local clipboard
''';
  }

  /// Configure background fetch interval
  Future<bool> configureBackgroundFetch({
    Duration minimumInterval = const Duration(minutes: 15),
  }) async {
    if (!isIos) return false;

    try {
      final result = await _channel.invokeMethod<bool>('configureBackgroundFetch', {
        'minimumIntervalSeconds': minimumInterval.inSeconds,
      });
      return result ?? false;
    } catch (e) {
      LoggingService.debug('Background fetch configuration failed: $e');
      return false;
    }
  }

  /// Clean up service resources
  void dispose() {
    _onSyncRequested = null;
    _onClipboardReceived = null;
    _initialized = false;
  }
}
