import 'package:flutter/material.dart';

import '../../../core/models/clipboard_item.dart';

class ClipboardPreviewCard extends StatelessWidget {
  final ClipboardItem? item;
  final VoidCallback? onRefresh;

  const ClipboardPreviewCard({
    super.key,
    this.item,
    this.onRefresh,
  });

  @override
  Widget build(BuildContext context) {
    if (item == null) {
      return _EmptyClipboard(onRefresh: onRefresh);
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Icon(
                  _getContentTypeIcon(item!.contentType),
                  size: 20,
                  color: Theme.of(context).colorScheme.primary,
                ),
                const SizedBox(width: 8),
                Text(
                  item!.contentType.displayName,
                  style: Theme.of(context).textTheme.labelMedium?.copyWith(
                        color: Theme.of(context).colorScheme.primary,
                      ),
                ),
                const Spacer(),
                Text(
                  item!.formattedSize,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.outline,
                      ),
                ),
                if (onRefresh != null) ...[
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.refresh, size: 18),
                    onPressed: onRefresh,
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ],
            ),
            const SizedBox(height: 12),

            // Preview content
            Expanded(
              child: _buildPreview(context),
            ),

            // Source info
            if (item!.sourceDeviceName != null) ...[
              const Divider(),
              Row(
                children: [
                  Icon(
                    Icons.arrow_downward,
                    size: 14,
                    color: Theme.of(context).colorScheme.outline,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    'From ${item!.sourceDeviceName}',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Theme.of(context).colorScheme.outline,
                        ),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildPreview(BuildContext context) {
    switch (item!.contentType) {
      case ClipboardContentType.text:
      case ClipboardContentType.richText:
      case ClipboardContentType.url:
        return SingleChildScrollView(
          child: SelectableText(
            item!.preview,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        );

      case ClipboardContentType.image:
        return Container(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(8),
          ),
          child: const Center(
            child: Icon(Icons.image, size: 48),
          ),
        );

      case ClipboardContentType.file:
        return Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.insert_drive_file,
              size: 48,
              color: Theme.of(context).colorScheme.outline,
            ),
            const SizedBox(height: 8),
            Text(
              item!.preview,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ],
        );
    }
  }

  IconData _getContentTypeIcon(ClipboardContentType type) {
    switch (type) {
      case ClipboardContentType.text:
        return Icons.text_fields;
      case ClipboardContentType.richText:
        return Icons.format_paint;
      case ClipboardContentType.image:
        return Icons.image;
      case ClipboardContentType.file:
        return Icons.attach_file;
      case ClipboardContentType.url:
        return Icons.link;
    }
  }
}

class _EmptyClipboard extends StatelessWidget {
  final VoidCallback? onRefresh;

  const _EmptyClipboard({this.onRefresh});

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Center(
        child: SingleChildScrollView(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                Icons.content_paste_off,
                size: 48,
                color: Theme.of(context).colorScheme.outline,
              ),
              const SizedBox(height: 16),
              Text(
                'Clipboard is empty',
                style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                      color: Theme.of(context).colorScheme.outline,
                    ),
              ),
              if (onRefresh != null) ...[
                const SizedBox(height: 16),
                TextButton.icon(
                  onPressed: onRefresh,
                  icon: const Icon(Icons.refresh),
                  label: const Text('Refresh'),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
