# API Reference

Rust FFI API for Flutter integration.

## Initialization

### `initToss`

Initialize the Toss core library.

```dart
api.initToss(
  dataDir: '/path/to/data',
  deviceName: 'My Device',
);
```

**Parameters:**
- `dataDir`: Path to data directory
- `deviceName`: Friendly device name

**Returns:** `void`

## Device Management

### `getDeviceId`

Get the current device's unique identifier.

```dart
String deviceId = api.getDeviceId();
```

**Returns:** `String` - Device ID

### `setDeviceName`

Set the device's friendly name.

```dart
api.setDeviceName(name: 'My Device');
```

**Parameters:**
- `name`: Device name

**Returns:** `void`

## Pairing

### `startPairing`

Start a pairing session.

```dart
PairingInfo info = await api.startPairing();
```

**Returns:** `PairingInfo` with:
- `code`: 6-digit pairing code
- `qrData`: QR code data
- `expiresAt`: Expiration timestamp
- `publicKey`: Public key for pairing

### `completePairingQr`

Complete pairing using QR code.

```dart
bool success = await api.completePairingQr(
  qrData: 'qr-code-data',
);
```

**Parameters:**
- `qrData`: QR code data from other device

**Returns:** `bool` - Success status

### `completePairingCode`

Complete pairing using 6-digit code.

```dart
bool success = await api.completePairingCode(
  code: '123456',
);
```

**Parameters:**
- `code`: 6-digit pairing code

**Returns:** `bool` - Success status

## Device List

### `getPairedDevices`

Get list of paired devices.

```dart
List<DeviceInfo> devices = await api.getPairedDevices();
```

**Returns:** `List<DeviceInfo>` with:
- `id`: Device ID
- `name`: Device name
- `isOnline`: Online status
- `lastSeen`: Last seen timestamp

## Clipboard Operations

### `sendClipboard`

Send clipboard content to paired devices.

```dart
bool success = await api.sendClipboard(
  contentType: 'text/plain',
  content: 'Hello, World!',
);
```

**Parameters:**
- `contentType`: MIME type of content
- `content`: Base64-encoded content

**Returns:** `bool` - Success status

### `getClipboardHistory`

Get clipboard history.

```dart
List<ClipboardItemInfo> history = await api.getClipboardHistory(
  limit: 50,
);
```

**Parameters:**
- `limit`: Maximum number of items

**Returns:** `List<ClipboardItemInfo>` with:
- `contentType`: Content type
- `preview`: Preview text
- `sizeBytes`: Size in bytes
- `timestamp`: Timestamp
- `sourceDevice`: Source device ID

## Events

### `pollEvent`

Poll for network events.

```dart
NetworkEvent? event = await api.pollEvent();
```

**Returns:** `NetworkEvent?` - Event or null if none

## Settings

### `getSettings`

Get current settings.

```dart
TossSettings settings = await api.getSettings();
```

**Returns:** `TossSettings` with:
- `autoSync`: Auto-sync enabled
- `syncText`: Sync text content
- `syncImages`: Sync images
- `syncFiles`: Sync files
- `maxFileSizeMb`: Max file size in MB
- `historyEnabled`: History enabled
- `historyDays`: History retention days
- `relayUrl`: Relay server URL

### `updateSettings`

Update settings.

```dart
api.updateSettings(settings: settings);
```

**Parameters:**
- `settings`: Updated settings object

**Returns:** `void`

## Error Handling

All API functions may throw exceptions. Handle errors appropriately:

```dart
try {
  await api.startPairing();
} catch (e) {
  print('Error: $e');
}
```

## Next Steps

- [Architecture](architecture.md) - System design
- [Platform Support](platform-support.md) - Platform-specific details
- [Testing](testing.md) - Testing guide
