# PRD: Toss Codebase Completion & Bug Fixes

## Overview
This PRD addresses all identified incomplete implementations, bugs, dead code, and missing features in the Toss codebase. The goal is to bring the application to a production-ready state where all existing code paths are functional, all features from SPECIFICATION.md are implemented, and the codebase follows best practices for error handling and maintainability.

## Goals
- Fix all critical bugs (settings persistence, session key rotation, STUN/TURN integration)
- Replace unsafe `unwrap()` calls with proper error handling
- Complete all partially implemented features (auto-sync, history cleanup, iOS background)
- Wire up unused API endpoints and remove truly dead code
- Implement future enhancements from SPECIFICATION.md (streaming, compression, selective sync, browser extension, team support)
- Add admin web dashboard to relay server
- Integrate WebSocket for real-time sync with polling fallback
- Ensure all integration tests are functional

## Quality Gates

These commands must pass for every user story:

**For Rust stories (rust_core, relay_server):**
- `cargo fmt --check` - Formatting
- `cargo clippy -- -D warnings` - Linting
- `cargo test` - Unit tests

**For Flutter stories:**
- `flutter analyze` - Static analysis
- `flutter test` - Unit tests

**Final validation for all stories:**
- `make ci` - Full CI pipeline

## User Stories

### US-001: Persist Settings to SQLite Database
As a user, I want my settings to be saved permanently so that they persist across app restarts.

**Acceptance Criteria:**
- [ ] Implement `save_settings()` function in `rust_core/src/storage/mod.rs` that writes to the `settings` table
- [ ] Implement `load_settings()` function that reads from the `settings` table on startup
- [ ] Update `api.updateSettings()` in `rust_core/src/api/mod.rs` to call `save_settings()`
- [ ] Load settings from database in `TossApi::new()` initialization
- [ ] Update `TossService` in Flutter to verify settings persist after app restart
- [ ] Add migration handling for settings schema changes

### US-002: Implement Automatic Session Key Rotation
As a user, I want session keys to rotate automatically so that my clipboard data remains secure over long sessions.

**Acceptance Criteria:**
- [ ] Add message counter to track messages per session in `rust_core/src/network/mod.rs`
- [ ] Add timestamp tracking for session key age
- [ ] Implement automatic key rotation trigger after 1000 messages OR 24 hours (per SPECIFICATION.md section 3.5)
- [ ] Call `rotate_session_key()` automatically when thresholds are met
- [ ] Ensure key rotation doesn't interrupt active transfers
- [ ] Add unit tests for rotation triggers

### US-003: Integrate STUN/TURN for NAT Traversal
As a user, I want peer-to-peer connections to work through NATs so that I can sync clipboards without relay server dependency.

**Acceptance Criteria:**
- [ ] Integrate `NatTraversal` from `rust_core/src/network/nat_traversal.rs` into `NetworkManager::connect()`
- [ ] Perform NAT type detection on connection attempt
- [ ] Use STUN for symmetric NAT hole punching
- [ ] Fall back to TURN relay when direct connection fails
- [ ] Configure default STUN/TURN servers (or use relay server as TURN)
- [ ] Add connection type indicator to Flutter UI (direct vs relayed)
- [ ] Remove or use the 10+ unused constants in nat_traversal.rs

### US-004: Replace Unsafe unwrap() Calls with Proper Error Handling
As a developer, I want the codebase to handle errors gracefully so that the app doesn't crash unexpectedly.

**Acceptance Criteria:**
- [ ] Replace mutex `.unwrap()` calls with `.expect("mutex poisoned - this is a bug")` at lines 336, 405, 553, 724, 737, 811, 1089, 1238, 1277, 1311 in `rust_core/src/api/mod.rs`
- [ ] Replace timestamp conversion unwraps with `Result<T, E>` propagation
- [ ] Update FFI boundary functions to return `Result` types where appropriate
- [ ] Update Flutter FFI bindings to handle error results
- [ ] Add error logging for all converted error paths
- [ ] Ensure no `unwrap()` on user-provided or network data

