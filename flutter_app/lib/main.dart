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

/// Write to a crash log file before Flutter is fully initialized
/// This helps diagnose crashes that happen very early
void _writeEarlyCrashLog(String message, [Object? error, StackTrace? stack]) {
  try {
    final logDir = Platform.isWindows
        ? '${Platform.environment['LOCALAPPDATA']}\\toss\\logs'
        : Platform.isMacOS
            ? '${Platform.environment['HOME']}/Library/Application Support/toss/logs'
            : '${Platform.environment['HOME']}/.local/share/toss/logs';

    final dir = Directory(logDir);
    if (!dir.existsSync()) {
      dir.createSync(recursive: true);
    }

    final logFile = File('$logDir${Platform.pathSeparator}early-crash.log');
    final timestamp = DateTime.now().toIso8601String();
    final content = '[$timestamp] $message\n${error ?? ''}\n${stack ?? ''}\n\n';

    logFile.writeAsStringSync(content, mode: FileMode.append, flush: true);
  } catch (_) {
    // Last resort - can't even write to file
  }
}

void main() async {
  // Write startup marker immediately
  _writeEarlyCrashLog('=== Toss starting ===');

  try {
    _writeEarlyCrashLog('Initializing Flutter binding...');
    WidgetsFlutterBinding.ensureInitialized();
    _writeEarlyCrashLog('Flutter binding initialized OK');
  } catch (e, stack) {
    _writeEarlyCrashLog('FATAL: Flutter binding failed', e, stack);
    rethrow;
  }

  // Catch Flutter framework errors
  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    debugPrint('Flutter error: ${details.exception}');
    debugPrint('Stack trace: ${details.stack}');
    _writeEarlyCrashLog('Flutter error', details.exception, details.stack);
  };

  try {
    await _initializeApp();
  } catch (e, stack) {
    debugPrint('FATAL: Initialization failed: $e\n$stack');
    _writeEarlyCrashLog('FATAL: Initialization failed', e, stack);

    // Show error screen instead of blank window
    runApp(_ErrorApp(error: e.toString()));
    return;
  }
}

Future<void> _initializeApp() async {
  // Initialize logging service first for early error capture
  _writeEarlyCrashLog('Step 1: Initializing LoggingService...');
  try {
    await LoggingService.initialize();
    LoggingService.info('Toss app starting...');
    _writeEarlyCrashLog('Step 1: LoggingService OK');
  } catch (e, stack) {
    _writeEarlyCrashLog('Step 1 FAILED: LoggingService', e, stack);
    rethrow;
  }

  // Initialize local storage
  _writeEarlyCrashLog('Step 2: Initializing StorageService (Hive)...');
  try {
    await StorageService.initialize();
    _writeEarlyCrashLog('Step 2: StorageService OK');
  } catch (e, stack) {
    _writeEarlyCrashLog('Step 2 FAILED: StorageService', e, stack);
    rethrow;
  }

  // Initialize update service (desktop only)
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    _writeEarlyCrashLog('Step 3: Initializing UpdateService...');
    try {
      await UpdateService.initialize();
      _writeEarlyCrashLog('Step 3: UpdateService OK');

      // Apply any pending updates before starting the UI
      if (await UpdateService.hasPendingUpdate()) {
        await UpdateService.applyPendingUpdate();
        // App may have restarted, continue if not
      }
    } catch (e, stack) {
      _writeEarlyCrashLog('Step 3 FAILED: UpdateService', e, stack);
      rethrow;
    }
  }

  // Initialize desktop window settings
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    _writeEarlyCrashLog('Step 4: Initializing WindowManager...');
    try {
      await windowManager.ensureInitialized();
      _writeEarlyCrashLog('Step 4a: WindowManager initialized');

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
      _writeEarlyCrashLog('Step 4b: Window ready');
    } catch (e, stack) {
      _writeEarlyCrashLog('Step 4 FAILED: WindowManager', e, stack);
      rethrow;
    }
  }

  // Initialize Toss core (Rust FFI)
  _writeEarlyCrashLog('Step 5: Initializing TossService (Rust FFI)...');
  try {
    await TossService.initialize();
    _writeEarlyCrashLog('Step 5: TossService OK (FFI available: ${TossService.isFfiAvailable})');
  } catch (e, stack) {
    _writeEarlyCrashLog('Step 5 FAILED: TossService', e, stack);
    rethrow;
  }

  // Initialize notification service
  _writeEarlyCrashLog('Step 6: Initializing NotificationService...');
  try {
    await NotificationService().initialize();
    _writeEarlyCrashLog('Step 6: NotificationService OK');
  } catch (e, stack) {
    _writeEarlyCrashLog('Step 6 FAILED: NotificationService', e, stack);
    rethrow;
  }

  // Initialize system tray (desktop only)
  if (Platform.isWindows || Platform.isLinux || Platform.isMacOS) {
    _writeEarlyCrashLog('Step 7: Initializing TrayService...');
    try {
      await TrayService().initialize();
      _writeEarlyCrashLog('Step 7: TrayService OK');
    } catch (e, stack) {
      _writeEarlyCrashLog('Step 7 FAILED: TrayService', e, stack);
      rethrow;
    }
  }

  // Create provider container for background update check
  _writeEarlyCrashLog('Step 8: Starting app UI...');
  final container = ProviderContainer();

  runApp(
    UncontrolledProviderScope(
      container: container,
      child: const TossApp(),
    ),
  );
  _writeEarlyCrashLog('Step 8: App UI started successfully');

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
