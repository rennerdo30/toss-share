//! Notification service for showing app notifications

import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:permission_handler/permission_handler.dart';

/// Service for managing app notifications
class NotificationService {
  static final NotificationService _instance = NotificationService._internal();
  factory NotificationService() => _instance;
  NotificationService._internal();

  final FlutterLocalNotificationsPlugin _notifications =
      FlutterLocalNotificationsPlugin();
  bool _initialized = false;

  /// Initialize notification service
  Future<bool> initialize() async {
    if (_initialized) return true;

    // Request notification permission (only on platforms that support it)
    // permission_handler doesn't support notification permission on macOS/Linux
    if (Platform.isAndroid || Platform.isIOS) {
      try {
        final status = await Permission.notification.request();
        if (!status.isGranted) {
          return false;
        }
      } catch (e) {
        debugPrint('Warning: Could not request notification permission: $e');
      }
    }

    // Initialize Android settings
    const androidSettings = AndroidInitializationSettings('@mipmap/ic_launcher');

    // Initialize iOS/macOS settings
    const darwinSettings = DarwinInitializationSettings(
      requestAlertPermission: true,
      requestBadgePermission: true,
      requestSoundPermission: true,
    );

    // Initialize Linux settings
    const linuxSettings = LinuxInitializationSettings(
      defaultActionName: 'Open notification',
    );

    const initSettings = InitializationSettings(
      android: androidSettings,
      iOS: darwinSettings,
      macOS: darwinSettings,
      linux: linuxSettings,
    );

    final initialized = await _notifications.initialize(
      initSettings,
      onDidReceiveNotificationResponse: _onNotificationTapped,
    );

    _initialized = initialized ?? false;
    return _initialized;
  }

  /// Handle notification tap
  void _onNotificationTapped(NotificationResponse response) {
    // Handle notification tap - can navigate to specific screen
    // This will be implemented based on notification payload
  }

  /// Show notification for pairing request
  Future<void> showPairingRequest(String deviceName) async {
    if (!_initialized) return;

    const androidDetails = AndroidNotificationDetails(
      'pairing',
      'Pairing Requests',
      channelDescription: 'Notifications for device pairing requests',
      importance: Importance.high,
      priority: Priority.high,
    );

    const iosDetails = DarwinNotificationDetails(
      presentAlert: true,
      presentBadge: true,
      presentSound: true,
    );

    const details = NotificationDetails(
      android: androidDetails,
      iOS: iosDetails,
    );

    await _notifications.show(
      1,
      'Pairing Request',
      'Device "$deviceName" wants to pair',
      details,
    );
  }

  /// Show notification for clipboard received
  Future<void> showClipboardReceived(String preview) async {
    if (!_initialized) return;

    const androidDetails = AndroidNotificationDetails(
      'clipboard',
      'Clipboard Updates',
      channelDescription: 'Notifications when clipboard is received from other devices',
      importance: Importance.low,
      priority: Priority.low,
    );

    const iosDetails = DarwinNotificationDetails(
      presentAlert: false,
      presentBadge: true,
      presentSound: false,
    );

    const details = NotificationDetails(
      android: androidDetails,
      iOS: iosDetails,
    );

    final truncatedPreview = preview.length > 50
        ? '${preview.substring(0, 50)}...'
        : preview;

    await _notifications.show(
      2,
      'Clipboard Received',
      truncatedPreview,
      details,
    );
  }

  /// Show notification for connection status
  Future<void> showConnectionStatus(bool connected, int deviceCount) async {
    if (!_initialized) return;

    const androidDetails = AndroidNotificationDetails(
      'connection',
      'Connection Status',
      channelDescription: 'Network connection status updates',
      importance: Importance.low,
      priority: Priority.low,
    );

    const iosDetails = DarwinNotificationDetails(
      presentAlert: false,
      presentBadge: false,
      presentSound: false,
    );

    const details = NotificationDetails(
      android: androidDetails,
      iOS: iosDetails,
    );

    final message = connected
        ? 'Connected to $deviceCount device(s)'
        : 'Disconnected from network';

    await _notifications.show(
      3,
      'Connection Status',
      message,
      details,
    );
  }

  /// Show error notification
  Future<void> showError(String message) async {
    if (!_initialized) return;

    const androidDetails = AndroidNotificationDetails(
      'errors',
      'Errors',
      channelDescription: 'Error notifications',
      importance: Importance.high,
      priority: Priority.high,
    );

    const iosDetails = DarwinNotificationDetails(
      presentAlert: true,
      presentBadge: true,
      presentSound: true,
    );

    const details = NotificationDetails(
      android: androidDetails,
      iOS: iosDetails,
    );

    await _notifications.show(
      4,
      'Error',
      message,
      details,
    );
  }

  /// Show notification for clipboard conflict
  Future<void> showConflictDetected(String sourceDevice) async {
    if (!_initialized) return;

    const androidDetails = AndroidNotificationDetails(
      'conflicts',
      'Clipboard Conflicts',
      channelDescription: 'Notifications when clipboard conflicts are detected',
      importance: Importance.low,
      priority: Priority.low,
    );

    const iosDetails = DarwinNotificationDetails(
      presentAlert: false,
      presentBadge: true,
      presentSound: false,
    );

    const details = NotificationDetails(
      android: androidDetails,
      iOS: iosDetails,
    );

    await _notifications.show(
      5,
      'Clipboard Conflict',
      'Ignored clipboard from $sourceDevice (local preference)',
      details,
    );
  }

  /// Cancel all notifications
  Future<void> cancelAll() async {
    await _notifications.cancelAll();
  }
}
