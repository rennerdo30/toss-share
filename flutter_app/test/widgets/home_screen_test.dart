import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'package:toss/src/features/home/home_screen.dart';
import 'package:toss/src/core/providers/toss_provider.dart';
import 'package:toss/src/core/providers/devices_provider.dart';
import 'package:toss/src/core/providers/clipboard_provider.dart';

void main() {
  group('HomeScreen Widget Tests', () {
    testWidgets('displays app bar with title and action buttons', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HomeScreen(),
          ),
        ),
      );

      expect(find.text('Toss'), findsOneWidget);
      expect(find.byIcon(Icons.history), findsOneWidget);
      expect(find.byIcon(Icons.settings), findsOneWidget);
    });

    testWidgets('shows empty devices card when no devices', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HomeScreen(),
          ),
        ),
      );

      expect(find.text('No devices paired'), findsOneWidget);
      expect(find.text('Pair a device to start sharing your clipboard'), findsOneWidget);
    });

    testWidgets('shows devices section header', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HomeScreen(),
          ),
        ),
      );

      expect(find.text('Devices'), findsOneWidget);
      expect(find.text('Add'), findsOneWidget);
    });

    testWidgets('shows clipboard section header', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HomeScreen(),
          ),
        ),
      );

      expect(find.text('Clipboard'), findsOneWidget);
    });

    testWidgets('shows send button', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HomeScreen(),
          ),
        ),
      );

      expect(find.text('Send'), findsOneWidget);
    });
  });
}
