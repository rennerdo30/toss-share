# Toss - Project Context

## Critical Rules
- **ALL tests must pass before declaring work complete** - run `make ci`
- **No commits without 100% passing tests** - blocked by CI
- **Test coverage must be as close to 100% as possible**
- **Keep documentation up to date**: README.md, SPECIFICATION.md, this file
- **Keep SPECIFICATION.md in sync** with implementation changes

## Project Overview
Cross-platform clipboard sharing with E2E encryption, P2P networking, and cloud relay fallback.

## Tech Stack
- **Rust Core** (`rust_core/`): crypto, networking (QUIC/WebSocket), clipboard, storage (SQLite)
- **Flutter App** (`flutter_app/`): UI with Riverpod, FFI via `flutter_rust_bridge`
- **Relay Server** (`relay_server/`): Cloud fallback for NAT traversal

## Key Commands
```bash
make ci              # Run all CI checks (fmt, lint, test) - REQUIRED before commit
make test            # Run all tests
make build           # Build everything
make run-flutter     # Run Flutter app
```

## Commit Guidelines
Use [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <description>`
- Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `ci`

## Architecture
```
FlutterUI <-> FFI Bridge <-> Rust Core
                              ├── crypto/     (X25519, AES-256-GCM)
                              ├── network/    (QUIC, mDNS, TURN, WebSocket)
                              ├── clipboard/  (arboard + platform-specific)
                              ├── protocol/   (bincode serialization)
                              └── storage/    (SQLite)
```

## Platform Notes
| Platform | Notes |
|----------|-------|
| macOS | Requires accessibility permissions (`AXIsProcessTrusted`) |
| Windows | Custom clipboard format handling (CF_UNICODETEXT, CF_HDROP, CF_DIB) |
| Linux | X11 + Wayland support |
| iOS | Limited background clipboard access |
| Android | Keystore for secure storage, clipboard restrictions in Android 10+ |

## Security
- Never log clipboard contents
- Clear sensitive data from memory
- Constant-time crypto comparisons
- Validate all incoming data
- Rate limit connections

## CI Workflows
- `ci.yml`: fmt, clippy, test on every push/PR
- `nightly.yml`: Full platform builds on main
- `security.yml`: cargo audit for vulnerabilities
- `release.yml`: Version tag releases

## Documentation

- **Always use Mermaid** for diagrams in documentation
- Mermaid is supported in the Starlight docs site
- Avoid external diagram tools or image files when possible
