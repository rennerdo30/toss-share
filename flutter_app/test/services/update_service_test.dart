import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:toss/src/core/services/storage_service.dart';
import 'package:toss/src/core/services/update_service.dart';

void main() {
  group('UpdateService path resolution', () {
    test('resolveMacOSAppBundle returns app bundle directory', () {
      final executableDir = Directory('/Applications/Toss.app/Contents/MacOS');
      final appBundle = UpdateService.resolveMacOSAppBundle(executableDir);

      expect(appBundle, isNotNull);
      expect(appBundle!.path, '/Applications/Toss.app');
    });

    test('resolveMacOSAppBundle rejects non-app parent directory', () {
      final executableDir =
          Directory('/tmp/build/macos/Build/Products/Debug/Contents/MacOS');
      final appBundle = UpdateService.resolveMacOSAppBundle(executableDir);

      expect(appBundle, isNull);
    });
  });

  group('UpdateService apply pending update', () {
    late Directory extractedDir;
    late Map<String, dynamic> settingsStore;

    const testSha = 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

    setUp(() async {
      UpdateService.resetTestingOverrides();

      extractedDir = await Directory.systemTemp.createTemp('toss-update-test-');
      settingsStore = {
        SettingsKeys.pendingUpdatePath: extractedDir.path,
        SettingsKeys.pendingUpdateSha: testSha,
      };

      UpdateService.testSettingsStore = settingsStore;
      UpdateService.testPlatformOverride = UpdatePlatform.macOS;
      UpdateService.testResolvedExecutableOverride =
          '/Applications/Toss.app/Contents/MacOS/Toss';
    });

    tearDown(() async {
      UpdateService.resetTestingOverrides();
      if (await extractedDir.exists()) {
        await extractedDir.delete(recursive: true);
      }
    });

    test('failed apply clears pending markers and returns failed', () async {
      var applyCalls = 0;
      UpdateService.testApplyUpdateOverride = (
        _,
        __,
        ___,
      ) async {
        applyCalls++;
        return false;
      };

      final result = await UpdateService.applyPendingUpdate();

      expect(result, UpdateApplyResult.failed);
      expect(applyCalls, 1);
      expect(
          settingsStore.containsKey(SettingsKeys.pendingUpdatePath), isFalse);
      expect(settingsStore.containsKey(SettingsKeys.pendingUpdateSha), isFalse);
      expect(
        settingsStore[SettingsKeys.lastUpdateApplyFailureSha],
        testSha,
      );
      expect(
        settingsStore.containsKey(SettingsKeys.lastUpdateApplyFailureAt),
        isTrue,
      );
    });

    test('failed apply is not retried on immediate next startup pass',
        () async {
      var applyCalls = 0;
      UpdateService.testApplyUpdateOverride = (
        _,
        __,
        ___,
      ) async {
        applyCalls++;
        return false;
      };

      final first = await UpdateService.applyPendingUpdate();
      final second = await UpdateService.applyPendingUpdate();

      expect(first, UpdateApplyResult.failed);
      expect(second, UpdateApplyResult.skipped);
      expect(applyCalls, 1);
    });
  });
}
