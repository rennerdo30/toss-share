/// Represents a clipboard item
class ClipboardItem {
  final String id;
  final ClipboardContentType contentType;
  final String preview;
  final int sizeBytes;
  final DateTime timestamp;
  final String? sourceDeviceId;
  final String? sourceDeviceName;

  const ClipboardItem({
    required this.id,
    required this.contentType,
    required this.preview,
    required this.sizeBytes,
    required this.timestamp,
    this.sourceDeviceId,
    this.sourceDeviceName,
  });

  bool get isLocal => sourceDeviceId == null;

  String get formattedSize {
    if (sizeBytes < 1024) {
      return '$sizeBytes B';
    } else if (sizeBytes < 1024 * 1024) {
      return '${(sizeBytes / 1024).toStringAsFixed(1)} KB';
    } else {
      return '${(sizeBytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
  }
}

/// Content types
enum ClipboardContentType {
  text,
  richText,
  image,
  file,
  url,
}

extension ClipboardContentTypeExtension on ClipboardContentType {
  String get displayName {
    switch (this) {
      case ClipboardContentType.text:
        return 'Text';
      case ClipboardContentType.richText:
        return 'Rich Text';
      case ClipboardContentType.image:
        return 'Image';
      case ClipboardContentType.file:
        return 'File';
      case ClipboardContentType.url:
        return 'URL';
    }
  }

  String get iconName {
    switch (this) {
      case ClipboardContentType.text:
        return 'text_fields';
      case ClipboardContentType.richText:
        return 'format_paint';
      case ClipboardContentType.image:
        return 'image';
      case ClipboardContentType.file:
        return 'attach_file';
      case ClipboardContentType.url:
        return 'link';
    }
  }
}
