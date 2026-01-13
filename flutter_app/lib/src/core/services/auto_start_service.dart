//! Auto-start service for desktop platforms
//!
//! Manages auto-start functionality for Windows, macOS, and Linux

import 'package:flutter/services.dart';
import 'dart:io';

/// Service for managing auto-start on desktop platforms
class AutoStartService {
  static const MethodChannel _channel = MethodChannel('toss.app/auto_start');

  /// Check if auto-start is supported on this platform
  static bool get isSupported => Platform.isWindows || Platform.isMacOS || Platform.isLinux;

  /// Enable auto-start
  static Future<bool> enable() async {
    if (!isSupported) return false;

    try {
      // Get the executable path
      final appPath = Platform.resolvedExecutable;
      final result = await _channel.invokeMethod<bool>(
        'enableAutoStart',
        {'appPath': appPath},
      );
      return result ?? false;
    } catch (e) {
      return false;
    }
  }

  /// Disable auto-start
  static Future<bool> disable() async {
    if (!isSupported) return false;

    try {
      final result = await _channel.invokeMethod<bool>('disableAutoStart');
      return result ?? false;
    } catch (e) {
      return false;
    }
  }

  /// Check if auto-start is currently enabled
  static Future<bool> isEnabled() async {
    if (!isSupported) return false;

    try {
      final result = await _channel.invokeMethod<bool>('isAutoStartEnabled');
      return result ?? false;
    } catch (e) {
      return false;
    }
  }
}
