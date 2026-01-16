import 'dart:io';
import 'package:flutter/material.dart';

/// A context menu item
class ContextMenuItem {
  final IconData? icon;
  final String label;
  final VoidCallback onTap;
  final bool isDangerous;
  final bool enabled;

  const ContextMenuItem({
    this.icon,
    required this.label,
    required this.onTap,
    this.isDangerous = false,
    this.enabled = true,
  });
}

/// A widget that shows a context menu on right-click (desktop) or long-press (mobile)
class ContextMenuRegion extends StatelessWidget {
  final Widget child;
  final List<ContextMenuItem> items;
  final VoidCallback? onTap;

  const ContextMenuRegion({
    super.key,
    required this.child,
    required this.items,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final isDesktop =
        Platform.isWindows || Platform.isLinux || Platform.isMacOS;

    if (isDesktop) {
      return GestureDetector(
        onTap: onTap,
        onSecondaryTapDown: (details) {
          _showContextMenu(context, details.globalPosition);
        },
        child: child,
      );
    }

    // Mobile: use long press
    return GestureDetector(
      onTap: onTap,
      onLongPress: () {
        // Show bottom sheet on mobile
        _showMobileMenu(context);
      },
      child: child,
    );
  }

  void _showContextMenu(BuildContext context, Offset position) {
    final theme = Theme.of(context);
    final overlay = Overlay.of(context).context.findRenderObject() as RenderBox;

    showMenu(
      context: context,
      position: RelativeRect.fromRect(
        Rect.fromLTWH(position.dx, position.dy, 0, 0),
        Offset.zero & overlay.size,
      ),
      items: items.map((item) {
        return PopupMenuItem(
          enabled: item.enabled,
          onTap: item.enabled ? item.onTap : null,
          child: Row(
            children: [
              if (item.icon != null) ...[
                Icon(
                  item.icon,
                  size: 18,
                  color: item.isDangerous
                      ? theme.colorScheme.error
                      : (item.enabled
                          ? theme.colorScheme.onSurface
                          : theme.colorScheme.outline),
                ),
                const SizedBox(width: 12),
              ],
              Text(
                item.label,
                style: TextStyle(
                  color: item.isDangerous
                      ? theme.colorScheme.error
                      : (item.enabled ? null : theme.colorScheme.outline),
                ),
              ),
            ],
          ),
        );
      }).toList(),
    );
  }

  void _showMobileMenu(BuildContext context) {
    final theme = Theme.of(context);

    showModalBottomSheet(
      context: context,
      builder: (context) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: items.map((item) {
            return ListTile(
              enabled: item.enabled,
              leading: item.icon != null
                  ? Icon(
                      item.icon,
                      color: item.isDangerous
                          ? theme.colorScheme.error
                          : (item.enabled
                              ? theme.colorScheme.onSurface
                              : theme.colorScheme.outline),
                    )
                  : null,
              title: Text(
                item.label,
                style: TextStyle(
                  color: item.isDangerous
                      ? theme.colorScheme.error
                      : (item.enabled ? null : theme.colorScheme.outline),
                ),
              ),
              onTap: item.enabled
                  ? () {
                      Navigator.pop(context);
                      item.onTap();
                    }
                  : null,
            );
          }).toList(),
        ),
      ),
    );
  }
}

/// A card widget with built-in context menu support
class ContextMenuCard extends StatefulWidget {
  final Widget child;
  final List<ContextMenuItem> menuItems;
  final VoidCallback? onTap;
  final EdgeInsetsGeometry? padding;
  final EdgeInsetsGeometry? margin;

  const ContextMenuCard({
    super.key,
    required this.child,
    required this.menuItems,
    this.onTap,
    this.padding,
    this.margin,
  });

  @override
  State<ContextMenuCard> createState() => _ContextMenuCardState();
}

class _ContextMenuCardState extends State<ContextMenuCard> {
  bool _isHovered = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return ContextMenuRegion(
      items: widget.menuItems,
      onTap: widget.onTap,
      child: MouseRegion(
        onEnter: (_) => setState(() => _isHovered = true),
        onExit: (_) => setState(() => _isHovered = false),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          margin: widget.margin ?? const EdgeInsets.symmetric(vertical: 4),
          decoration: BoxDecoration(
            color: _isHovered
                ? theme.colorScheme.surfaceContainerHighest
                : theme.colorScheme.surface,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: _isHovered
                  ? theme.colorScheme.outlineVariant
                  : theme.colorScheme.outline.withValues(alpha: 0.2),
            ),
          ),
          child: Padding(
            padding: widget.padding ?? const EdgeInsets.all(12),
            child: widget.child,
          ),
        ),
      ),
    );
  }
}

/// Convenience widget for common device context menu actions
class DeviceContextMenu {
  static List<ContextMenuItem> build({
    required VoidCallback onSendClipboard,
    required VoidCallback onEditName,
    required VoidCallback onCopyId,
    required VoidCallback onRemove,
    bool isOnline = false,
  }) {
    return [
      ContextMenuItem(
        icon: Icons.send,
        label: 'Send clipboard',
        onTap: onSendClipboard,
        enabled: isOnline,
      ),
      ContextMenuItem(
        icon: Icons.edit,
        label: 'Edit name',
        onTap: onEditName,
      ),
      ContextMenuItem(
        icon: Icons.copy,
        label: 'Copy device ID',
        onTap: onCopyId,
      ),
      ContextMenuItem(
        icon: Icons.delete,
        label: 'Remove device',
        onTap: onRemove,
        isDangerous: true,
      ),
    ];
  }
}

/// Convenience widget for common clipboard history context menu actions
class ClipboardHistoryContextMenu {
  static List<ContextMenuItem> build({
    required VoidCallback onCopy,
    required VoidCallback onSendToDevice,
    required VoidCallback onDelete,
    bool hasDevices = false,
  }) {
    return [
      ContextMenuItem(
        icon: Icons.content_copy,
        label: 'Copy',
        onTap: onCopy,
      ),
      ContextMenuItem(
        icon: Icons.send,
        label: 'Send to devices',
        onTap: onSendToDevice,
        enabled: hasDevices,
      ),
      ContextMenuItem(
        icon: Icons.delete,
        label: 'Delete',
        onTap: onDelete,
        isDangerous: true,
      ),
    ];
  }
}

/// Convenience widget for clipboard preview context menu actions
class ClipboardPreviewContextMenu {
  static List<ContextMenuItem> build({
    required VoidCallback onCopy,
    required VoidCallback onClear,
    bool hasContent = false,
  }) {
    return [
      ContextMenuItem(
        icon: Icons.content_copy,
        label: 'Copy',
        onTap: onCopy,
        enabled: hasContent,
      ),
      ContextMenuItem(
        icon: Icons.clear,
        label: 'Clear clipboard',
        onTap: onClear,
        enabled: hasContent,
        isDangerous: true,
      ),
    ];
  }
}
