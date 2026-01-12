import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../models/clipboard_item.dart';

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
    // TODO: Load from Rust FFI / local storage
  }
}
