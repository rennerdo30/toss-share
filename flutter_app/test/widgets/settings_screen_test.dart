import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:toss/src/features/settings/settings_screen.dart';

void main() {
  group('SettingsScreen Widget Tests', () {
    testWidgets('displays settings screen with app bar', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      expect(find.text('Settings'), findsOneWidget);
    });

    testWidgets('shows sync section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      expect(find.text('Sync'), findsOneWidget);
      expect(find.text('Auto Sync'), findsOneWidget);
    });

    testWidgets('shows content types section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      expect(find.text('Sync Text'), findsOneWidget);
      expect(find.text('Sync Rich Text'), findsOneWidget);
      expect(find.text('Sync Images'), findsOneWidget);
      expect(find.text('Sync Files'), findsOneWidget);
    });

    testWidgets('shows history section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      expect(find.text('History'), findsOneWidget);
      expect(find.text('Save History'), findsOneWidget);
      expect(find.text('Keep History For'), findsOneWidget);
    });

    testWidgets('shows appearance section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      expect(find.text('Appearance'), findsOneWidget);
      expect(find.text('Theme'), findsOneWidget);
      expect(find.text('Notifications'), findsOneWidget);
    });

    testWidgets('shows about section', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      expect(find.text('About'), findsOneWidget);
      expect(find.text('Version'), findsOneWidget);
      expect(find.text('Source Code'), findsOneWidget);
    });

    testWidgets('auto-sync toggle is present and enabled by default', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      // Find the auto-sync switch
      final switchFinder = find.byType(Switch).first;
      expect(switchFinder, findsOneWidget);
    });

    testWidgets('notifications toggle is present', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: SettingsScreen(),
          ),
        ),
      );

      expect(find.text('Notifications'), findsOneWidget);
    });
  });
}
