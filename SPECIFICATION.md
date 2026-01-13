# Toss - Technical Specification

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
- Plain text
- Rich text (HTML/RTF)
- Images (PNG, JPEG, BMP, GIF)
- Files (with size limits)
- URLs (with metadata preview)

---

## 2. Architecture

### 2.1 System Components

```mermaid
graph TB
    subgraph DeviceA["Device A"]
        FlutterUI["Flutter UI"]
        FFIBridge["FFI Bridge"]
        RustCore["Rust Core"]
        Crypto["Crypto"]
        Network["Network"]
        Clipboard["Clipboard"]
        Protocol["Protocol"]
        
        FlutterUI <--> FFIBridge
        FFIBridge <--> RustCore
        RustCore --> Crypto
        RustCore --> Network
        RustCore --> Clipboard
        RustCore --> Protocol
    end
    
    LocalNetwork["Local Network<br/>(mDNS + P2P)"]
    RelayServer["Relay Server<br/>(Cloud Hosted)"]
    DeviceB["Device B"]
    
    RustCore --> LocalNetwork
    RustCore --> RelayServer
    LocalNetwork --> DeviceB
    RelayServer --> DeviceB
```

### 2.2 Communication Flow

```mermaid
flowchart TD
    Start([Clipboard Change Detected]) --> Encrypt[Encrypt with Session Key]
    Encrypt --> TryP2P{Try P2P Connection}
    TryP2P -->|Success| P2P[Send via P2P QUIC]
    TryP2P -->|Fail| Relay[Send via Relay Server]
    P2P --> Receive[Device B Receives]
    Relay --> Receive
    Receive --> Decrypt[Decrypt Content]
    Decrypt --> Update[Update Clipboard]
    Update --> End([Complete])
    
    style Start fill:#e1f5ff
    style End fill:#d4edda
    style Encrypt fill:#fff3cd
    style Decrypt fill:#fff3cd
```

**Steps:**

1. **Device Discovery**
   - mDNS broadcast on local network
   - Relay server registration for remote access

2. **Connection Establishment**
   - Try direct P2P connection first (QUIC/UDP)
   - Fall back to relay server if P2P fails
   - NAT traversal using STUN/TURN-like techniques

3. **Data Synchronization**
   - Clipboard change detection
   - Encrypt content with session key
   - Transmit to paired devices
   - Decrypt and update local clipboard

---

## 3. Security Specification

### 3.1 Cryptographic Primitives

| Purpose | Algorithm | Key Size |
|---------|-----------|----------|
| Key Exchange | X25519 | 256-bit |
| Symmetric Encryption | AES-256-GCM | 256-bit |
| Key Derivation | HKDF-SHA256 | - |
| Message Authentication | HMAC-SHA256 | 256-bit |
| Device Identity | Ed25519 | 256-bit |

### 3.2 Key Management

#### 3.2.1 Device Identity Keys
- Generated on first launch
- Stored in platform secure storage (Keychain, Credential Manager, etc.)
- Never transmitted; only public key shared

#### 3.2.2 Session Keys
- Derived using X25519 key exchange during pairing
- Rotated periodically (every 24 hours or 1000 messages)
- Forward secrecy via ephemeral keys

### 3.3 Device Pairing Protocol

```mermaid
sequenceDiagram
    participant A as Device A
    participant B as Device B
    
    A->>A: 1. Generate pairing code (6 digits)
    A->>A: 2. Display QR code / numeric code
    B->>A: 3. Scan QR / Enter code
    A->>B: 4. Exchange public keys (X25519)<br/>PubKey_A
    B->>A: 4. Exchange public keys (X25519)<br/>PubKey_B
    A->>A: 5. Derive shared secret
    B->>B: 5. Derive shared secret
    A->>B: 6. Verify via pairing code<br/>Verify_A
    B->>A: 6. Verify via pairing code<br/>Verify_B
    A->>B: 7. Pairing complete
    B->>A: 7. Pairing complete
```

### 3.4 Message Encryption

```mermaid
graph LR
    subgraph Message["Encrypted Message"]
        Header["Header<br/>(8 bytes)<br/>version(2) + type(2) + length(4)"]
        Nonce["Nonce<br/>(12 bytes)"]
        EncryptedData["Encrypted Data<br/>(variable)"]
        AuthTag["Auth Tag<br/>(16 bytes)"]
        
        Header --> Nonce
        Nonce --> EncryptedData
        EncryptedData --> AuthTag
    end
```

### 3.5 Relay Server Security

- Relay sees only encrypted blobs
- No message content logging
- Device authentication via signed tokens
- Rate limiting per device
- Optional self-hosted relay

---

## 4. Network Protocol

### 4.1 Transport Layer

- **Primary**: QUIC (UDP-based, built-in encryption)
- **Fallback**: WebSocket over TLS (for restrictive networks)

### 4.2 Message Types

