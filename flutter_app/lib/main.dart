import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:window_manager/window_manager.dart';
import 'dart:io';

import 'src/app.dart';
import 'src/core/services/toss_service.dart';
import 'src/core/services/storage_service.dart';
import 'src/core/services/update_service.dart';
import 'src/core/services/tray_service.dart';
import 'src/core/services/notification_service.dart';
import 'src/core/providers/update_provider.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize local storage
  await StorageService.initialize();

  // Initialize update service (desktop only)
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    await UpdateService.initialize();

    // Apply any pending updates before starting the UI
    if (await UpdateService.hasPendingUpdate()) {
      await UpdateService.applyPendingUpdate();
      // App may have restarted, continue if not
    }
  }

  // Initialize desktop window settings
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    await windowManager.ensureInitialized();

    const windowOptions = WindowOptions(
      size: Size(960, 700),
      minimumSize: Size(400, 500),
      center: true,
      backgroundColor: Colors.transparent,
      skipTaskbar: false,
      titleBarStyle: TitleBarStyle.hidden,
      title: 'Toss',
    );

    await windowManager.waitUntilReadyToShow(windowOptions, () async {
      await windowManager.show();
      await windowManager.focus();
    });
  }

  // Initialize Toss core
  await TossService.initialize();

  // Initialize notification service
  await NotificationService().initialize();

  // Initialize system tray (desktop only)
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    await TrayService().initialize();
  }

  // Create provider container for background update check
  final container = ProviderContainer();

  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const TossApp(),
    ),
  );

  // Check for updates in background after app starts (desktop only)
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    // Delay to let the UI initialize first
    Future.delayed(const Duration(seconds: 5), () {
      container.read(updateProvider.notifier).checkForUpdates();
    });
  }
}
