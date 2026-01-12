#!/bin/bash
# Toss - Development Environment Setup
# Checks and installs required dependencies

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() {
    echo -e "\n${BLUE}=== $1 ===${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}!${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}→${NC} $1"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Darwin*)  OS="macos" ;;
        Linux*)   OS="linux" ;;
        MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
        *)        OS="unknown" ;;
    esac
    echo $OS
}

OS=$(detect_os)

print_header "Toss Development Setup"
echo "Detected OS: $OS"
echo ""

# Track what needs to be installed
MISSING_DEPS=()
OPTIONAL_MISSING=()

# ==============================================================================
# Check Required Dependencies
# ==============================================================================

print_header "Checking Required Dependencies"

# Rust
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version | cut -d' ' -f2)
    print_success "Rust $RUST_VERSION"
else
    print_error "Rust not installed"
    MISSING_DEPS+=("rust")
fi

# Cargo
if command -v cargo &> /dev/null; then
    print_success "Cargo $(cargo --version | cut -d' ' -f2)"
else
    print_error "Cargo not installed"
    MISSING_DEPS+=("cargo")
fi

# Flutter
if command -v flutter &> /dev/null; then
    FLUTTER_VERSION=$(flutter --version | head -1 | cut -d' ' -f2)
    print_success "Flutter $FLUTTER_VERSION"
else
    print_error "Flutter not installed"
    MISSING_DEPS+=("flutter")
fi

# Git
if command -v git &> /dev/null; then
    print_success "Git $(git --version | cut -d' ' -f3)"
else
    print_error "Git not installed"
    MISSING_DEPS+=("git")
fi

# ==============================================================================
# Check Platform-Specific Dependencies
# ==============================================================================

print_header "Checking Platform Dependencies"

if [ "$OS" = "macos" ]; then
    # Xcode
    if command -v xcodebuild &> /dev/null; then
        XCODE_VERSION=$(xcodebuild -version | head -1)
        print_success "$XCODE_VERSION"

        # Check if Xcode command line tools are properly set up
        if ! xcode-select -p &> /dev/null; then
            print_warning "Xcode command line tools not configured"
            OPTIONAL_MISSING+=("xcode-select")
        fi
    else
        print_error "Xcode not installed (required for macOS/iOS builds)"
        MISSING_DEPS+=("xcode")
    fi

    # CocoaPods
    if command -v pod &> /dev/null; then
        POD_VERSION=$(pod --version)
        print_success "CocoaPods $POD_VERSION"
    else
        print_error "CocoaPods not installed (required for macOS/iOS builds)"
        MISSING_DEPS+=("cocoapods")
    fi

    # Homebrew (optional but recommended)
    if command -v brew &> /dev/null; then
        print_success "Homebrew $(brew --version | head -1 | cut -d' ' -f2)"
    else
        print_warning "Homebrew not installed (recommended for dependency management)"
        OPTIONAL_MISSING+=("homebrew")
    fi
fi

if [ "$OS" = "linux" ]; then
    # Check for required Linux packages
    LINUX_DEPS=("clang" "cmake" "ninja-build" "pkg-config" "libgtk-3-dev" "liblzma-dev")

    for dep in "${LINUX_DEPS[@]}"; do
        if command -v "$dep" &> /dev/null || dpkg -l | grep -q "^ii  $dep"; then
            print_success "$dep"
        else
            print_warning "$dep not found"
            OPTIONAL_MISSING+=("$dep")
        fi
    done
fi

# ==============================================================================
# Check Optional Dependencies
# ==============================================================================

print_header "Checking Optional Dependencies"

# Docker
if command -v docker &> /dev/null; then
    DOCKER_VERSION=$(docker --version | cut -d' ' -f3 | tr -d ',')
    print_success "Docker $DOCKER_VERSION"
else
    print_warning "Docker not installed (needed for relay server deployment)"
    OPTIONAL_MISSING+=("docker")
fi

# Android SDK (for Android builds)
if [ -n "$ANDROID_HOME" ] || [ -n "$ANDROID_SDK_ROOT" ]; then
    print_success "Android SDK found"
else
    print_warning "Android SDK not configured (needed for Android builds)"
    OPTIONAL_MISSING+=("android-sdk")
fi

