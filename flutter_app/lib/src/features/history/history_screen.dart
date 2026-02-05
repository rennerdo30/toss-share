import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers/clipboard_provider.dart';
import '../../core/providers/devices_provider.dart';
import '../../core/models/clipboard_item.dart';
import '../../core/services/toss_service.dart';
import '../../shared/widgets/responsive_layout.dart';
import '../../shared/widgets/context_menu.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/utils/content_type_utils.dart';
import '../../shared/utils/timestamp_utils.dart';
import 'widgets/history_data_table.dart';

class HistoryScreen extends ConsumerStatefulWidget {
  const HistoryScreen({super.key});

  @override
  ConsumerState<HistoryScreen> createState() => _HistoryScreenState();
}

class _HistoryScreenState extends ConsumerState<HistoryScreen> {
  final TextEditingController _searchController = TextEditingController();
  ClipboardContentType? _selectedContentType;
  String? _selectedDeviceId;
  DateTime? _startDate;
  DateTime? _endDate;
  bool _showFilters = false;
  bool _isLoading = false;

  @override
  void initState() {
    super.initState();
    // Load history when screen is first shown
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadHistory();
    });
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _loadHistory() async {
    if (_isLoading) return;
    setState(() => _isLoading = true);
    try {
      await ref.read(clipboardHistoryProvider.notifier).loadHistory(
            startDate: _startDate,
            endDate: _endDate,
          );
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _copyItem(BuildContext context, ClipboardItem item) async {
    try {
      // Get decrypted content from history
      final content = await TossService.getHistoryItemContent(item.id);
      if (content == null) {
        // Fallback to preview if decryption fails
        await Clipboard.setData(ClipboardData(text: item.preview));
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('Copied preview to clipboard')),
          );
        }
        return;
      }

      // Copy based on content type
      switch (item.contentType) {
        case ClipboardContentType.text:
        case ClipboardContentType.richText:
        case ClipboardContentType.url:
          // Decode text from bytes
          final text = String.fromCharCodes(content.data);
          await Clipboard.setData(ClipboardData(text: text));
          // Also send via Toss if available
          await TossService.sendText(text);
          break;
        case ClipboardContentType.image:
          // For images, set image data directly
          // Note: This requires platform-specific clipboard handling
          // For now, fallback to text preview
          await Clipboard.setData(ClipboardData(text: item.preview));
          break;
        case ClipboardContentType.file:
          // Files would need special handling
          await Clipboard.setData(ClipboardData(text: item.preview));
          break;
      }
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Copied to clipboard')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to copy: $e')),
        );
      }
    }
  }

  Future<void> _deleteItem(ClipboardItem item) async {
    // Remove from Rust and Flutter
    await TossService.removeHistoryItem(item.id);
    ref.read(clipboardHistoryProvider.notifier).removeItem(item.id);
  }

  Future<void> _sendToDevice(BuildContext context, ClipboardItem item) async {
    try {
      // Get decrypted content from history
      final content = await TossService.getHistoryItemContent(item.id);
      if (content == null) {
        if (context.mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(content: Text('Failed to decrypt content')),
          );
        }
        return;
      }

      // Send based on content type
      switch (item.contentType) {
        case ClipboardContentType.text:
        case ClipboardContentType.richText:
        case ClipboardContentType.url:
          final text = String.fromCharCodes(content.data);
          await TossService.sendText(text);
          break;
        case ClipboardContentType.image:
        case ClipboardContentType.file:
          // Binary content not yet supported for direct sending
          if (context.mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                  content: Text('Binary content sending not yet supported')),
            );
          }
          return;
      }

      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Sent to devices')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Failed to send: $e')),
        );
      }
    }
  }

  List<ClipboardItem> _filterHistory(List<ClipboardItem> history) {
    var filtered = history;

    // Search filter
    final query = _searchController.text.toLowerCase();
    if (query.isNotEmpty) {
      filtered = filtered.where((item) {
        return item.preview.toLowerCase().contains(query);
      }).toList();
    }

    // Content type filter
    if (_selectedContentType != null) {
      filtered = filtered.where((item) {
        return item.contentType == _selectedContentType;
      }).toList();
    }

    // Source device filter
    if (_selectedDeviceId != null) {
      filtered = filtered.where((item) {
        return item.sourceDeviceId == _selectedDeviceId;
      }).toList();
    }

    // Date range filter
    if (_startDate != null || _endDate != null) {
      filtered = filtered.where((item) {
        if (_startDate != null && item.timestamp.isBefore(_startDate!)) {
          return false;
        }
        if (_endDate != null && item.timestamp.isAfter(_endDate!)) {
          return false;
        }
        return true;
      }).toList();
    }

    return filtered;
  }

  @override
  Widget build(BuildContext context) {
    final history = ref.watch(clipboardHistoryProvider);
    final filteredHistory = _filterHistory(history);

    // Show loading indicator while loading
    if (_isLoading && history.isEmpty) {
      return Scaffold(
        appBar: AppBar(title: const Text('Clipboard History')),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('Clipboard History'),
        actions: [
          IconButton(
            icon: Icon(
                _showFilters ? Icons.filter_list : Icons.filter_list_outlined),
            tooltip: 'Filters',
            onPressed: () {
              setState(() {
                _showFilters = !_showFilters;
              });
            },
          ),
          if (filteredHistory.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.delete_sweep),
              tooltip: 'Clear History',
              onPressed: () {
                _showClearDialog(context, ref);
              },
            ),
        ],
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.all(8.0),
            child: TextField(
              controller: _searchController,
              decoration: InputDecoration(
                hintText: 'Search history...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchController.text.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear),
                        onPressed: () {
                          setState(() {
                            _searchController.clear();
                          });
                        },
                      )
                    : null,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(12),
                ),
              ),
              onChanged: (_) => setState(() {}),
            ),
          ),
          // Filters
          if (_showFilters) _buildFilters(context),
          // History list - responsive view
          Expanded(
            child: filteredHistory.isEmpty
                ? _EmptyState(
                    hasFilters: _searchController.text.isNotEmpty ||
                        _selectedContentType != null ||
                        _selectedDeviceId != null ||
                        _startDate != null ||
                        _endDate != null,
                  )
                : ResponsiveBuilder(
                    builder: (context, isMobile, isTablet, isDesktop) {
                      final devices = ref.watch(devicesProvider);
                      final hasDevices = devices.isNotEmpty;

                      // Desktop/Tablet: DataTable view
                      if (!isMobile) {
                        return HistoryDataTable(
                          items: filteredHistory,
                          hasDevices: hasDevices,
                          onCopy: (item) => _copyItem(context, item),
                          onDelete: (item) => _deleteItem(item),
                          onSendToDevice: hasDevices
                              ? (item) => _sendToDevice(context, item)
                              : null,
                        );
                      }

                      // Mobile: Card list view
                      return ListView.builder(
                        padding: const EdgeInsets.all(16),
                        itemCount: filteredHistory.length,
                        itemBuilder: (context, index) {
                          final item = filteredHistory[index];
                          return ContextMenuRegion(
                            items: ClipboardHistoryContextMenu.build(
                              onCopy: () => _copyItem(context, item),
                              onSendToDevice: () =>
                                  _sendToDevice(context, item),
                              onDelete: () => _deleteItem(item),
                              hasDevices: hasDevices,
                            ),
                            child: _HistoryItem(
                              item: item,
                              onCopy: () => _copyItem(context, item),
                              onDelete: () => _deleteItem(item),
                            ),
                          );
                        },
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }

  void _showClearDialog(BuildContext context, WidgetRef ref) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Clear History'),
        content:
            const Text('This will delete all clipboard history. Continue?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () async {
              // Clear history in Rust and Flutter
              await TossService.clearClipboardHistory();
              if (context.mounted) {
                ref.read(clipboardHistoryProvider.notifier).clearHistory();
                Navigator.pop(context);
              }
            },
            child: const Text('Clear'),
          ),
        ],
      ),
    );
  }

  Widget _buildFilters(BuildContext context) {
    final history = ref.watch(clipboardHistoryProvider);
    final devices = history
        .where((item) => item.sourceDeviceId != null)
        .map((item) => item.sourceDeviceId!)
        .toSet()
        .toList();

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        border: Border(
          bottom: BorderSide(
            color: Theme.of(context).dividerColor,
          ),
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Text(
                'Filters',
                style: Theme.of(context).textTheme.titleSmall,
              ),
              const Spacer(),
              TextButton(
                onPressed: () {
                  final hadDateFilter = _startDate != null || _endDate != null;
                  setState(() {
                    _selectedContentType = null;
                    _selectedDeviceId = null;
                    _startDate = null;
                    _endDate = null;
                  });
                  // Reload history if date filters were cleared (to fetch all data)
                  if (hadDateFilter) {
                    _loadHistory();
                  }
                },
                child: const Text('Clear Filters'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Content type filter
          Wrap(
            spacing: 8,
            children: [
              _FilterChip(
                label: 'All Types',
                selected: _selectedContentType == null,
                onSelected: (selected) {
                  if (selected) {
                    setState(() {
                      _selectedContentType = null;
                    });
                  }
                },
              ),
              for (final type in ClipboardContentType.values)
                _FilterChip(
                  label: type.displayName,
                  selected: _selectedContentType == type,
                  onSelected: (selected) {
                    setState(() {
                      _selectedContentType = selected ? type : null;
                    });
                  },
                ),
            ],
          ),
          const SizedBox(height: 8),
          // Device filter
          if (devices.isNotEmpty) ...[
            DropdownButtonFormField<String>(
              initialValue: _selectedDeviceId,
              decoration: const InputDecoration(
                labelText: 'Source Device',
                border: OutlineInputBorder(),
                isDense: true,
              ),
              items: [
                const DropdownMenuItem(
                  value: null,
                  child: Text('All Devices'),
                ),
                ...devices.map((deviceId) {
                  final deviceName = history
                          .firstWhere((item) => item.sourceDeviceId == deviceId)
                          .sourceDeviceName ??
                      'Unknown Device';
                  return DropdownMenuItem(
                    value: deviceId,
                    child: Text(deviceName),
                  );
                }),
              ],
              onChanged: (value) {
                setState(() {
                  _selectedDeviceId = value;
                });
              },
            ),
            const SizedBox(height: 8),
          ],
          // Date range filter
          Row(
            children: [
              Expanded(
                child: _startDate != null
                    ? FilledButton.icon(
                        onPressed: () async {
                          final date = await showDatePicker(
                            context: context,
                            initialDate: _startDate ?? DateTime.now(),
                            firstDate: DateTime(2020),
                            lastDate: DateTime.now(),
                          );
                          if (date != null) {
                            setState(() {
                              _startDate = date;
                            });
                            _loadHistory();
                          }
                        },
                        icon: const Icon(Icons.calendar_today, size: 16),
                        label: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                                '${_startDate!.month}/${_startDate!.day}/${_startDate!.year}'),
                            const SizedBox(width: 4),
                            GestureDetector(
                              onTap: () {
                                setState(() {
                                  _startDate = null;
                                });
                                _loadHistory();
                              },
                              child: const Icon(Icons.close, size: 16),
                            ),
                          ],
                        ),
                      )
                    : OutlinedButton.icon(
                        onPressed: () async {
                          final date = await showDatePicker(
                            context: context,
                            initialDate: DateTime.now(),
                            firstDate: DateTime(2020),
                            lastDate: DateTime.now(),
                          );
                          if (date != null) {
                            setState(() {
                              _startDate = date;
                            });
                            _loadHistory();
                          }
                        },
                        icon: const Icon(Icons.calendar_today, size: 16),
                        label: const Text('Start Date'),
                      ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _endDate != null
                    ? FilledButton.icon(
                        onPressed: () async {
                          final date = await showDatePicker(
                            context: context,
                            initialDate: _endDate ?? DateTime.now(),
                            firstDate: _startDate ?? DateTime(2020),
                            lastDate: DateTime.now(),
                          );
                          if (date != null) {
                            setState(() {
                              _endDate = date;
                            });
                            _loadHistory();
                          }
                        },
                        icon: const Icon(Icons.calendar_today, size: 16),
                        label: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                                '${_endDate!.month}/${_endDate!.day}/${_endDate!.year}'),
                            const SizedBox(width: 4),
                            GestureDetector(
                              onTap: () {
                                setState(() {
                                  _endDate = null;
                                });
                                _loadHistory();
                              },
                              child: const Icon(Icons.close, size: 16),
                            ),
                          ],
                        ),
                      )
                    : OutlinedButton.icon(
                        onPressed: () async {
                          final date = await showDatePicker(
                            context: context,
                            initialDate: DateTime.now(),
                            firstDate: _startDate ?? DateTime(2020),
                            lastDate: DateTime.now(),
                          );
                          if (date != null) {
                            setState(() {
                              _endDate = date;
                            });
                            _loadHistory();
                          }
                        },
                        icon: const Icon(Icons.calendar_today, size: 16),
                        label: const Text('End Date'),
                      ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _FilterChip extends StatelessWidget {
  final String label;
  final bool selected;
  final ValueChanged<bool> onSelected;

  const _FilterChip({
    required this.label,
    required this.selected,
    required this.onSelected,
  });

  @override
  Widget build(BuildContext context) {
    return FilterChip(
      label: Text(label),
      selected: selected,
      onSelected: onSelected,
    );
  }
}

class _HistoryItem extends StatelessWidget {
  final ClipboardItem item;
  final VoidCallback onCopy;
  final VoidCallback onDelete;

  const _HistoryItem({
    required this.item,
    required this.onCopy,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: InkWell(
        onTap: onCopy,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Content type icon
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(
                  getContentTypeIcon(item.contentType),
                  size: 20,
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                ),
              ),
              const SizedBox(width: 12),

              // Content preview
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      item.preview,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.bodyMedium,
                    ),
                    const SizedBox(height: 4),
                    Row(
                      children: [
                        Text(
                          formatRelativeTimestamp(item.timestamp),
                          style: Theme.of(context)
                              .textTheme
                              .bodySmall
                              ?.copyWith(
                                color: Theme.of(context).colorScheme.outline,
                              ),
                        ),
                        if (item.sourceDeviceName != null) ...[
                          Text(
                            ' • ',
                            style: Theme.of(context)
                                .textTheme
                                .bodySmall
                                ?.copyWith(
                                  color: Theme.of(context).colorScheme.outline,
                                ),
                          ),
                          Text(
                            'from ${item.sourceDeviceName}',
                            style: Theme.of(context)
                                .textTheme
                                .bodySmall
                                ?.copyWith(
                                  color: Theme.of(context).colorScheme.outline,
                                ),
                          ),
                        ],
                      ],
                    ),
                  ],
                ),
              ),

              // Actions
              IconButton(
                icon: const Icon(Icons.delete_outline),
                tooltip: 'Delete',
                onPressed: onDelete,
                visualDensity: VisualDensity.compact,
              ),
            ],
          ),
        ),
      ),
    );
  }

}

class _EmptyState extends StatelessWidget {
  final bool hasFilters;

  const _EmptyState({this.hasFilters = false});

  @override
  Widget build(BuildContext context) {
    return EmptyState(
      icon: hasFilters ? Icons.filter_alt_off : Icons.history,
      title: hasFilters ? 'No items match filters' : 'No clipboard history',
      subtitle: hasFilters
          ? 'Try adjusting your search or filters'
          : 'Synced clipboard items will appear here',
    );
  }
}
