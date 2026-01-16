import 'package:flutter/material.dart';

/// Responsive breakpoints for different device sizes
class Breakpoints {
  static const double mobile = 600;
  static const double tablet = 900;
  static const double desktop = 1200;

  // Sidebar width when visible
  static const double sidebarWidth = 250;
  static const double collapsedSidebarWidth = 72;
}

/// Provides responsive layout utilities
class ResponsiveLayout extends StatelessWidget {
  final Widget mobile;
  final Widget? tablet;
  final Widget desktop;

  const ResponsiveLayout({
    super.key,
    required this.mobile,
    this.tablet,
    required this.desktop,
  });

  /// Check if current layout is mobile (< 600px)
  static bool isMobile(BuildContext context) =>
      MediaQuery.of(context).size.width < Breakpoints.mobile;

  /// Check if current layout is tablet (600-900px)
  static bool isTablet(BuildContext context) =>
      MediaQuery.of(context).size.width >= Breakpoints.mobile &&
      MediaQuery.of(context).size.width < Breakpoints.tablet;

  /// Check if current layout is desktop (>= 900px)
  static bool isDesktop(BuildContext context) =>
      MediaQuery.of(context).size.width >= Breakpoints.tablet;

  /// Check if sidebar should be visible (tablet or desktop)
  static bool shouldShowSidebar(BuildContext context) =>
      MediaQuery.of(context).size.width >= Breakpoints.mobile;

  /// Check if sidebar should be collapsed (tablet only)
  static bool shouldCollapseSidebar(BuildContext context) =>
      isTablet(context);

  /// Get the current sidebar width based on screen size
  static double getSidebarWidth(BuildContext context) {
    if (!shouldShowSidebar(context)) return 0;
    if (shouldCollapseSidebar(context)) return Breakpoints.collapsedSidebarWidth;
    return Breakpoints.sidebarWidth;
  }

  /// Get responsive padding based on screen size
  static EdgeInsets getScreenPadding(BuildContext context) {
    if (isMobile(context)) {
      return const EdgeInsets.all(16);
    } else if (isTablet(context)) {
      return const EdgeInsets.all(20);
    } else {
      return const EdgeInsets.all(24);
    }
  }

  /// Get the number of columns for a grid based on screen size
  static int getGridColumns(BuildContext context, {int mobileColumns = 1, int tabletColumns = 2, int desktopColumns = 3}) {
    if (isMobile(context)) return mobileColumns;
    if (isTablet(context)) return tabletColumns;
    return desktopColumns;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= Breakpoints.tablet) {
          return desktop;
        } else if (constraints.maxWidth >= Breakpoints.mobile) {
          return tablet ?? mobile;
        }
        return mobile;
      },
    );
  }
}

/// A widget that adapts its child based on screen size
class ResponsiveBuilder extends StatelessWidget {
  final Widget Function(BuildContext context, bool isMobile, bool isTablet, bool isDesktop) builder;

  const ResponsiveBuilder({
    super.key,
    required this.builder,
  });

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isMobile = constraints.maxWidth < Breakpoints.mobile;
        final isTablet = constraints.maxWidth >= Breakpoints.mobile &&
                         constraints.maxWidth < Breakpoints.tablet;
        final isDesktop = constraints.maxWidth >= Breakpoints.tablet;

        return builder(context, isMobile, isTablet, isDesktop);
      },
    );
  }
}

/// A responsive visibility widget that shows/hides children based on breakpoints
class ResponsiveVisibility extends StatelessWidget {
  final Widget child;
  final bool visibleOnMobile;
  final bool visibleOnTablet;
  final bool visibleOnDesktop;
  final Widget? replacement;

  const ResponsiveVisibility({
    super.key,
    required this.child,
    this.visibleOnMobile = true,
    this.visibleOnTablet = true,
    this.visibleOnDesktop = true,
    this.replacement,
  });

  @override
  Widget build(BuildContext context) {
    return ResponsiveBuilder(
      builder: (context, isMobile, isTablet, isDesktop) {
        final shouldShow = (isMobile && visibleOnMobile) ||
            (isTablet && visibleOnTablet) ||
            (isDesktop && visibleOnDesktop);

        if (shouldShow) {
          return child;
        }
        return replacement ?? const SizedBox.shrink();
      },
    );
  }
}
