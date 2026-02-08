import 'dart:io';

import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/app_update.dart';
import '../services/update_service.dart';

part 'update_provider.g.dart';

/// Adapter around [UpdateService] so provider behavior can be tested.
class UpdateServiceAdapter {
  const UpdateServiceAdapter();

  bool get isSupported => UpdateService.isSupported;

  String get currentVersion => UpdateService.currentVersion;

  Future<AppUpdate?> checkForUpdate() => UpdateService.checkForUpdate();

  Future<void> updateLastCheckTime() => UpdateService.updateLastCheckTime();

  Future<UpdateStagingDecision> evaluateStagingDecision(String sha) {
    return UpdateService.evaluateStagingDecision(sha);
  }

  Future<File?> downloadUpdate(
    AppUpdate update, {
    void Function(double progress)? onProgress,
  }) {
    return UpdateService.downloadUpdate(update, onProgress: onProgress);
  }

  Future<bool> stageUpdate(File file, AppUpdate update) {
    return UpdateService.stageUpdate(file, update);
  }

  Future<UpdateApplyResult> applyPendingUpdate() {
    return UpdateService.applyPendingUpdate();
  }

  Future<bool> hasPendingUpdate() => UpdateService.hasPendingUpdate();
}

final updateServiceAdapterProvider = Provider<UpdateServiceAdapter>(
  (ref) => const UpdateServiceAdapter(),
);

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
  Future<void> checkForUpdates({bool automatic = false}) async {
    final updateService = ref.read(updateServiceAdapterProvider);
    if (!updateService.isSupported) {
      return;
    }

    state = state.copyWith(status: UpdateStatus.checking);

    try {
      final update = await updateService.checkForUpdate();
      await updateService.updateLastCheckTime();

      if (update != null) {
        state = state.copyWith(
          status: UpdateStatus.available,
          availableUpdate: update,
          errorMessage: null,
        );

        // Keep automatic startup behavior while preserving manual checks.
        await downloadAndStage(automatic: automatic);
      } else {
        state = state.copyWith(
          status: UpdateStatus.upToDate,
          errorMessage: null,
        );
      }
    } catch (e) {
      state = state.copyWith(
        status: UpdateStatus.error,
        errorMessage: e.toString(),
      );
    }
  }

  /// Download and stage update for next restart
  Future<void> downloadAndStage({bool automatic = false}) async {
    final update = state.availableUpdate;
    if (update == null) return;

    final updateService = ref.read(updateServiceAdapterProvider);
    final decision = await updateService.evaluateStagingDecision(update.sha);

    if (decision == UpdateStagingDecision.alreadyStaged) {
      state = state.copyWith(
        status: UpdateStatus.ready,
        errorMessage: null,
      );
      return;
    }

    if (decision == UpdateStagingDecision.blockedByRecentFailure) {
      state = state.copyWith(
        status: UpdateStatus.error,
        errorMessage: automatic
            ? 'Update deferred after recent apply failure'
            : 'Please restart and try update again in a few minutes',
      );
      return;
    }

    state = state.copyWith(
      status: UpdateStatus.downloading,
      downloadProgress: 0.0,
      errorMessage: null,
    );

    try {
      final file = await updateService.downloadUpdate(
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

      final staged = await updateService.stageUpdate(file, update);

      if (staged) {
        state = state.copyWith(
          status: UpdateStatus.ready,
          errorMessage: null,
        );
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
    final updateService = ref.read(updateServiceAdapterProvider);
    final result = await updateService.applyPendingUpdate();
    if (result == UpdateApplyResult.applied && !Platform.isWindows) {
      // Windows handles restart in the update script
      exit(0);
    }
    if (result == UpdateApplyResult.failed) {
      state = state.copyWith(
        status: UpdateStatus.error,
        errorMessage: 'Failed to apply update',
      );
    }
  }

  /// Get current version string
  String get currentVersion =>
      ref.read(updateServiceAdapterProvider).currentVersion;

  /// Check if there's a pending update ready to install
  Future<bool> hasPendingUpdate() async {
    return ref.read(updateServiceAdapterProvider).hasPendingUpdate();
  }
}