### US-005: Implement Auto-Sync for Outgoing Clipboard Changes
As a user, I want my clipboard to automatically sync to paired devices when auto-sync is enabled so that I don't have to manually send each time.

**Acceptance Criteria:**
- [ ] Extend `ClipboardMonitorService` in Flutter to detect LOCAL clipboard changes
- [ ] When `auto_sync` setting is true, automatically call send to all paired devices
- [ ] Add debouncing to prevent rapid-fire syncs (e.g., 500ms delay)
- [ ] Respect per-device sync preferences when implemented
- [ ] Add visual indicator in UI when auto-sync sends content
- [ ] Ensure auto-sync doesn't create infinite loops between devices

### US-006: Implement History Auto-Cleanup Based on Age
As a user, I want old clipboard history to be automatically deleted so that my storage doesn't grow indefinitely.

**Acceptance Criteria:**
- [ ] Implement `cleanup_old_history()` function in `rust_core/src/storage/mod.rs`
- [ ] Use `history_days` setting from `rust_core/src/api/mod.rs:52` to determine retention period
- [ ] Run cleanup on app startup
- [ ] Run cleanup periodically (e.g., daily) while app is running
- [ ] Add manual "Clear History" option in Flutter settings
- [ ] Log number of entries cleaned up

### US-007: Implement History Date Range Filtering
As a user, I want to filter clipboard history by date range so that I can find items from specific time periods.

**Acceptance Criteria:**
- [ ] Update `_loadHistory()` in `flutter_app/lib/src/features/history/history_screen.dart` to use date filter values
- [ ] Add date range parameters to the Rust API `get_history()` function
- [ ] Implement SQL query filtering by timestamp range
- [ ] Update UI to show active filter state
- [ ] Add "Clear Filters" button

### US-008: Complete iOS Background Service Implementation
As an iOS user, I want clipboard sync to work when the app is in the background so that I receive clipboard content without opening the app.

**Acceptance Criteria:**
- [ ] Implement `handleShortcutAction()` in `flutter_app/lib/src/core/services/ios_background_service.dart:56`
- [ ] Implement `syncOnForeground()` at line 81
- [ ] Register for background fetch capabilities in iOS project
- [ ] Handle clipboard access restrictions in background mode
- [ ] Test background sync on physical iOS device
- [ ] Document iOS-specific limitations

### US-009: Implement Integration Tests
As a developer, I want comprehensive integration tests so that I can verify the app works end-to-end.

**Acceptance Criteria:**
- [ ] Implement all 11 placeholder tests in `flutter_app/integration_test/app_test.dart`
- [ ] Test device pairing flow
- [ ] Test clipboard send/receive
- [ ] Test history view and search
- [ ] Test settings persistence
- [ ] Test offline/online transitions
- [ ] Ensure tests can run in CI environment

### US-010: Wire Up Device Status API Endpoint
As a user, I want to see real-time status of my paired devices so that I know which devices are online.

**Acceptance Criteria:**
- [ ] Call `/api/v1/devices/{device_id}/status` from Flutter app
- [ ] Display online/offline status in device list
- [ ] Show last seen timestamp for offline devices
- [ ] Implement periodic status polling (every 30 seconds)
- [ ] Update UI immediately when status changes

### US-011: Implement WebSocket Real-Time Sync
As a user, I want clipboard content to sync instantly so that I don't have to wait for polling intervals.

**Acceptance Criteria:**
- [ ] Connect to `/api/v1/ws` WebSocket endpoint from Flutter
- [ ] Implement WebSocket message handling for clipboard events
- [ ] Keep polling as fallback when WebSocket disconnects
- [ ] Implement automatic WebSocket reconnection with exponential backoff
- [ ] Show connection status indicator in UI (WebSocket vs polling)
- [ ] Handle WebSocket authentication

### US-012: Build Admin Web Dashboard for Relay Server
As a server administrator, I want a web dashboard to monitor and manage the relay server so that I can oversee system health.

