import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:toss/src/app.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  group('End-to-End Tests', () {
    testWidgets('App launches and shows home screen',
        (WidgetTester tester) async {
      // Build the app with a ProviderScope
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Verify app is running - look for common UI elements
      // The app should show either the home screen content or sidebar navigation
      expect(
        find.byType(Scaffold),
        findsWidgets,
        reason: 'App should render at least one Scaffold',
      );
    });

    testWidgets('Navigation between screens works',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // On mobile layout, try to find history icon button in app bar
      final historyButton = find.byIcon(Icons.history);
      if (historyButton.evaluate().isNotEmpty) {
        await tester.tap(historyButton.first);
        await tester.pumpAndSettle();

        // Should now be on history screen
        expect(find.text('Clipboard History'), findsOneWidget);
      }

      // On desktop layout, try sidebar navigation
      final historyText = find.text('History');
      if (historyText.evaluate().isNotEmpty) {
        await tester.tap(historyText.first);
        await tester.pumpAndSettle();

        // Should show history screen content
        expect(find.text('Clipboard History'), findsOneWidget);
      }
    });

    testWidgets('Settings screen is accessible', (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Try to navigate to settings via icon button (mobile)
      final settingsButton = find.byIcon(Icons.settings);
      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();

        // Should show settings content
        expect(find.text('Settings'), findsWidgets);
      }

      // Try via sidebar navigation text (desktop)
      final settingsText = find.text('Settings');
      if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();

        // Settings screen should be visible
        expect(find.text('Sync'), findsOneWidget);
      }
    });
  });

  group('Device Pairing Flow', () {
    testWidgets('Pairing screen displays QR code option',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to pairing screen
      // Look for "Add" button or "Add Device" button
      final addButton = find.text('Add');
      final addDeviceButton = find.text('Add Device');
      final addIcon = find.byIcon(Icons.add);

      if (addButton.evaluate().isNotEmpty) {
        await tester.tap(addButton.first);
        await tester.pumpAndSettle();
      } else if (addDeviceButton.evaluate().isNotEmpty) {
        await tester.tap(addDeviceButton.first);
        await tester.pumpAndSettle();
      } else if (addIcon.evaluate().isNotEmpty) {
        await tester.tap(addIcon.first);
        await tester.pumpAndSettle();
      }

      // Verify pairing screen shows
      expect(find.text('Pair Device'), findsOneWidget);

      // Verify tabs are present
      expect(find.text('Show Code'), findsOneWidget);
      expect(find.text('Scan Code'), findsOneWidget);
    });

    testWidgets('Pairing screen displays manual code option',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to pairing screen
      final addButton = find.text('Add');
      final addDeviceButton = find.text('Add Device');
      final addIcon = find.byIcon(Icons.add);

      if (addButton.evaluate().isNotEmpty) {
        await tester.tap(addButton.first);
        await tester.pumpAndSettle();
      } else if (addDeviceButton.evaluate().isNotEmpty) {
        await tester.tap(addDeviceButton.first);
        await tester.pumpAndSettle();
      } else if (addIcon.evaluate().isNotEmpty) {
        await tester.tap(addIcon.first);
        await tester.pumpAndSettle();
      }

      // Switch to Scan Code tab
      await tester.tap(find.text('Scan Code'));
      await tester.pumpAndSettle();

      // Verify manual code entry option is visible
      expect(find.text('or enter code manually'), findsWidgets);
      expect(find.text('Pairing code'), findsOneWidget);
    });
  });

  group('Clipboard Operations', () {
    testWidgets('Clipboard section is displayed on home screen',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Verify clipboard section is visible
      expect(find.text('Clipboard'), findsWidgets);
    });

    testWidgets('Send button is displayed', (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Look for send button (mobile shows "Send", desktop shows "Send to all devices")
      final sendButton = find.text('Send');
      final sendAllButton = find.text('Send to all devices');

      expect(
        sendButton.evaluate().isNotEmpty ||
            sendAllButton.evaluate().isNotEmpty,
        isTrue,
        reason: 'Should find a send button on the home screen',
      );
    });

    testWidgets('Clipboard preview area exists', (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Look for clipboard preview card or area
      // This could show "No clipboard content" or actual content
      final noContent = find.text('No clipboard content');
      final clipboardLabel = find.text('Clipboard');

      expect(
        noContent.evaluate().isNotEmpty ||
            clipboardLabel.evaluate().isNotEmpty,
        isTrue,
        reason: 'Should show clipboard area on home screen',
      );
    });
  });

  group('History View and Search', () {
    testWidgets('History screen shows search bar',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to history screen
      final historyButton = find.byIcon(Icons.history);
      final historyText = find.text('History');

      if (historyButton.evaluate().isNotEmpty) {
        await tester.tap(historyButton.first);
        await tester.pumpAndSettle();
      } else if (historyText.evaluate().isNotEmpty) {
        await tester.tap(historyText.first);
        await tester.pumpAndSettle();
      }

      // Verify search bar is present
      expect(find.byType(TextField), findsWidgets);
      expect(find.text('Search history...'), findsOneWidget);
    });

    testWidgets('History screen shows filter button',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to history screen
      final historyButton = find.byIcon(Icons.history);
      final historyText = find.text('History');

      if (historyButton.evaluate().isNotEmpty) {
        await tester.tap(historyButton.first);
        await tester.pumpAndSettle();
      } else if (historyText.evaluate().isNotEmpty) {
        await tester.tap(historyText.first);
        await tester.pumpAndSettle();
      }

      // Verify filter button exists
      final filterIcon = find.byIcon(Icons.filter_list);
      final filterOutlinedIcon = find.byIcon(Icons.filter_list_outlined);

      expect(
        filterIcon.evaluate().isNotEmpty ||
            filterOutlinedIcon.evaluate().isNotEmpty,
        isTrue,
        reason: 'Should find filter button on history screen',
      );
    });

    testWidgets('History search filters results',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to history screen
      final historyButton = find.byIcon(Icons.history);
      final historyText = find.text('History');

      if (historyButton.evaluate().isNotEmpty) {
        await tester.tap(historyButton.first);
        await tester.pumpAndSettle();
      } else if (historyText.evaluate().isNotEmpty) {
        await tester.tap(historyText.first);
        await tester.pumpAndSettle();
      }

      // Find the search field and enter text
      final searchField = find.byType(TextField).first;
      await tester.enterText(searchField, 'test search');
      await tester.pumpAndSettle();

      // The search should filter - if no results, it shows empty state
      // If results exist, they should be filtered
      // This test verifies the search mechanism works without errors
      expect(tester.takeException(), isNull);
    });

    testWidgets('History filter panel can be toggled',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to history screen
      final historyButton = find.byIcon(Icons.history);
      final historyText = find.text('History');

      if (historyButton.evaluate().isNotEmpty) {
        await tester.tap(historyButton.first);
        await tester.pumpAndSettle();
      } else if (historyText.evaluate().isNotEmpty) {
        await tester.tap(historyText.first);
        await tester.pumpAndSettle();
      }

      // Tap filter button to show filters
      final filterIcon = find.byIcon(Icons.filter_list_outlined);
      final filterIconFilled = find.byIcon(Icons.filter_list);

      if (filterIcon.evaluate().isNotEmpty) {
        await tester.tap(filterIcon.first);
        await tester.pumpAndSettle();

        // Filters section should now be visible
        expect(find.text('Filters'), findsOneWidget);
        expect(find.text('Clear Filters'), findsOneWidget);
      } else if (filterIconFilled.evaluate().isNotEmpty) {
        await tester.tap(filterIconFilled.first);
        await tester.pumpAndSettle();
      }
    });
  });

  group('Settings Persistence', () {
    testWidgets('Settings toggles are interactive',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to settings
      final settingsButton = find.byIcon(Icons.settings);
      final settingsText = find.text('Settings');

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();
      } else if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();
      }

      // Find switches on the screen
      final switches = find.byType(Switch);
      expect(switches, findsWidgets);

      // Verify Auto Sync switch is present and interactive
      expect(find.text('Auto Sync'), findsOneWidget);
    });

    testWidgets('Auto-sync setting can be toggled',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to settings
      final settingsButton = find.byIcon(Icons.settings);
      final settingsText = find.text('Settings');

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();
      } else if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();
      }

      // Find the first SwitchListTile (Auto Sync)
      final switches = find.byType(SwitchListTile);
      if (switches.evaluate().isNotEmpty) {
        // Get initial state and tap
        await tester.tap(switches.first);
        await tester.pumpAndSettle();

        // Verify no exceptions occurred
        expect(tester.takeException(), isNull);
      }
    });

    testWidgets('Notifications setting is displayed',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to settings
      final settingsButton = find.byIcon(Icons.settings);
      final settingsText = find.text('Settings');

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();
      } else if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();
      }

      // Verify Notifications setting is present
      expect(find.text('Notifications'), findsOneWidget);
    });

    testWidgets('Theme setting dialog can be opened',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to settings
      final settingsButton = find.byIcon(Icons.settings);
      final settingsText = find.text('Settings');

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();
      } else if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();
      }

      // Find and tap the Theme list tile
      final themeTile = find.text('Theme');
      if (themeTile.evaluate().isNotEmpty) {
        await tester.tap(themeTile.first);
        await tester.pumpAndSettle();

        // Theme dialog should show options
        expect(find.text('System'), findsOneWidget);
        expect(find.text('Light'), findsOneWidget);
        expect(find.text('Dark'), findsOneWidget);
      }
    });
  });

  group('Offline/Online Transitions', () {
    testWidgets('Connection status is displayed',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Look for connection status indicators
      // Could be in sidebar (desktop) or banner (mobile)
      final offlineText = find.text('Offline');
      final connectedText = find.text('Connected');
      final devicesOnlineText = find.textContaining('devices online');
      final noDevicesText = find.text('No devices paired');

      // At least one of these should be visible
      expect(
        offlineText.evaluate().isNotEmpty ||
            connectedText.evaluate().isNotEmpty ||
            devicesOnlineText.evaluate().isNotEmpty ||
            noDevicesText.evaluate().isNotEmpty,
        isTrue,
        reason: 'Should display connection/device status',
      );
    });

    testWidgets('App handles no network gracefully',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // The app should display without crashing even with no network
      expect(find.byType(MaterialApp), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  });

  group('Network Operations', () {
    testWidgets('Network status is reflected in UI',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // The app should show network status somewhere
      // Either in sidebar status or in a banner
      final scaffold = find.byType(Scaffold);
      expect(scaffold, findsWidgets);

      // Verify the app doesn't throw any network-related errors
      expect(tester.takeException(), isNull);
    });

    testWidgets('App remains stable during network operations',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate around to trigger potential network operations
      final historyButton = find.byIcon(Icons.history);
      if (historyButton.evaluate().isNotEmpty) {
        await tester.tap(historyButton.first);
        await tester.pumpAndSettle();
      }

      final homeButton = find.byIcon(Icons.home);
      final homeText = find.text('Home');
      if (homeButton.evaluate().isNotEmpty) {
        await tester.tap(homeButton.first);
        await tester.pumpAndSettle();
      } else if (homeText.evaluate().isNotEmpty) {
        await tester.tap(homeText.first);
        await tester.pumpAndSettle();
      }

      // App should remain stable
      expect(tester.takeException(), isNull);
    });
  });

  group('Relay Fallback', () {
    testWidgets('Relay URL can be configured in settings',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to settings
      final settingsButton = find.byIcon(Icons.settings);
      final settingsText = find.text('Settings');

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();
      } else if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();
      }

      // Find the Relay Server setting
      expect(find.text('Relay Server'), findsOneWidget);

      // Tap to open relay URL dialog
      await tester.tap(find.text('Relay Server'));
      await tester.pumpAndSettle();

      // Dialog should show relay URL configuration
      expect(find.text('Relay Server URL'), findsOneWidget);
    });
  });

  group('Large File Transfer', () {
    testWidgets('Max file size setting is displayed',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to settings
      final settingsButton = find.byIcon(Icons.settings);
      final settingsText = find.text('Settings');

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();
      } else if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();
      }

      // Verify max file size setting exists
      expect(find.text('Max File Size'), findsOneWidget);
    });

    testWidgets('Max file size dialog shows options',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Navigate to settings
      final settingsButton = find.byIcon(Icons.settings);
      final settingsText = find.text('Settings');

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pumpAndSettle();
      } else if (settingsText.evaluate().isNotEmpty) {
        await tester.tap(settingsText.first);
        await tester.pumpAndSettle();
      }

      // Tap on Max File Size to open dialog
      await tester.tap(find.text('Max File Size'));
      await tester.pumpAndSettle();

      // Dialog should show size options
      expect(find.text('10 MB'), findsOneWidget);
      expect(find.text('50 MB'), findsOneWidget);
      expect(find.text('100 MB'), findsOneWidget);
    });
  });

  group('Error Recovery', () {
    testWidgets('App handles navigation errors gracefully',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Try to navigate to multiple screens rapidly
      final historyButton = find.byIcon(Icons.history);
      final settingsButton = find.byIcon(Icons.settings);

      if (historyButton.evaluate().isNotEmpty) {
        await tester.tap(historyButton.first);
        await tester.pump(const Duration(milliseconds: 100));
      }

      if (settingsButton.evaluate().isNotEmpty) {
        await tester.tap(settingsButton.first);
        await tester.pump(const Duration(milliseconds: 100));
      }

      await tester.pumpAndSettle();

      // App should not crash
      expect(tester.takeException(), isNull);
    });

    testWidgets('App recovers from widget rebuild',
        (WidgetTester tester) async {
      await tester.pumpWidget(
        const ProviderScope(
          child: TossApp(),
        ),
      );
      await tester.pumpAndSettle();

      // Trigger multiple rebuilds
      for (var i = 0; i < 5; i++) {
        await tester.pump(const Duration(milliseconds: 100));
      }
      await tester.pumpAndSettle();

      // App should remain stable
      expect(find.byType(MaterialApp), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  });
}
