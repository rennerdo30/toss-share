# Getting Started with Toss

**Welcome to Toss!** This guide will help you get started with the Toss clipboard sharing application.

## Quick Start

### 1. Prerequisites

Ensure you have the following installed:

- **Rust** (1.75+): [Install Rust](https://rustup.rs)
- **Flutter** (3.24+): [Install Flutter](https://flutter.dev/docs/get-started/install)
- **Git**: [Install Git](https://git-scm.com)

### 2. Clone and Setup

```bash
# Clone the repository
git clone https://github.com/rennerdo30/toss-share.git
cd toss-share

# Run setup script
./scripts/setup.sh

# Or manually check dependencies
make check-deps
```

### 3. Verify FFI Setup

Before generating FFI bindings, verify everything is configured:

```bash
make verify-ffi
```

This checks:
- ✅ Rust toolchain
- ✅ Flutter SDK
- ✅ flutter_rust_bridge_codegen
- ✅ Configuration files
- ✅ Rust compilation

### 4. Generate FFI Bindings

```bash
make generate-ffi
```

This generates:
- `flutter_app/lib/src/rust/api.dart` - Dart bindings
- `rust_core/src/api/toss_api.h` - C header

### 5. Build the Project

```bash
# Build Rust core
make build-rust

# Build Flutter app
make build-flutter

# Or build everything
make build
```

### 6. Run the Application

```bash
# Run Flutter app
make run-flutter

# Or manually
cd flutter_app && flutter run
```

## Development Workflow

### Daily Development

1. **Start Development**
   ```bash
   # Verify setup
   make verify-ffi
   
   # Generate code (if needed)
   make generate-ffi
   
   # Run app
   make run-flutter
   ```

2. **Make Changes**
   - Edit Rust code in `rust_core/src/`
   - Edit Flutter code in `flutter_app/lib/`
   - Update FFI API in `rust_core/src/api/mod.rs`

3. **After Rust API Changes**
   ```bash
   # Regenerate FFI bindings
   make generate-ffi
   ```

4. **Test Changes**
   ```bash
   # Run tests
   make test-all
   
   # Or individually
   make test-rust
   make test-flutter
   ```

### Code Quality

```bash
# Format code
make fmt

# Lint code
make lint

# Run all checks
make check
```

## Project Structure

```
toss/
├── rust_core/          # Rust core library
│   └── src/
│       ├── api/       # FFI API (mod.rs)
│       ├── clipboard/ # Clipboard operations
│       ├── crypto/    # Encryption
│       ├── network/   # Networking (P2P, Relay)
│       └── storage/   # SQLite storage
├── flutter_app/       # Flutter application
│   └── lib/
│       ├── src/
│       │   ├── core/  # Services, providers
│       │   └── features/ # UI screens
│       └── rust/      # Generated FFI bindings
├── relay_server/      # Relay server (optional)
└── docs/              # Documentation
```

## Common Tasks

### Generate FFI Bindings
```bash
make generate-ffi
```

### Run Tests
```bash
make test-all
```

### Build for Release
```bash
make release
```

### Clean Build Artifacts
```bash
make clean
```

## Troubleshooting

### FFI Generation Issues

**Problem**: `flutter_rust_bridge_codegen: command not found`
```bash
dart pub global activate flutter_rust_bridge_codegen
```

**Problem**: Rust compilation errors
```bash
cd rust_core && cargo check
# Fix errors shown
```

**Problem**: Configuration file not found
```bash
# Verify frb_options.yaml exists
ls flutter_app/frb_options.yaml
```

### Build Issues

**Problem**: Flutter build fails
```bash
cd flutter_app
flutter clean
flutter pub get
flutter build
```

**Problem**: Rust build fails
```bash
cd rust_core
cargo clean
cargo build
```

## Next Steps

After getting started:

1. **Read Documentation**
   - [QUICK_START.md](QUICK_START.md) - Detailed development guide
   - [NEXT_STEPS.md](NEXT_STEPS.md) - Next development steps
   - [docs/INDEX.md](docs/INDEX.md) - All documentation

2. **Explore the Code**
   - Start with `rust_core/src/api/mod.rs` for the FFI API
   - Check `flutter_app/lib/src/core/services/toss_service.dart` for Flutter integration
   - Review `flutter_app/lib/src/features/` for UI screens

3. **Contribute**
   - See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines
   - Check [TODO.md](TODO.md) for open items
   - Review [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)

## Resources

### Documentation
- [SUMMARY.md](SUMMARY.md) - Quick project summary
- [FINAL_STATUS.md](FINAL_STATUS.md) - Project status
- [FFI_READY.md](FFI_READY.md) - FFI generation guide
- [CHECKLIST.md](CHECKLIST.md) - Pre-release checklist

### External Resources
- [Flutter Documentation](https://flutter.dev/docs)
- [Rust Documentation](https://doc.rust-lang.org)
- [flutter_rust_bridge](https://cjycode.com/flutter_rust_bridge)

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/rennerdo30/toss-share/issues)
- **Documentation**: See `docs/` directory
- **Questions**: Check [TODO.md](TODO.md) or [SPECIFICATION.md](SPECIFICATION.md)

---

**Status**: ✅ MVP Complete - Ready for Development  
**Last Updated**: 2024-12-19
