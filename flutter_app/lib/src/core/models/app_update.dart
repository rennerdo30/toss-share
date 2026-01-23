/// Represents an available app update
class AppUpdate {
  final String version;
  final String downloadUrl;
  final int size;
  final String sha;
  final DateTime publishedAt;
  final String? releaseNotes;

  const AppUpdate({
    required this.version,
    required this.downloadUrl,
    required this.size,
    required this.sha,
    required this.publishedAt,
    this.releaseNotes,
  });

  String get formattedSize {
    if (size < 1024) {
      return '$size B';
    } else if (size < 1024 * 1024) {
      return '${(size / 1024).toStringAsFixed(1)} KB';
    } else {
      return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
  }

  /// Extract commit SHA from release body
  /// Looks for pattern: "**Built from:** {sha}" in nightly releases
  static String? _extractShaFromBody(String? body) {
    if (body == null) return null;

    // Match "**Built from:** " followed by a 40-char hex SHA
    final regex = RegExp(r'\*\*Built from:\*\*\s*([a-f0-9]{40})', caseSensitive: false);
    final match = regex.firstMatch(body);
    if (match != null) {
      return match.group(1);
    }

    // Fallback: look for any 40-char hex string that looks like a SHA
    final shaRegex = RegExp(r'\b([a-f0-9]{40})\b', caseSensitive: false);
    final shaMatch = shaRegex.firstMatch(body);
    return shaMatch?.group(1);
  }

  factory AppUpdate.fromGitHubRelease(
    Map<String, dynamic> json,
    Map<String, dynamic> asset,
  ) {
    final body = json['body'] as String?;

    // Try to extract SHA from release body first (nightly builds include it)
    // Fall back to target_commitish only if it looks like a SHA (not a branch name)
    String sha = _extractShaFromBody(body) ?? '';

    if (sha.isEmpty) {
      final targetCommitish = json['target_commitish'] as String? ?? '';
      // Only use target_commitish if it looks like a SHA (40 hex chars)
      if (RegExp(r'^[a-f0-9]{40}$', caseSensitive: false).hasMatch(targetCommitish)) {
        sha = targetCommitish;
      } else {
        // Last resort: use published_at timestamp as a pseudo-version
        // This ensures updates are detected even without SHA
        sha = 'ts-${json['published_at']}';
      }
    }

    return AppUpdate(
      version: json['tag_name'] as String,
      downloadUrl: asset['browser_download_url'] as String,
      size: asset['size'] as int,
      sha: sha,
      publishedAt: DateTime.parse(json['published_at'] as String),
      releaseNotes: body,
    );
  }
}

/// Update status states
enum UpdateStatus {
  idle,
  checking,
  available,
  downloading,
  ready,
  upToDate,
  error,
}

extension UpdateStatusExtension on UpdateStatus {
  String get displayName {
    switch (this) {
      case UpdateStatus.idle:
        return 'Not checked';
      case UpdateStatus.checking:
        return 'Checking for updates...';
      case UpdateStatus.available:
        return 'Update available';
      case UpdateStatus.downloading:
        return 'Downloading update...';
      case UpdateStatus.ready:
        return 'Update ready (restart to apply)';
      case UpdateStatus.upToDate:
        return 'Up to date';
      case UpdateStatus.error:
        return 'Update check failed';
    }
  }
}
