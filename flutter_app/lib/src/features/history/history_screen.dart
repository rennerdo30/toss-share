import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers/clipboard_provider.dart';
import '../../core/models/clipboard_item.dart';
import '../../core/services/toss_service.dart';

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
      await ref.read(clipboardHistoryProvider.notifier).loadHistory();
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
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
            icon: Icon(_showFilters ? Icons.filter_list : Icons.filter_list_outlined),
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
          // History list
          Expanded(
            child: filteredHistory.isEmpty
                ? _EmptyState(
                    hasFilters: _searchController.text.isNotEmpty ||
                        _selectedContentType != null ||
                        _selectedDeviceId != null ||
                        _startDate != null ||
                        _endDate != null,
                  )
                : ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: filteredHistory.length,
                    itemBuilder: (context, index) {
                      final item = filteredHistory[index];
                      return _HistoryItem(
                        item: item,
                        onCopy: () async {
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
                        },
                        onDelete: () async {
                          // Remove from Rust and Flutter
                          await TossService.removeHistoryItem(item.id);
                          ref.read(clipboardHistoryProvider.notifier).removeItem(item.id);
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
        content: const Text('This will delete all clipboard history. Continue?'),
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
        color: Theme.of(context).colorScheme.surfaceVariant,
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
                  setState(() {
                    _selectedContentType = null;
                    _selectedDeviceId = null;
                    _startDate = null;
                    _endDate = null;
                  });
                },
                child: const Text('Clear'),
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
              value: _selectedDeviceId,
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
                child: OutlinedButton.icon(
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
                    }
                  },
                  icon: const Icon(Icons.calendar_today, size: 16),
                  label: Text(_startDate == null
                      ? 'Start Date'
                      : '${_startDate!.month}/${_startDate!.day}/${_startDate!.year}'),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
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
                    }
                  },
                  icon: const Icon(Icons.calendar_today, size: 16),
                  label: Text(_endDate == null
                      ? 'End Date'
                      : '${_endDate!.month}/${_endDate!.day}/${_endDate!.year}'),
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
                  _getContentTypeIcon(item.contentType),
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
                          _formatTimestamp(item.timestamp),
                          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                            color: Theme.of(context).colorScheme.outline,
                          ),
                        ),
                        if (item.sourceDeviceName != null) ...[
                          Text(
                            ' • ',
                            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              color: Theme.of(context).colorScheme.outline,
                            ),
                          ),
                          Text(
                            'from ${item.sourceDeviceName}',
                            style: Theme.of(context).textTheme.bodySmall?.copyWith(
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
                onPressed: onDelete,
                visualDensity: VisualDensity.compact,
              ),
            ],
          ),
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

    return '${timestamp.month}/${timestamp.day}';
  }
}

class _EmptyState extends StatelessWidget {
  final bool hasFilters;

  const _EmptyState({this.hasFilters = false});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(
            hasFilters ? Icons.filter_alt_off : Icons.history,
            size: 64,
            color: Theme.of(context).colorScheme.outline,
          ),
          const SizedBox(height: 16),
          Text(
            hasFilters ? 'No items match filters' : 'No clipboard history',
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          Text(
            hasFilters
                ? 'Try adjusting your search or filters'
                : 'Synced clipboard items will appear here',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: Theme.of(context).colorScheme.outline,
            ),
          ),
        ],
      ),
    );
  }
}