**Acceptance Criteria:**
- [ ] Create HTML templates in `relay_server/templates/` for admin dashboard
- [ ] Implement dashboard home page showing server stats (uptime, memory, connections)
- [ ] Add connected devices list view with details
- [ ] Add active sessions view
- [ ] Implement authentication for admin routes (use existing `admin_auth.rs`)
- [ ] Add ability to disconnect devices
- [ ] Add server logs viewer
- [ ] Style with CSS (minimal, functional design)

### US-013: Implement Admin Authentication Flow
As an administrator, I want secure login to the admin dashboard so that unauthorized users cannot access server controls.

**Acceptance Criteria:**
- [ ] Implement login page in `relay_server/templates/`
- [ ] Use `admin_auth.rs` for authentication logic
- [ ] Implement session-based auth with secure cookies
- [ ] Add logout functionality
- [ ] Implement rate limiting on login attempts
- [ ] Add password hashing (argon2 or bcrypt)

### US-014: Clean Up Dead Code and Unused Structs
As a developer, I want the codebase free of dead code so that it's easier to maintain.

**Acceptance Criteria:**
- [ ] Remove or integrate `RegisterRequest`/`RegisterResponse` in `rust_core/src/network/relay_client.rs:25-41`
- [ ] Remove `#[allow(dead_code)]` annotations and either use or delete the methods in `rust_core/src/clipboard/rich_text.rs`
- [ ] Evaluate `MemoryStorage` fallback in `rust_core/src/storage/secure_storage.rs:759-791` - integrate or remove
- [ ] Remove or document `start_event_listener()` if `poll_event()` is the intended API
- [ ] Consolidate redundant pairing functions or document why all three are needed
- [ ] Run `cargo clippy` and address all dead_code warnings

### US-015: Fix Test Assertions
As a developer, I want tests to use proper assertions so that failures are clear and debugging is easier.

**Acceptance Criteria:**
- [ ] Replace `panic!("Expected X")` with `assert!()` or `assert_eq!()` in `rust_core/src/protocol/message.rs` at lines 341, 365, 384, 410, 434
- [ ] Ensure all tests have descriptive assertion messages
- [ ] Verify tests still pass after refactoring

### US-016: Implement Clipboard Streaming for Large Content
As a user, I want to sync large clipboard content (like images) efficiently so that transfers don't time out or use excessive memory.

