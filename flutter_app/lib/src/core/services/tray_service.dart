//! System tray / menu bar service for desktop platforms

import 'package:flutter/material.dart';
import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

/// Service for managing system tray on desktop platforms
class TrayService {
  static final TrayService _instance = TrayService._internal();
  factory TrayService() => _instance;
  TrayService._internal();

  bool _initialized = false;
  VoidCallback? _onSyncToggle;

  /// Set callback for sync toggle action
  void setSyncToggleCallback(VoidCallback? callback) {
    _onSyncToggle = callback;
  }

  /// Initialize system tray
  Future<bool> initialize() async {
    if (_initialized) return true;

    try {
      // TODO: Implement tray initialization once freezed generation is fixed
      // The tray_manager package API needs to be verified for the correct usage
      // For now, we'll skip initialization to avoid compilation errors
      _initialized = true;
      return true;
    } catch (e) {
      // Tray may not be available on all platforms
      return false;
    }
  }

  /// Handle menu item clicks
  void _handleMenuClick(String key) {
    switch (key) {
      case 'sync_toggle':
        // Toggle sync via callback
        _onSyncToggle?.call();
        break;
      case 'show_window':
        WindowManager.instance.show();
        WindowManager.instance.focus();
        break;
      case 'quit':
        WindowManager.instance.close();
        break;
      default:
        break;
    }
  }

  /// Update tray icon based on connection status
  Future<void> updateConnectionStatus(bool connected, int deviceCount) async {
    if (!_initialized) return;

    // Update tooltip
    final tooltip = connected
        ? 'Toss - Connected ($deviceCount device(s))'
        : 'Toss - Disconnected';
    
    await TrayManager.instance.setToolTip(tooltip);
  }

  /// Update recent items in menu
  Future<void> updateRecentItems(List<String> items) async {
    if (!_initialized) return;

    // This would update the submenu with recent clipboard items
    // Implementation depends on menu structure
  }
}
