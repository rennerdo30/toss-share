import 'package:intl/intl.dart';

/// Day and month in the current locale's order (e.g. "8/15" or "15.08.").
final DateFormat _dayMonthFormat = DateFormat.Md();

/// Day, month and year in the current locale's order.
final DateFormat _shortDateFormat = DateFormat.yMd();

/// Time of day in the current locale's 12/24 hour convention.
final DateFormat _timeFormat = DateFormat.Hm();

/// Format a timestamp as a relative time string (e.g., "5m ago")
String formatRelativeTimestamp(DateTime timestamp) {
  final now = DateTime.now();
  final diff = now.difference(timestamp);

  if (diff.inMinutes < 1) return 'Just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
  if (diff.inHours < 24) return '${diff.inHours}h ago';
  if (diff.inDays < 7) return '${diff.inDays}d ago';

  return _dayMonthFormat.format(timestamp);
}

/// Format a timestamp with time included (e.g., "1/15 14:30")
String formatAbsoluteTimestamp(DateTime timestamp) {
  final now = DateTime.now();
  final diff = now.difference(timestamp);

  if (diff.inMinutes < 1) return 'Just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
  if (diff.inHours < 24) return '${diff.inHours}h ago';
  if (diff.inDays < 7) return '${diff.inDays}d ago';

  return '${_dayMonthFormat.format(timestamp)} '
      '${_timeFormat.format(timestamp)}';
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
  return _shortDateFormat.format(lastSeen);
}
