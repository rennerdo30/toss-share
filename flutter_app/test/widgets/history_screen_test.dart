import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:toss/src/features/history/history_screen.dart';

void main() {
  group('HistoryScreen Widget Tests', () {
    testWidgets('displays history screen with app bar', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      expect(find.text('Clipboard History'), findsOneWidget);
    });

    testWidgets('shows search bar', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      expect(find.byType(TextField), findsOneWidget);
      expect(find.text('Search history...'), findsOneWidget);
    });

    testWidgets('shows filter button in app bar', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      expect(find.byIcon(Icons.filter_list_outlined), findsOneWidget);
    });

    testWidgets('shows empty state when no history', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      // Wait for loading to complete
      await tester.pumpAndSettle();

      expect(find.text('No clipboard history'), findsOneWidget);
      expect(
          find.text('Synced clipboard items will appear here'), findsOneWidget);
    });

    testWidgets('tapping filter button toggles filter panel', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      // Initially filters should not be visible
      expect(find.text('Filters'), findsNothing);

      // Tap filter button
      await tester.tap(find.byIcon(Icons.filter_list_outlined));
      await tester.pumpAndSettle();

      // Filters should now be visible
      expect(find.text('Filters'), findsOneWidget);
      expect(find.text('All Types'), findsOneWidget);
    });

    testWidgets('filter panel shows content type chips', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      // Open filters
      await tester.tap(find.byIcon(Icons.filter_list_outlined));
      await tester.pumpAndSettle();

      // Check for content type filter chips
      expect(find.text('All Types'), findsOneWidget);
      expect(find.text('Text'), findsOneWidget);
      expect(find.text('URL'), findsOneWidget);
    });

    testWidgets('filter panel has clear button', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      // Open filters
      await tester.tap(find.byIcon(Icons.filter_list_outlined));
      await tester.pumpAndSettle();

      // Check for clear button
      expect(find.text('Clear'), findsOneWidget);
    });

    testWidgets('filter panel has date range buttons', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      // Open filters
      await tester.tap(find.byIcon(Icons.filter_list_outlined));
      await tester.pumpAndSettle();

      // Check for date range buttons
      expect(find.text('Start Date'), findsOneWidget);
      expect(find.text('End Date'), findsOneWidget);
    });

    testWidgets('search bar can be typed into', (tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: MaterialApp(
            home: HistoryScreen(),
          ),
        ),
      );

      // Enter text in search bar
      await tester.enterText(find.byType(TextField), 'test search');
      await tester.pump();

      expect(find.text('test search'), findsOneWidget);
    });
  });
}
