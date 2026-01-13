# Using Toss

Learn how to use Toss to sync clipboard content between devices.

## Basic Usage

Once devices are paired, Toss works automatically:

1. **Copy** content on one device (text, images, files, URLs)
2. **Paste** on any other paired device - the content appears automatically!

## Supported Content Types

Toss supports syncing:

- **Text**: Plain text, rich text (HTML/RTF)
- **Images**: PNG, JPEG, BMP, GIF
- **Files**: With configurable size limits
- **URLs**: With metadata preview

## Clipboard History

Toss maintains a history of clipboard items:

- View recent clipboard items in the History screen
- Tap any item to copy it again
- History is stored locally and encrypted

## Settings

Configure Toss behavior in Settings:

- **Auto Sync**: Enable/disable automatic syncing
- **Content Types**: Choose which types to sync (text, images, files)
- **File Size Limit**: Set maximum file size for syncing
- **History**: Enable/disable clipboard history
- **Relay Server**: Configure custom relay server URL

## Connection Status

Monitor connection status:

- **Online**: Connected to paired devices
- **P2P**: Direct peer-to-peer connection (fastest)
- **Relay**: Using relay server (fallback)
- **Offline**: No connection available

## Privacy & Security

- All clipboard content is encrypted before transmission
- Zero-knowledge architecture - relay servers can't read your data
- Session keys are rotated regularly for forward secrecy
- Device identity keys never leave your device

## Troubleshooting

### Content Not Syncing

- Check connection status
- Verify devices are paired
- Check settings for content type filters
- Ensure file size is within limits

### Slow Syncing

- P2P connections are faster than relay
- Check network connection quality
- Large files may take longer to sync

### Connection Issues

- Try re-pairing devices
- Check firewall settings
- Verify network connectivity

## Next Steps

- [Pairing Devices](pairing.md) - Connect your devices
- [Overview](overview.md) - Return to user guide
