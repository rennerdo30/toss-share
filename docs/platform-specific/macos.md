# macOS Platform Support

macOS-specific implementation details for Toss.

## Accessibility Permissions

macOS requires accessibility permissions for clipboard monitoring. The app must:

1. Request accessibility permissions on first launch
2. Check permission status before accessing clipboard
3. Provide instructions if permissions are denied
4. Handle permission denied gracefully

**Implementation Status**: Basic structure created in `permissions_service.dart`. Full implementation requires native code integration.

**Files**:
- `flutter_app/lib/src/core/services/permissions_service.dart`
- `flutter_app/macos/Runner/Info.plist` (needs `NSAccessibilityUsageDescription`)

## Implementation Details

### Requesting Permissions

The app should request accessibility permissions on first launch:

```swift
// In AppDelegate.swift
let options = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true]
let accessEnabled = AXIsProcessTrustedWithOptions(options as CFDictionary)
```

### Checking Permissions

Before accessing clipboard, check permission status:

```swift
let accessEnabled = AXIsProcessTrusted()
if !accessEnabled {
    // Show instructions to user
}
```

### Info.plist Configuration

Add to `Info.plist`:

```xml
<key>NSAccessibilityUsageDescription</key>
<string>Toss needs accessibility permissions to monitor clipboard changes.</string>
```

## System Integration

- **Keychain**: Secure storage for device keys
- **System Tray**: Menu bar integration
- **Notifications**: User notifications for events

## Next Steps

- [Platform Overview](overview.md) - Return to platform overview
- [iOS Implementation](ios.md) - iOS platform details
