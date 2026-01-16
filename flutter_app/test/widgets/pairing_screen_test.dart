import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:toss/src/features/pairing/pairing_screen.dart';

void main() {
  group('PairingScreen Widget Tests', () {
    testWidgets('displays app bar with tabs', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: PairingScreen(),
          ),
        ),
      );

      expect(find.text('Pair Device'), findsOneWidget);
      expect(find.text('Show Code'), findsOneWidget);
      expect(find.text('Scan Code'), findsOneWidget);
    });

    testWidgets('shows tab content when Show Code tab is selected',
        (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: PairingScreen(),
          ),
        ),
      );

      // The pairing screen should render without errors
      // Show Code tab is selected by default
      expect(find.byType(TabBarView), findsOneWidget);
    });
  });
}
