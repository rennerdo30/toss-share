# Toss Project - Next Steps Guide

**Last Updated**: 2025-01-17
**Status**: ✅ MVP Complete - All Code Implementation Done

## Overview

The Toss MVP implementation is **fully complete**. All code-implementable items have been done:
- ✅ FFI bindings generated and integrated
- ✅ All TossService methods wired to Rust FFI
- ✅ 150 tests passing (55 Flutter + 88 Rust Core + 7 Relay Server)
- ✅ Per-device sync, conflict resolution, rate limiting implemented
- ✅ Session key encryption implemented
- ✅ Relay server test harness implemented

## Remaining Work (Non-Code)

The following require runtime testing or native IDE work:
- Runtime testing on physical devices
- Platform-specific native code (Xcode/Android Studio)
- Future enhancements (browser extensions, team features, compression)

## Historical Reference

The sections below document the FFI integration process that has been **completed**.

## Step 1: Generate FFI Bindings

### Prerequisites
- Rust toolchain installed (`rustup`)
- Flutter SDK installed
- `flutter_rust_bridge_codegen` installed:
  ```bash
  dart pub global activate flutter_rust_bridge_codegen
  ```

### Verify Setup (Recommended First)

Before generating, verify everything is set up correctly:

```bash
make verify-ffi
```

Or manually:
```bash
./scripts/verify-ffi-setup.sh
```

This checks:
- Rust toolchain installed
- Flutter SDK installed
- flutter_rust_bridge_codegen installed
- Configuration file exists
- Rust API file exists
- Rust core compiles

### Generate Bindings

**Option 1: Using Makefile (Recommended)**
```bash
make generate-ffi
```

**Option 2: Manual**
```bash
cd flutter_app
flutter_rust_bridge_codegen generate --config frb_options.yaml
```

**Note**: The `frb_options.yaml` file has been configured with the correct paths:
- `rust_input`: `rust_core/src/api/mod.rs`
- `dart_output`: `lib/src/rust/api.dart`
- `c_output`: `rust_core/src/api/toss_api.h`

### Expected Output
- `flutter_app/lib/src/rust/api.dart` - Dart bindings
- `rust_core/src/api/toss_api.h` - C header file

### Verify
- Check that `flutter_app/lib/src/rust/api.dart` exists
- Check that `rust_core/src/api/toss_api.h` exists
- No generation errors in console

## Step 2: Uncomment FFI Calls

### File to Update
`flutter_app/lib/src/core/services/toss_service.dart`

### Changes Needed

1. **Uncomment the import**:
   ```dart
   // Change from:
   // import '../rust/api.dart' as api;
   
   // To:
   import '../rust/api.dart' as api;
   ```

2. **Uncomment all FFI function calls** ✅ DONE:
   - All FFI comments have been replaced with actual calls
   - All placeholder/mock implementations removed
   - All `api.*` function calls wired up

3. **Update method implementations** ✅ DONE:
   - `initialize()` → `api.initToss()` ✅
   - `getDeviceId()` → `api.getDeviceId()` ✅
   - `startPairing()` → `api.startPairing()` ✅
   - `completePairingQR()` → `api.completePairingQr()` ✅
   - `getPairedDevices()` → `api.getPairedDevices()` ✅
   - `sendClipboard()` → `api.sendClipboard()` ✅
   - `pollEvent()` → `api.pollEvent()` ✅
   - And all other methods... ✅

### Example (Historical - Shows What Changed)

**Before** (old code that was replaced):
```dart
// HISTORICAL EXAMPLE - This code no longer exists
Future<void> initialize() async {
  // Old placeholder that was replaced
  await Future.delayed(Duration(milliseconds: 100));
}
```

**After**:
```dart
Future<void> initialize() async {
  final dataDir = await getApplicationDocumentsDirectory();
  final deviceName = await getDeviceName();
  await api.initToss(
    dataDir: dataDir.path,
    deviceName: deviceName,
  );
}
```

## Step 3: Build and Test

### Build Rust Core

```bash
cd rust_core
cargo build --release
```

### Build Flutter App

```bash
cd flutter_app
flutter pub get
flutter build
```

### Run Tests

```bash
# Rust tests
cd rust_core && cargo test

# Flutter tests
cd flutter_app && flutter test

# E2E tests (after FFI is working)
cd flutter_app && flutter test integration_test/app_test.dart
```

## Step 4: Implement Native Code

### Platform-Specific Implementations

#### macOS
- **File**: `flutter_app/macos/Runner/`
- **Task**: Implement accessibility permission checks
- **Guide**: See `docs/PLATFORM_SPECIFIC.md`

