//! Platform-specific permissions service

import 'package:flutter/services.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:toss/src/core/services/logging_service.dart';
import 'dart:io';

/// Platform channel for macOS accessibility permissions
const _permissionsChannel = MethodChannel('toss.app/permissions');

/// Service for managing platform-specific permissions
class PermissionsService {
  static final PermissionsService _instance = PermissionsService._internal();
  factory PermissionsService() => _instance;
  PermissionsService._internal();

  /// Check if clipboard access is available
  /// On macOS, this requires accessibility permissions
  Future<bool> checkClipboardAccess() async {
    if (Platform.isMacOS) {
      try {
        final bool? isTrusted =
            await _permissionsChannel.invokeMethod<bool>('checkAccessibilityPermission');
        return isTrusted ?? false;
      } on PlatformException catch (e) {
        LoggingService.warn('Failed to check accessibility permission: $e');
        return false;
      } on MissingPluginException {
        // Channel not available (e.g., running in debug without native code)
        LoggingService.debug('Permissions channel not available');
        return true;
      }
    }

    // On other platforms, clipboard access is typically available
    return true;
  }

  /// Request clipboard access permissions
  /// On macOS, this prompts the user and optionally opens System Preferences
  Future<bool> requestClipboardAccess() async {
    if (Platform.isMacOS) {
      try {
        // First try to request permission (will show system prompt if not granted)
        final bool? isTrusted =
            await _permissionsChannel.invokeMethod<bool>('requestAccessibilityPermission');

        if (isTrusted == true) {
          return true;
        }

        // If still not granted, open accessibility settings
        await openAccessibilitySettings();
        return false;
      } on PlatformException catch (e) {
        LoggingService.warn('Failed to request accessibility permission: $e');
        return false;
      } on MissingPluginException {
        LoggingService.debug('Permissions channel not available');
        return true;
      }
    }

    // On other platforms, clipboard access is typically granted automatically
    return true;
  }

  /// Check notification permissions
  Future<bool> checkNotificationPermission() async {
    final status = await Permission.notification.status;
    return status.isGranted;
  }

  /// Request notification permissions
  Future<bool> requestNotificationPermission() async {
    final status = await Permission.notification.request();
    return status.isGranted;
  }

  /// Check camera permissions (for QR code scanning)
  Future<bool> checkCameraPermission() async {
    final status = await Permission.camera.status;
    return status.isGranted;
  }

  /// Request camera permissions
  Future<bool> requestCameraPermission() async {
    final status = await Permission.camera.request();
    return status.isGranted;
  }

  /// Open system settings for accessibility (macOS)
  Future<void> openAccessibilitySettings() async {
    if (Platform.isMacOS) {
      try {
        await _permissionsChannel.invokeMethod<bool>('openAccessibilitySettings');
      } on PlatformException catch (e) {
        LoggingService.warn('Failed to open accessibility settings: $e');
      } on MissingPluginException {
        LoggingService.debug('Permissions channel not available');
      }
    }
  }

  /// Show permission denied dialog with instructions
  Future<void> showPermissionDeniedDialog() async {
    // This would show a dialog explaining how to grant permissions
    // Implementation depends on UI framework
  }
}
