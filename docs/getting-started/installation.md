# Installation

Install Toss on your platform.

## Desktop Platforms

### macOS

1. Download the latest release from [GitHub Releases](https://github.com/rennerdo30/toss-share/releases/latest)
2. Open the downloaded `.dmg` file
3. Drag Toss.app to Applications folder
4. Open Toss from Applications
5. Grant accessibility permissions when prompted

### Windows

1. Download the latest release from [GitHub Releases](https://github.com/rennerdo30/toss-share/releases/latest)
2. Run the installer or extract the portable version
3. Launch `toss.exe`
4. Windows may prompt for firewall permissions - allow access

### Linux

1. Download the AppImage from [GitHub Releases](https://github.com/rennerdo30/toss-share/releases/latest)
2. Make it executable:
   ```bash
   chmod +x toss-*.AppImage
   ```
3. Run the AppImage:
   ```bash
   ./toss-*.AppImage
   ```

## Build from Source

### Prerequisites

**All Platforms:**
- [Rust](https://rustup.rs/) (1.75+)
- [Flutter](https://flutter.dev/docs/get-started/install) (3.24+)
- [Git](https://git-scm.com/)

**macOS:**
- Xcode (from App Store)
- Xcode Command Line Tools: `xcode-select --install`
- CocoaPods: `brew install cocoapods`

**Linux (Debian/Ubuntu):**
```bash
sudo apt-get update
sudo apt-get install -y clang cmake ninja-build pkg-config \
    libgtk-3-dev liblzma-dev libstdc++-12-dev
```

**Windows:**
- Visual Studio 2022 with "Desktop development with C++" workload
- Windows 10 SDK

### Build Steps

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
```

Build outputs are placed in the `dist/` directory.

## Troubleshooting

### macOS: "CocoaPods not installed"
```bash
brew install cocoapods
cd flutter_app/macos && pod install
```

### macOS: Xcode errors
```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
sudo xcodebuild -runFirstLaunch
```

### Flutter: Platform not enabled
```bash
flutter config --enable-macos-desktop
flutter config --enable-linux-desktop
flutter config --enable-windows-desktop
```

### General: Dependency issues
```bash
cd flutter_app
flutter clean
flutter pub get
```

## Next Steps

- [Quick Start](quick-start.md) - Get started using Toss
- [Development Setup](development-setup.md) - Set up development environment
