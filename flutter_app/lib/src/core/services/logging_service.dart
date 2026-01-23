import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:intl/intl.dart';

/// Service for file-based logging
/// Provides logging to both console and file for debugging purposes
class LoggingService {
  LoggingService._();

  static File? _logFile;
  static IOSink? _logSink;
  static bool _initialized = false;
  static final DateFormat _dateFormat = DateFormat('yyyy-MM-dd HH:mm:ss.SSS');

  /// Get the log directory path based on platform
  static Future<String> _getLogDirectory() async {
    if (Platform.isWindows) {
      // %LOCALAPPDATA%\toss\logs
      final appData = Platform.environment['LOCALAPPDATA'];
      if (appData != null) {
        return '$appData\\toss\\logs';
      }
      final docs = await getApplicationDocumentsDirectory();
      return '${docs.path}\\toss\\logs';
    } else if (Platform.isMacOS) {
      // ~/Library/Application Support/toss/logs
      final support = await getApplicationSupportDirectory();
      return '${support.path}/logs';
    } else if (Platform.isLinux) {
      // ~/.local/share/toss/logs
      final home = Platform.environment['HOME'];
      if (home != null) {
        return '$home/.local/share/toss/logs';
      }
      final docs = await getApplicationDocumentsDirectory();
      return '${docs.path}/toss/logs';
    } else {
      // iOS/Android - use documents directory
      final docs = await getApplicationDocumentsDirectory();
      return '${docs.path}/toss/logs';
    }
  }

  /// Initialize the logging service
  static Future<void> initialize() async {
    if (_initialized) return;

    try {
      final logDirPath = await _getLogDirectory();
      final logDir = Directory(logDirPath);

      if (!await logDir.exists()) {
        await logDir.create(recursive: true);
      }

      // Create log file with date in filename
      final today = DateFormat('yyyy-MM-dd').format(DateTime.now());
      final logFilePath = '${logDir.path}${Platform.pathSeparator}toss-$today.log';
      _logFile = File(logFilePath);

      // Open file for appending
      _logSink = _logFile!.openWrite(mode: FileMode.append);

      _initialized = true;

      // Log initialization
      log('LoggingService initialized', level: LogLevel.info);
      log('Log file: $logFilePath', level: LogLevel.info);

      // Clean up old log files (keep last 7 days)
      _cleanupOldLogs(logDir);
    } catch (e) {
      debugPrint('Warning: Failed to initialize LoggingService: $e');
    }
  }

  /// Clean up log files older than 7 days
  static Future<void> _cleanupOldLogs(Directory logDir) async {
    try {
      final now = DateTime.now();
      final files = await logDir.list().toList();

      for (final entity in files) {
        if (entity is File && entity.path.endsWith('.log')) {
          final stat = await entity.stat();
          final age = now.difference(stat.modified);
          if (age.inDays > 7) {
            await entity.delete();
            log('Deleted old log file: ${entity.path}', level: LogLevel.debug);
          }
        }
      }
    } catch (e) {
      debugPrint('Warning: Failed to cleanup old logs: $e');
    }
  }

  /// Log a message
  static void log(String message, {LogLevel level = LogLevel.debug}) {
    final timestamp = _dateFormat.format(DateTime.now());
    final levelStr = level.name.toUpperCase().padRight(5);
    final formattedMessage = '[$timestamp] [$levelStr] $message';

    // Always print to console in debug mode
    if (kDebugMode) {
      debugPrint(formattedMessage);
    }

    // ALWAYS write to file (debug AND release) if initialized
    if (_initialized && _logSink != null) {
      _logSink!.writeln(formattedMessage);
      // Flush immediately for crash debugging
      _logSink!.flush();
    }
  }

  /// Log an error with optional stack trace
  static void error(String message, [Object? error, StackTrace? stackTrace]) {
    log('$message${error != null ? ': $error' : ''}', level: LogLevel.error);
    if (stackTrace != null) {
      log('Stack trace:\n$stackTrace', level: LogLevel.error);
    }
  }

  /// Log a warning
  static void warn(String message) {
    log(message, level: LogLevel.warn);
  }

  /// Log info
  static void info(String message) {
    log(message, level: LogLevel.info);
  }

  /// Log debug message
  static void debug(String message) {
    log(message, level: LogLevel.debug);
  }

  /// Flush logs to disk
  static Future<void> flush() async {
    await _logSink?.flush();
  }

  /// Close the logging service
  static Future<void> close() async {
    if (!_initialized) return;

    log('LoggingService shutting down', level: LogLevel.info);
    await _logSink?.flush();
    await _logSink?.close();
    _logSink = null;
    _logFile = null;
    _initialized = false;
  }

  /// Get the current log file path
  static String? get logFilePath => _logFile?.path;

  /// Check if logging is initialized
  static bool get isInitialized => _initialized;
}

/// Log levels
enum LogLevel {
  debug,
  info,
  warn,
  error,
}
