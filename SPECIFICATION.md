# Toss - Technical Specification

**Version:** 1.0 | **Protocol Version:** 1

---

## 1. Overview

### 1.1 Purpose
**Toss** is a cross-platform application that synchronizes clipboard content between devices with end-to-end encryption. It prioritizes privacy, security, and seamless user experience.

### 1.2 Supported Platforms
- Windows 10/11
- macOS 11+
- Linux (X11 and Wayland)
- iOS 14+
- Android 10+

### 1.3 Supported Content Types
| Type | Code | Description |
|------|------|-------------|
| PlainText | 0 | UTF-8 text |
| RichText | 1 | HTML or RTF |
| Image | 2 | PNG, JPEG, GIF, WebP, BMP, TIFF |
| File | 3 | Binary data or file list |
| Url | 4 | Auto-detected from text |

---

## 2. Architecture

### 2.1 System Components

```
FlutterUI <-> FFI Bridge <-> Rust Core
                              ├── crypto/     (X25519, AES-256-GCM, Ed25519)
                              ├── network/    (QUIC, mDNS, STUN/TURN, WebSocket)
                              ├── clipboard/  (arboard + platform-specific)
                              ├── protocol/   (bincode serialization)
                              └── storage/    (SQLite)
```

### 2.2 Communication Flow

1. **Device Discovery**: mDNS broadcast on local network, relay server registration
2. **Connection Establishment**: Try P2P (QUIC), fallback to relay, NAT traversal via STUN/TURN
3. **Data Synchronization**: Clipboard change detection → encrypt → transmit → decrypt → update

---

## 3. Security Specification

### 3.1 Cryptographic Primitives

| Purpose | Algorithm | Key Size | Library |
|---------|-----------|----------|---------|
| Key Exchange | X25519 ECDH | 32 bytes | x25519_dalek |
| Symmetric Encryption | AES-256-GCM | 32 bytes | aes_gcm |
| Key Derivation | HKDF-SHA256 | 32 bytes | hkdf |
| Device Identity | Ed25519 | 32 bytes | ed25519_dalek |
| Content Hash | SHA-256 | 32 bytes | sha2 |

### 3.2 Encryption Constants

| Constant | Value |
|----------|-------|
| KEY_SIZE | 32 bytes |
| NONCE_SIZE | 12 bytes |
| TAG_SIZE | 16 bytes |

### 3.3 Key Derivation Purposes

| Purpose | Context String |
|---------|----------------|
| Session Encryption | `b"toss-session-encryption-v1"` |
| Message Authentication | `b"toss-message-auth-v1"` |
| Storage Encryption | `b"toss-storage-encryption-v1"` |

### 3.4 Device Identity
- Generated on first launch, stored in platform secure storage
- Device ID = `SHA-256(public_key_bytes)`
- Signs ephemeral keys during key rotation

### 3.5 Session Key Rotation

| Trigger | Threshold |
|---------|-----------|
| Message count | 1000 messages |
| Time elapsed | 86400 seconds (24 hours) |
| Manual | On request |

### 3.6 Relay Server Security
- Relay sees only encrypted blobs (zero-knowledge)
- Device authentication via Ed25519 signed tokens
- Rate limiting per device

---

## 4. Network Protocol

### 4.1 Transport Layer

| Protocol | Use Case |
|----------|----------|
| QUIC | Primary P2P transport |
| WebSocket | Relay fallback |

**QUIC Configuration:**
| Parameter | Value |
|-----------|-------|
| IDLE_TIMEOUT | 30 seconds |
| KEEP_ALIVE | 5 seconds |
| Certificate | Self-signed |
| MAX_MESSAGE_SIZE | 50 MB |

### 4.2 Message Types