| Type | Code | Description |
|------|------|-------------|
| PING | 0x01 | Keepalive |
| PONG | 0x02 | Keepalive response |
| CLIPBOARD_UPDATE | 0x10 | New clipboard content |
| CLIPBOARD_ACK | 0x11 | Acknowledge receipt |
| CLIPBOARD_REQUEST | 0x12 | Request current clipboard |
| DEVICE_INFO | 0x20 | Device metadata exchange |
| KEY_ROTATION | 0x30 | Session key rotation |
| ERROR | 0xFF | Error message |

### 4.3 Clipboard Update Message

```rust
struct ClipboardUpdate {
    message_id: u64,
    timestamp: u64,           // Unix timestamp (ms)
    content_type: ContentType,
    content_hash: [u8; 32],   // SHA-256 of plaintext
    encrypted_content: Vec<u8>,
    metadata: Option<Metadata>,
}

enum ContentType {
    PlainText = 0,
    RichText = 1,
    Image = 2,
    File = 3,
    Url = 4,
}

struct Metadata {
    filename: Option<String>,
    mime_type: Option<String>,
    dimensions: Option<(u32, u32)>,  // For images
    preview: Option<Vec<u8>>,         // Thumbnail
}
```

### 4.4 Discovery Protocol (mDNS)

```mermaid
sequenceDiagram
    participant DeviceA as Device A
    participant mDNS as mDNS Service
    participant DeviceB as Device B
    
    DeviceA->>mDNS: Broadcast Service<br/>_toss._udp.local
    Note over DeviceA,mDNS: TXT: v=1, id=hash, name=DeviceA
    mDNS->>DeviceB: Service Discovery
    DeviceB->>DeviceA: Connect Request
    DeviceA->>DeviceB: Connection Established
```

**Service Details:**
- Service type: `_toss._udp.local`
- TXT records:
  - `v=1` (protocol version)
  - `id=<device_id_hash>` (truncated device ID)
  - `name=<device_name>` (user-friendly name)

---

## 5. Data Storage

### 5.1 Local Database Schema

```mermaid
erDiagram
    devices ||--o{ clipboard_history : "has"
    
    devices {
        TEXT id PK "Device public key hash"
        TEXT name "Device name"
        BLOB public_key "Public key"
        BLOB session_key "Encrypted session key"
        INTEGER last_seen "Last seen timestamp"
        INTEGER created_at "Creation timestamp"
        BOOLEAN is_active "Active status"
    }
    
    clipboard_history {
        INTEGER id PK "Primary key"
        INTEGER content_type "Content type enum"
        TEXT content_hash "SHA-256 hash"
        BLOB encrypted_content "Encrypted content"
        TEXT preview "Preview text"
        TEXT source_device FK "Source device ID"
        INTEGER created_at "Creation timestamp"
    }
    
    settings {
        TEXT key PK "Setting key"
        TEXT value "Setting value"
    }
```

**SQL Schema:**

```sql
-- Paired devices
CREATE TABLE devices (
    id TEXT PRIMARY KEY,           -- Device public key hash
    name TEXT NOT NULL,
    public_key BLOB NOT NULL,
    session_key BLOB,              -- Encrypted with device master key
    last_seen INTEGER,
    created_at INTEGER,
    is_active BOOLEAN DEFAULT TRUE
);

-- Clipboard history (optional feature)
CREATE TABLE clipboard_history (
    id INTEGER PRIMARY KEY,
    content_type INTEGER,
    content_hash TEXT,
    encrypted_content BLOB,
    preview TEXT,                  -- For quick display
    source_device TEXT,
    created_at INTEGER,
    FOREIGN KEY (source_device) REFERENCES devices(id)
);

-- Settings
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
```

### 5.2 Secure Storage

| Platform | Storage Method |
|----------|----------------|
| macOS | Keychain Services |
| iOS | Keychain Services |
| Windows | Credential Manager / DPAPI |
| Linux | Secret Service API / libsecret |
| Android | Android Keystore |

---

## 6. User Interface

### 6.1 Main Screens

1. **Home / Dashboard**
   - Connection status indicator
   - List of paired devices (online/offline)
   - Current clipboard preview
   - Quick actions (send, clear)

2. **Device Pairing**
   - QR code display/scanner
   - Manual code entry
   - Pairing confirmation

3. **Clipboard History** (optional)
   - Chronological list of shared items
   - Search and filter
   - Re-copy to clipboard

4. **Settings**
   - Device name
   - Auto-sync toggle
   - Content type filters
   - History retention
   - Relay server configuration
   - Theme (light/dark/system)

### 6.2 System Tray / Menu Bar

- Status icon (connected/disconnected)
- Quick toggle sync on/off
- Recent clipboard items
- Open main window
- Quit application

### 6.3 Notifications

- New device pairing request
- Clipboard received (optional)
- Connection lost/restored
- Error alerts

