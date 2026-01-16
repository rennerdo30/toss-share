import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:toss/src/core/providers/settings_provider.dart';

void main() {
  group('AppSettings', () {
    test('has correct default values', () {
      const settings = AppSettings();

      expect(settings.autoSync, true);
      expect(settings.syncText, true);
      expect(settings.syncRichText, true);
      expect(settings.syncImages, true);
      expect(settings.syncFiles, true);
      expect(settings.maxFileSizeMb, 50);
      expect(settings.historyEnabled, true);
      expect(settings.historyDays, 7);
      expect(settings.relayUrl, null);
      expect(settings.showNotifications, true);
      expect(settings.notifyOnPairing, true);
      expect(settings.notifyOnClipboard, true);
      expect(settings.notifyOnConnection, false);
    });

    test('copyWith creates new instance with updated values', () {
      const settings = AppSettings();
      final updated = settings.copyWith(
        autoSync: false,
        maxFileSizeMb: 100,
        historyDays: 14,
      );

      expect(updated.autoSync, false);
      expect(updated.maxFileSizeMb, 100);
      expect(updated.historyDays, 14);
      // Other values should remain unchanged
      expect(updated.syncText, true);
      expect(updated.showNotifications, true);
    });

    test('copyWith preserves values when not specified', () {
      final settings = const AppSettings().copyWith(
        relayUrl: 'https://relay.example.com',
      );

      expect(settings.relayUrl, 'https://relay.example.com');
      expect(settings.autoSync, true);
      expect(settings.historyEnabled, true);
    });
  });

  group('Settings Provider', () {
    late ProviderContainer container;

    setUp(() {
      container = ProviderContainer();
    });

    tearDown(() {
      container.dispose();
    });

    test('initial state has default values', () {
      final settings = container.read(settingsProvider);

      expect(settings.autoSync, true);
      expect(settings.showNotifications, true);
    });

    test('updateAutoSync changes autoSync state', () {
      container.read(settingsProvider.notifier).updateAutoSync(false);
      final settings = container.read(settingsProvider);

      expect(settings.autoSync, false);
    });

    test('updateSyncText changes syncText state', () {
      container.read(settingsProvider.notifier).updateSyncText(false);
      final settings = container.read(settingsProvider);

      expect(settings.syncText, false);
    });

    test('updateSyncRichText changes syncRichText state', () {
      container.read(settingsProvider.notifier).updateSyncRichText(false);
      final settings = container.read(settingsProvider);

      expect(settings.syncRichText, false);
    });

    test('updateSyncImages changes syncImages state', () {
      container.read(settingsProvider.notifier).updateSyncImages(false);
      final settings = container.read(settingsProvider);

      expect(settings.syncImages, false);
    });

    test('updateSyncFiles changes syncFiles state', () {
      container.read(settingsProvider.notifier).updateSyncFiles(false);
      final settings = container.read(settingsProvider);

      expect(settings.syncFiles, false);
    });

    test('updateMaxFileSize changes maxFileSizeMb state', () {
      container.read(settingsProvider.notifier).updateMaxFileSize(100);
      final settings = container.read(settingsProvider);

      expect(settings.maxFileSizeMb, 100);
    });

    test('updateHistoryEnabled changes historyEnabled state', () {
      container.read(settingsProvider.notifier).updateHistoryEnabled(false);
      final settings = container.read(settingsProvider);

      expect(settings.historyEnabled, false);
    });

    test('updateHistoryDays changes historyDays state', () {
      container.read(settingsProvider.notifier).updateHistoryDays(30);
      final settings = container.read(settingsProvider);

      expect(settings.historyDays, 30);
    });

    test('updateShowNotifications changes showNotifications state', () {
      container.read(settingsProvider.notifier).updateShowNotifications(false);
      final settings = container.read(settingsProvider);

      expect(settings.showNotifications, false);
    });

    test('updateNotifyOnPairing changes notifyOnPairing state', () {
      container.read(settingsProvider.notifier).updateNotifyOnPairing(false);
      final settings = container.read(settingsProvider);

      expect(settings.notifyOnPairing, false);
    });

    test('updateNotifyOnClipboard changes notifyOnClipboard state', () {
      container.read(settingsProvider.notifier).updateNotifyOnClipboard(false);
      final settings = container.read(settingsProvider);

      expect(settings.notifyOnClipboard, false);
    });

    test('updateNotifyOnConnection changes notifyOnConnection state', () {
      container.read(settingsProvider.notifier).updateNotifyOnConnection(true);
      final settings = container.read(settingsProvider);

      expect(settings.notifyOnConnection, true);
    });

    test('updateRelayUrl changes relayUrl state', () {
      container
          .read(settingsProvider.notifier)
          .updateRelayUrl('https://relay.example.com');
      final settings = container.read(settingsProvider);

      expect(settings.relayUrl, 'https://relay.example.com');
    });

    test('multiple updates accumulate correctly', () {
      final notifier = container.read(settingsProvider.notifier);

      notifier.updateAutoSync(false);
      notifier.updateMaxFileSize(200);
      notifier.updateHistoryDays(14);
      notifier.updateShowNotifications(false);

      final settings = container.read(settingsProvider);

      expect(settings.autoSync, false);
      expect(settings.maxFileSizeMb, 200);
      expect(settings.historyDays, 14);
      expect(settings.showNotifications, false);
      // Unchanged values should remain default
      expect(settings.syncText, true);
    });
  });
}
