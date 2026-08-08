/// Layout constants used throughout the app
class LayoutConstants {
  LayoutConstants._();

  // Breakpoints
  static const double mobileBreakpoint = 600;
  static const double tabletBreakpoint = 900;
  static const double desktopBreakpoint = 1200;

  // Component heights
  static const double deviceListHeight = 100;
  static const double deviceListHeightLandscape = 80;
  static const double cameraHeight = 250;
  static const double titleBarHeight = 38;

  // Widths
  static const double sidebarWidth = 250;
  static const double maxDialogWidth = 400;
  static const double historyPreviewWidth = 300;
  static const double deviceCardWidth = 100;

  // Spacing
  static const double defaultPadding = 16.0;
  static const double smallPadding = 8.0;
  static const double largePadding = 24.0;
  static const double gutter = 12.0;

  /// Space reserved below scrollable content so a floating action button never
  /// covers the last item.
  static const double fabClearance = 88.0;

  // Border radius
  static const double defaultBorderRadius = 12.0;
  static const double smallBorderRadius = 8.0;

  // Icon sizes
  static const double smallIconSize = 16.0;
  static const double defaultIconSize = 24.0;
  static const double largeIconSize = 32.0;
  static const double emptyStateIconSize = 64.0;
}