| Type | Code | Description |
|------|------|-------------|
| Ping | 0x01 | Keep-alive with timestamp |
| Pong | 0x02 | Ping response |
| ClipboardUpdate | 0x10 | Clipboard content sync |
| ClipboardAck | 0x11 | Acknowledge receipt |
| ClipboardRequest | 0x12 | Request clipboard from peer |
| DeviceInfo | 0x20 | Device metadata exchange |
| KeyRotation | 0x30 | Session key rotation |
| Error | 0xFF | Error notification |

### 4.3 Frame Format

```
Header (24 bytes, unencrypted):
┌─────────┬──────┬──────────┬────────────┬───────────┬────────────────┐
│ version │ type │ reserved │ message_id │ timestamp │ payload_length │
│ 2 bytes │ 1    │ 1        │ 8 bytes    │ 8 bytes   │ 4 bytes        │
└─────────┴──────┴──────────┴────────────┴───────────┴────────────────┘

Encrypted payload:
┌───────────┬────────────────────┬─────────┐
│ nonce     │ ciphertext         │ tag     │
│ 12 bytes  │ N bytes            │ 16 bytes│
└───────────┴────────────────────┴─────────┘
```

**Encryption:** AES-256-GCM with serialized MessageHeader as AAD

### 4.4 Message Structures

```rust
struct ClipboardUpdate {
    content: ClipboardContent,
    content_hash: [u8; 32],  // SHA-256
}

struct ClipboardAck {
    message_id: u64,
    content_hash: [u8; 32],
    success: bool,
    error: Option<String>,
}

struct DeviceInfo {
    device_id: [u8; 32],
    device_name: String,
    platform: Platform,  // 0=Unknown, 1=macOS, 2=Windows, 3=Linux, 4=iOS, 5=Android
    version: String,
}

struct KeyRotation {
    new_public_key: [u8; 32],
    signature: [u8; 64],     // Ed25519, base64 encoded
    reason: KeyRotationReason,
}
```

### 4.5 mDNS Discovery

| Parameter | Value |
|-----------|-------|
| Service type | `_toss._udp.local.` |
| Pairing type | `_toss-pair._udp.local.` |

**TXT Records:**
- `v`: Protocol version (e.g., "1")
- `id`: Device ID (16-char hex prefix)
- `name`: Human-readable device name

### 4.6 NAT Traversal

**STUN:**
| Parameter | Value |
|-----------|-------|
| Magic cookie | 0x2112A442 |
| Binding request | 0x0001 |
| Binding response | 0x0101 |
| Default server | stun.l.google.com:19302 |
| Timeout | 5 seconds |

**TURN:**
| Parameter | Value |
|-----------|-------|
| Allocate request | 0x0003 |
| Allocate response | 0x0103 |
| Create permission | 0x0008 |
| Send indication | 0x0016 |
| Data indication | 0x0017 |
| Default lifetime | 600 seconds |

**NAT Types:**
- None, FullCone, RestrictedCone, PortRestrictedCone → Direct P2P possible
- Symmetric → Requires TURN relay

---

## 5. Relay Server

### 5.1 API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| WebSocket | `/api/v1/ws` | Real-time message relay |
| POST | `/api/v1/pairing/register` | Register pairing code |
| GET | `/api/v1/pairing/find/{code}` | Lookup pairing |
| DELETE | `/api/v1/pairing/{code}` | Cancel pairing |

### 5.2 Authentication Message
```json
{
  "type": "auth",
  "device_id": "<hex-encoded>",
  "timestamp": <unix_timestamp>,
  "signature": "<base64-encoded-signature>"
}
```

### 5.3 Relay Message Format
```json
{
  "from_device": "<hex-device-id>",
  "to_device": "<hex-device-id>",
  "encrypted_payload": "<base64-encoded>",
  "timestamp": <unix_millis>
}
```

### 5.4 Rate Limits

| Endpoint | Limit |
|----------|-------|
| Register | 10/hour |
| Relay message | 100/minute |
| Poll messages | 60/minute |

---

## 6. Data Storage

### 6.1 SQLite Schema

