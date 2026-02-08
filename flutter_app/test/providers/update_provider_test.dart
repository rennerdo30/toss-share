import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:toss/src/core/models/app_update.dart';
import 'package:toss/src/core/providers/update_provider.dart';
import 'package:toss/src/core/services/update_service.dart';

class FakeUpdateServiceAdapter extends UpdateServiceAdapter {
  FakeUpdateServiceAdapter({required this.update});

  final AppUpdate update;

  int downloadCalls = 0;
  int stageCalls = 0;

  @override
  bool get isSupported => true;

  @override
  Future<AppUpdate?> checkForUpdate() async => update;

  @override
  Future<void> updateLastCheckTime() async {}

  @override
  Future<UpdateStagingDecision> evaluateStagingDecision(String sha) async {
    expect(sha, update.sha);
    return UpdateStagingDecision.alreadyStaged;
  }

  @override
  Future<File?> downloadUpdate(
    AppUpdate update, {
    void Function(double progress)? onProgress,
  }) async {
    downloadCalls++;
    return null;
  }

  @override
  Future<bool> stageUpdate(File file, AppUpdate update) async {
    stageCalls++;
    return false;
  }
}

void main() {
  group('Update provider', () {
    late ProviderContainer container;
    late FakeUpdateServiceAdapter fakeService;

    setUp(() {
      fakeService = FakeUpdateServiceAdapter(
        update: AppUpdate(
          version: 'nightly',
          downloadUrl: 'https://example.com/toss-macos-nightly.zip',
          size: 123,
          sha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          publishedAt: DateTime(2026, 2, 8),
        ),
      );

      container = ProviderContainer(
        overrides: [
          updateServiceAdapterProvider.overrideWithValue(fakeService),
        ],
      );
    });

    tearDown(() {
      container.dispose();
    });

    test('skips auto-download when same SHA is already staged', () async {
      await container
          .read(updateProvider.notifier)
          .checkForUpdates(automatic: true);

      final state = container.read(updateProvider);
      expect(state.status, UpdateStatus.ready);
      expect(fakeService.downloadCalls, 0);
      expect(fakeService.stageCalls, 0);
    });
  });
}
