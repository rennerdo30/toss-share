import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:window_manager/window_manager.dart';

import 'responsive_layout.dart';
import 'app_sidebar.dart';

/// Provider for sidebar collapsed state
final sidebarCollapsedProvider = StateProvider<bool>((ref) => false);

/// Desktop shell that wraps content with sidebar on larger screens
class DesktopShell extends ConsumerWidget {
  final Widget child;

  const DesktopShell({
    super.key,
    required this.child,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isDesktop = Platform.isWindows || Platform.isLinux || Platform.isMacOS;
    final currentRoute = GoRouterState.of(context).uri.path;
    final isCollapsed = ref.watch(sidebarCollapsedProvider);

    return Scaffold(
      body: Column(
        children: [
          // Custom title bar for desktop
          if (isDesktop) _CustomTitleBar(),

          // Main content with sidebar
          Expanded(
            child: ResponsiveBuilder(
              builder: (context, isMobile, isTablet, isDesktopSize) {
                // Mobile: no sidebar, just content
                if (isMobile) {
                  return child;
                }

                // Tablet/Desktop: sidebar + content
                final shouldCollapse = isTablet || isCollapsed;
                return Row(
                  children: [
                    AppSidebar(
                      currentRoute: currentRoute,
                      isCollapsed: shouldCollapse,
                      onToggleCollapse: isDesktopSize
                          ? () => ref.read(sidebarCollapsedProvider.notifier)
                              .state = !isCollapsed
                          : null,
                    ),
                    Expanded(
                      child: child,
                    ),
                  ],
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}

/// Custom title bar for window controls
class _CustomTitleBar extends StatefulWidget {
  @override
  State<_CustomTitleBar> createState() => _CustomTitleBarState();
}

class _CustomTitleBarState extends State<_CustomTitleBar> {
  bool _isMaximized = false;

  @override
  void initState() {
    super.initState();
    _checkMaximized();
  }

  Future<void> _checkMaximized() async {
    final maximized = await windowManager.isMaximized();
    if (mounted) {
      setState(() => _isMaximized = maximized);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isMacOS = Platform.isMacOS;

    return GestureDetector(
      onPanStart: (_) => windowManager.startDragging(),
      child: Container(
        height: 38,
        decoration: BoxDecoration(
          color: theme.colorScheme.surface,
          border: Border(
            bottom: BorderSide(
              color: theme.colorScheme.outlineVariant,
              width: 1,
            ),
          ),
        ),
        child: Row(
          children: [
            // macOS traffic lights spacing
            if (isMacOS) const SizedBox(width: 78),

            // Title
            Expanded(
              child: Center(
                child: Text(
                  'Toss',
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ),

            // Window controls for Windows/Linux
            if (!isMacOS) ...[
              _WindowButton(
                icon: Icons.remove,
                onPressed: () => windowManager.minimize(),
                tooltip: 'Minimize',
              ),
              _WindowButton(
                icon: _isMaximized ? Icons.filter_none : Icons.crop_square,
                onPressed: () async {
                  if (_isMaximized) {
                    await windowManager.unmaximize();
                  } else {
                    await windowManager.maximize();
                  }
                  await _checkMaximized();
                },
                tooltip: _isMaximized ? 'Restore' : 'Maximize',
              ),
              _WindowButton(
                icon: Icons.close,
                onPressed: () => windowManager.close(),
                tooltip: 'Close',
                isClose: true,
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _WindowButton extends StatefulWidget {
  final IconData icon;
  final VoidCallback onPressed;
  final String tooltip;
  final bool isClose;

  const _WindowButton({
    required this.icon,
    required this.onPressed,
    required this.tooltip,
    this.isClose = false,
  });

  @override
  State<_WindowButton> createState() => _WindowButtonState();
}

class _WindowButtonState extends State<_WindowButton> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return MouseRegion(
      onEnter: (_) => setState(() => _isHovered = true),
      onExit: (_) => setState(() => _isHovered = false),
      child: Tooltip(
        message: widget.tooltip,
        child: GestureDetector(
          onTap: widget.onPressed,
          child: Container(
            width: 46,
            height: 38,
            color: _isHovered
                ? (widget.isClose
                    ? Colors.red
                    : theme.colorScheme.onSurface.withValues(alpha: 0.1))
                : Colors.transparent,
            child: Icon(
              widget.icon,
              size: 16,
              color: _isHovered && widget.isClose
                  ? Colors.white
                  : theme.colorScheme.onSurface.withValues(alpha: 0.7),
            ),
          ),
        ),
      ),
    );
  }
}
