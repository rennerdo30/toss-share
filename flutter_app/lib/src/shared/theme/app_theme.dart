import 'package:flutter/material.dart';

import '../constants/layout_constants.dart';

/// Semantic status colours (online / offline / warning) that do not belong in
/// [ColorScheme]. Exposed as a [ThemeExtension] so both themes can provide
/// values with enough contrast on their own background.
@immutable
class AppStatusColors extends ThemeExtension<AppStatusColors> {
  const AppStatusColors({
    required this.online,
    required this.offline,
    required this.warning,
  });

  final Color online;
  final Color offline;
  final Color warning;

  static const AppStatusColors lightColors = AppStatusColors(
    online: Color(0xFF16A34A),
    offline: Color(0xFF94A3B8),
    warning: Color(0xFFD97706),
  );

  static const AppStatusColors darkColors = AppStatusColors(
    online: Color(0xFF4ADE80),
    offline: Color(0xFF94A3B8),
    warning: Color(0xFFFBBF24),
  );

  /// Status colours for the current theme, falling back to the light set when
  /// no extension is registered (for example in isolated widget tests).
  static AppStatusColors of(BuildContext context) =>
      Theme.of(context).extension<AppStatusColors>() ?? lightColors;

  @override
  AppStatusColors copyWith({Color? online, Color? offline, Color? warning}) {
    return AppStatusColors(
      online: online ?? this.online,
      offline: offline ?? this.offline,
      warning: warning ?? this.warning,
    );
  }

  @override
  AppStatusColors lerp(ThemeExtension<AppStatusColors>? other, double t) {
    if (other is! AppStatusColors) return this;
    return AppStatusColors(
      online: Color.lerp(online, other.online, t) ?? online,
      offline: Color.lerp(offline, other.offline, t) ?? offline,
      warning: Color.lerp(warning, other.warning, t) ?? warning,
    );
  }
}

class AppTheme {
  AppTheme._();

  // Brand colors
  static const Color primaryColor = Color(0xFF6366F1);
  static const Color secondaryColor = Color(0xFF8B5CF6);

  // Motion
  static const Duration fastAnimation = Duration(milliseconds: 120);
  static const Duration mediumAnimation = Duration(milliseconds: 220);

  /// Returns [duration], or [Duration.zero] when the platform asks for reduced
  /// motion. Use for implicit animations so the app honours the system
  /// "reduce motion" accessibility setting.
  static Duration motion(
    BuildContext context, [
    Duration duration = mediumAnimation,
  ]) {
    return MediaQuery.disableAnimationsOf(context) ? Duration.zero : duration;
  }

  /// Light theme
  static final ThemeData light = _build(Brightness.light);

  /// Dark theme
  static final ThemeData dark = _build(Brightness.dark);

  /// Builds a theme for [brightness]. Both themes share every component style
  /// so light and dark can never drift apart.
  static ThemeData _build(Brightness brightness) {
    final isDark = brightness == Brightness.dark;
    final colorScheme = ColorScheme.fromSeed(
      seedColor: primaryColor,
      brightness: brightness,
    );
    final base = ThemeData(useMaterial3: true, colorScheme: colorScheme);
    final borderRadius =
        BorderRadius.circular(LayoutConstants.defaultBorderRadius);
    final smallBorderRadius =
        BorderRadius.circular(LayoutConstants.smallBorderRadius);
    const buttonPadding = EdgeInsets.symmetric(
      horizontal: LayoutConstants.largePadding,
      vertical: LayoutConstants.defaultPadding - 2,
    );

    return base.copyWith(
      extensions: <ThemeExtension<dynamic>>[
        isDark ? AppStatusColors.darkColors : AppStatusColors.lightColors,
      ],
      textTheme: _refineTextTheme(base.textTheme),
      appBarTheme: const AppBarTheme(
        centerTitle: true,
        elevation: 0,
        scrolledUnderElevation: 1,
      ),
      cardTheme: CardThemeData(
        elevation: 0,
        clipBehavior: Clip.antiAlias,
        shape: RoundedRectangleBorder(
          borderRadius: borderRadius,
          side: BorderSide(color: colorScheme.outlineVariant),
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: colorScheme.surfaceContainerHighest,
        border: OutlineInputBorder(
          borderRadius: borderRadius,
          borderSide: BorderSide.none,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: borderRadius,
          borderSide: BorderSide.none,
        ),
        // Without an explicit focused border a filled field gives no visible
        // focus indication at all.
        focusedBorder: OutlineInputBorder(
          borderRadius: borderRadius,
          borderSide: BorderSide(color: colorScheme.primary, width: 2),
        ),
        errorBorder: OutlineInputBorder(
          borderRadius: borderRadius,
          borderSide: BorderSide(color: colorScheme.error),
        ),
        focusedErrorBorder: OutlineInputBorder(
          borderRadius: borderRadius,
          borderSide: BorderSide(color: colorScheme.error, width: 2),
        ),
        contentPadding: const EdgeInsets.symmetric(
          horizontal: LayoutConstants.defaultPadding,
          vertical: LayoutConstants.defaultPadding - 2,
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          elevation: 0,
          padding: buttonPadding,
          shape: RoundedRectangleBorder(borderRadius: borderRadius),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          padding: buttonPadding,
          shape: RoundedRectangleBorder(borderRadius: borderRadius),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          padding: buttonPadding,
          shape: RoundedRectangleBorder(borderRadius: borderRadius),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          padding: const EdgeInsets.symmetric(
            horizontal: LayoutConstants.defaultPadding,
            vertical: LayoutConstants.smallPadding + 2,
          ),
          shape: RoundedRectangleBorder(borderRadius: smallBorderRadius),
        ),
      ),
      listTileTheme: ListTileThemeData(
        shape: RoundedRectangleBorder(borderRadius: smallBorderRadius),
      ),
      dividerTheme: DividerThemeData(
        color: colorScheme.outlineVariant,
        space: 1,
        thickness: 1,
      ),
      chipTheme: ChipThemeData(
        shape: RoundedRectangleBorder(borderRadius: smallBorderRadius),
        side: BorderSide(color: colorScheme.outlineVariant),
      ),
      tooltipTheme: TooltipThemeData(
        waitDuration: const Duration(milliseconds: 400),
        decoration: BoxDecoration(
          color: colorScheme.inverseSurface,
          borderRadius: smallBorderRadius,
        ),
        textStyle: base.textTheme.bodySmall?.copyWith(
          color: colorScheme.onInverseSurface,
        ),
      ),
      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        shape: RoundedRectangleBorder(borderRadius: smallBorderRadius),
      ),
    );
  }

  /// A slightly tighter type scale: headings get more weight, body text keeps
  /// the Material defaults so long content stays readable.
  static TextTheme _refineTextTheme(TextTheme base) {
    return base.copyWith(
      headlineSmall: base.headlineSmall?.copyWith(
        fontWeight: FontWeight.w600,
        letterSpacing: -0.4,
      ),
      titleLarge: base.titleLarge?.copyWith(
        fontWeight: FontWeight.w600,
        letterSpacing: -0.2,
      ),
      titleMedium: base.titleMedium?.copyWith(fontWeight: FontWeight.w600),
      titleSmall: base.titleSmall?.copyWith(fontWeight: FontWeight.w600),
      labelLarge: base.labelLarge?.copyWith(fontWeight: FontWeight.w600),
    );
  }
}
