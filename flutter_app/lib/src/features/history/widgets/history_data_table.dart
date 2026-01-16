import 'package:flutter/material.dart';

import '../../../core/models/clipboard_item.dart';
import '../../../shared/widgets/context_menu.dart';

/// Column types for sorting
enum HistorySortColumn {
  type,
  preview,
  timestamp,
  device,
  size,
}

/// Desktop data table view for clipboard history
class HistoryDataTable extends StatefulWidget {
  final List<ClipboardItem> items;
  final Function(ClipboardItem) onCopy;
  final Function(ClipboardItem) onDelete;
  final Function(ClipboardItem)? onSendToDevice;
  final bool hasDevices;

  const HistoryDataTable({
    super.key,
    required this.items,
    required this.onCopy,
    required this.onDelete,
    this.onSendToDevice,
    this.hasDevices = false,
  });

  @override
  State<HistoryDataTable> createState() => _HistoryDataTableState();
}

class _HistoryDataTableState extends State<HistoryDataTable> {
  HistorySortColumn _sortColumn = HistorySortColumn.timestamp;
  bool _sortAscending = false;

  List<ClipboardItem> get _sortedItems {
    final items = List<ClipboardItem>.from(widget.items);
    items.sort((a, b) {
      int result;
      switch (_sortColumn) {
        case HistorySortColumn.type:
          result =
              a.contentType.displayName.compareTo(b.contentType.displayName);
          break;
        case HistorySortColumn.preview:
          result = a.preview.compareTo(b.preview);
          break;
        case HistorySortColumn.timestamp:
          result = a.timestamp.compareTo(b.timestamp);
          break;
        case HistorySortColumn.device:
          result =
              (a.sourceDeviceName ?? '').compareTo(b.sourceDeviceName ?? '');
          break;
        case HistorySortColumn.size:
          result = a.sizeBytes.compareTo(b.sizeBytes);
          break;
      }
      return _sortAscending ? result : -result;
    });
    return items;
  }

  void _onSort(HistorySortColumn column) {
    setState(() {
      if (_sortColumn == column) {
        _sortAscending = !_sortAscending;
      } else {
        _sortColumn = column;
        _sortAscending = true;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final sortedItems = _sortedItems;

    if (sortedItems.isEmpty) {
      return const SizedBox.shrink();
    }

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      child: SingleChildScrollView(
        child: DataTable(
          sortColumnIndex: _sortColumn.index,
          sortAscending: _sortAscending,
          headingRowColor: WidgetStateProperty.all(
            theme.colorScheme.surfaceContainerHighest,
          ),
          columns: [
            DataColumn(
              label: const Text('Type'),
              onSort: (_, __) => _onSort(HistorySortColumn.type),
            ),
            DataColumn(
              label: const Text('Content'),
              onSort: (_, __) => _onSort(HistorySortColumn.preview),
            ),
            DataColumn(
              label: const Text('Time'),
              onSort: (_, __) => _onSort(HistorySortColumn.timestamp),
            ),
            DataColumn(
              label: const Text('Source'),
              onSort: (_, __) => _onSort(HistorySortColumn.device),
            ),
            DataColumn(
              label: const Text('Size'),
              numeric: true,
              onSort: (_, __) => _onSort(HistorySortColumn.size),
            ),
            const DataColumn(
              label: Text('Actions'),
            ),
          ],
          rows: sortedItems.map((item) {
            return DataRow(
              cells: [
                // Type
                DataCell(
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        _getContentTypeIcon(item.contentType),
                        size: 16,
                        color: theme.colorScheme.primary,
                      ),
                      const SizedBox(width: 8),
                      Text(item.contentType.displayName),
                    ],
                  ),
                ),
                // Content preview
                DataCell(
                  ContextMenuRegion(
                    items: ClipboardHistoryContextMenu.build(
                      onCopy: () => widget.onCopy(item),
                      onSendToDevice: widget.onSendToDevice != null
                          ? () => widget.onSendToDevice!(item)
                          : () {},
                      onDelete: () => widget.onDelete(item),
                      hasDevices: widget.hasDevices,
                    ),
                    onTap: () => widget.onCopy(item),
                    child: SizedBox(
                      width: 300,
                      child: Text(
                        item.preview,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ),
                ),
                // Timestamp
                DataCell(
                  Text(_formatTimestamp(item.timestamp)),
                ),
                // Source device
                DataCell(
                  Text(item.sourceDeviceName ?? 'Local'),
                ),
                // Size
                DataCell(
                  Text(item.formattedSize),
                ),
                // Actions
                DataCell(
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        icon: const Icon(Icons.content_copy, size: 18),
                        tooltip: 'Copy',
                        onPressed: () => widget.onCopy(item),
                        visualDensity: VisualDensity.compact,
                      ),
                      if (widget.onSendToDevice != null && widget.hasDevices)
                        IconButton(
                          icon: const Icon(Icons.send, size: 18),
                          tooltip: 'Send to devices',
                          onPressed: () => widget.onSendToDevice!(item),
                          visualDensity: VisualDensity.compact,
                        ),
                      IconButton(
                        icon: Icon(
                          Icons.delete_outline,
                          size: 18,
                          color: theme.colorScheme.error,
                        ),
                        tooltip: 'Delete',
                        onPressed: () => widget.onDelete(item),
                        visualDensity: VisualDensity.compact,
                      ),
                    ],
                  ),
                ),
              ],
            );
          }).toList(),
        ),
      ),
    );
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

  String _formatTimestamp(DateTime timestamp) {
    final now = DateTime.now();
    final diff = now.difference(timestamp);

    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
    if (diff.inHours < 24) return '${diff.inHours}h ago';
    if (diff.inDays < 7) return '${diff.inDays}d ago';

    return '${timestamp.month}/${timestamp.day} ${timestamp.hour}:${timestamp.minute.toString().padLeft(2, '0')}';
  }
}
