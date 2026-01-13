import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:toss/src/app.dart';
import 'package:toss/src/core/models/clipboard_item.dart';
import 'package:toss/src/core/models/device.dart';

void main() {
  group('TossApp Widget Tests', () {
    testWidgets('App should render without errors', (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );

      // Verify app renders
      expect(find.text('Toss'), findsOneWidget);
    });
  });

  group('ClipboardItem Model Tests', () {
    test('ClipboardItem should format size correctly', () {
      final item = ClipboardItem(
        id: 'test-1',
        contentType: ClipboardContentType.text,
        preview: 'Hello World',
        sizeBytes: 11,
        timestamp: DateTime(2024, 1, 1),
      );

      expect(item.formattedSize, '11 B');
      expect(item.isLocal, true);
    });

    test('ClipboardItem should format KB correctly', () {
      final item = ClipboardItem(
        id: 'test-2',
        contentType: ClipboardContentType.text,
        preview: 'Test',
        sizeBytes: 2048,
        timestamp: DateTime(2024, 1, 1),
      );

      expect(item.formattedSize, '2.0 KB');
    });

    test('ClipboardItem should format MB correctly', () {
      final item = ClipboardItem(
        id: 'test-3',
        contentType: ClipboardContentType.image,
        preview: '[Image]',
        sizeBytes: 5 * 1024 * 1024,
        timestamp: DateTime(2024, 1, 1),
      );

      expect(item.formattedSize, '5.0 MB');
    });

    test('ClipboardItem should indicate remote source', () {
      final item = ClipboardItem(
        id: 'test-4',
        contentType: ClipboardContentType.text,
        preview: 'Remote text',
        sizeBytes: 11,
        timestamp: DateTime(2024, 1, 1),
        sourceDeviceId: 'device-123',
        sourceDeviceName: 'Remote Device',
      );

      expect(item.isLocal, false);
    });
  });

  group('ClipboardContentType Tests', () {
    test('Content types should have display names', () {
      expect(ClipboardContentType.text.displayName, 'Text');
      expect(ClipboardContentType.richText.displayName, 'Rich Text');
      expect(ClipboardContentType.image.displayName, 'Image');
      expect(ClipboardContentType.file.displayName, 'File');
      expect(ClipboardContentType.url.displayName, 'URL');
    });

    test('Content types should have icon names', () {
      expect(ClipboardContentType.text.iconName, 'text_fields');
      expect(ClipboardContentType.image.iconName, 'image');
      expect(ClipboardContentType.file.iconName, 'attach_file');
      expect(ClipboardContentType.url.iconName, 'link');
    });
  });

  group('Device Model Tests', () {
    test('Device should track online status', () {
      final device = Device(
        id: 'device-1',
        name: 'Test Device',
        isOnline: true,
        lastSeen: DateTime(2024, 1, 1),
      );

      expect(device.isOnline, true);
      expect(device.name, 'Test Device');
    });

    test('Device should support copyWith', () {
      final device = Device(
        id: 'device-2',
        name: 'Original Device',
        isOnline: false,
        lastSeen: DateTime(2024, 1, 1),
      );

      final updated = device.copyWith(isOnline: true, name: 'Updated Device');

      expect(updated.isOnline, true);
      expect(updated.name, 'Updated Device');
      expect(updated.id, device.id);
    });

    test('Device should have platform', () {
      final device = Device(
        id: 'device-3',
        name: 'Mac Device',
        platform: DevicePlatform.macos,
      );

      expect(device.platform, DevicePlatform.macos);
    });
  });
}
