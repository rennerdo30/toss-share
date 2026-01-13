import 'dart:io';

import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/app_update.dart';
import '../services/update_service.dart';

part 'update_provider.g.dart';

/// Update state
class UpdateState {
  final UpdateStatus status;
  final AppUpdate? availableUpdate;
  final double downloadProgress;
  final String? errorMessage;

  const UpdateState({
    this.status = UpdateStatus.idle,
    this.availableUpdate,
    this.downloadProgress = 0.0,
    this.errorMessage,
  });

  UpdateState copyWith({
    UpdateStatus? status,
    AppUpdate? availableUpdate,
    double? downloadProgress,
    String? errorMessage,
  }) {
    return UpdateState(
      status: status ?? this.status,
      availableUpdate: availableUpdate ?? this.availableUpdate,
      downloadProgress: downloadProgress ?? this.downloadProgress,
      errorMessage: errorMessage ?? this.errorMessage,
    );
  }
}

/// Update state provider
@Riverpod(keepAlive: true)
class Update extends _$Update {
  @override
  UpdateState build() {
    return const UpdateState();
  }

  /// Check for available updates
  Future<void> checkForUpdates() async {
    if (!UpdateService.isSupported) {
      return;
    }

    state = state.copyWith(status: UpdateStatus.checking);

    try {
      final update = await UpdateService.checkForUpdate();
      await UpdateService.updateLastCheckTime();

      if (update != null) {
        state = state.copyWith(
          status: UpdateStatus.available,
          availableUpdate: update,
        );

        // Auto-download for fully automatic updates
        await downloadAndStage();
      } else {
        state = state.copyWith(status: UpdateStatus.upToDate);
      }
    } catch (e) {
      state = state.copyWith(
        status: UpdateStatus.error,
        errorMessage: e.toString(),
      );
    }
  }

  /// Download and stage update for next restart
  Future<void> downloadAndStage() async {
    final update = state.availableUpdate;
    if (update == null) return;

    state = state.copyWith(
      status: UpdateStatus.downloading,
      downloadProgress: 0.0,
    );

    try {
      final file = await UpdateService.downloadUpdate(
        update,
        onProgress: (progress) {
          state = state.copyWith(downloadProgress: progress);
        },
      );

      if (file == null) {
        state = state.copyWith(
          status: UpdateStatus.error,
          errorMessage: 'Download failed',
        );
        return;
      }

      final staged = await UpdateService.stageUpdate(file, update);

      if (staged) {
        state = state.copyWith(status: UpdateStatus.ready);
      } else {
        state = state.copyWith(
          status: UpdateStatus.error,
          errorMessage: 'Failed to stage update',
        );
      }
    } catch (e) {
      state = state.copyWith(
        status: UpdateStatus.error,
        errorMessage: e.toString(),
      );
    }
  }

  /// Apply pending update and restart app
  Future<void> applyAndRestart() async {
    final success = await UpdateService.applyPendingUpdate();
    if (success && !Platform.isWindows) {
      // Windows handles restart in the update script
      exit(0);
    }
  }

  /// Get current version string
  String get currentVersion => UpdateService.currentVersion;

  /// Check if there's a pending update ready to install
  Future<bool> hasPendingUpdate() async {
    return UpdateService.hasPendingUpdate();
  }
}
