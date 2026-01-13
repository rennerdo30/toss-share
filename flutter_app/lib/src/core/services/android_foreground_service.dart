//! Android foreground service for clipboard monitoring
//!
//! Android 10+ restricts background clipboard access. This service provides:
//! - Foreground service with persistent notification
//! - Clipboard monitoring while service is running
//! - Workarounds for Android 10+ restrictions

import 'dart:io';
import 'package:flutter/services.dart';

/// Service for handling Android foreground service requirements
class AndroidForegroundService {
  static final AndroidForegroundService _instance =
      AndroidForegroundService._internal();
  factory AndroidForegroundService() => _instance;
  AndroidForegroundService._internal();

  static const MethodChannel _channel = MethodChannel('toss.android.service');

  /// Check if running on Android
  bool get isAndroid => Platform.isAndroid;

  /// Check if Android version is 10 or higher
  Future<bool> isAndroid10Plus() async {
    if (!isAndroid) return false;

    try {
      final version = await _channel.invokeMethod<int>('getAndroidVersion');
      return version != null && version >= 10;
    } catch (e) {
      return false;
    }
  }

  /// Start foreground service
  /// Required for clipboard access on Android 10+
  Future<bool> startForegroundService() async {
    if (!isAndroid) return false;

    try {
      final result = await _channel.invokeMethod<bool>('startForegroundService', {
        'title': 'Toss Clipboard Sync',
        'content': 'Monitoring clipboard for sync',
        'icon': '@mipmap/ic_launcher',
      });
      return result ?? false;
    } catch (e) {
      return false;
    }
  }

  /// Stop foreground service
  Future<bool> stopForegroundService() async {
    if (!isAndroid) return false;

    try {
      final result = await _channel.invokeMethod<bool>('stopForegroundService');
      return result ?? false;
    } catch (e) {
      return false;
    }
  }

  /// Update foreground service notification
  Future<void> updateNotification({
    String? title,
    String? content,
    bool? isSyncing,
  }) async {
    if (!isAndroid) return;

    try {
      await _channel.invokeMethod('updateNotification', {
        'title': title,
        'content': content,
        'isSyncing': isSyncing,
      });
    } catch (e) {
      // Notification update failed
    }
  }

  /// Check if foreground service is running
  Future<bool> isServiceRunning() async {
    if (!isAndroid) return false;

    try {
      final result = await _channel.invokeMethod<bool>('isServiceRunning');
      return result ?? false;
    } catch (e) {
      return false;
    }
  }

  /// Request notification permission (required for foreground service)
  Future<bool> requestNotificationPermission() async {
    if (!isAndroid) return false;

    try {
      final result = await _channel.invokeMethod<bool>('requestNotificationPermission');
      return result ?? false;
    } catch (e) {
      return false;
    }
  }

  /// Handle clipboard access restrictions
  /// Provides workarounds for Android 10+ clipboard restrictions
  Future<void> handleClipboardRestrictions() async {
    if (!isAndroid) return;

    final isAndroid10 = await isAndroid10Plus();
    if (!isAndroid10) return;

    // On Android 10+, we need to:
    // 1. Ensure foreground service is running
    // 2. Show persistent notification
    // 3. Monitor clipboard only when app is in foreground or service is active

    if (!await isServiceRunning()) {
      await startForegroundService();
    }
  }

  /// Initialize Android service
  Future<bool> initialize() async {
    if (!isAndroid) return false;

    // Request notification permission first
    await requestNotificationPermission();

    // Check if we need foreground service
    if (await isAndroid10Plus()) {
      return await startForegroundService();
    }

    return true;
  }
}
