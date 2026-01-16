import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/clipboard_item.dart';
import '../services/toss_service.dart';

part 'clipboard_provider.g.dart';

/// Provider for current clipboard content
@Riverpod(keepAlive: true)
class CurrentClipboard extends _$CurrentClipboard {
  @override
  ClipboardItem? build() {
    return null;
  }

  void update(ClipboardItem item) {
    state = item;
  }

  void clear() {
    state = null;
  }

  Future<void> refresh() async {
    final item = await TossService.getCurrentClipboard();
    if (item != null) {
      state = _convertToClipboardItem(item, 'current');
    }
  }

  ClipboardItem _convertToClipboardItem(ClipboardItemInfo info,
      [String? prefix]) {
    // Generate ID from timestamp and hash if available
    final id = prefix != null
        ? '$prefix-${info.timestamp}'
        : info.sourceDevice != null
            ? '${info.sourceDevice}-${info.timestamp}'
            : 'local-${info.timestamp}';

    return ClipboardItem(
      id: id,
      contentType: _parseContentType(info.contentType),
      preview: info.preview,
      sizeBytes: info.sizeBytes,
      timestamp: DateTime.fromMillisecondsSinceEpoch(info.timestamp),
      sourceDeviceId: info.sourceDevice,
    );
  }

  ClipboardContentType _parseContentType(String type) {
    // Parse content type string from Rust (e.g., "PlainText", "Image", etc.)
    final lower = type.toLowerCase();
    if (lower.contains('text') && !lower.contains('rich')) {
      return ClipboardContentType.text;
    } else if (lower.contains('rich')) {
      return ClipboardContentType.richText;
    } else if (lower.contains('image')) {
      return ClipboardContentType.image;
    } else if (lower.contains('file')) {
      return ClipboardContentType.file;
    } else if (lower.contains('url')) {
      return ClipboardContentType.url;
    }
    return ClipboardContentType.text;
  }
}

/// Provider for clipboard history
@Riverpod(keepAlive: true)
class ClipboardHistory extends _$ClipboardHistory {
  @override
  List<ClipboardItem> build() {
    return [];
  }

  void addItem(ClipboardItem item) {
    // Add to front, limit to 100 items
    state = [item, ...state].take(100).toList();
  }

  void removeItem(String id) {
    state = state.where((item) => item.id != id).toList();
  }

  void clearHistory() {
    state = [];
  }

  Future<void> loadHistory() async {
    final items = await TossService.getClipboardHistory();
    state = items.map((info) => _convertToClipboardItem(info)).toList();
  }

  ClipboardItem _convertToClipboardItem(ClipboardItemInfo info) {
    return ClipboardItem(
      id: info.id,
      contentType: _parseContentType(info.contentType),
      preview: info.preview,
      sizeBytes: info.sizeBytes,
      timestamp: DateTime.fromMillisecondsSinceEpoch(info.timestamp),
      sourceDeviceId: info.sourceDevice,
    );
  }

  ClipboardContentType _parseContentType(String type) {
    final lower = type.toLowerCase();
    if (lower.contains('text') && !lower.contains('rich')) {
      return ClipboardContentType.text;
    } else if (lower.contains('rich')) {
      return ClipboardContentType.richText;
    } else if (lower.contains('image')) {
      return ClipboardContentType.image;
    } else if (lower.contains('file')) {
      return ClipboardContentType.file;
    } else if (lower.contains('url')) {
      return ClipboardContentType.url;
    }
    return ClipboardContentType.text;
  }
}
