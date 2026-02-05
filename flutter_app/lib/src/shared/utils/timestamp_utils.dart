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

/// Format a timestamp that may be in the past or future (e.g., "5 min ago", "in 3 hours")
String formatSmartTimestamp(DateTime dateTime) {
  final now = DateTime.now();
  final diff = now.difference(dateTime);

  if (diff.isNegative) {
    // Future date
    final absDiff = dateTime.difference(now);
    if (absDiff.inHours < 1) {
      return 'in ${absDiff.inMinutes} min';
    } else if (absDiff.inHours < 24) {
      return 'in ${absDiff.inHours} hours';
    } else {
      return 'in ${absDiff.inDays} days';
    }
  }

  if (diff.inDays > 0) {
    return '${diff.inDays} day${diff.inDays > 1 ? 's' : ''} ago';
  } else if (diff.inHours > 0) {
    return '${diff.inHours} hour${diff.inHours > 1 ? 's' : ''} ago';
  } else if (diff.inMinutes > 0) {
    return '${diff.inMinutes} min ago';
  } else {
    return 'just now';
  }
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
