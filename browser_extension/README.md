# Toss Browser Extension

Browser extension for Toss clipboard sync - share clipboard content across devices with end-to-end encryption.

## Features

- **Cross-browser support**: Chrome (Manifest V3) and Firefox (Manifest V2)
- **Clipboard sync**: Send clipboard content to paired devices
- **Context menu**: Right-click to send selected text, links, or images
- **Popup UI**: View clipboard history and manage paired devices
- **Settings**: Configure auto-sync, target devices, and content types
- **Secure**: End-to-end encrypted using AES-256-GCM

## Building

### Prerequisites

- Node.js 18+
- npm

### Install Dependencies

```bash
cd browser_extension
npm install
```

### Build Both Extensions

```bash
npm run build
```

This creates:
- `dist/chrome/` - Chrome extension (Manifest V3)
- `dist/firefox/` - Firefox extension (Manifest V2)

### Build Specific Browser

```bash
npm run build:chrome   # Chrome only
npm run build:firefox  # Firefox only
```

### Create Distribution Packages

```bash
npm run package
```

This creates:
- `dist/toss-chrome.zip` - Ready for Chrome Web Store
- `dist/toss-firefox.zip` - Ready for Firefox Add-ons

## Development

### Watch Mode

```bash
npm run watch
```

Automatically rebuilds when source files change.

### Load Unpacked Extension

#### Chrome

1. Open `chrome://extensions/`
2. Enable "Developer mode"
3. Click "Load unpacked"
4. Select the `dist/chrome` directory

#### Firefox

1. Open `about:debugging`
2. Click "This Firefox"
3. Click "Load Temporary Add-on"
4. Select any file in the `dist/firefox` directory

## Project Structure

```
browser_extension/
├── src/
│   ├── chrome/              # Chrome-specific files
│   │   ├── manifest.json    # Manifest V3
│   │   └── background.js    # Service worker
│   ├── firefox/             # Firefox-specific files
│   │   ├── manifest.json    # Manifest V2
│   │   └── background.js    # Background script
│   └── shared/              # Shared code
│       ├── js/
│       │   ├── crypto.js    # Cryptographic utilities
│       │   ├── relay-client.js  # WebSocket client
│       │   ├── storage.js   # Storage manager
│       │   ├── popup.js     # Popup UI logic
│       │   └── options.js   # Options page logic
│       ├── css/
│       │   ├── popup.css    # Popup styles
│       │   └── options.css  # Options styles
│       ├── html/
│       │   ├── popup.html   # Popup UI
│       │   └── options.html # Settings page
│       └── icons/           # Extension icons
├── scripts/
│   ├── build.js             # Build script
│   └── generate-icons.js    # Icon generation
├── dist/                    # Build output
└── package.json
```

## Architecture

```
┌─────────────────────────────────────────┐
│           Browser Extension             │
├─────────────────────────────────────────┤
│  Popup UI         │  Options Page       │
│  - History list   │  - Settings         │
│  - Device list    │  - Data management  │
│  - Quick actions  │                     │
├───────────────────┴─────────────────────┤
│           Background Service            │
│  - WebSocket connection                 │
│  - Context menu handling                │
│  - Message relay                        │
├─────────────────────────────────────────┤
│           Relay Client                  │
│  - Authentication                       │
│  - Message encryption                   │
│  - Connection management                │
├─────────────────────────────────────────┤
│           Storage Manager               │
│  - Settings                             │
│  - Clipboard history                    │
│  - Paired devices                       │
└─────────────────────────────────────────┘
          │
          │ WebSocket (wss://)
          ▼
┌─────────────────────────────────────────┐
│           Relay Server                  │
│  (relay_server from main project)       │
└─────────────────────────────────────────┘
```

## Configuration

### Settings

- **Relay Server URL**: WebSocket URL for the relay server
- **Auto-sync**: Automatically sync clipboard when connected
- **Target Device**: Default device to send clipboard to
- **Content Types**: Enable/disable sync for text, images, URLs
- **Notifications**: Show notifications for received items
- **Max History**: Number of items to keep in history

### Default Relay URL

```
wss://localhost:8080/api/v1/ws
```

## Security

- **End-to-end encryption**: AES-256-GCM with random nonces
- **Device authentication**: ECDSA signatures with timestamp validation
- **Secure storage**: Uses browser's extension storage API
- **No clipboard logging**: Clipboard contents are never logged

## WebSocket Protocol

The extension communicates with the relay server using JSON messages:

### Authentication

```json
{
  "type": "auth",
  "device_id": "...",
  "timestamp": 1234567890,
  "signature": "...",
  "public_key": "..."
}
```

### Send Clipboard

```json
{
  "type": "send",
  "to_device": "...",
  "encrypted_payload": "..."
}
```

### Receive Clipboard

```json
{
  "type": "relay",
  "message": {
    "id": "...",
    "from_device": "...",
    "to_device": "...",
    "encrypted_payload": "...",
    "timestamp": 1234567890
  }
}
```

## License

MIT License - See main project LICENSE file.
