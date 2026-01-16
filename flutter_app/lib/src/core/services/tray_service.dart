//! System tray / menu bar service for desktop platforms

import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

/// Service for managing system tray on desktop platforms
class TrayService with TrayListener {
  static final TrayService _instance = TrayService._internal();
  factory TrayService() => _instance;
  TrayService._internal();

  bool _initialized = false;
  VoidCallback? _onSyncToggle;
  bool _syncEnabled = true;

  /// Set callback for sync toggle action
  void setSyncToggleCallback(VoidCallback? callback) {
    _onSyncToggle = callback;
  }

  /// Initialize system tray
  Future<bool> initialize() async {
    if (_initialized) return true;

    // Only initialize on desktop platforms
    if (!Platform.isWindows && !Platform.isMacOS && !Platform.isLinux) {
      return false;
    }

    try {
      // Add listener for tray events
      trayManager.addListener(this);

      // Set tray icon
      // Note: On Windows, .ico files are preferred but .png works
      // On macOS/Linux, .png is standard
      String iconPath;
      if (Platform.isWindows) {
        // Windows prefers .ico but can use .png
        iconPath = 'assets/tray_icon.png';
      } else if (Platform.isMacOS) {
        // macOS uses template images for menu bar
        iconPath = 'assets/tray_icon.png';
      } else {
        // Linux
        iconPath = 'assets/tray_icon.png';
      }

      await trayManager.setIcon(iconPath);
      await trayManager.setToolTip('Toss - Clipboard Sharing');

      // Create context menu
      await _updateMenu();

      _initialized = true;
      return true;
    } catch (e) {
      // Tray may not be available on all platforms/configurations
      debugPrint('Warning: Failed to initialize system tray: $e');
      return false;
    }
  }

  /// Update the context menu
  Future<void> _updateMenu() async {
    final menu = Menu(
      items: [
        MenuItem(
          key: 'show_window',
          label: 'Show Toss',
        ),
        MenuItem.separator(),
        MenuItem(
          key: 'sync_toggle',
          label: _syncEnabled ? 'Pause Sync' : 'Resume Sync',
        ),
        MenuItem.separator(),
        MenuItem(
          key: 'quit',
          label: 'Quit Toss',
        ),
      ],
    );

    await trayManager.setContextMenu(menu);
  }

  /// Handle tray icon click (show context menu)
  @override
  void onTrayIconMouseDown() {
    // On Windows/Linux, show context menu on left click
    if (Platform.isWindows || Platform.isLinux) {
      trayManager.popUpContextMenu();
    }
  }

  /// Handle tray icon right click
  @override
  void onTrayIconRightMouseDown() {
    // Show context menu on right click (all platforms)
    trayManager.popUpContextMenu();
  }

  /// Handle tray icon double click
  @override
  void onTrayIconMouseUp() {
    // On macOS, double-click to show window
    if (Platform.isMacOS) {
      _showWindow();
    }
  }

  /// Handle menu item clicks
  @override
  void onTrayMenuItemClick(MenuItem menuItem) {
    switch (menuItem.key) {
      case 'show_window':
        _showWindow();
        break;
      case 'sync_toggle':
        _syncEnabled = !_syncEnabled;
        _onSyncToggle?.call();
        _updateMenu();
        break;
      case 'quit':
        _quit();
        break;
    }
  }

  /// Show and focus the main window
  void _showWindow() {
    windowManager.show();
    windowManager.focus();
  }

  /// Quit the application
  void _quit() {
    // Clean up tray before quitting
    destroy();
    windowManager.destroy();
  }

  /// Update tray icon based on connection status
  Future<void> updateConnectionStatus(bool connected, int deviceCount) async {
    if (!_initialized) return;

    // Update tooltip with connection status
    final tooltip = connected
        ? 'Toss - Connected ($deviceCount device${deviceCount != 1 ? 's' : ''})'
        : 'Toss - Disconnected';

    await trayManager.setToolTip(tooltip);
  }

  /// Update sync enabled state (for menu display)
  Future<void> updateSyncState(bool enabled) async {
    if (!_initialized) return;

    _syncEnabled = enabled;
    await _updateMenu();
  }

  /// Clean up tray resources
  Future<void> destroy() async {
    if (!_initialized) return;

    try {
      trayManager.removeListener(this);
      await trayManager.destroy();
      _initialized = false;
    } catch (e) {
      debugPrint('Warning: Failed to destroy tray: $e');
    }
  }
}
