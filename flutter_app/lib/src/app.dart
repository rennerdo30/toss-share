import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'dart:io';

import 'core/router/app_router.dart';
import 'core/services/tray_service.dart';
import 'core/services/clipboard_monitor_service.dart';
import 'core/services/toss_service.dart';
import 'core/services/ios_background_service.dart';
import 'core/services/websocket_service.dart';
import 'core/providers/settings_provider.dart';
import 'core/providers/clipboard_provider.dart';
import 'core/providers/devices_provider.dart';
import 'core/providers/websocket_provider.dart';
import 'shared/theme/app_theme.dart';

class TossApp extends ConsumerStatefulWidget {
  const TossApp({super.key});

  @override
  ConsumerState<TossApp> createState() => _TossAppState();
}

class _TossAppState extends ConsumerState<TossApp> with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    // Register for app lifecycle events
    WidgetsBinding.instance.addObserver(this);

    // Set up tray service callback after first frame
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
        TrayService().setSyncToggleCallback(() {
          final settings = ref.read(settingsProvider);
          ref
              .read(settingsProvider.notifier)
              .updateAutoSync(!settings.autoSync);
        });
      }

      // Initialize iOS background service
      if (Platform.isIOS) {
        _initializeIosBackgroundService();
      }

      // Load clipboard history on app start
      ref.read(clipboardHistoryProvider.notifier).loadHistory();

      // Load devices on app start and start status polling
      ref.read(devicesProvider.notifier).refresh().then((_) {
        // Start device status polling after devices are loaded
        final settings = ref.read(settingsProvider);
        ref.read(devicesProvider.notifier).startStatusPolling(
              relayUrl: settings.relayUrl,
            );
      });

      // Start network after initialization
      TossService.startNetwork().catchError((e) {
        debugPrint('Warning: Failed to start network: $e');
      });

      // Start WebSocket connection for real-time sync (if relay is configured)
      ref.read(webSocketProvider.notifier).connect().catchError((e) {
        debugPrint('Warning: Failed to start WebSocket: $e');
      });
    });
  }

  /// Initialize iOS-specific background service
  Future<void> _initializeIosBackgroundService() async {
    final iosService = IosBackgroundService();
    await iosService.initialize();

    // Set up sync callback
    iosService.setOnSyncRequested(() async {
      // Refresh clipboard history from network
      await ref.read(clipboardHistoryProvider.notifier).loadHistory();
      // Refresh devices
      await ref.read(devicesProvider.notifier).refresh();
    });
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    super.didChangeAppLifecycleState(state);

    // Handle WebSocket reconnection on app lifecycle changes
    switch (state) {
      case AppLifecycleState.resumed:
        // App came to foreground - try to reconnect WebSocket if disconnected
        final wsState = ref.read(webSocketProvider);
        if (!wsState.isConnected) {
          ref.read(webSocketProvider.notifier).connect();
        }
        break;
      case AppLifecycleState.paused:
        // App going to background - WebSocket will auto-reconnect if needed
        break;
      default:
        break;
    }

    if (Platform.isIOS) {
      final iosService = IosBackgroundService();
      switch (state) {
        case AppLifecycleState.resumed:
          // App came to foreground - sync clipboard
          iosService.syncOnForeground();
          break;
        case AppLifecycleState.paused:
          // App going to background - update widget
          iosService.updateWidget();
          break;
        default:
          break;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final router = ref.watch(appRouterProvider);
    final themeMode = ref.watch(themeModeProvider);
    final settings = ref.watch(settingsProvider);

    // Start or stop clipboard monitoring based on auto-sync setting
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (settings.autoSync) {
        ClipboardMonitorService().startMonitoring(ref);
      } else {
        ClipboardMonitorService().stopMonitoring();
      }
    });

    return MaterialApp.router(
      title: 'Toss',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light,
      darkTheme: AppTheme.dark,
      themeMode: themeMode,
      routerConfig: router,
      builder: (context, child) {
        // Wrap with keyboard shortcuts on desktop
        if (Platform.isWindows || Platform.isMacOS || Platform.isLinux) {
          return _KeyboardShortcutsWrapper(
            router: router,
            child: child ?? const SizedBox.shrink(),
          );
        }
        return child ?? const SizedBox.shrink();
      },
    );
  }

  @override
  void dispose() {
    // Remove lifecycle observer
    WidgetsBinding.instance.removeObserver(this);

    ClipboardMonitorService().stopMonitoring();

    // Disconnect WebSocket
    WebSocketService.instance.disconnect();

    // Note: Device status polling timer is automatically cleaned up
    // by the devicesProvider's onDispose callback

    // Clean up iOS background service
    if (Platform.isIOS) {
      IosBackgroundService().dispose();
    }

    // Stop network on app disposal
    TossService.stopNetwork().catchError((e) {
      debugPrint('Warning: Failed to stop network: $e');
    });
    super.dispose();
  }
}

/// Theme mode provider
final themeModeProvider = StateProvider<ThemeMode>((ref) => ThemeMode.system);

/// Keyboard shortcuts wrapper for desktop platforms
class _KeyboardShortcutsWrapper extends StatelessWidget {
  final GoRouter router;
  final Widget child;

  const _KeyboardShortcutsWrapper({
    required this.router,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: _buildShortcuts(context),
      child: Focus(
        autofocus: true,
        child: child,
      ),
    );
  }

  Map<ShortcutActivator, VoidCallback> _buildShortcuts(BuildContext context) {
    final isMacOS = Platform.isMacOS;

    return {
      // Cmd/Ctrl+S: Sync clipboard now
      SingleActivator(LogicalKeyboardKey.keyS,
          meta: isMacOS, control: !isMacOS): () {
        _syncClipboard(context);
      },

      // Cmd/Ctrl+H: Open clipboard history
      SingleActivator(LogicalKeyboardKey.keyH,
          meta: isMacOS, control: !isMacOS): () {
        _navigateTo('/history');
      },

      // Cmd/Ctrl+,: Open settings (standard macOS shortcut)
      SingleActivator(LogicalKeyboardKey.comma,
          meta: isMacOS, control: !isMacOS): () {
        _navigateTo('/settings');
      },

      // Cmd/Ctrl+P: Pair new device
      SingleActivator(LogicalKeyboardKey.keyP,
          meta: isMacOS, control: !isMacOS): () {
        _navigateTo('/pairing');
      },

      // Escape: Go back / close dialog
      const SingleActivator(LogicalKeyboardKey.escape): () {
        _goBack(context);
      },
    };
  }

  void _syncClipboard(BuildContext context) async {
    try {
      await TossService.sendClipboard();
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('Clipboard synced'),
            duration: Duration(seconds: 1),
          ),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Sync failed: $e')),
        );
      }
    }
  }

  void _navigateTo(String path) {
    final currentPath = router.routerDelegate.currentConfiguration.uri.path;
    if (currentPath != path) {
      router.push(path);
    }
  }

  void _goBack(BuildContext context) {
    // Check if we can pop (not on root route)
    if (router.canPop()) {
      router.pop();
    }
  }
}
