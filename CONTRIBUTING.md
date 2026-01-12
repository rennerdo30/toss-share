# Contributing to Toss

Thank you for your interest in contributing to Toss! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in [Issues](https://github.com/rennerdo30/toss-share/issues)
2. If not, create a new issue using the bug report template
3. Include as much detail as possible: OS, version, steps to reproduce, expected vs actual behavior

### Suggesting Features

1. Check if the feature has already been suggested in [Issues](https://github.com/rennerdo30/toss-share/issues)
2. If not, create a new issue using the feature request template
3. Describe the use case and why this feature would be valuable

### Contributing Code

1. Fork the repository
2. Create a new branch from `main`: `git checkout -b feature/your-feature-name`
3. Make your changes
4. Write or update tests as needed
5. Ensure all tests pass
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- Flutter 3.24+ (install via [flutter.dev](https://flutter.dev/docs/get-started/install))
- For Linux: `sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev`

### Building

```bash
# Build Rust core
cd rust_core
cargo build

# Build Flutter app
cd ../flutter_app
flutter pub get
flutter build macos  # or linux, windows
```

### Running Tests

```bash
# Rust tests
cd rust_core
cargo test

# Flutter tests
cd flutter_app
flutter test

# Relay server tests
cd relay_server
cargo test
```

### Code Style

#### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and fix any warnings
- Write doc comments for public APIs

#### Flutter/Dart

- Follow the [Effective Dart](https://dart.dev/guides/language/effective-dart) guidelines
- Run `flutter analyze` and fix any issues
- Use `flutter format` for consistent formatting

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

Examples:
```
feat(crypto): add X25519 key exchange
fix(clipboard): handle empty clipboard on Windows
docs(readme): add installation instructions
```

## Project Structure

```
toss/
├── rust_core/          # Rust core library
│   ├── src/
│   │   ├── api/        # FFI bridge to Flutter
│   │   ├── crypto/     # Encryption and key management
│   │   ├── clipboard/  # Clipboard operations
│   │   ├── network/    # P2P and relay networking
│   │   └── protocol/   # Wire protocol
│   └── tests/
├── flutter_app/        # Flutter application
│   ├── lib/
│   │   └── src/
│   │       ├── core/       # Providers, services, models
│   │       ├── features/   # UI screens
│   │       └── shared/     # Common widgets
│   └── test/
└── relay_server/       # Relay server
    └── src/
```

## Security

If you discover a security vulnerability, please follow our [Security Policy](SECURITY.md) for responsible disclosure. Do NOT create a public issue for security vulnerabilities.

## Questions?

Feel free to open a [Discussion](https://github.com/rennerdo30/toss-share/discussions) for questions or ideas.

## License

By contributing to Toss, you agree that your contributions will be licensed under the MIT License.
