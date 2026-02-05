import 'package:flutter/material.dart';

import '../../core/models/device.dart';

/// Get the icon for a device platform
IconData getPlatformIcon(DevicePlatform platform) {
  switch (platform) {
    case DevicePlatform.macos:
      return Icons.laptop_mac;
    case DevicePlatform.windows:
      return Icons.laptop_windows;
    case DevicePlatform.linux:
      return Icons.computer;
    case DevicePlatform.ios:
      return Icons.phone_iphone;
    case DevicePlatform.android:
      return Icons.phone_android;
    case DevicePlatform.unknown:
      return Icons.devices_other;
  }
}

/// Get a human-readable label for a device platform
String getPlatformLabel(DevicePlatform platform) {
  switch (platform) {
    case DevicePlatform.macos:
      return 'macOS';
    case DevicePlatform.windows:
      return 'Windows';
    case DevicePlatform.linux:
      return 'Linux';
    case DevicePlatform.ios:
      return 'iOS';
    case DevicePlatform.android:
      return 'Android';
    case DevicePlatform.unknown:
      return 'Unknown';
  }
}
