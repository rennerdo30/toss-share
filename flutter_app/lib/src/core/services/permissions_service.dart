//! Platform-specific permissions service

import 'package:permission_handler/permission_handler.dart';
import 'dart:io';

/// Service for managing platform-specific permissions
class PermissionsService {
  static final PermissionsService _instance = PermissionsService._internal();
  factory PermissionsService() => _instance;
  PermissionsService._internal();

  /// Check if clipboard access is available
  /// On macOS, this requires accessibility permissions
  Future<bool> checkClipboardAccess() async {
    if (Platform.isMacOS) {
      // On macOS, we need to check accessibility permissions
      // This is typically done via native code or platform channels
      // For now, return true - full implementation requires native code
      return true;
    }

    // On other platforms, clipboard access is typically available
    return true;
  }

  /// Request clipboard access permissions
  /// On macOS, this opens System Preferences
  Future<bool> requestClipboardAccess() async {
    if (Platform.isMacOS) {
      // On macOS, we need to request accessibility permissions
      // This typically involves:
      // 1. Checking current permission status
      // 2. If denied, opening System Preferences to the accessibility section
      // 3. Providing instructions to the user

      // For now, return true - full implementation requires native code
      // that can check and request accessibility permissions
      return true;
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
      // Open System Preferences to Accessibility section
      // This requires platform-specific code
      // For now, this is a placeholder
    }
  }

  /// Show permission denied dialog with instructions
  Future<void> showPermissionDeniedDialog() async {
    // This would show a dialog explaining how to grant permissions
    // Implementation depends on UI framework
  }
}