**Acceptance Criteria:**
- [ ] Implement chunked transfer protocol for content > 1MB
- [ ] Add progress indicator for large transfers in Flutter UI
- [ ] Implement resume capability for interrupted transfers
- [ ] Add memory-efficient streaming (don't load entire content in memory)
- [ ] Update SPECIFICATION.md section 12 to reflect implementation
- [ ] Add configuration for chunk size

### US-017: Implement Selective Sync Per Device
As a user, I want to choose which devices receive my clipboard so that I have control over where my data goes.

**Acceptance Criteria:**
- [ ] Add per-device sync toggle in Flutter device settings
- [ ] Store sync preferences in database
- [ ] Filter target devices in send operation based on preferences
- [ ] Add "Sync to All" quick action
- [ ] Show sync status per device in device list
- [ ] Sync preferences should sync across devices (meta-sync)

### US-018: Implement Compression for Clipboard Content
As a user, I want clipboard content to be compressed during transfer so that syncing is faster on slow networks.

**Acceptance Criteria:**
- [ ] Add compression for content > 10KB (configurable threshold)
- [ ] Use zstd or lz4 for compression (fast decompression)
- [ ] Add compression flag to protocol messages
- [ ] Implement automatic decompression on receive
- [ ] Show compression ratio in transfer stats (optional)
- [ ] Ensure backwards compatibility with uncompressed messages

### US-019: Create Browser Extension for Clipboard Sync
As a user, I want to sync clipboard content directly from my browser so that I can share content without switching apps.

**Acceptance Criteria:**
- [ ] Create Chrome extension with manifest v3
- [ ] Implement Firefox extension (WebExtension API)
- [ ] Add popup UI showing recent clipboard items
- [ ] Implement "Send to Device" context menu option
- [ ] Connect to relay server via WebSocket
- [ ] Implement extension authentication with main app
- [ ] Add extension settings (auto-sync toggle, target device)
- [ ] Create extension build process and documentation

### US-020: Implement Team/Organization Support
As a team administrator, I want to manage clipboard sharing within my organization so that team members can securely share content.

**Acceptance Criteria:**
- [ ] Add organization/team model to database schema
- [ ] Implement team creation and invitation flow
- [ ] Add team device discovery (see all team devices)
- [ ] Implement team-wide clipboard broadcast option
- [ ] Add role-based permissions (admin, member)
- [ ] Create team management UI in Flutter
- [ ] Add team management to relay server admin dashboard
- [ ] Implement team audit log for compliance

## Functional Requirements

- FR-1: Settings must persist to SQLite and load on app startup
- FR-2: Session keys must rotate automatically after 1000 messages or 24 hours
- FR-3: P2P connections must attempt STUN/TURN before falling back to relay
- FR-4: All user/network data operations must use Result types, not unwrap()
- FR-5: Auto-sync must detect local clipboard changes and send to enabled devices
- FR-6: History older than `history_days` setting must be automatically deleted
- FR-7: History view must support date range filtering
- FR-8: iOS app must sync clipboard when returning to foreground
- FR-9: Device status must be visible and update in real-time
- FR-10: WebSocket must be primary sync mechanism with polling fallback
- FR-11: Admin dashboard must require authentication
- FR-12: Large clipboard content (>1MB) must use chunked streaming
- FR-13: Users must be able to enable/disable sync per device
- FR-14: Content >10KB must be compressed during transfer
- FR-15: Browser extensions must connect via WebSocket to relay server
- FR-16: Teams must support invite-based membership and role permissions

## Non-Goals

- Mobile-to-mobile direct P2P (always uses relay for mobile)
- End-to-end encrypted team channels (v2 feature)
- Clipboard history search across devices (local search only)
- Safari browser extension (WebExtension API not fully supported)
- Self-hosted relay server documentation (separate docs project)
- Kubernetes deployment manifests (separate ops project)

## Technical Considerations

- **Error Handling Strategy:** Use `Result<T, E>` for fallible operations (network, IO, database). Use `.expect("message")` only for states that indicate bugs (mutex poisoning). Never use bare `unwrap()` on external data.
- **WebSocket + Polling Hybrid:** WebSocket is primary for real-time sync. Polling (every 30s) runs as fallback and consistency check. If WebSocket disconnects, polling continues until WebSocket reconnects.
- **Compression:** Use zstd (level 3) for balance of speed and ratio. Add `compressed: bool` field to protocol messages for backwards compatibility.
- **Browser Extension Auth:** Extension generates a pairing code displayed in popup. User enters code in Flutter app to link extension as a device.
- **Team Data Model:** Teams have many users, users have many devices. Team clipboard broadcasts go to all online team member devices.
- **Admin Dashboard:** Server-side rendered HTML with minimal JavaScript. Use existing Actix-web template support. No SPA framework needed.

## Success Metrics

- All `make ci` checks pass
- Zero `unwrap()` calls on user or network data
- Settings persist across 100% of app restarts
- Session keys rotate correctly at thresholds
- P2P connection success rate >80% on non-symmetric NAT
- WebSocket maintains connection >99% of session time
- Admin dashboard loads in <2 seconds
- Browser extension available in Chrome and Firefox stores
- Team features support organizations up to 100 members

## Open Questions

- Should browser extension support Safari via Safari Web Extension API (different from WebExtension)?
- What STUN/TURN servers should be default? (Google's public STUN? Self-hosted TURN?)
- Should team audit logs be stored indefinitely or have retention policy?
- Should compression be optional/configurable per user preference?
- What's the maximum supported team size before performance degrades?