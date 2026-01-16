import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:toss/src/features/home/home_screen.dart';

void main() {
  group('HomeScreen Widget Tests', () {
    // Helper to create a mobile-sized screen for testing mobile layout
    Widget buildMobileHome() {
      return const ProviderScope(
        child: MaterialApp(
          home: MediaQuery(
            data: MediaQueryData(size: Size(400, 800)),
            child: HomeScreen(),
          ),
        ),
      );
    }

    // Helper to create a desktop-sized screen for testing desktop layout
    Widget buildDesktopHome() {
      return const ProviderScope(
        child: MaterialApp(
          home: MediaQuery(
            data: MediaQueryData(size: Size(1200, 800)),
            child: HomeScreen(),
          ),
        ),
      );
    }

    testWidgets('mobile: displays app bar with title', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildMobileHome());

      expect(find.text('Toss'), findsOneWidget);
    });

    testWidgets('mobile: shows empty devices message when no devices',
        (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildMobileHome());

      expect(find.text('No devices paired'), findsOneWidget);
    });

    testWidgets('mobile: shows devices section header', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildMobileHome());

      expect(find.text('Devices'), findsOneWidget);
    });

    testWidgets('mobile: shows clipboard section header', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildMobileHome());

      expect(find.text('Clipboard'), findsOneWidget);
    });

    testWidgets('mobile: shows send button', (tester) async {
      tester.view.physicalSize = const Size(400, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildMobileHome());

      expect(find.text('Send'), findsOneWidget);
    });

    testWidgets('desktop: shows clipboard header', (tester) async {
      tester.view.physicalSize = const Size(1200, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildDesktopHome());

      expect(find.text('Clipboard'), findsOneWidget);
    });

    testWidgets('desktop: shows send to all devices button', (tester) async {
      tester.view.physicalSize = const Size(1200, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildDesktopHome());

      expect(find.text('Send to all devices'), findsOneWidget);
    });

    testWidgets('desktop: shows no devices paired message', (tester) async {
      tester.view.physicalSize = const Size(1200, 800);
      tester.view.devicePixelRatio = 1.0;
      addTearDown(tester.view.resetPhysicalSize);

      await tester.pumpWidget(buildDesktopHome());

      expect(find.text('No devices paired'), findsOneWidget);
    });
  });
}
