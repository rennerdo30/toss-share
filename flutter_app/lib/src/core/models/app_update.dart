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

  factory AppUpdate.fromGitHubRelease(
    Map<String, dynamic> json,
    Map<String, dynamic> asset,
  ) {
    return AppUpdate(
      version: json['tag_name'] as String,
      downloadUrl: asset['browser_download_url'] as String,
      size: asset['size'] as int,
      sha: json['target_commitish'] as String,
      publishedAt: DateTime.parse(json['published_at'] as String),
      releaseNotes: json['body'] as String?,
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