```sql
-- Paired devices
CREATE TABLE devices (
    id TEXT PRIMARY KEY,           -- SHA-256 of public key
    name TEXT NOT NULL,
    public_key BLOB NOT NULL,
    session_key BLOB,              -- Encrypted with storage key
    last_seen INTEGER,
    created_at INTEGER NOT NULL,
    is_active INTEGER DEFAULT 1,
    platform TEXT                  -- "macos", "windows", "linux", "ios", "android"
);

-- Clipboard history
CREATE TABLE clipboard_history (
    id TEXT PRIMARY KEY,
    content_type INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    encrypted_content BLOB,
    preview TEXT,
    source_device TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (source_device) REFERENCES devices(id)
);

CREATE INDEX idx_clipboard_history_created_at
    ON clipboard_history(created_at DESC);

-- App settings
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

### 6.2 Secure Storage by Platform

| Platform | Storage Method |
|----------|----------------|
| macOS | Keychain Services |
| iOS | Keychain Services |
| Windows | DPAPI |
| Linux | Secret Service API |
| Android | Android Keystore |

### 6.3 Encryption at Rest
- Storage key derived via HKDF with `StorageEncryption` purpose
- Encrypted fields: `session_key`, `encrypted_content`

---

## 7. Device Pairing

### 7.1 Pairing Code
- Format: Alphanumeric string (4-8 characters)
- Lifetime: 300 seconds (5 minutes)

### 7.2 Pairing Process

1. **Advertise**: Generate code, advertise via mDNS (`_toss-pair._udp.local.`) + relay
2. **Discover**: Search mDNS (3s timeout), fallback to relay server
3. **Connect**: Initiate QUIC connection, exchange DeviceInfo
4. **Establish**: X25519 key exchange, derive session key via HKDF
5. **Store**: Save device with encrypted session key

### 7.3 mDNS Pairing Properties
- `code`: Pairing code
- `pk`: Base64 public key (43 chars)
- `name`: Device name

---

## 8. Platform-Specific Implementation

| Platform | Clipboard | Permissions | Notes |
|----------|-----------|-------------|-------|
| macOS | NSPasteboard via arboard | Accessibility | `AXIsProcessTrusted()` check |
| Windows | Win32 API | None | CF_UNICODETEXT, CF_HDROP, CF_DIB formats |
| Linux | X11/Wayland via arboard | None | Dual protocol support |
| iOS | UIPasteboard (Flutter) | Local Network | Limited background access, iOS 14+ restrictions |
| Android | ClipboardManager (Flutter) | None | Android 10+ restrictions, Keystore for storage |

### 8.1 Windows Clipboard Formats
- **CF_UNICODETEXT**: Unicode text
- **CF_HDROP**: File list (drag & drop)
- **CF_DIB**: Device-independent bitmap

### 8.2 iOS Background Service Implementation

iOS has significant restrictions on background clipboard access starting with iOS 14. The following strategies are implemented to provide the best possible user experience:

#### 8.2.1 iOS Clipboard Restrictions

| iOS Version | Restriction |
|-------------|-------------|
| iOS 14+ | Clipboard read shows user notification ("App pasted from...") |
| iOS 14+ | Background clipboard reading is blocked |
| iOS 16+ | App must be in foreground to read clipboard |

#### 8.2.2 Background Sync Strategies

1. **Foreground Sync**: Clipboard sync triggers automatically when app returns to foreground
2. **Siri Shortcuts Integration**: Users can create shortcuts for quick sync:
   - "Sync Clipboard" - Sends current clipboard to paired devices
   - "Send Clipboard" - Same as sync
   - "Get Latest Clipboard" - Receives clipboard from devices
3. **Background Fetch**: Limited background processing for receiving (not reading) clipboard
4. **App Extensions**: Share extension for sending content without opening main app
5. **Widgets**: WidgetKit widget for quick status view and sync trigger

#### 8.2.3 Implementation Details

```
iOS Background Service Flow:
┌─────────────────────────────────────────────────────────┐
│ App in Foreground                                       │
│ ├── Full clipboard access                               │
│ ├── Monitor clipboard changes (250ms polling)          │
│ └── Auto-sync on change (rate limited)                 │
├─────────────────────────────────────────────────────────┤
│ App Becomes Active (from background)                   │
│ ├── syncOnForeground() called                          │
│ ├── Check for local clipboard changes                  │
│ └── Receive pending content from network               │
├─────────────────────────────────────────────────────────┤
│ App Goes to Background                                  │
│ ├── Update widget with current status                  │
│ ├── Schedule background refresh task                   │
│ └── Cannot read clipboard (iOS restriction)            │
├─────────────────────────────────────────────────────────┤
│ Background Fetch Triggered                              │
│ ├── Receive content from paired devices                │
│ ├── Update widget                                       │
│ └── Cannot access local clipboard                      │
├─────────────────────────────────────────────────────────┤
│ Siri Shortcut Invoked                                   │
│ ├── handleShortcutAction() processes request           │
│ ├── Can read clipboard (user interaction)              │
│ └── Sync content to/from devices                       │
└─────────────────────────────────────────────────────────┘
```

#### 8.2.4 User Recommendations for iOS

- **Enable Background App Refresh** in iOS Settings for best sync experience
- **Use Siri Shortcuts** for quick clipboard sync without opening the app
- **Add Home Screen Widget** for at-a-glance status and quick sync
- **Note**: iOS will show a notification when the app reads the clipboard (this is expected)

---

## 9. Performance Requirements

| Metric | Target |
|--------|--------|
| Text sync latency (local) | < 100ms |
| Text sync latency (relay) | < 500ms |
| Image sync (1MB, local) | < 1s |
| Memory usage (idle) | < 50MB |
| Battery impact (mobile) | < 2%/day |
| Max clipboard size | 50 MB |
| Max preview size | 256 KB |

---

## 10. Protocol Flows

### 10.1 Clipboard Sync
```
A: Clipboard change detected
A: Create ClipboardUpdate (content + SHA-256 hash)
A: Encrypt with session key (AES-256-GCM, header as AAD)
A: Send via QUIC/relay
B: Decrypt and verify hash
B: Send ClipboardAck
B: Update local clipboard
```

### 10.2 Key Rotation
```
Trigger: 1000 messages OR 24 hours
A: Generate new ephemeral X25519 keypair
A: Sign new public key with Ed25519 identity key
A: Send KeyRotation message
B: Verify signature with A's identity key
B: Derive new session key via HKDF
Both: Reset message counters
```

### 10.3 Device Pairing
```
A: Generate pairing code
A: Advertise on mDNS + register on relay (300s expiry)
B: Enter code, search mDNS (3s timeout)
B: Fallback to relay if not found
B: Initiate QUIC connection
Both: Exchange DeviceInfo messages
Both: X25519 key exchange
Both: Derive session key via HKDF
Both: Store paired device
```

---

## 11. CI/CD Pipeline

### 11.1 Quality Gates (Required Before Commit)
- `cargo fmt --check` - Code formatting
- `cargo clippy -- -D warnings` - Linting
- `cargo test` - All tests passing (100%)
- `cargo audit` - No security vulnerabilities
- `flutter analyze` - Static analysis
- `flutter test` - Widget and unit tests

### 11.2 Commands
```bash
make ci       # Run all CI checks
make test     # Run all tests
make build    # Build everything
```

### 11.3 Release Artifacts

| Platform | Artifacts |
|----------|-----------|
| Windows | `.zip` (portable), `.msi` installer |
| macOS | `.dmg` (Universal) |
| Linux | `.AppImage`, `.deb`, `.tar.gz` |
| iOS | `.ipa` (TestFlight) |
| Android | `.apk`, `.aab` |

---

## 12. Chunked Transfer Protocol

### 12.1 Overview

Large clipboard content (> 1 MB by default) uses a chunked transfer protocol for efficient streaming. This provides:
- Memory-efficient transfers (content is not loaded entirely into memory)
- Progress tracking for large transfers
- Resume capability for interrupted transfers
- Configurable chunk sizes

### 12.2 Configuration

| Parameter | Default | Min | Max | Description |
|-----------|---------|-----|-----|-------------|
| `streaming_chunk_size` | 1 MB | 64 KB | 4 MB | Size of each chunk |
| `chunked_threshold` | 1 MB | - | - | Content size threshold for chunked transfer |
| `streaming_enabled` | true | - | - | Enable/disable chunked transfers |

### 12.3 Message Types

| Type | Code | Description |
|------|------|-------------|
| ChunkedTransferInit | 0x13 | Initiate chunked transfer with metadata |
| ChunkedTransferData | 0x14 | Individual chunk with sequence number |
| ChunkedTransferAck | 0x15 | Acknowledge receipt of chunk |
| ChunkedTransferComplete | 0x16 | Signal transfer completion/cancellation |

### 12.4 Message Structures

```rust
struct ChunkedTransferInit {
    transfer_id: u64,           // Unique transfer identifier
    total_chunks: u32,          // Total number of chunks
    total_size: u64,            // Total size in bytes
    chunk_size: u32,            // Chunk size in bytes
    content_type: ContentType,  // Clipboard content type
    metadata: ContentMetadata,  // Preview, dimensions, etc.
    content_hash: [u8; 32],     // SHA-256 of full content
}

