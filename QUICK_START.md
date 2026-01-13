# Quick Start Guide - Toss Development

This guide helps you get started with Toss development after the MVP implementation is complete.

## Prerequisites

Ensure you have all dependencies installed:

```bash
./scripts/setup.sh
```

Required:
- Rust 1.75+
- Flutter 3.24+
- Platform-specific build tools (Xcode, Android SDK, etc.)

## First Steps

### 1. Generate FFI Bindings

The Rust core has FFI attributes configured, but bindings need to be generated:

```bash
make generate-ffi
```

This will:
- Generate Dart bindings from Rust FFI attributes
- Create `flutter_app/lib/src/rust/api.dart`
- Create C header file `rust_core/src/api/toss_api.h`

### 2. Uncomment FFI Calls

After generating bindings, update `flutter_app/lib/src/core/services/toss_service.dart`:

```dart
// Change from:
// import '../rust/api.dart' as api;

// To:
import '../rust/api.dart' as api;

// Then uncomment all FFI calls throughout the file
```

### 3. Build Rust Core

```bash
cd rust_core
cargo build --release
```

### 4. Run Flutter App

```bash
cd flutter_app
flutter pub get
flutter run
```

## Platform-Specific Setup

### macOS

1. **Accessibility Permissions**: 
   - App will request on first launch
   - Or manually: System Preferences → Security & Privacy → Accessibility
   - See `docs/PLATFORM_SPECIFIC.md` for details

2. **Native Code** (if implementing):
   - Add accessibility permission checks in `flutter_app/macos/Runner/`
   - See `docs/IOS_ANDROID_IMPLEMENTATION.md`

### Windows

1. **Clipboard Formats**:
   - Basic support via `arboard` crate
   - For advanced formats, see `rust_core/src/clipboard/windows_formats.rs`
   - See `docs/PLATFORM_SPECIFIC.md` for implementation details

### Linux

1. **Display Server Detection**:
   - Automatically detects X11 vs Wayland
   - See `rust_core/src/clipboard/linux_display.rs`
   - May need additional dependencies:
     ```bash
     # X11
     sudo apt-get install libxcb1-dev libxcb-render0-dev
     
     # Wayland
     sudo apt-get install libwayland-dev
     ```

### iOS

1. **Background Limitations**:
   - Structure ready in `ios_background_service.dart`
   - Requires native code for:
     - Share Extension
     - Shortcuts integration
     - Widget
   - See `docs/IOS_ANDROID_IMPLEMENTATION.md`

2. **Info.plist**:
   - Already updated with background modes
   - May need additional permissions

### Android

1. **Foreground Service**:
   - Structure ready in `android_foreground_service.dart`
   - Requires native Kotlin service
   - Update `AndroidManifest.xml`:
     ```xml
     <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
     <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
     ```
   - See `docs/IOS_ANDROID_IMPLEMENTATION.md`

## Testing

### Run Rust Tests

```bash
cd rust_core
cargo test
```

### Run Flutter Tests

```bash
cd flutter_app
flutter test
```

### Run E2E Tests

```bash
cd flutter_app
flutter test integration_test/app_test.dart
```

**Note**: E2E tests require FFI bindings to be generated first.

## Development Workflow

### 1. Make Changes

- Rust changes: Edit files in `rust_core/src/`
- Flutter changes: Edit files in `flutter_app/lib/`
- Regenerate FFI bindings after Rust API changes

### 2. Test Locally

```bash
# Rust
cd rust_core && cargo test

# Flutter
cd flutter_app && flutter test

# Both
make test-all
```

### 3. Check Quality

```bash
# Rust formatting
cd rust_core && cargo fmt

# Rust clippy
cd rust_core && cargo clippy --all-targets --all-features -- -D warnings

# Flutter analyze
cd flutter_app && flutter analyze
```

### 4. Commit

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
git commit -m "feat(core): add new feature"
git commit -m "fix(ui): fix bug in pairing screen"
git commit -m "docs(readme): update installation instructions"
```

## Common Tasks

### Add New FFI Function

1. Add function to `rust_core/src/api/mod.rs`:
   ```rust
   #[frb(sync)]  // or #[frb] for async
   pub fn my_new_function() -> Result<(), String> {
       // Implementation
   }
   ```

2. Regenerate bindings:
   ```bash
   make generate-ffi
   ```

3. Use in Flutter:
   ```dart
   import '../rust/api.dart' as api;
   
   await api.myNewFunction();
   ```

### Add New Platform Feature

1. Create service in `flutter_app/lib/src/core/services/`
2. Add platform-specific code if needed
3. Update `docs/PLATFORM_SPECIFIC.md`
4. Test on target platform

### Update Storage Schema

1. Modify `rust_core/src/storage/mod.rs` `init_schema()`
2. Add migration logic if needed
3. Update storage models in `rust_core/src/storage/models.rs`
4. Test with existing data

## Troubleshooting

### FFI Bindings Not Generated

```bash
# Check flutter_rust_bridge_codegen is installed
dart pub global activate flutter_rust_bridge_codegen

# Or use make
make generate-ffi
```

### Rust Compilation Errors

```bash
# Update dependencies
cd rust_core && cargo update

# Clean and rebuild
cd rust_core && cargo clean && cargo build
```

### Flutter Build Errors

```bash
# Clean Flutter build
cd flutter_app && flutter clean

# Get dependencies
cd flutter_app && flutter pub get

# Rebuild
cd flutter_app && flutter build
```

### Platform-Specific Issues

- See `docs/PLATFORM_SPECIFIC.md` for platform details
- See `docs/IOS_ANDROID_IMPLEMENTATION.md` for mobile platforms
- Check platform-specific service files in `flutter_app/lib/src/core/services/`

## Project Structure

```
toss/
├── rust_core/           # Rust core library
│   ├── src/
│   │   ├── api/        # FFI API
│   │   ├── clipboard/  # Clipboard operations
│   │   ├── crypto/     # Encryption
│   │   ├── network/    # Networking (mDNS, QUIC, Relay)
│   │   ├── protocol/   # Message protocol
│   │   └── storage/   # SQLite storage
│   └── Cargo.toml
├── flutter_app/        # Flutter UI
│   ├── lib/
│   │   ├── src/
│   │   │   ├── core/   # Services, providers, models
│   │   │   └── features/ # UI screens
│   │   └── main.dart
│   └── pubspec.yaml
├── relay_server/       # Optional relay server
├── docs/               # Documentation
│   ├── PLATFORM_SPECIFIC.md
│   ├── IOS_ANDROID_IMPLEMENTATION.md
│   └── FUTURE_ENHANCEMENTS.md
├── .github/workflows/  # CI/CD
├── TODO.md             # Project TODO list
└── IMPLEMENTATION_SUMMARY.md
```

## Next Steps

1. **Generate FFI bindings** - `make generate-ffi`
2. **Implement native code** - See platform-specific docs
3. **Test on devices** - Verify functionality
4. **Future enhancements** - See `docs/FUTURE_ENHANCEMENTS.md`

## Resources

- [TODO.md](TODO.md) - Complete project status
- [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) - Implementation overview
- [SPECIFICATION.md](SPECIFICATION.md) - Project specification
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines

## Getting Help

- Check documentation in `docs/` directory
- Review `TODO.md` for implementation status
- See `IMPLEMENTATION_SUMMARY.md` for completed work
- Check GitHub Issues for known problems

---

**Status**: MVP implementation complete, ready for FFI generation and testing
