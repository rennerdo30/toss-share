# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure
- End-to-end encrypted clipboard sync
- Device pairing via QR code and 6-digit code
- P2P local network sync using QUIC
- Relay server for remote sync
- Support for text, images, files, and URLs
- Cross-platform support: macOS, Windows, Linux, iOS, Android
- System tray integration (desktop)
- Clipboard history (optional)
- Dark/light theme support

### Security
- X25519 key exchange for device pairing
- AES-256-GCM encryption for all data
- Ed25519 signatures for device identity
- Secure key storage using platform APIs

## [0.1.0] - TBD

### Added
- Initial release

[Unreleased]: https://github.com/rennerdo30/toss-share/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/rennerdo30/toss-share/releases/tag/v0.1.0
