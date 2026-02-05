/// Format a timestamp as a relative time string (e.g., "5m ago")
String formatRelativeTimestamp(DateTime timestamp) {
  final now = DateTime.now();
  final diff = now.difference(timestamp);

  if (diff.inMinutes < 1) return 'Just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
  if (diff.inHours < 24) return '${diff.inHours}h ago';
  if (diff.inDays < 7) return '${diff.inDays}d ago';

  return '${timestamp.month}/${timestamp.day}';
}

/// Format a timestamp with time included (e.g., "1/15 14:30")
String formatAbsoluteTimestamp(DateTime timestamp) {
  final now = DateTime.now();
  final diff = now.difference(timestamp);

  if (diff.inMinutes < 1) return 'Just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
  if (diff.inHours < 24) return '${diff.inHours}h ago';
  if (diff.inDays < 7) return '${diff.inDays}d ago';

  return '${timestamp.month}/${timestamp.day} ${timestamp.hour}:${timestamp.minute.toString().padLeft(2, '0')}';
}

/// Format a "last seen" timestamp for devices
String formatLastSeen(DateTime lastSeen) {
  final diff = DateTime.now().difference(lastSeen);
  if (diff.inMinutes < 1) return 'just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
  if (diff.inHours < 24) return '${diff.inHours}h ago';
  if (diff.inDays < 7) return '${diff.inDays}d ago';
  return '${lastSeen.month}/${lastSeen.day}/${lastSeen.year}';
}
