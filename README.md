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

There is no stable release yet. The only published build is the rolling
[**nightly** pre-release](https://github.com/rennerdo30/toss-share/releases/tag/nightly),
which is rebuilt from `main` and carries artifacts for every platform:

| Platform | Artifact |
|----------|----------|
| macOS | `toss-macos-nightly.zip` |
| Windows (x64) | `toss-windows-x64-nightly.zip` |
| Linux (x64) | `toss-linux-x64-nightly.tar.gz` |
| Android | `toss-android-nightly.apk` |
| iOS | `toss-ios-nightly.ipa` (unsigned — needs your own signing to install) |

Nightlies are untagged development builds: expect rough edges. Toss is not on the
App Store or Google Play, so mobile installs mean sideloading the artifacts above
or [building from source](#build-from-source).

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
cp .env.example .env      # then set JWT_SECRET to a random string
docker compose up -d
```

The compose file falls back to a placeholder `JWT_SECRET` if you do not provide
one, so set it before exposing the relay to a network. The server listens on
`:8080` and exposes `/health` for the container healthcheck. State is kept in a
SQLite database on the `relay_data` volume.

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

## Development

Common tasks are wrapped in the `Makefile` — `make help` lists every target.

```bash
make build            # Build the Rust core + relay server
make test             # Rust core + relay server test suites
make test-flutter     # Flutter unit/widget tests
make fmt lint         # cargo fmt + clippy / dart analyze
make ci               # fmt + lint + test, the same set CI runs
make generate-ffi     # Regenerate flutter_rust_bridge bindings after changing rust_core
make run              # Run the Flutter app on the host platform
make run-relay        # Run the relay server locally
```

The `flutter_rust_bridge` bindings under `flutter_app/lib/src/rust/` are generated
and committed, so `make generate-ffi` is only needed after changing the public API
in `rust_core`. `TossService` degrades gracefully when the native library cannot be
loaded: it logs the failure and falls back to a local-only device ID rather than
crashing.

## Project Status

Pre-release. The Rust core, Flutter app, relay server, and browser extension build
and are wired together over `flutter_rust_bridge`, and nightly artifacts are
produced for every target platform. Not done yet: a tagged stable release, app
store distribution, and signed iOS builds. See [TODO.md](TODO.md) for the
remaining work.

## Documentation

Full documentation is published at
[toss.docs.renner.dev](https://toss.docs.renner.dev/); its source is the Astro
Starlight site in [`docs/`](docs), served locally with:

```bash
cd docs
npm install
npm run dev
```

In-repo references:

- [GETTING_STARTED.md](GETTING_STARTED.md) — installation and pairing walkthrough
- [QUICK_START.md](QUICK_START.md) — development quick start
- [SPECIFICATION.md](SPECIFICATION.md) — protocol and architecture specification
- [TODO.md](TODO.md) — remaining work and status
- [CONTRIBUTING.md](CONTRIBUTING.md) — contribution guidelines
- [CHANGELOG.md](CHANGELOG.md) — release history
- [SECURITY.md](SECURITY.md) — vulnerability reporting

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
