import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'dart:io';

import 'core/router/app_router.dart';
import 'core/services/tray_service.dart';
import 'core/services/clipboard_monitor_service.dart';
import 'core/services/toss_service.dart';
import 'core/providers/settings_provider.dart';
import 'core/providers/clipboard_provider.dart';
import 'core/providers/devices_provider.dart';
import 'shared/theme/app_theme.dart';

class TossApp extends ConsumerStatefulWidget {
  const TossApp({super.key});

  @override
  ConsumerState<TossApp> createState() => _TossAppState();
}

class _TossAppState extends ConsumerState<TossApp> {
  @override
  void initState() {
    super.initState();
    // Set up tray service callback after first frame
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
        TrayService().setSyncToggleCallback(() {
          final settings = ref.read(settingsProvider);
          ref.read(settingsProvider.notifier).updateAutoSync(!settings.autoSync);
        });
      }
      
      // Load clipboard history on app start
      ref.read(clipboardHistoryProvider.notifier).loadHistory();
      
      // Load devices on app start
      ref.read(devicesProvider.notifier).refresh();
      
      // Start network after initialization
      TossService.startNetwork().catchError((e) {
        print('Warning: Failed to start network: $e');
      });
    });
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
    ClipboardMonitorService().stopMonitoring();
    // Stop network on app disposal
    TossService.stopNetwork().catchError((e) {
      print('Warning: Failed to stop network: $e');
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
      SingleActivator(LogicalKeyboardKey.keyS, meta: isMacOS, control: !isMacOS): () {
        _syncClipboard(context);
      },

      // Cmd/Ctrl+H: Open clipboard history
      SingleActivator(LogicalKeyboardKey.keyH, meta: isMacOS, control: !isMacOS): () {
        _navigateTo('/history');
      },

      // Cmd/Ctrl+,: Open settings (standard macOS shortcut)
      SingleActivator(LogicalKeyboardKey.comma, meta: isMacOS, control: !isMacOS): () {
        _navigateTo('/settings');
      },

      // Cmd/Ctrl+P: Pair new device
      SingleActivator(LogicalKeyboardKey.keyP, meta: isMacOS, control: !isMacOS): () {
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
