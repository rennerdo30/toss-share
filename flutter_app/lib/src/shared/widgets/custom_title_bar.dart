import 'dart:io';

import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';

/// Custom title bar for desktop platforms with window controls
class CustomTitleBar extends StatelessWidget {
  final String title;
  final Widget? leading;
  final List<Widget>? actions;
  final bool showBackButton;
  final VoidCallback? onBackPressed;

  const CustomTitleBar({
    super.key,
    this.title = 'Toss',
    this.leading,
    this.actions,
    this.showBackButton = false,
    this.onBackPressed,
  });

  @override
  Widget build(BuildContext context) {
    // Only show custom title bar on desktop
    if (!Platform.isWindows && !Platform.isMacOS && !Platform.isLinux) {
      return const SizedBox.shrink();
    }

    final theme = Theme.of(context);

    return GestureDetector(
      onPanStart: (_) => windowManager.startDragging(),
      child: Container(
        height: 38,
        decoration: BoxDecoration(
          color: theme.colorScheme.surface,
          border: Border(
            bottom: BorderSide(
              color: theme.dividerColor.withValues(alpha: 0.1),
            ),
          ),
        ),
        child: Row(
          children: [
            // macOS: window controls on left
            if (Platform.isMacOS) ...[
              const SizedBox(width: 78), // Space for traffic lights
            ],

            // Leading widget or back button
            if (showBackButton)
              IconButton(
                icon: const Icon(Icons.arrow_back, size: 18),
                onPressed: onBackPressed ?? () => Navigator.of(context).pop(),
                tooltip: 'Back',
                padding: const EdgeInsets.all(8),
                constraints: const BoxConstraints(
                  minWidth: 32,
                  minHeight: 32,
                ),
              )
            else if (leading != null)
              leading!
            else
              const SizedBox(width: 8),

            // Title
            Expanded(
              child: Text(
                title,
                style: theme.textTheme.titleSmall?.copyWith(
                  fontWeight: FontWeight.w500,
                ),
                textAlign: Platform.isMacOS ? TextAlign.center : TextAlign.left,
              ),
            ),

            // Actions
            if (actions != null) ...actions!,

            // Windows/Linux: window controls on right
            if (Platform.isWindows || Platform.isLinux) ...[
              _WindowButton(
                icon: Icons.remove,
                onPressed: () => windowManager.minimize(),
                tooltip: 'Minimize',
              ),
              _WindowButton(
                icon: Icons.crop_square,
                onPressed: () async {
                  if (await windowManager.isMaximized()) {
                    windowManager.unmaximize();
                  } else {
                    windowManager.maximize();
                  }
                },
                tooltip: 'Maximize',
              ),
              _WindowButton(
                icon: Icons.close,
                onPressed: () => windowManager.close(),
                tooltip: 'Close',
                isClose: true,
              ),
            ],

            // macOS: add some padding on right
            if (Platform.isMacOS) const SizedBox(width: 8),
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
