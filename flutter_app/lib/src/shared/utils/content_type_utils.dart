import 'package:flutter/material.dart';

import '../../core/models/clipboard_item.dart';

/// Get the icon for a clipboard content type
IconData getContentTypeIcon(ClipboardContentType type) {
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

/// Get the color for a clipboard content type
Color getContentTypeColor(ClipboardContentType type, ColorScheme colorScheme) {
  switch (type) {
    case ClipboardContentType.text:
      return colorScheme.primary;
    case ClipboardContentType.richText:
      return colorScheme.secondary;
    case ClipboardContentType.image:
      return colorScheme.tertiary;
    case ClipboardContentType.file:
      return colorScheme.error;
    case ClipboardContentType.url:
      return colorScheme.primary;
  }
}

/// Get a human-readable label for a clipboard content type
String getContentTypeLabel(ClipboardContentType type) {
  return type.displayName;
}
