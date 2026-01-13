import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
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
