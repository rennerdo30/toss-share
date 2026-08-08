# Toss

> Securely share your clipboard across all your devices

[![CI](https://github.com/rennerdo30/toss-share/actions/workflows/ci.yml/badge.svg)](https://github.com/rennerdo30/toss-share/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Toss is a cross-platform clipboard sharing application with end-to-end encryption. Copy on one device, paste on another - securely and instantly.

## Features

- **End-to-End Encryption**: All clipboard data is encrypted using AES-256-GCM before leaving your device
- **Cross-Platform**: Works on macOS, Windows, Linux, iOS, and Android
- **Local-First**: Direct peer-to-peer sync on local networks for minimal latency
- **Relay Fallback**: Cloud relay server for syncing when devices aren't on the same network
- **Multiple Content Types**: Supports text, images, files, and URLs
- **Easy Pairing**: QR code or 6-digit code for secure device pairing
- **Team Support**: Create teams with role-based access, invitation codes, and audit logging
- **Compression**: Automatic zstd compression for efficient transfers
- **Browser Extension**: Chrome/Firefox extension for clipboard sync from the browser
- **Privacy Focused**: Zero-knowledge architecture - relay servers can't read your data

## Installation

### Desktop

Download the latest release for your platform:

- [macOS (Universal)](https://github.com/rennerdo30/toss-share/releases/latest)
- [Windows (x64)](https://github.com/rennerdo30/toss-share/releases/latest)
- [Linux (AppImage)](https://github.com/rennerdo30/toss-share/releases/latest)

### Mobile

- iOS: Coming soon to the App Store
- Android: Coming soon to Google Play

### Build from Source

#### Quick Setup

Run the setup script to check and install all dependencies:

```bash
./scripts/setup.sh
```

#### Prerequisites

**All Platforms:**
- [Rust](https://rustup.rs/) (1.75+)
- [Flutter](https://flutter.dev/docs/get-started/install) (3.24+)
- [Git](https://git-scm.com/)

**macOS:**
- Xcode (from App Store)
- Xcode Command Line Tools: `xcode-select --install`
- CocoaPods: `brew install cocoapods` or `sudo gem install cocoapods`
- After installing Xcode, run:
  ```bash
  sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
  sudo xcodebuild -runFirstLaunch
  sudo xcodebuild -license accept
  ```

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get update
sudo apt-get install -y clang cmake ninja-build pkg-config \
    libgtk-3-dev liblzma-dev libstdc++-12-dev
```

**Linux (Fedora):**
```bash
sudo dnf install -y clang cmake ninja-build pkgconfig gtk3-devel xz-devel
```

**Linux (Arch):**
```bash
sudo pacman -S clang cmake ninja pkg-config gtk3 xz
```

**Windows:**
- Visual Studio 2022 with "Desktop development with C++" workload
- Windows 10 SDK

**Android (optional):**
- Android Studio with Android SDK
- Set `ANDROID_HOME` environment variable
- Run `flutter doctor --android-licenses`

#### Build Commands

```bash
# Clone the repository
git clone https://github.com/rennerdo30/toss-share.git
cd toss-share

# Run setup script (recommended)
./scripts/setup.sh

# Or manually build:
make build              # Build Rust components
make release-macos      # Build macOS app
make release-linux      # Build Linux app
make release-windows    # Build Windows app
make release-android    # Build Android APK
make release-ios        # Build iOS app

# Build everything for all platforms
make release-all

# Create distributable archives
make package-all
```

Build outputs are placed in the `dist/` directory:
```
dist/
├── macos/          # Toss.app
├── linux/          # Linux bundle
├── windows/        # Windows executable
├── android/        # toss.apk
├── ios/            # iOS app (unsigned)
└── relay-server/   # Relay server binary + Docker image
```

#### Troubleshooting

**macOS: "CocoaPods not installed"**
```bash
brew install cocoapods
cd flutter_app/macos && pod install
```

**macOS: Xcode errors**
```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
sudo xcodebuild -runFirstLaunch
```

**Flutter: Platform not enabled**
```bash
flutter config --enable-macos-desktop
flutter config --enable-linux-desktop
flutter config --enable-windows-desktop
```

**General: Dependency issues**
```bash
cd flutter_app
flutter clean
flutter pub get
```

### Logging and Diagnostics

Log files are stored in platform-specific locations:

| Platform | Path |
|----------|------|
| Windows | `%LOCALAPPDATA%\toss\logs\` |
| macOS | `~/Library/Application Support/toss/logs/` |
| Linux | `~/.local/share/toss/logs/` |

**Access logs from the app:** Settings → About → Open Log Folder

**Windows console output:** Run `start /wait toss.exe` from cmd.exe to see log output.

**Crash diagnostics:** Check `panic.log` in the logs directory for crash details.

## Quick Start

1. **Install Toss** on two or more devices
2. **Open Toss** on both devices
3. **Pair devices**:
   - On Device A: Click "Add Device" to show a QR code
   - On Device B: Scan the QR code or enter the 6-digit code
4. **Start syncing**: Copy something on one device, it appears on the other!

## Architecture

Toss uses a hybrid architecture for optimal performance and reliability:

```
┌─────────────┐     P2P (QUIC)      ┌─────────────┐
│  Device A   │◄───────────────────►│  Device B   │
│  (Flutter)  │                     │  (Flutter)  │
└──────┬──────┘                     └──────┬──────┘
       │                                   │
       │         Relay (Fallback)          │
       └──────────────►┌───┐◄──────────────┘
                       │ R │
                       │ E │
                       │ L │
                       │ A │
                       │ Y │
                       └───┘
```

- **Rust Core**: Handles encryption, networking, and clipboard operations
- **Flutter UI**: Cross-platform user interface
- **Relay Server**: Optional fallback for remote sync (self-hostable)

## Security

Toss takes security seriously:

- **X25519** key exchange for secure device pairing
- **AES-256-GCM** authenticated encryption for all data
- **Ed25519** signatures for device identity
- **Zero-knowledge relay**: The relay server only sees encrypted blobs
- **Forward secrecy**: Session keys are rotated regularly

See [SECURITY.md](SECURITY.md) for our security policy and how to report vulnerabilities.

## Self-Hosting the Relay Server

You can run your own relay server:

```bash
cd relay_server
docker-compose up -d
```

Then configure Toss to use your relay:
Settings → Relay Server → Enter your server URL

### Relay Server Configuration

The relay server is configured entirely through environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `8080` | HTTP/WebSocket port |
| `DATABASE_URL` | `sqlite:./data/toss.db?mode=rwc` | SQLite database location |
| `JWT_SECRET` | random per start | Signing key for device tokens (set it, or tokens break on restart) |
| `JWT_EXPIRATION` | `86400` | Token lifetime in seconds |
| `RATE_LIMIT_MESSAGES` | `100` | Relay messages allowed per minute |
| `RATE_LIMIT_REGISTER` | `10` | Device registrations allowed per hour |
| `SESSION_SECRET` | random per start | Signing key for admin dashboard cookies |
| `ADMIN_USERNAME` | unset | Enables the admin dashboard when set together with the password hash |
| `ADMIN_PASSWORD_HASH` | unset | bcrypt hash of the admin password |
| `RUST_LOG` | `info` | Log filter (`error`, `warn`, `info`, `debug`, `trace`) |

### Admin Dashboard

When `ADMIN_USERNAME` and `ADMIN_PASSWORD_HASH` are both set, the relay serves a
server-rendered dashboard at `/admin` with an overview of devices, teams,
pairing sessions, recent log entries and maintenance actions (cleaning up stale
devices, expired pairings and queued messages). Health checks live at `/health`.

## Project Status

Toss is a personal project under active development. The Rust core, Flutter
client, relay server and browser extension are all implemented; releases are
built per platform from the `Makefile` targets listed above.

Working on the Flutter client requires generated FFI bindings — run
`make generate-ffi` after a `make build`, otherwise `flutter analyze` reports
unresolved bindings in `lib/src/rust/`.

## Documentation

Full documentation is published at
[toss.docs.renner.dev](http://toss.docs.renner.dev/) and lives in
[`docs/`](docs) as an Astro Starlight site:

```bash
cd docs
npm install
npm run dev
```

In-repo references:

- **[GETTING_STARTED.md](GETTING_STARTED.md)** - First build and run
- **[QUICK_START.md](QUICK_START.md)** - Development quick start
- **[SPECIFICATION.md](SPECIFICATION.md)** - Protocol and architecture specification
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - Contribution guidelines
- **[SECURITY.md](SECURITY.md)** - Security policy
- **[CHANGELOG.md](CHANGELOG.md)** - Release history

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Toss is open source software licensed under the [MIT License](LICENSE).

## Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Core library
- [Flutter](https://flutter.dev/) - Cross-platform UI
- [Quinn](https://github.com/quinn-rs/quinn) - QUIC implementation
- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge) - Rust/Dart FFI
