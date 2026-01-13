//! System tray / menu bar service for desktop platforms

import 'package:tray_manager/tray_manager.dart';
import 'package:window_manager/window_manager.dart';

/// Service for managing system tray on desktop platforms
class TrayService {
  static final TrayService _instance = TrayService._internal();
  factory TrayService() => _instance;
  TrayService._internal();

  bool _initialized = false;

  /// Initialize system tray
  Future<bool> initialize() async {
    if (_initialized) return true;

    try {
      // Set up tray icon and menu
      await tray_manager.setIcon('assets/icons/tray_icon.png');
      
      // Create context menu
      final menu = Menu(
        items: [
          MenuItem(
            key: 'sync_toggle',
            label: 'Sync Enabled',
            type: MenuItemType.checkbox,
          ),
          MenuItem(
            key: 'separator1',
            type: MenuItemType.separator,
          ),
          MenuItem(
            key: 'recent_items',
            label: 'Recent Items',
            submenu: Menu(
              items: [
                MenuItem(
                  key: 'no_items',
                  label: 'No recent items',
                  enabled: false,
                ),
              ],
            ),
          ),
          MenuItem(
            key: 'separator2',
            type: MenuItemType.separator,
          ),
          MenuItem(
            key: 'show_window',
            label: 'Show Window',
          ),
          MenuItem(
            key: 'separator3',
            type: MenuItemType.separator,
          ),
          MenuItem(
            key: 'quit',
            label: 'Quit',
          ),
        ],
      );

      await tray_manager.setContextMenu(menu);

      // Set up tray click handler
      tray_manager.onTrayIconMouseDown((event) {
        if (event.button == MouseButton.left) {
          window_manager.show();
          window_manager.focus();
        }
      });

      // Set up menu click handler
      tray_manager.onMenuItemClick((menuItem) {
        _handleMenuClick(menuItem.key);
      });

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
        // Toggle sync - will be implemented with settings
        break;
      case 'show_window':
        window_manager.show();
        window_manager.focus();
        break;
      case 'quit':
        window_manager.close();
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
    
    await tray_manager.setToolTip(tooltip);
  }

  /// Update recent items in menu
  Future<void> updateRecentItems(List<String> items) async {
    if (!_initialized) return;

    // This would update the submenu with recent clipboard items
    // Implementation depends on menu structure
  }
}
