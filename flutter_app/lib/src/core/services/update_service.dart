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
    LoggingService.debug('UpdateService: Checking for updates...');
    try {
      final url = Uri.parse(
        'https://api.github.com/repos/$_repoOwner/$_repoName/releases/tags/$_releaseTag',
      );

      LoggingService.debug('UpdateService: Fetching release info from $url');
      final response = await http.get(url, headers: {
        'Accept': 'application/vnd.github.v3+json',
      });

      if (response.statusCode != 200) {
        LoggingService.warn('UpdateService: GitHub API returned ${response.statusCode}');
        return null;
      }

      final json = jsonDecode(response.body) as Map<String, dynamic>;
      final assets = json['assets'] as List<dynamic>;
      LoggingService.debug('UpdateService: Found ${assets.length} release assets');

      // Find the correct asset for this platform
      final assetName = _getPlatformAssetName();
      LoggingService.debug('UpdateService: Looking for asset: $assetName');

      final asset = assets.cast<Map<String, dynamic>>().firstWhere(
            (a) => a['name'] == assetName,
            orElse: () => <String, dynamic>{},
          );

      if (asset.isEmpty) {
        LoggingService.warn('UpdateService: No matching asset found for platform');
        return null;
      }

      final update = AppUpdate.fromGitHubRelease(json, asset);
      LoggingService.info('UpdateService: Found release - SHA: ${update.sha}, Size: ${update.formattedSize}');

      // Check if this is a newer version by comparing SHA
      LoggingService.debug('UpdateService: Comparing SHAs - current: $_currentSha, remote: ${update.sha}');
      if (_currentSha != null && _currentSha == update.sha) {
        LoggingService.info('UpdateService: Already on latest version');
        return null; // Already on this version
      }

      LoggingService.info('UpdateService: Update available!');
      return update;
    } catch (e, stack) {
      LoggingService.error('UpdateService: Failed to check for updates', e, stack);
      return null;
    }
  }

  /// Download update to temp directory
  static Future<File?> downloadUpdate(
    AppUpdate update, {
    void Function(double progress)? onProgress,
  }) async {
    LoggingService.info('UpdateService: Starting download of ${update.formattedSize}');
    try {
      final tempDir = await getTemporaryDirectory();
      final downloadPath = '${tempDir.path}/toss_update_${update.sha.substring(0, 8)}';
      final assetName = _getPlatformAssetName();
      final downloadFile = File('$downloadPath/$assetName');

      LoggingService.debug('UpdateService: Download path: ${downloadFile.path}');

      // Create download directory
      await Directory(downloadPath).create(recursive: true);

      // Download with progress tracking
      final request = http.Request('GET', Uri.parse(update.downloadUrl));
      final streamedResponse = await request.send();

      if (streamedResponse.statusCode != 200) {
        LoggingService.error('UpdateService: Download failed with status ${streamedResponse.statusCode}');
        return null;
      }

      final totalBytes = streamedResponse.contentLength ?? update.size;
      var receivedBytes = 0;
      var lastLoggedPercent = 0;

      final sink = downloadFile.openWrite();
      await for (final chunk in streamedResponse.stream) {
        sink.add(chunk);
        receivedBytes += chunk.length;
        final progress = receivedBytes / totalBytes;
        onProgress?.call(progress);

        // Log progress every 10%
        final percent = (progress * 100).floor();
        if (percent >= lastLoggedPercent + 10) {
          LoggingService.debug('UpdateService: Download progress: $percent%');
          lastLoggedPercent = percent;
        }
      }
      await sink.close();

      // Verify download size
      final downloadedSize = await downloadFile.length();
      LoggingService.debug('UpdateService: Downloaded $downloadedSize bytes, expected ${update.size}');

      if (downloadedSize != update.size) {
        LoggingService.error('UpdateService: Size mismatch! Downloaded: $downloadedSize, Expected: ${update.size}');
        await downloadFile.delete();
        return null;
      }

      LoggingService.info('UpdateService: Download complete: ${downloadFile.path}');
      return downloadFile;
    } catch (e, stack) {
      LoggingService.error('UpdateService: Download failed', e, stack);
      return null;
    }
  }

  /// Extract downloaded update archive
  static Future<Directory?> extractUpdate(File archiveFile) async {
    LoggingService.info('UpdateService: Extracting ${archiveFile.path}');
    try {
      final bytes = await archiveFile.readAsBytes();
      final extractDir = Directory(
        archiveFile.path.replaceAll(RegExp(r'\.(zip|tar\.gz)$'), '_extracted'),
      );

      await extractDir.create(recursive: true);
      LoggingService.debug('UpdateService: Extracting to ${extractDir.path}');

      Archive archive;
      if (archiveFile.path.endsWith('.tar.gz')) {
        LoggingService.debug('UpdateService: Decompressing tar.gz...');
        final decompressed = GZipDecoder().decodeBytes(bytes);
        archive = TarDecoder().decodeBytes(decompressed);
      } else {
        LoggingService.debug('UpdateService: Extracting zip...');
        archive = ZipDecoder().decodeBytes(bytes);
      }

      LoggingService.debug('UpdateService: Archive contains ${archive.length} entries');
      var fileCount = 0;

      for (final file in archive) {
        final filePath = '${extractDir.path}/${file.name}';
        if (file.isFile) {
          final outFile = File(filePath);
          await outFile.create(recursive: true);
          await outFile.writeAsBytes(file.content as List<int>);
          fileCount++;

          // Preserve executable permissions on Unix
          if (!Platform.isWindows && file.mode != 0) {
            await Process.run('chmod', ['+x', filePath]);
          }
        } else {
          await Directory(filePath).create(recursive: true);
        }
      }

      LoggingService.info('UpdateService: Extracted $fileCount files to ${extractDir.path}');
      return extractDir;
    } catch (e, stack) {
      LoggingService.error('UpdateService: Extraction failed', e, stack);
      return null;
    }
  }

  /// Stage update for installation on next restart
  static Future<bool> stageUpdate(File archiveFile, AppUpdate update) async {
    LoggingService.info('UpdateService: Staging update for SHA: ${update.sha}');
    try {
      final extractedDir = await extractUpdate(archiveFile);
      if (extractedDir == null) {
        LoggingService.error('UpdateService: Failed to extract archive');
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

      LoggingService.info('UpdateService: Update staged at ${extractedDir.path}');
      return true;
    } catch (e, stack) {
      LoggingService.error('UpdateService: Failed to stage update', e, stack);
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
    LoggingService.info('UpdateService: Checking for pending update...');
    try {
      final pendingPath = StorageService.getSetting<String>(
        SettingsKeys.pendingUpdatePath,
      );
      final pendingSha = StorageService.getSetting<String>(
        SettingsKeys.pendingUpdateSha,
      );

      if (pendingPath == null || pendingSha == null) {
        LoggingService.debug('UpdateService: No pending update found');
        return false;
      }

      LoggingService.info('UpdateService: Found pending update - SHA: $pendingSha, Path: $pendingPath');

      final extractedDir = Directory(pendingPath);
      if (!extractedDir.existsSync()) {
        LoggingService.warn('UpdateService: Pending update directory missing, clearing...');
        await _clearPendingUpdate();
        return false;
      }

      // Get current executable location
      final currentExe = Platform.resolvedExecutable;
      final currentDir = File(currentExe).parent;
      LoggingService.debug('UpdateService: Current executable: $currentExe');
      LoggingService.debug('UpdateService: Current directory: ${currentDir.path}');

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
        LoggingService.info('UpdateService: Update applied, storing new SHA: $pendingSha');
        await StorageService.setSetting(
            SettingsKeys.currentBuildSha, pendingSha);
        await _clearPendingUpdate();

        // Clean up temp files
        try {
          await extractedDir.delete(recursive: true);
        } catch (_) {}
      }

      return success;
    } catch (e, stack) {
      LoggingService.error('UpdateService: Failed to apply pending update', e, stack);
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
    LoggingService.info('UpdateService: Applying macOS update...');
    Directory? backup;
    try {
      // On macOS, we need to replace the .app bundle
      // Current executable is inside Toss.app/Contents/MacOS/
      final appBundle = currentDir.parent.parent.parent;
      LoggingService.debug('UpdateService: App bundle path: ${appBundle.path}');

      // Find Toss.app in extracted directory
      final newApp = Directory('${extractedDir.path}/Toss.app');
      if (!newApp.existsSync()) {
        LoggingService.error('UpdateService: Toss.app not found in extracted files');
        return false;
      }

      // Create backup
      final backupPath = '${appBundle.path}.backup';
      LoggingService.debug('UpdateService: Creating backup at $backupPath');
      if (Directory(backupPath).existsSync()) {
        await Directory(backupPath).delete(recursive: true);
      }

      // Move current to backup, move new to current location
      backup = await appBundle.rename(backupPath);
      LoggingService.debug('UpdateService: Backed up current app');

      await newApp.rename(appBundle.path);
      LoggingService.debug('UpdateService: Installed new app');

      // Remove backup on success
      await backup.delete(recursive: true);
      LoggingService.info('UpdateService: macOS update applied successfully');

      return true;
    } catch (e, stack) {
      LoggingService.error('UpdateService: macOS update failed', e, stack);

      // Attempt rollback
      if (backup != null && backup.existsSync()) {
        LoggingService.warn('UpdateService: Attempting rollback...');
        try {
          final appBundle = currentDir.parent.parent.parent;
          if (Directory(appBundle.path).existsSync()) {
            await Directory(appBundle.path).delete(recursive: true);
          }
          await backup.rename(appBundle.path);
          LoggingService.info('UpdateService: Rollback successful');
        } catch (rollbackError) {
          LoggingService.error('UpdateService: Rollback failed', rollbackError);
        }
      }

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
      // Use a PowerShell script to handle file locking with proper error handling

      final scriptPath = '${extractedDir.path}\\update.ps1';

      // Escape paths for PowerShell (replace single quotes with doubled single quotes)
      String escapePath(String path) {
        return path.replaceAll("'", "''");
      }

      final currentPath = escapePath(currentDir.path);
      final extractedPath = escapePath(extractedDir.path);
      final backupPath = escapePath('${currentDir.path}.backup');

      // PowerShell script with error handling and rollback
      final script = '''
\$ErrorActionPreference = "Stop"
\$currentPath = '${currentPath}'
\$extractedPath = '${extractedPath}'
\$backupPath = '${backupPath}'

# Log file for debugging
\$logFile = "\$env:LOCALAPPDATA\\toss\\logs\\update.log"
\$timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"

function Write-Log {
    param(\$Message)
    Add-Content -Path \$logFile -Value "[\$timestamp] \$Message"
}

try {
    Write-Log "Starting update process..."

    # Wait for the main app to exit
    Start-Sleep -Seconds 3

    # Create backup of current installation
    Write-Log "Creating backup at \$backupPath"
    if (Test-Path \$backupPath) {
        Remove-Item -Path \$backupPath -Recurse -Force
    }
    Copy-Item -Path \$currentPath -Destination \$backupPath -Recurse -Force

    # Remove current files (except backup)
    Write-Log "Removing current files..."
    Get-ChildItem -Path \$currentPath -Exclude "*.backup" | Remove-Item -Recurse -Force

    # Copy new files
    Write-Log "Copying new files from \$extractedPath"
    Get-ChildItem -Path \$extractedPath | Copy-Item -Destination \$currentPath -Recurse -Force

    # Verify toss.exe exists
    \$exePath = Join-Path \$currentPath "toss.exe"
    if (-not (Test-Path \$exePath)) {
        throw "toss.exe not found after update!"
    }

    # Remove backup on success
    Write-Log "Update successful, removing backup..."
    Remove-Item -Path \$backupPath -Recurse -Force -ErrorAction SilentlyContinue

    # Clean up extracted files
    Remove-Item -Path \$extractedPath -Recurse -Force -ErrorAction SilentlyContinue

    Write-Log "Starting updated app..."
    Start-Process \$exePath

    Write-Log "Update completed successfully!"

} catch {
    Write-Log "ERROR: \$_"

    # Rollback: restore from backup
    if (Test-Path \$backupPath) {
        Write-Log "Rolling back to backup..."
        try {
            Get-ChildItem -Path \$currentPath | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
            Get-ChildItem -Path \$backupPath | Copy-Item -Destination \$currentPath -Recurse -Force
            Write-Log "Rollback completed"

            # Try to start the old version
            \$exePath = Join-Path \$currentPath "toss.exe"
            if (Test-Path \$exePath) {
                Start-Process \$exePath
            }
        } catch {
            Write-Log "Rollback failed: \$_"
        }
    }
}

# Clean up this script
Remove-Item -Path \$MyInvocation.MyCommand.Path -Force -ErrorAction SilentlyContinue
''';

      await File(scriptPath).writeAsString(script);
      LoggingService.info('UpdateService: Windows update script created at $scriptPath');

      // Run the script detached and exit current process
      await Process.start(
        'powershell',
        ['-ExecutionPolicy', 'Bypass', '-WindowStyle', 'Hidden', '-File', scriptPath],
        mode: ProcessStartMode.detached,
      );

      LoggingService.info('UpdateService: Update script started, exiting app...');

      // Exit current process to allow replacement
      exit(0);
    } catch (e, stack) {
      LoggingService.error('UpdateService: Windows update failed', e, stack);
      return false;
    }
  }

  /// Apply update on Linux
  static Future<bool> _applyLinuxUpdate(
    Directory extractedDir,
    Directory currentDir,
  ) async {
    LoggingService.info('UpdateService: Applying Linux update...');
    final backupPath = '${currentDir.path}.backup';

    try {
      // On Linux, replace the bundle directory contents
      LoggingService.debug('UpdateService: Current dir: ${currentDir.path}');
      LoggingService.debug('UpdateService: Creating backup at $backupPath');

      if (Directory(backupPath).existsSync()) {
        await Directory(backupPath).delete(recursive: true);
      }

      // Copy current to backup
      final backupResult = await Process.run('cp', ['-r', currentDir.path, backupPath]);
      if (backupResult.exitCode != 0) {
        LoggingService.error('UpdateService: Backup failed: ${backupResult.stderr}');
        return false;
      }
      LoggingService.debug('UpdateService: Backup created');

      // Copy new files over current
      LoggingService.debug('UpdateService: Copying new files...');
      final copyResult = await Process.run(
          'cp', ['-rf', '${extractedDir.path}/.', currentDir.path]);
      if (copyResult.exitCode != 0) {
        throw Exception('Copy failed: ${copyResult.stderr}');
      }

      // Verify executable exists
      final exePath = '${currentDir.path}/toss';
      if (!File(exePath).existsSync()) {
        throw Exception('toss executable not found after update');
      }

      // Ensure executable permission
      await Process.run('chmod', ['+x', exePath]);

      // Remove backup on success
      LoggingService.debug('UpdateService: Removing backup...');
      await Directory(backupPath).delete(recursive: true);

      LoggingService.info('UpdateService: Linux update applied successfully');
      return true;
    } catch (e, stack) {
      LoggingService.error('UpdateService: Linux update failed', e, stack);

      // Attempt rollback
      if (Directory(backupPath).existsSync()) {
        LoggingService.warn('UpdateService: Attempting rollback...');
        try {
          // Remove failed update
          final files = currentDir.listSync();
          for (final file in files) {
            if (!file.path.endsWith('.backup')) {
              if (file is File) {
                await file.delete();
              } else if (file is Directory) {
                await file.delete(recursive: true);
              }
            }
          }

          // Restore from backup
          final restoreResult = await Process.run(
              'cp', ['-rf', '$backupPath/.', currentDir.path]);
          if (restoreResult.exitCode == 0) {
            LoggingService.info('UpdateService: Rollback successful');
          }
        } catch (rollbackError) {
          LoggingService.error('UpdateService: Rollback failed', rollbackError);
        }
      }

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
