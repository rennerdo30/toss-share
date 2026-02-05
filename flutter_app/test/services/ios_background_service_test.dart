//! Tests for iOS background service
//!
//! Note: These tests verify the Dart-side logic of the iOS background service.
//! Native iOS functionality requires integration tests on actual devices.

import 'package:flutter_test/flutter_test.dart';
import 'package:toss/src/core/services/ios_background_service.dart';

void main() {
  group('IosBackgroundService', () {
    late IosBackgroundService service;

    setUp(() {
      service = IosBackgroundService();
    });

    test('singleton pattern returns same instance', () {
      final instance1 = IosBackgroundService();
      final instance2 = IosBackgroundService();

      expect(identical(instance1, instance2), true);
    });

    test('isIos returns false on non-iOS platforms', () {
      // This test runs on the test platform (not iOS)
      // On CI/desktop, this should be false
      expect(service.isIos, false);
    });

    test('initialize returns false on non-iOS platforms', () async {
      final result = await service.initialize();
      expect(result, false);
    });

    test('registerShortcutAction returns false on non-iOS platforms', () async {
      final result = await service.registerShortcutAction(
        'test_action',
        'Test Action',
      );
      expect(result, false);
    });

    test('handleShortcutAction completes without error on non-iOS', () async {
      // Should not throw on non-iOS platforms
      await expectLater(
        service.handleShortcutAction('test_action'),
        completes,
      );
    });

    test('updateWidget completes without error on non-iOS', () async {
      await expectLater(
        service.updateWidget(),
        completes,
      );
    });

    test('syncOnForeground returns error result on non-iOS', () async {
      final result = await service.syncOnForeground();

      expect(result.success, false);
      expect(result.error, 'Not running on iOS');
    });

    test('setupAppExtension returns false on non-iOS', () async {
      final result = await service.setupAppExtension();
      expect(result, false);
    });

    test('hasClipboardRestrictions returns false on non-iOS', () async {
      final result = await service.hasClipboardRestrictions();
      expect(result, false);
    });

    test('requestClipboardAccess returns true on non-iOS', () async {
      final result = await service.requestClipboardAccess();
      expect(result, true);
    });

    test('getClipboardLimitationsMessage returns non-empty string', () {
      final message = service.getClipboardLimitationsMessage();

      expect(message.isNotEmpty, true);
      expect(message.contains('iOS'), true);
      expect(message.contains('clipboard'), true);
    });

    test('configureBackgroundFetch returns false on non-iOS', () async {
      final result = await service.configureBackgroundFetch(
        minimumInterval: const Duration(minutes: 30),
      );
      expect(result, false);
    });

    test('dispose cleans up callbacks', () {
      service.setOnSyncRequested(() async {
        // This callback would be called on sync
      });

      service.dispose();

      // After dispose, callback should be null and not called
      // We can't directly test internal state, but we can verify no error
      expect(service.isInitialized, false);
    });

    test('setOnSyncRequested stores callback without error', () {
      // Should not throw when setting callback
      expect(
        () => service.setOnSyncRequested(() async {}),
        returnsNormally,
      );
    });

    test('setOnClipboardReceived stores callback without error', () {
      // Should not throw when setting callback
      expect(
        () => service.setOnClipboardReceived((preview) async {}),
        returnsNormally,
      );
    });

    test('isInForeground defaults to true', () {
      expect(service.isInForeground, true);
    });
  });

  group('IosSyncResult', () {
    test('creates success result', () {
      const result = IosSyncResult(
        success: true,
        itemsSynced: 5,
      );

      expect(result.success, true);
      expect(result.error, null);
      expect(result.itemsSynced, 5);
    });

    test('creates failure result', () {
      const result = IosSyncResult(
        success: false,
        error: 'Test error',
      );

      expect(result.success, false);
      expect(result.error, 'Test error');
      expect(result.itemsSynced, null);
    });

    test('creates rate-limited result', () {
      const result = IosSyncResult(
        success: true,
        error: 'Sync skipped - rate limited',
        itemsSynced: 0,
      );

      expect(result.success, true);
      expect(result.error, 'Sync skipped - rate limited');
      expect(result.itemsSynced, 0);
    });
  });
}
