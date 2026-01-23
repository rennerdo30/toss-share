import 'dart:convert';
import 'dart:io';

import 'package:archive/archive.dart';
import 'package:http/http.dart' as http;
import 'package:package_info_plus/package_info_plus.dart';
import 'package:path_provider/path_provider.dart';

import '../models/app_update.dart';
import 'storage_service.dart';
import 'logging_service.dart';

/// Service for checking and applying app updates from GitHub Releases
class UpdateService {
  UpdateService._();

  static const String _repoOwner = 'rennerdo30';
  static const String _repoName = 'toss-share';
  static const String _releaseTag = 'nightly';

  static String? _currentVersion;
  static String? _currentSha;

  /// Initialize the update service
  static Future<void> initialize() async {
    LoggingService.debug('UpdateService: Getting package info...');
    final packageInfo = await PackageInfo.fromPlatform();
    _currentVersion = packageInfo.version;
    LoggingService.debug('UpdateService: Version: $_currentVersion');

    // Load stored SHA of current installation
    _currentSha =
        StorageService.getSetting<String>(SettingsKeys.currentBuildSha);
    LoggingService.info('UpdateService: Initialized (SHA: ${_currentSha ?? "none"})');
  }

  /// Get current app version
  static String get currentVersion => _currentVersion ?? '0.0.0';

  /// Get current build SHA
  static String? get currentSha => _currentSha;

  /// Check for available updates from GitHub Releases
  static Future<AppUpdate?> checkForUpdate() async {
    try {
      final url = Uri.parse(
        'https://api.github.com/repos/$_repoOwner/$_repoName/releases/tags/$_releaseTag',
      );

      final response = await http.get(url, headers: {
        'Accept': 'application/vnd.github.v3+json',
      });

      if (response.statusCode != 200) {
        return null;
      }

      final json = jsonDecode(response.body) as Map<String, dynamic>;
      final assets = json['assets'] as List<dynamic>;

      // Find the correct asset for this platform
      final assetName = _getPlatformAssetName();
      final asset = assets.cast<Map<String, dynamic>>().firstWhere(
            (a) => a['name'] == assetName,
            orElse: () => <String, dynamic>{},
          );

      if (asset.isEmpty) {
        return null;
      }

      final update = AppUpdate.fromGitHubRelease(json, asset);

      // Check if this is a newer version by comparing SHA
      if (_currentSha != null && _currentSha == update.sha) {
        return null; // Already on this version
      }

      return update;
    } catch (e) {
      return null;
    }
  }

  /// Download update to temp directory
  static Future<File?> downloadUpdate(
    AppUpdate update, {
    void Function(double progress)? onProgress,
  }) async {
    try {
      final tempDir = await getTemporaryDirectory();
      final downloadPath = '${tempDir.path}/toss_update_${update.sha}';
      final assetName = _getPlatformAssetName();
      final downloadFile = File('$downloadPath/$assetName');

      // Create download directory
      await Directory(downloadPath).create(recursive: true);

      // Download with progress tracking
      final request = http.Request('GET', Uri.parse(update.downloadUrl));
      final streamedResponse = await request.send();

      if (streamedResponse.statusCode != 200) {
        return null;
      }

      final totalBytes = streamedResponse.contentLength ?? update.size;
      var receivedBytes = 0;

      final sink = downloadFile.openWrite();
      await for (final chunk in streamedResponse.stream) {
        sink.add(chunk);
        receivedBytes += chunk.length;
        onProgress?.call(receivedBytes / totalBytes);
      }
      await sink.close();

      // Verify download size
      final downloadedSize = await downloadFile.length();
      if (downloadedSize != update.size) {
        await downloadFile.delete();
        return null;
      }

      return downloadFile;
    } catch (e) {
      return null;
    }
  }

  /// Extract downloaded update archive
  static Future<Directory?> extractUpdate(File archiveFile) async {
    try {
      final bytes = await archiveFile.readAsBytes();
      final extractDir = Directory(
        archiveFile.path.replaceAll(RegExp(r'\.(zip|tar\.gz)$'), '_extracted'),
      );

      await extractDir.create(recursive: true);

      Archive archive;
      if (archiveFile.path.endsWith('.tar.gz')) {
        final decompressed = GZipDecoder().decodeBytes(bytes);
        archive = TarDecoder().decodeBytes(decompressed);
      } else {
        archive = ZipDecoder().decodeBytes(bytes);
      }

      for (final file in archive) {
        final filePath = '${extractDir.path}/${file.name}';
        if (file.isFile) {
          final outFile = File(filePath);
          await outFile.create(recursive: true);
          await outFile.writeAsBytes(file.content as List<int>);

          // Preserve executable permissions on Unix
          if (!Platform.isWindows && file.mode != 0) {
            await Process.run('chmod', ['+x', filePath]);
          }
        } else {
          await Directory(filePath).create(recursive: true);
        }
      }

      return extractDir;
    } catch (e) {
      return null;
    }
  }

  /// Stage update for installation on next restart
  static Future<bool> stageUpdate(File archiveFile, AppUpdate update) async {
    try {
      final extractedDir = await extractUpdate(archiveFile);
      if (extractedDir == null) {
        return false;
      }

      // Store the path to extracted update and metadata
      await StorageService.setSetting(
        SettingsKeys.pendingUpdatePath,
        extractedDir.path,
      );
      await StorageService.setSetting(
        SettingsKeys.pendingUpdateSha,
        update.sha,
      );

      return true;
    } catch (e) {
      return false;
    }
  }

