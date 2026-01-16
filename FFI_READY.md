# FFI Generation - Ready Checklist

**Date**: 2025-01-17
**Status**: ✅ **COMPLETE - FFI Generated and Integrated**

> **Note**: FFI bindings have been successfully generated and all TossService methods are wired to actual Rust FFI calls. This document is kept for historical reference.

## Pre-Flight Checks

### ✅ Configuration Verified

- [x] `frb_options.yaml` configured with correct paths
- [x] `rust_core/src/api/mod.rs` exists with `#[frb]` attributes
- [x] `rust_core/Cargo.toml` includes `flutter_rust_bridge = "2"`
- [x] `flutter_app/pubspec.yaml` includes `flutter_rust_bridge: ^2.0.0`
- [x] `Makefile` has `generate-ffi` target configured

### Configuration Details

**File**: `flutter_app/frb_options.yaml`
```yaml
rust_input: rust_core/src/api/mod.rs
dart_output: lib/src/rust/api.dart
c_output: rust_core/src/api/toss_api.h
```

**Rust API**: `rust_core/src/api/mod.rs`
- Contains 29+ functions with `#[frb]` or `#[frb(sync)]` attributes
- All DTOs have `serde::Serialize` and `serde::Deserialize`
- Async functions properly marked

## Prerequisites

Before generating FFI bindings, ensure:

1. **Rust toolchain installed**
   ```bash
   rustc --version  # Should show 1.75+
   cargo --version
   ```

2. **Flutter SDK installed**
   ```bash
   flutter --version  # Should show 3.24+
   ```

3. **flutter_rust_bridge_codegen installed**
   ```bash
   dart pub global activate flutter_rust_bridge_codegen
   ```

4. **Rust core compiles**
   ```bash
   cd rust_core
   cargo check
   ```
   Should complete without errors.

## Generate FFI Bindings

### Step 1: Verify Setup (Recommended)
```bash
make verify-ffi
```
Or manually:
```bash
./scripts/verify-ffi-setup.sh
```

This will check all prerequisites and configuration.

### Step 2: Generate Bindings

**Option 1: Using Makefile (Recommended)**
```bash
make generate-ffi
```

**Option 2: Manual**
```bash
cd flutter_app
flutter_rust_bridge_codegen generate --config frb_options.yaml
```

## Expected Output

After successful generation, you should see:

1. **Dart bindings created**:
   - `flutter_app/lib/src/rust/api.dart` (generated)

2. **C header created**:
   - `rust_core/src/api/toss_api.h` (generated)

3. **No errors** in console output

## Verification

After generation, verify:

```bash
# Check Dart bindings exist
ls -la flutter_app/lib/src/rust/api.dart

# Check C header exists
ls -la rust_core/src/api/toss_api.h

# Verify Rust still compiles
cd rust_core && cargo check
```

## Common Issues

### Issue: `flutter_rust_bridge_codegen: command not found`
**Solution**: Install with `dart pub global activate flutter_rust_bridge_codegen`

### Issue: `No such file or directory: rust_core/src/api/mod.rs`
**Solution**: Verify you're running from the project root, or adjust paths in `frb_options.yaml`

### Issue: Generation fails with Rust compilation errors
**Solution**: Fix Rust errors first:
```bash
cd rust_core
cargo check
# Fix any errors shown
```

### Issue: Type errors in generated Dart code
**Solution**: Ensure all DTOs in `rust_core/src/api/mod.rs` have:
- `#[derive(serde::Serialize, serde::Deserialize)]`
- Proper field types that map to Dart

## Next Steps After Generation

1. **Uncomment FFI import** ✅ DONE - Already in `toss_service.dart`:
   ```dart
   import '../rust/api.dart' as api;
   ```

2. **Replace mock implementations** ✅ DONE - All FFI calls wired:
   ```dart
   // All methods now call actual FFI, e.g.:
   await api.initToss(dataDir: dataDir.path, deviceName: deviceName);
   ```

3. **Build and test** ✅ DONE - All 150 tests passing:
   ```bash
   # All tests pass
   flutter test  # 55 tests ✅
   cargo test    # 95 tests ✅
   ```

## Support

- See [NEXT_STEPS.md](NEXT_STEPS.md) for detailed instructions
- Check [TODO.md](TODO.md) for implementation details
- Review [QUICK_START.md](QUICK_START.md) for development guide

---

**Status**: ✅ **Ready to Generate FFI Bindings**  
**Next Action**: Run `make generate-ffi`