# ==============================================================================
# Installation Helpers
# ==============================================================================

install_deps() {
    print_header "Installing Missing Dependencies"

    if [ "$OS" = "macos" ]; then
        # Install Homebrew if missing
        if ! command -v brew &> /dev/null; then
            print_info "Installing Homebrew..."
            /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        fi

        # Install Rust
        if [[ " ${MISSING_DEPS[*]} " =~ " rust " ]]; then
            print_info "Installing Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        fi

        # Install CocoaPods
        if [[ " ${MISSING_DEPS[*]} " =~ " cocoapods " ]]; then
            print_info "Installing CocoaPods..."
            if command -v brew &> /dev/null; then
                brew install cocoapods
            else
                sudo gem install cocoapods
            fi
            pod setup
        fi

        # Setup Xcode
        if [[ " ${OPTIONAL_MISSING[*]} " =~ " xcode-select " ]]; then
            print_info "Configuring Xcode command line tools..."
            sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
            sudo xcodebuild -runFirstLaunch
            sudo xcodebuild -license accept 2>/dev/null || true
        fi

        # Install Flutter (via Homebrew)
        if [[ " ${MISSING_DEPS[*]} " =~ " flutter " ]]; then
            print_info "Installing Flutter..."
            brew install --cask flutter
        fi

    elif [ "$OS" = "linux" ]; then
        # Install Rust
        if [[ " ${MISSING_DEPS[*]} " =~ " rust " ]]; then
            print_info "Installing Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        fi

        # Install Linux build dependencies
        print_info "Installing Linux build dependencies..."
        if command -v apt-get &> /dev/null; then
            sudo apt-get update
            sudo apt-get install -y clang cmake ninja-build pkg-config libgtk-3-dev liblzma-dev
        elif command -v dnf &> /dev/null; then
            sudo dnf install -y clang cmake ninja-build pkgconfig gtk3-devel xz-devel
        elif command -v pacman &> /dev/null; then
            sudo pacman -S --noconfirm clang cmake ninja pkg-config gtk3 xz
        fi

        # Flutter instructions
        if [[ " ${MISSING_DEPS[*]} " =~ " flutter " ]]; then
            print_warning "Flutter must be installed manually on Linux"
            print_info "Visit: https://docs.flutter.dev/get-started/install/linux"
        fi
    fi
}

# ==============================================================================
# Flutter Setup
# ==============================================================================

setup_flutter() {
    print_header "Setting Up Flutter"

    if command -v flutter &> /dev/null; then
        print_info "Running flutter doctor..."
        flutter doctor

        print_info "Getting Flutter dependencies..."
        cd "$(dirname "$0")/../flutter_app"
        flutter pub get

        if [ "$OS" = "macos" ] && command -v pod &> /dev/null; then
            print_info "Installing CocoaPods dependencies..."
            cd macos
            pod install || pod install --repo-update
            cd ..
        fi

        print_success "Flutter setup complete"
    else
        print_error "Flutter not installed - skipping Flutter setup"
    fi
}

# ==============================================================================
# Summary & Actions
# ==============================================================================

print_header "Summary"

if [ ${#MISSING_DEPS[@]} -eq 0 ]; then
    print_success "All required dependencies installed!"
else
    print_error "Missing required dependencies: ${MISSING_DEPS[*]}"
fi

if [ ${#OPTIONAL_MISSING[@]} -gt 0 ]; then
    print_warning "Missing optional dependencies: ${OPTIONAL_MISSING[*]}"
fi

echo ""

# Ask to install if there are missing deps
if [ ${#MISSING_DEPS[@]} -gt 0 ] || [ ${#OPTIONAL_MISSING[@]} -gt 0 ]; then
    echo "Would you like to install missing dependencies? (y/n)"
    read -r REPLY
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        install_deps
    fi
fi

# Setup Flutter
if command -v flutter &> /dev/null; then
    echo ""
    echo "Would you like to set up Flutter dependencies? (y/n)"
    read -r REPLY
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        setup_flutter
    fi
fi

print_header "Next Steps"
echo "1. Run 'make build' to build all components"
echo "2. Run 'make test' to run tests"
echo "3. Run 'make release-all' to build for all platforms"
echo ""
echo "For more information, see README.md"