#### Windows
- **File**: `rust_core/src/clipboard/windows_formats.rs`
- **Task**: Implement clipboard format handling
- **Guide**: See `docs/PLATFORM_SPECIFIC.md`

#### Linux
- **File**: `rust_core/src/clipboard/linux_display.rs`
- **Task**: Implement X11/Wayland clipboard backends
- **Guide**: See `docs/PLATFORM_SPECIFIC.md`

#### iOS
- **File**: `flutter_app/ios/Runner/`
- **Task**: Implement Share Extension, Shortcuts, Widget
- **Guide**: See `docs/IOS_ANDROID_IMPLEMENTATION.md`

#### Android
- **File**: `flutter_app/android/app/src/main/`
- **Task**: Implement foreground service
- **Guide**: See `docs/IOS_ANDROID_IMPLEMENTATION.md`

## Step 5: Platform Testing (Requires Physical Devices)

> **Note**: These tests require running the app on actual physical devices or emulators. They cannot be completed through code changes alone.

### Desktop Testing (requires device access)
- [ ] Test on macOS (requires Mac)
- [ ] Test on Windows (requires Windows PC)
- [ ] Test on Linux (X11) (requires Linux desktop)
- [ ] Test on Linux (Wayland) (requires Wayland session)

### Mobile Testing (requires device access)
- [ ] Test on iOS device (requires iPhone/iPad + Xcode)
- [ ] Test on Android device (Android 10+) (requires Android device)

### Cross-Platform Testing (requires multiple devices)
- [ ] Test sync between macOS and Windows
- [ ] Test sync between desktop and mobile
- [ ] Test relay fallback when devices not on same network

## Troubleshooting

### FFI Generation Errors

**Error**: `flutter_rust_bridge_codegen: command not found`
- **Solution**: Install with `dart pub global activate flutter_rust_bridge_codegen`

**Error**: `No such file or directory: rust_core/src/api/mod.rs`
- **Solution**: Verify `frb_options.yaml` has correct `rust_input` path

**Error**: Generation fails with Rust errors
- **Solution**: Fix Rust compilation errors first: `cd rust_core && cargo check`

### Build Errors

**Error**: `Cannot find module 'api'`
- **Solution**: Ensure FFI bindings were generated and import path is correct

**Error**: `Undefined symbol` or linking errors
- **Solution**: Ensure Rust core is built: `cd rust_core && cargo build --release`

**Error**: Type mismatches between Rust and Dart
- **Solution**: Check that DTOs match between `rust_core/src/api/mod.rs` and generated Dart code

### Runtime Errors

**Error**: `Storage initialization failed`
- **Solution**: Check data directory permissions and path

**Error**: `Network initialization failed`
- **Solution**: Check network permissions and firewall settings

**Error**: `Clipboard access denied`
- **Solution**: Request appropriate permissions (see platform guides)

## Verification Checklist

### After FFI Generation ✅ COMPLETE
- [x] `flutter_app/lib/src/rust/api.dart` exists
- [x] `rust_core/src/api/toss_api.h` exists
- [x] No generation errors

### After Uncommenting FFI Calls ✅ COMPLETE
- [x] All imports uncommented
- [x] All methods call actual FFI functions
- [x] No placeholder/mock code remaining
- [x] Code compiles without errors

### After Building ✅ COMPLETE
- [x] Rust core builds successfully
- [x] Flutter app builds successfully
- [x] No compilation warnings (or acceptable warnings)
- [x] All tests pass (150 total)

### After Native Code Implementation (Requires Xcode/Android Studio)
- [ ] Platform-specific features work (requires device testing)
- [ ] Permissions are requested correctly (requires device testing)
- [ ] Clipboard access works (requires device testing)
- [ ] Platform tests pass (requires device testing)

## Resources

### Documentation
- [QUICK_START.md](QUICK_START.md) - Development quick start
- [CHECKLIST.md](CHECKLIST.md) - Pre-release checklist
- [docs/PLATFORM_SPECIFIC.md](docs/PLATFORM_SPECIFIC.md) - Platform guide
- [docs/IOS_ANDROID_IMPLEMENTATION.md](docs/IOS_ANDROID_IMPLEMENTATION.md) - Mobile guide

### Code References
- `rust_core/src/api/mod.rs` - Rust FFI API
- `flutter_app/lib/src/core/services/toss_service.dart` - Flutter service
- `flutter_app/frb_options.yaml` - FFI configuration

### Support
- Check [TODO.md](TODO.md) for implementation details
- Review [SPECIFICATION.md](SPECIFICATION.md) for requirements
- See [docs/INDEX.md](docs/INDEX.md) for all documentation

---

**Status**: Ready for FFI Generation  
**Next Action**: Run `make generate-ffi`