  /// Check if there's a pending update to apply
  static Future<bool> hasPendingUpdate() async {
    final pendingPath = StorageService.getSetting<String>(
      SettingsKeys.pendingUpdatePath,
    );
    if (pendingPath == null) return false;

    final dir = Directory(pendingPath);
    return dir.existsSync();
  }

  /// Apply pending update (called early in app startup)
  static Future<bool> applyPendingUpdate() async {
    try {
      final pendingPath = StorageService.getSetting<String>(
        SettingsKeys.pendingUpdatePath,
      );
      final pendingSha = StorageService.getSetting<String>(
        SettingsKeys.pendingUpdateSha,
      );

      if (pendingPath == null || pendingSha == null) {
        return false;
      }

      final extractedDir = Directory(pendingPath);
      if (!extractedDir.existsSync()) {
        await _clearPendingUpdate();
        return false;
      }

      // Get current executable location
      final currentExe = Platform.resolvedExecutable;
      final currentDir = File(currentExe).parent;

      bool success = false;

      if (Platform.isMacOS) {
        success = await _applyMacOSUpdate(extractedDir, currentDir);
      } else if (Platform.isWindows) {
        success = await _applyWindowsUpdate(extractedDir, currentDir);
      } else if (Platform.isLinux) {
        success = await _applyLinuxUpdate(extractedDir, currentDir);
      }

      if (success) {
        // Update stored SHA to mark this version as current
        await StorageService.setSetting(
            SettingsKeys.currentBuildSha, pendingSha);
        await _clearPendingUpdate();

        // Clean up temp files
        try {
          await extractedDir.delete(recursive: true);
        } catch (_) {}
      }

      return success;
    } catch (e) {
      return false;
    }
  }

  /// Clear pending update markers
  static Future<void> _clearPendingUpdate() async {
    await StorageService.removeSetting(SettingsKeys.pendingUpdatePath);
    await StorageService.removeSetting(SettingsKeys.pendingUpdateSha);
  }

  /// Apply update on macOS
  static Future<bool> _applyMacOSUpdate(
    Directory extractedDir,
    Directory currentDir,
  ) async {
    try {
      // On macOS, we need to replace the .app bundle
      // Current executable is inside Toss.app/Contents/MacOS/
      final appBundle = currentDir.parent.parent.parent;

      // Find Toss.app in extracted directory
      final newApp = Directory('${extractedDir.path}/Toss.app');
      if (!newApp.existsSync()) {
        return false;
      }

      // Create backup
      final backupPath = '${appBundle.path}.backup';
      if (Directory(backupPath).existsSync()) {
        await Directory(backupPath).delete(recursive: true);
      }

      // Move current to backup, move new to current location
      await appBundle.rename(backupPath);
      await newApp.rename(appBundle.path);

      // Remove backup on success
      await Directory(backupPath).delete(recursive: true);

      return true;
    } catch (e) {
      return false;
    }
  }

  /// Apply update on Windows
  static Future<bool> _applyWindowsUpdate(
    Directory extractedDir,
    Directory currentDir,
  ) async {
    try {
      // On Windows, we need to replace the entire Release folder
      // Use a PowerShell script to handle file locking

      final scriptPath = '${extractedDir.path}/update.ps1';
      final script = '''
Start-Sleep -Seconds 2
Remove-Item -Path "${currentDir.path}\\*" -Recurse -Force
Copy-Item -Path "${extractedDir.path}\\*" -Destination "${currentDir.path}" -Recurse -Force
Start-Process "${currentDir.path}\\toss.exe"
''';

      await File(scriptPath).writeAsString(script);

      // Run the script detached and exit current process
      await Process.start(
        'powershell',
        ['-ExecutionPolicy', 'Bypass', '-File', scriptPath],
        mode: ProcessStartMode.detached,
      );

      // Exit current process to allow replacement
      exit(0);
    } catch (e) {
      return false;
    }
  }

  /// Apply update on Linux
  static Future<bool> _applyLinuxUpdate(
    Directory extractedDir,
    Directory currentDir,
  ) async {
    try {
      // On Linux, replace the bundle directory contents
      // Create backup
      final backupPath = '${currentDir.path}.backup';
      if (Directory(backupPath).existsSync()) {
        await Directory(backupPath).delete(recursive: true);
      }

      // Copy current to backup
      await Process.run('cp', ['-r', currentDir.path, backupPath]);

      // Copy new files over current
      await Process.run(
          'cp', ['-rf', '${extractedDir.path}/.', currentDir.path]);

      // Remove backup on success
      await Directory(backupPath).delete(recursive: true);

      return true;
    } catch (e) {
      return false;
    }
  }

  /// Get the correct asset name for current platform
  static String _getPlatformAssetName() {
    if (Platform.isMacOS) {
      return 'toss-macos-nightly.zip';
    } else if (Platform.isWindows) {
      return 'toss-windows-x64-nightly.zip';
    } else if (Platform.isLinux) {
      return 'toss-linux-x64-nightly.tar.gz';
    }
    throw UnsupportedError('Unsupported platform for auto-update');
  }

  /// Check if auto-update is supported on current platform
  static bool get isSupported {
    return Platform.isMacOS || Platform.isWindows || Platform.isLinux;
  }

  /// Get last update check time
  static DateTime? get lastCheckTime {
    final timestamp =
        StorageService.getSetting<int>(SettingsKeys.lastUpdateCheck);
    if (timestamp == null) return null;
    return DateTime.fromMillisecondsSinceEpoch(timestamp);
  }

  /// Update last check time
  static Future<void> updateLastCheckTime() async {
    await StorageService.setSetting(
      SettingsKeys.lastUpdateCheck,
      DateTime.now().millisecondsSinceEpoch,
    );
  }
}
