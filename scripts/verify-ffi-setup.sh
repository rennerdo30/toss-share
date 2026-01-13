#!/bin/bash
# Verify FFI setup before generation

set -e

echo "🔍 Verifying FFI Setup..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0

# Check Rust toolchain
echo -n "Checking Rust toolchain... "
if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
    rustc --version
else
    echo -e "${RED}✗${NC}"
    echo "  ERROR: Rust not installed. Install from: https://rustup.rs"
    ERRORS=$((ERRORS + 1))
fi

# Check Flutter SDK
echo -n "Checking Flutter SDK... "
if command -v flutter >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
    flutter --version | head -1
else
    echo -e "${RED}✗${NC}"
    echo "  ERROR: Flutter not installed. Install from: https://flutter.dev"
    ERRORS=$((ERRORS + 1))
fi

# Check flutter_rust_bridge_codegen
echo -n "Checking flutter_rust_bridge_codegen... "
if command -v flutter_rust_bridge_codegen >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
    flutter_rust_bridge_codegen --version 2>/dev/null || echo "  (version check not available)"
else
    echo -e "${YELLOW}⚠${NC}"
    echo "  WARNING: flutter_rust_bridge_codegen not found."
    echo "  Install with: dart pub global activate flutter_rust_bridge_codegen"
    ERRORS=$((ERRORS + 1))
fi

# Check configuration file
echo -n "Checking frb_options.yaml... "
if [ -f "flutter_app/frb_options.yaml" ]; then
    echo -e "${GREEN}✓${NC}"
    if grep -q "rust_input:" flutter_app/frb_options.yaml; then
        echo "  Configuration format looks correct"
    else
        echo -e "  ${YELLOW}WARNING: Configuration format may be incorrect${NC}"
    fi
else
    echo -e "${RED}✗${NC}"
    echo "  ERROR: frb_options.yaml not found"
    ERRORS=$((ERRORS + 1))
fi

# Check Rust API file
echo -n "Checking Rust API file... "
if [ -f "rust_core/src/api/mod.rs" ]; then
    echo -e "${GREEN}✓${NC}"
    FRB_COUNT=$(grep -c "#\[frb" rust_core/src/api/mod.rs || echo "0")
    echo "  Found $FRB_COUNT #[frb] attributes"
else
    echo -e "${RED}✗${NC}"
    echo "  ERROR: rust_core/src/api/mod.rs not found"
    ERRORS=$((ERRORS + 1))
fi

# Check Rust core compiles
echo -n "Checking Rust core compiles... "
cd rust_core
if cargo check --quiet 2>/dev/null; then
    echo -e "${GREEN}✓${NC}"
    echo "  Rust core compiles successfully"
else
    echo -e "${RED}✗${NC}"
    echo "  ERROR: Rust core has compilation errors"
    echo "  Run 'cd rust_core && cargo check' to see errors"
    ERRORS=$((ERRORS + 1))
fi
cd ..

# Check Flutter dependencies
echo -n "Checking Flutter dependencies... "
cd flutter_app
if flutter pub get --quiet >/dev/null 2>&1; then
    if grep -q "flutter_rust_bridge:" pubspec.yaml; then
        echo -e "${GREEN}✓${NC}"
        echo "  flutter_rust_bridge dependency found"
    else
        echo -e "${YELLOW}⚠${NC}"
        echo "  WARNING: flutter_rust_bridge not in pubspec.yaml"
    fi
else
    echo -e "${YELLOW}⚠${NC}"
    echo "  WARNING: Could not verify Flutter dependencies"
fi
cd ..

echo ""
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✅ All checks passed! Ready to generate FFI bindings.${NC}"
    echo ""
    echo "Run: make generate-ffi"
    exit 0
else
    echo -e "${RED}❌ Found $ERRORS issue(s). Please fix them before generating FFI bindings.${NC}"
    echo ""
    echo "See FFI_READY.md for troubleshooting help."
    exit 1
fi
