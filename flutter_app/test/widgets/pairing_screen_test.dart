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

    testWidgets('shows QR code tab content', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: PairingScreen(),
          ),
        ),
      );

      // Wait for initial load
      await tester.pumpAndSettle();

      // Should show instructions or QR code
      expect(
        find.textContaining('Scan this QR code', findRichText: true),
        findsAny,
      );
    });
  });
}
