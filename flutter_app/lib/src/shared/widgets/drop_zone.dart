import 'dart:io';

import 'package:flutter/material.dart';
import 'package:desktop_drop/desktop_drop.dart';

/// A widget that accepts file drops from the operating system
class DropZone extends StatefulWidget {
  final Widget child;
  final Function(DropDoneDetails details)? onFilesDropped;
  final bool enabled;

  const DropZone({
    super.key,
    required this.child,
    this.onFilesDropped,
    this.enabled = true,
  });

  @override
  State<DropZone> createState() => _DropZoneState();
}

class _DropZoneState extends State<DropZone> {
  bool _isDragging = false;

  @override
  Widget build(BuildContext context) {
    // Only enable on desktop platforms
    final isDesktop =
        Platform.isWindows || Platform.isLinux || Platform.isMacOS;
    if (!isDesktop || !widget.enabled) {
      return widget.child;
    }

    return DropTarget(
      onDragEntered: (details) {
        setState(() => _isDragging = true);
      },
      onDragExited: (details) {
        setState(() => _isDragging = false);
      },
      onDragDone: (details) {
        setState(() => _isDragging = false);
        if (details.files.isNotEmpty) {
          widget.onFilesDropped?.call(details);
        }
      },
      child: Stack(
        children: [
          widget.child,
          if (_isDragging)
            Positioned.fill(
              child: _DropOverlay(),
            ),
        ],
      ),
    );
  }
}

/// Overlay shown when dragging files over the drop zone
class _DropOverlay extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.primary.withValues(alpha: 0.1),
        border: Border.all(
          color: theme.colorScheme.primary,
          width: 2,
        ),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.file_upload_outlined,
              size: 48,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(height: 16),
            Text(
              'Drop files to share',
              style: theme.textTheme.titleMedium?.copyWith(
                color: theme.colorScheme.primary,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              'Files will be sent to all connected devices',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// A card-style drop zone with visual feedback
class DropZoneCard extends StatefulWidget {
  final Function(DropDoneDetails details)? onFilesDropped;
  final bool enabled;
  final String? title;
  final String? subtitle;

  const DropZoneCard({
    super.key,
    this.onFilesDropped,
    this.enabled = true,
    this.title,
    this.subtitle,
  });

  @override
  State<DropZoneCard> createState() => _DropZoneCardState();
}

class _DropZoneCardState extends State<DropZoneCard> {
  bool _isDragging = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDesktop =
        Platform.isWindows || Platform.isLinux || Platform.isMacOS;

    if (!isDesktop) {
      return const SizedBox.shrink();
    }

    return DropTarget(
      enable: widget.enabled,
      onDragEntered: (details) {
        setState(() => _isDragging = true);
      },
      onDragExited: (details) {
        setState(() => _isDragging = false);
      },
      onDragDone: (details) {
        setState(() => _isDragging = false);
        if (details.files.isNotEmpty) {
          widget.onFilesDropped?.call(details);
        }
      },
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        padding: const EdgeInsets.all(24),
        decoration: BoxDecoration(
          color: _isDragging
              ? theme.colorScheme.primaryContainer
              : theme.colorScheme.surfaceContainerHighest,
          border: Border.all(
            color: _isDragging
                ? theme.colorScheme.primary
                : theme.colorScheme.outline.withValues(alpha: 0.3),
            width: _isDragging ? 2 : 1,
            strokeAlign: BorderSide.strokeAlignInside,
          ),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              _isDragging ? Icons.file_download : Icons.file_upload_outlined,
              size: 32,
              color: _isDragging
                  ? theme.colorScheme.primary
                  : theme.colorScheme.outline,
            ),
            const SizedBox(height: 12),
            Text(
              widget.title ??
                  (_isDragging ? 'Drop to share' : 'Drag files here'),
              style: theme.textTheme.titleSmall?.copyWith(
                color: _isDragging
                    ? theme.colorScheme.primary
                    : theme.colorScheme.onSurface,
                fontWeight: _isDragging ? FontWeight.bold : FontWeight.normal,
              ),
            ),
            if (widget.subtitle != null || !_isDragging) ...[
              const SizedBox(height: 4),
              Text(
                widget.subtitle ?? 'Files will be sent to connected devices',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.outline,
                ),
                textAlign: TextAlign.center,
              ),
            ],
          ],
        ),
      ),
    );
  }
}
