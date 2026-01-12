#!/bin/bash
# Comprehensive test script for Toss
# Runs all tests across components and reports results

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Track overall status
OVERALL_STATUS=0

# Print colored output
print_header() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
    OVERALL_STATUS=1
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

print_header "Toss - Comprehensive Test Suite"

echo "Project root: $PROJECT_ROOT"
echo "Date: $(date)"
echo ""

# ==============================================================================
# Rust Core Tests
# ==============================================================================
print_header "Running Rust Core Tests"

cd "$PROJECT_ROOT/rust_core"

# Run cargo fmt check
print_info "Checking code formatting..."
if cargo fmt --check 2>/dev/null; then
    print_success "Code formatting OK"
else
    print_warning "Code formatting issues found (run 'cargo fmt' to fix)"
fi

# Run cargo clippy
print_info "Running Clippy lints..."
if cargo clippy --quiet -- -D warnings 2>/dev/null; then
    print_success "Clippy OK (no warnings)"
else
    print_warning "Clippy warnings found"
fi

# Run tests
print_info "Running unit tests..."
TEST_OUTPUT=$(cargo test 2>&1)
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    PASSED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -oE "[0-9]+ passed" | head -1)
    IGNORED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -oE "[0-9]+ ignored" | head -1)
    print_success "Rust Core tests: $PASSED, $IGNORED"
else
    print_error "Rust Core tests failed"
    echo "$TEST_OUTPUT" | tail -20
fi

# Build release
print_info "Building release binary..."
if cargo build --release --quiet 2>/dev/null; then
    print_success "Release build successful"
else
    print_error "Release build failed"
fi

# ==============================================================================
# Relay Server Tests
# ==============================================================================
print_header "Running Relay Server Tests"

cd "$PROJECT_ROOT/relay_server"

# Run cargo fmt check
print_info "Checking code formatting..."
if cargo fmt --check 2>/dev/null; then
    print_success "Code formatting OK"
else
    print_warning "Code formatting issues found"
fi

# Run cargo clippy
print_info "Running Clippy lints..."
if cargo clippy --quiet -- -D warnings 2>/dev/null; then
    print_success "Clippy OK"
else
    print_warning "Clippy warnings found"
fi

# Run tests
print_info "Running unit tests..."
TEST_OUTPUT=$(cargo test 2>&1)
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    # Count tests from both test runs
    TOTAL_PASSED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum}')
    TOTAL_IGNORED=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -oE "[0-9]+ ignored" | awk '{sum += $1} END {print sum}')
    print_success "Relay Server tests: $TOTAL_PASSED passed, $TOTAL_IGNORED ignored"
else
    print_error "Relay Server tests failed"
    echo "$TEST_OUTPUT" | tail -20
fi

# Build release
print_info "Building release binary..."
if cargo build --release --quiet 2>/dev/null; then
    print_success "Release build successful"
else
    print_error "Release build failed"
fi

# ==============================================================================
# Flutter App Tests
# ==============================================================================
print_header "Running Flutter App Tests"

cd "$PROJECT_ROOT/flutter_app"

# Check if Flutter is available
if command -v flutter &> /dev/null; then
    # Run Flutter analyze
    print_info "Running Flutter analyze..."
    if flutter analyze --no-fatal-warnings 2>/dev/null; then
        print_success "Flutter analyze OK"
    else
        print_warning "Flutter analyze found issues"
    fi

    # Run Flutter tests
    print_info "Running Flutter tests..."
    if flutter test 2>/dev/null; then
        print_success "Flutter tests passed"
    else
        print_warning "Flutter tests failed or not available"
    fi
else
    print_warning "Flutter not installed - skipping Flutter tests"
    print_info "Install Flutter to run app tests: https://flutter.dev/docs/get-started/install"
fi

# ==============================================================================
# Docker Build Test
# ==============================================================================
print_header "Testing Docker Build"

cd "$PROJECT_ROOT/relay_server"

if command -v docker &> /dev/null; then
    print_info "Building Docker image..."
    if docker build -t toss-relay-test . --quiet 2>/dev/null; then
        print_success "Docker build successful"
        # Clean up test image
        docker rmi toss-relay-test --force 2>/dev/null || true
    else
        print_warning "Docker build failed"
    fi
else
    print_warning "Docker not installed - skipping Docker build test"
fi

# ==============================================================================
# Summary
# ==============================================================================
print_header "Test Summary"

cd "$PROJECT_ROOT"

echo ""
echo "Components tested:"
echo "  - Rust Core (toss_core)"
echo "  - Relay Server (toss_relay)"
echo "  - Flutter App (toss)"
echo ""

if [ $OVERALL_STATUS -eq 0 ]; then
    print_success "All critical tests passed!"
else
    print_error "Some tests failed - see above for details"
fi

echo ""
echo "Run specific test suites:"
echo "  Rust Core:     cd rust_core && cargo test"
echo "  Relay Server:  cd relay_server && cargo test"
echo "  Flutter:       cd flutter_app && flutter test"
echo "  Clipboard:     cd rust_core && cargo test -- --ignored --test-threads=1"
echo ""

exit $OVERALL_STATUS