struct ChunkedTransferData {
    transfer_id: u64,           // Transfer identifier
    chunk_index: u32,           // Chunk sequence number (0-indexed)
    data: Vec<u8>,              // Chunk data
    chunk_hash: [u8; 32],       // SHA-256 of this chunk
}

struct ChunkedTransferAck {
    transfer_id: u64,           // Transfer identifier
    chunk_index: u32,           // Acknowledged chunk index
    success: bool,              // Whether chunk was received successfully
    error: Option<String>,      // Error message if failed
}

struct ChunkedTransferComplete {
    transfer_id: u64,           // Transfer identifier
    state: TransferState,       // Completed, Failed, or Cancelled
    error: Option<String>,      // Error message if failed
}

enum TransferState {
    Initiated,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}
```

### 12.5 Transfer Flow

```
Sender                                      Receiver
   |                                            |
   |------- ChunkedTransferInit --------------->|
   |        (metadata, total_chunks, hash)      |
   |                                            |
   |------- ChunkedTransferData (chunk 0) ----->|
   |<------ ChunkedTransferAck (optional) ------|
   |                                            |
   |------- ChunkedTransferData (chunk 1) ----->|
   |<------ ChunkedTransferAck (optional) ------|
   |                                            |
   |              ... (repeat) ...              |
   |                                            |
   |------- ChunkedTransferData (chunk N) ----->|
   |                                            |
   |------- ChunkedTransferComplete ----------->|
   |        (state=Completed)                   |
   |                                            |
   |        Receiver verifies full content hash |
   |        and assembles into ClipboardContent |
```

### 12.6 Error Handling

- **Chunk Hash Mismatch**: Receiver requests retransmission via ChunkedTransferAck
- **Transfer Timeout**: Transfers expire after 300 seconds (5 minutes)
- **Missing Chunks**: Receiver can request specific chunks via ChunkedTransferAck
- **Cancellation**: Either party can send ChunkedTransferComplete with state=Cancelled

### 12.7 Limits

| Limit | Value |
|-------|-------|
| Max concurrent transfers | 4 per connection |
| Transfer timeout | 300 seconds |
| Max transfer ID | u64::MAX |

---

## 13. Future Considerations

- Selective sync (choose devices)
- Team/Organization support
- Browser extension
- Conflict resolution
- Compression for large content
