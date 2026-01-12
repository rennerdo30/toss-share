# Toss - Makefile
# Build, test, and manage all components

.PHONY: all build build-rust build-relay build-flutter \
        test test-rust test-relay test-flutter \
        clean clean-rust clean-relay clean-flutter \
        fmt lint check docker run-relay help \
        release release-all package-all package-relay \
        release-macos release-linux release-windows release-android release-ios \
        setup check-deps check-deps-macos pod-install

# Default target
all: build

# ==============================================================================
# Setup & Dependencies
# ==============================================================================

## Run setup script to install dependencies
setup:
	@./scripts/setup.sh

## Check if all required dependencies are installed
check-deps:
	@echo "Checking dependencies..."
	@command -v rustc >/dev/null 2>&1 || { echo "ERROR: Rust not installed. Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "ERROR: Cargo not installed"; exit 1; }
	@command -v flutter >/dev/null 2>&1 || { echo "ERROR: Flutter not installed. See https://flutter.dev/docs/get-started/install"; exit 1; }
	@echo "✓ All required dependencies found"

## Check macOS-specific dependencies
check-deps-macos:
	@echo "Checking macOS dependencies..."
	@command -v xcodebuild >/dev/null 2>&1 || { echo "ERROR: Xcode not installed. Install from App Store."; exit 1; }
	@command -v pod >/dev/null 2>&1 || { echo "ERROR: CocoaPods not installed. Run: brew install cocoapods"; exit 1; }
	@echo "✓ macOS dependencies found"

## Install CocoaPods dependencies for Flutter
pod-install:
	@echo "Installing CocoaPods dependencies..."
	@cd flutter_app/macos && pod install --repo-update
	@echo "✓ CocoaPods dependencies installed"

# ==============================================================================
# Build Targets
# ==============================================================================

## Build all components
build: build-rust build-relay
	@echo "✓ All components built successfully"

## Build Rust core library
build-rust:
	@echo "Building Rust core..."
	@cd rust_core && cargo build --release
	@echo "✓ Rust core built"

## Build relay server
build-relay:
	@echo "Building relay server..."
	@cd relay_server && cargo build --release
	@echo "✓ Relay server built"

## Build Flutter app (requires Flutter SDK)
build-flutter:
	@echo "Building Flutter app..."
	@cd flutter_app && flutter pub get
	@cd flutter_app && flutter build macos
	@echo "✓ Flutter app built"

## Build Flutter app for all platforms
build-flutter-all: build-flutter
	@cd flutter_app && flutter build linux || true
	@cd flutter_app && flutter build windows || true
	@cd flutter_app && flutter build apk || true
	@cd flutter_app && flutter build ios || true

# ==============================================================================
# Test Targets
# ==============================================================================

## Run all tests
test: test-rust test-relay
	@echo "✓ All tests passed"

## Run Rust core tests
test-rust:
	@echo "Testing Rust core..."
	@cd rust_core && cargo test
	@echo "✓ Rust core tests passed"

## Run relay server tests
test-relay:
	@echo "Testing relay server..."
	@cd relay_server && cargo test
	@echo "✓ Relay server tests passed"

## Run Flutter tests (requires Flutter SDK)
test-flutter:
	@echo "Testing Flutter app..."
	@cd flutter_app && flutter test
	@echo "✓ Flutter tests passed"

## Run clipboard tests (requires single thread)
test-clipboard:
	@echo "Testing clipboard (single-threaded)..."
	@cd rust_core && cargo test -- --ignored --test-threads=1
	@echo "✓ Clipboard tests passed"

## Run comprehensive test suite
test-all:
	@./scripts/test-all.sh

# ==============================================================================
# Code Quality Targets
# ==============================================================================

## Format all Rust code
fmt:
	@echo "Formatting Rust code..."
	@cd rust_core && cargo fmt
	@cd relay_server && cargo fmt
	@echo "✓ Code formatted"

## Run lints on all Rust code
lint:
	@echo "Running Clippy..."
	@cd rust_core && cargo clippy -- -D warnings
	@cd relay_server && cargo clippy -- -D warnings
	@echo "✓ Lint passed"

## Check code without building (faster)
check:
	@echo "Checking Rust code..."
	@cd rust_core && cargo check
	@cd relay_server && cargo check
	@echo "✓ Check passed"

## Run all checks (fmt, lint, test)
ci: fmt lint test
	@echo "✓ CI checks passed"

# ==============================================================================
# Docker Targets
# ==============================================================================

## Build Docker image for relay server
docker:
	@echo "Building Docker image..."
	@cd relay_server && docker build -t toss-relay .
	@echo "✓ Docker image built: toss-relay"

## Run relay server in Docker
docker-run:
	@echo "Starting relay server in Docker..."
	@cd relay_server && docker-compose up -d
	@echo "✓ Relay server running on http://localhost:8080"

## Stop relay server Docker container
docker-stop:
	@echo "Stopping relay server..."
	@cd relay_server && docker-compose down
	@echo "✓ Relay server stopped"

## View relay server logs
docker-logs:
	@cd relay_server && docker-compose logs -f

# ==============================================================================
# Development Targets
# ==============================================================================

## Run relay server locally (development)
run-relay:
	@echo "Starting relay server..."
	@cd relay_server && cargo run

## Run relay server with hot reload
watch-relay:
	@echo "Starting relay server with hot reload..."
	@cd relay_server && cargo watch -x run

## Run Flutter app (development)
run-flutter:
	@cd flutter_app && flutter run

## Generate Rust FFI bindings for Flutter
generate-ffi:
	@echo "Generating FFI bindings..."
	@cd flutter_app && flutter_rust_bridge_codegen generate
	@echo "✓ FFI bindings generated"

## Generate Riverpod providers
generate-providers:
	@echo "Generating Riverpod providers..."
	@cd flutter_app && flutter pub run build_runner build --delete-conflicting-outputs
	@echo "✓ Providers generated"

# ==============================================================================
# Clean Targets
# ==============================================================================

## Clean all build artifacts
clean: clean-rust clean-relay clean-flutter
	@echo "✓ All clean"

## Clean Rust core build artifacts
clean-rust:
	@echo "Cleaning Rust core..."
	@cd rust_core && cargo clean
	@echo "✓ Rust core clean"

## Clean relay server build artifacts
clean-relay:
	@echo "Cleaning relay server..."
	@cd relay_server && cargo clean
	@echo "✓ Relay server clean"

## Clean Flutter build artifacts
clean-flutter:
	@echo "Cleaning Flutter app..."
	@cd flutter_app && flutter clean || true
	@echo "✓ Flutter app clean"

# ==============================================================================
# Release Targets
# ==============================================================================

## Create release builds (Rust only)
release: build
	@echo "Creating release..."
	@mkdir -p dist/relay-server
	@cp rust_core/target/release/libtoss_core.* dist/ 2>/dev/null || true
	@cp relay_server/target/release/toss-relay dist/relay-server/
	@echo "✓ Release artifacts in dist/"

## Package relay server for distribution
package-relay: build-relay
	@echo "Packaging relay server..."
	@mkdir -p dist/relay-server
	@cd relay_server && docker build -t toss-relay:latest .
	@docker save toss-relay:latest | gzip > dist/relay-server/toss-relay-docker.tar.gz
	@echo "✓ Packaged to dist/relay-server/toss-relay-docker.tar.gz"

## Build and package everything for all platforms
release-all: clean build package-relay release-macos release-linux release-windows release-android release-ios
	@echo ""
	@echo "=============================================="
	@echo "Release build complete!"
	@echo "=============================================="
	@echo ""
	@echo "Artifacts in dist/:"
	@ls -la dist/ 2>/dev/null || true
	@echo ""
	@echo "Platform packages:"
	@ls -la dist/*/ 2>/dev/null || true
	@echo ""
	@echo "✓ All releases packaged in dist/"

## Build macOS release
release-macos: check-deps-macos
	@echo "Building macOS release..."
	@mkdir -p dist/macos
	@cd flutter_app && flutter pub get
	@cd flutter_app/macos && pod install --repo-update || pod install
	@cd flutter_app && flutter build macos --release
	@cp -r flutter_app/build/macos/Build/Products/Release/Toss.app dist/macos/ 2>/dev/null || \
		cp -r flutter_app/build/macos/Build/Products/Release/*.app dist/macos/ 2>/dev/null || \
		echo "⚠ macOS build not available (may need to run on macOS)"
	@echo "✓ macOS release in dist/macos/"

## Build Linux release
release-linux:
	@echo "Building Linux release..."
	@mkdir -p dist/linux
	@cd flutter_app && flutter build linux --release 2>/dev/null && \
		cp -r flutter_app/build/linux/x64/release/bundle/* dist/linux/ || \
		echo "⚠ Linux build not available (may need to run on Linux)"
	@echo "✓ Linux release in dist/linux/"

## Build Windows release
release-windows:
	@echo "Building Windows release..."
	@mkdir -p dist/windows
	@cd flutter_app && flutter build windows --release 2>/dev/null && \
		cp -r flutter_app/build/windows/x64/runner/Release/* dist/windows/ || \
		cp -r flutter_app/build/windows/runner/Release/* dist/windows/ 2>/dev/null || \
		echo "⚠ Windows build not available (may need to run on Windows)"
	@echo "✓ Windows release in dist/windows/"

## Build Android release (APK)
release-android:
	@echo "Building Android release..."
	@mkdir -p dist/android
	@cd flutter_app && flutter build apk --release 2>/dev/null && \
		cp flutter_app/build/app/outputs/flutter-apk/app-release.apk dist/android/toss.apk || \
		echo "⚠ Android build not available (may need Android SDK)"
	@echo "✓ Android release in dist/android/"

## Build iOS release
release-ios:
	@echo "Building iOS release..."
	@mkdir -p dist/ios
	@cd flutter_app && flutter build ios --release --no-codesign 2>/dev/null && \
		cp -r flutter_app/build/ios/iphoneos/Runner.app dist/ios/Toss.app || \
		echo "⚠ iOS build not available (may need to run on macOS with Xcode)"
	@echo "✓ iOS release in dist/ios/"

## Create distributable archives
package-all: release-all
	@echo "Creating distribution archives..."
	@cd dist && [ -d macos ] && [ -n "$$(ls -A macos 2>/dev/null)" ] && tar -czvf toss-macos.tar.gz macos || true
	@cd dist && [ -d linux ] && [ -n "$$(ls -A linux 2>/dev/null)" ] && tar -czvf toss-linux.tar.gz linux || true
	@cd dist && [ -d windows ] && [ -n "$$(ls -A windows 2>/dev/null)" ] && zip -r toss-windows.zip windows || true
	@cd dist && [ -d android ] && [ -f android/toss.apk ] && cp android/toss.apk toss-android.apk || true
	@cd dist && [ -d ios ] && [ -n "$$(ls -A ios 2>/dev/null)" ] && tar -czvf toss-ios.tar.gz ios || true
	@echo ""
	@echo "Distribution archives:"
	@ls -la dist/*.tar.gz dist/*.zip dist/*.apk 2>/dev/null || echo "No archives created"
	@echo ""
	@echo "✓ Distribution packages ready in dist/"

# ==============================================================================
# Help
# ==============================================================================

## Show this help message
help:
	@echo "Toss - Build System"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Setup:"
	@echo "  setup          Run setup script to install dependencies"
	@echo "  check-deps     Check if required dependencies are installed"
	@echo "  pod-install    Install CocoaPods dependencies (macOS)"
	@echo ""
	@echo "Build Targets:"
	@echo "  build          Build all components (default)"
	@echo "  build-rust     Build Rust core library"
	@echo "  build-relay    Build relay server"
	@echo "  build-flutter  Build Flutter app (requires Flutter SDK)"
	@echo ""
	@echo "Test Targets:"
	@echo "  test           Run all tests"
	@echo "  test-rust      Run Rust core tests"
	@echo "  test-relay     Run relay server tests"
	@echo "  test-flutter   Run Flutter tests"
	@echo "  test-clipboard Run clipboard tests (single-threaded)"
	@echo "  test-all       Run comprehensive test suite"
	@echo ""
	@echo "Code Quality:"
	@echo "  fmt            Format all Rust code"
	@echo "  lint           Run Clippy lints"
	@echo "  check          Check code without building"
	@echo "  ci             Run all CI checks"
	@echo ""
	@echo "Docker:"
	@echo "  docker         Build Docker image"
	@echo "  docker-run     Start relay in Docker"
	@echo "  docker-stop    Stop relay Docker container"
	@echo "  docker-logs    View relay logs"
	@echo ""
	@echo "Development:"
	@echo "  run-relay      Run relay server locally"
	@echo "  run-flutter    Run Flutter app"
	@echo "  generate-ffi   Generate Rust FFI bindings"
	@echo ""
	@echo "Clean:"
	@echo "  clean          Clean all build artifacts"
	@echo ""
	@echo "Release:"
	@echo "  release        Create release builds (Rust only)"
	@echo "  release-all    Build ALL platforms and package to dist/"
	@echo "  package-all    Build all + create distributable archives"
	@echo "  release-macos  Build macOS app"
	@echo "  release-linux  Build Linux app"
	@echo "  release-windows Build Windows app"
	@echo "  release-android Build Android APK"
	@echo "  release-ios    Build iOS app"
	@echo "  package-relay  Package relay server Docker image"
