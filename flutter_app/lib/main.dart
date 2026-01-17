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
import 'src/core/services/logging_service.dart';
import 'src/core/providers/update_provider.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Catch Flutter framework errors
  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    debugPrint('Flutter error: ${details.exception}');
    debugPrint('Stack trace: ${details.stack}');
  };

  try {
    await _initializeApp();
  } catch (e, stack) {
    debugPrint('FATAL: Initialization failed: $e\n$stack');

    // Show error screen instead of blank window
    runApp(_ErrorApp(error: e.toString()));
    return;
  }
}

Future<void> _initializeApp() async {
  // Initialize logging service first for early error capture
  await LoggingService.initialize();
  LoggingService.info('Toss app starting...');

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

/// Error screen shown when initialization fails
class _ErrorApp extends StatelessWidget {
  final String error;

  const _ErrorApp({required this.error});

  String _getLogPath() {
    if (Platform.isWindows) {
      final appData = Platform.environment['LOCALAPPDATA'] ?? '';
      return '$appData\\toss\\logs';
    } else if (Platform.isMacOS) {
      final home = Platform.environment['HOME'] ?? '';
      return '$home/Library/Application Support/toss/logs';
    } else {
      final home = Platform.environment['HOME'] ?? '';
      return '$home/.local/share/toss/logs';
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(useMaterial3: true),
      home: Scaffold(
        backgroundColor: const Color(0xFF1A1A2E),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                Container(
                  padding: const EdgeInsets.all(20),
                  decoration: BoxDecoration(
                    color: Colors.red.shade900.withValues(alpha: 0.3),
                    shape: BoxShape.circle,
                  ),
                  child: const Icon(
                    Icons.error_outline,
                    color: Colors.redAccent,
                    size: 64,
                  ),
                ),
                const SizedBox(height: 24),
                const Text(
                  'Failed to Start',
                  style: TextStyle(
                    color: Colors.white,
                    fontSize: 28,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const SizedBox(height: 12),
                Text(
                  'Toss encountered an error during initialization.',
                  style: TextStyle(
                    color: Colors.white.withValues(alpha: 0.7),
                    fontSize: 16,
                  ),
                  textAlign: TextAlign.center,
                ),
                const SizedBox(height: 24),
                Container(
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: Colors.black.withValues(alpha: 0.3),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(
                      color: Colors.red.shade900.withValues(alpha: 0.5),
                    ),
                  ),
                  constraints: const BoxConstraints(maxWidth: 500),
                  child: SelectableText(
                    error,
                    style: TextStyle(
                      color: Colors.red.shade200,
                      fontFamily: 'monospace',
                      fontSize: 13,
                    ),
                    textAlign: TextAlign.center,
                  ),
                ),
                const SizedBox(height: 24),
                Text(
                  'Log files may be found at:',
                  style: TextStyle(
                    color: Colors.white.withValues(alpha: 0.5),
                    fontSize: 12,
                  ),
                ),
                const SizedBox(height: 4),
                SelectableText(
                  _getLogPath(),
                  style: TextStyle(
                    color: Colors.white.withValues(alpha: 0.7),
                    fontFamily: 'monospace',
                    fontSize: 12,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
