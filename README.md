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

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Toss is open source software licensed under the [MIT License](LICENSE).

## Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Core library
- [Flutter](https://flutter.dev/) - Cross-platform UI
- [Quinn](https://github.com/quinn-rs/quinn) - QUIC implementation
- [flutter_rust_bridge](https://github.com/aspect-build/flutter_rust_bridge) - Rust/Dart FFI