---

## 7. Platform-Specific Implementation

### 7.1 macOS

- **Clipboard**: `NSPasteboard` API
- **Background**: `NSApplication` background mode
- **Permissions**: Accessibility permission for clipboard monitoring
- **Menu Bar**: Native `NSStatusItem`

### 7.2 Windows

- **Clipboard**: `Win32 Clipboard API` with `AddClipboardFormatListener`
- **Background**: System tray application
- **Startup**: Registry key for auto-start

### 7.3 Linux

- **Clipboard**: X11 (`xcb`) and Wayland (`wl-clipboard`) support
- **Background**: D-Bus service / systemd user service
- **Tray**: `libappindicator` or SNI protocol

### 7.4 iOS

- **Clipboard**: `UIPasteboard.general`
- **Background**: Limited; use app extensions and Shortcuts
- **Sync Trigger**: On app open, Share extension, Widget

### 7.5 Android

- **Clipboard**: `ClipboardManager`
- **Background**: Foreground service with notification
- **Restrictions**: Android 10+ limits background clipboard access

---

## 8. Relay Server Specification

### 8.1 API Endpoints

```
POST   /api/v1/register          # Register device
DELETE /api/v1/register          # Unregister device
POST   /api/v1/relay/{device_id} # Send message to device
GET    /api/v1/messages          # Poll for messages (WebSocket upgrade available)
GET    /api/v1/devices/{id}/status # Check device online status
```

### 8.2 Authentication

- Device signs requests with Ed25519 private key
- Server verifies signature with registered public key
- JWT tokens for session management

### 8.3 Rate Limits

| Endpoint | Limit |
|----------|-------|
| Register | 10/hour |
| Relay message | 100/minute |
| Poll messages | 60/minute |

### 8.4 Self-Hosting

- Dockerized deployment
- Environment-based configuration
- Optional TLS termination (or use reverse proxy)
- SQLite or PostgreSQL backend

---

## 9. Error Handling

### 9.1 Error Codes

| Code | Category | Description |
|------|----------|-------------|
| 1xxx | Network | Connection errors |
| 2xxx | Crypto | Encryption/decryption failures |
| 3xxx | Protocol | Invalid messages |
| 4xxx | Storage | Database errors |
| 5xxx | Platform | OS-specific errors |

### 9.2 Recovery Strategies

- **Network failure**: Automatic reconnection with exponential backoff
- **Crypto failure**: Request key re-exchange
- **Corrupted data**: Skip and log, don't crash
- **Storage full**: Clear old history, warn user

---

## 10. Performance Requirements

| Metric | Target |
|--------|--------|
| Text sync latency (local) | < 100ms |
| Text sync latency (relay) | < 500ms |
| Image sync (1MB, local) | < 1s |
| Memory usage (idle) | < 50MB |
| Battery impact (mobile) | Minimal (< 2%/day) |
| Max clipboard size | 50MB |

---

## 11. Testing Strategy

### 11.1 Unit Tests
- Crypto operations
- Protocol serialization
- Clipboard format handling

### 11.2 Integration Tests
- Device pairing flow
- Message encryption/decryption round-trip
- Network reconnection

### 11.3 End-to-End Tests
- Cross-platform clipboard sync
- Relay fallback
- Large file transfer

### 11.4 Security Tests
- Fuzz testing on protocol parser
- Crypto implementation verification
- Penetration testing on relay server

---

## 12. CI/CD Pipeline

### 12.1 Continuous Integration

**Triggers**: Every push and pull request

```yaml
# Rust checks
- cargo fmt --check          # Code formatting
- cargo clippy -- -D warnings # Linting
- cargo test                  # Unit tests
- cargo audit                 # Security vulnerabilities

# Flutter checks
- flutter analyze             # Static analysis
- flutter test                # Widget and unit tests
- flutter test --coverage     # Coverage report
```

### 12.2 Release Pipeline

**Triggers**: Version tags (`v*.*.*`)

| Platform | Build Artifact |
|----------|----------------|
| Windows | `toss-windows-x64.zip` (portable), `.msi` installer |
| macOS | `toss-macos-universal.dmg` (Intel + Apple Silicon) |
| Linux | `.AppImage`, `.deb`, `.rpm` |
| iOS | `.ipa` (TestFlight / App Store) |
| Android | `.apk`, `.aab` (Play Store) |

### 12.3 Code Quality Gates

- Minimum test coverage: 70%
- No clippy warnings
- No security vulnerabilities (cargo audit)
- All tests passing
- PR approval required

---

## 13. Future Considerations

- **Clipboard streaming**: Real-time sync for rapid changes
- **Selective sync**: Choose which devices receive updates
- **Team/Organization support**: Shared clipboard groups
- **Browser extension**: Web-based clipboard access
- **Conflict resolution**: Handle simultaneous clipboard changes
- **Compression**: Reduce bandwidth for large content
