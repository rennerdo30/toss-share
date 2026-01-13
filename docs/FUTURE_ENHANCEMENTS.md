# Future Enhancements

This document describes planned future enhancements for Toss. These features are not currently in the MVP scope but are documented for future implementation.

## 22. Clipboard Streaming

### Overview

Real-time clipboard streaming for rapid clipboard changes. Instead of sending complete clipboard content on each change, stream incremental updates.

### Design Considerations

**Protocol Design:**
- Streaming protocol over QUIC or WebSocket
- Change rate limiting to prevent spam
- Conflict resolution for simultaneous changes
- Optimize for low latency (<100ms)

**Implementation Approach:**
1. Detect rapid clipboard changes (e.g., typing)
2. Buffer changes and send in batches
3. Use delta compression for text changes
4. Implement backpressure handling

**Files to Create:**
- `rust_core/src/protocol/streaming.rs` - Streaming protocol
- `rust_core/src/network/streaming.rs` - Streaming transport
- `flutter_app/lib/src/core/services/streaming_service.dart` - Flutter streaming client

**Dependencies:**
- Core sync working
- Network protocol stable
- Performance optimization needed

## 23. Selective Sync

### Overview

Allow users to choose which devices receive clipboard updates. Enable per-device sync settings and device groups.

### Design Considerations

**UI Components:**
- Device selection UI in settings
- Per-device sync toggle
- Device groups/tags
- Sync filters (text only, images only, etc.)

**Implementation Approach:**
1. Add `sync_enabled` flag to device storage
2. Update network broadcast to filter by device settings
3. Add device groups/tags to storage schema
4. Create UI for managing sync preferences

**Files to Create:**
- `rust_core/src/storage/device_storage.rs` - Add sync settings
- `flutter_app/lib/src/features/settings/device_sync_settings.dart` - UI component
- `flutter_app/lib/src/core/models/device_sync_preferences.dart` - Data model

**Dependencies:**
- Device management (#2)
- Settings UI
- Network broadcast logic

## 24. Team/Organization Support

### Overview

Shared clipboard groups for teams and organizations. Enable multiple users to share clipboard within a team.

### Design Considerations

**Data Model:**
- Team/Organization entities
- Role-based access (admin, member, viewer)
- Group management
- Invitation system

**Implementation Approach:**
1. Design team/organization database schema
2. Implement group management API
3. Add role-based access control
4. Update relay server for group messaging
5. Create team UI components

**Files to Create:**
- `rust_core/src/storage/team_storage.rs` - Team storage
- `rust_core/src/crypto/team_auth.rs` - Team authentication
- `relay_server/src/api/teams.rs` - Team API endpoints
- `flutter_app/lib/src/features/teams/` - Team UI

**Dependencies:**
- Core features complete
- Relay server
- Authentication system

## 25. Browser Extension

### Overview

Web-based clipboard access via browser extension. Enable clipboard sync from web browsers.

### Design Considerations

**Architecture:**
- Chrome extension (Manifest V3)
- Firefox extension (WebExtensions API)
- WebSocket connection to relay server
- Handle browser clipboard API limitations

**Implementation Approach:**
1. Design extension architecture
2. Implement Chrome extension
3. Implement Firefox extension
4. Add WebSocket client for relay connection
5. Handle browser clipboard restrictions

**Files to Create:**
- `browser_extension/chrome/` - Chrome extension
- `browser_extension/firefox/` - Firefox extension
- `browser_extension/shared/` - Shared code

**Dependencies:**
- Relay server
- WebSocket support
- Browser API knowledge

## 26. Conflict Resolution

### Overview

Handle simultaneous clipboard changes from multiple devices. Resolve conflicts when multiple devices change clipboard at the same time.

### Design Considerations

**Resolution Strategies:**
- Timestamp-based (last write wins)
- User preference (device priority)
- Manual resolution UI
- Conflict notifications

**Implementation Approach:**
1. Design conflict detection algorithm
2. Implement timestamp-based resolution
3. Add user preference for resolution strategy
4. Create conflict notification UI
5. Allow manual conflict resolution

**Files to Create:**
- `rust_core/src/protocol/conflict.rs` - Conflict resolution
- `flutter_app/lib/src/core/services/conflict_service.dart` - Conflict handling
- `flutter_app/lib/src/features/conflicts/` - Conflict UI

**Dependencies:**
- Core sync working
- Network events
- UI components

## 27. Content Compression

### Overview

Reduce bandwidth for large clipboard content. Compress clipboard data before transmission.

### Design Considerations

**Compression Library:**
- Use zstd (fast, good compression ratio)
- Compress only large content (>1MB)
- Add compression settings
- Optimize for mobile networks

**Implementation Approach:**
1. Add zstd dependency
2. Implement compression for large content
3. Add compression settings to UI
4. Test compression ratios
5. Optimize for mobile networks

**Files to Create:**
- `rust_core/src/protocol/compression.rs` - Compression utilities
- `rust_core/Cargo.toml` - Add zstd dependency
- `flutter_app/lib/src/features/settings/compression_settings.dart` - UI

**Dependencies:**
- Message protocol
- Settings system

## Implementation Priority

These enhancements are not in the current MVP scope. Priority would be:

1. **Content Compression** (#27) - Most practical, immediate benefit
2. **Selective Sync** (#23) - User-requested feature
3. **Conflict Resolution** (#26) - Important for multi-device scenarios
4. **Clipboard Streaming** (#22) - Performance optimization
5. **Browser Extension** (#25) - Expands platform support
6. **Team/Organization Support** (#24) - Enterprise feature, most complex

## Notes

- All future enhancements depend on core MVP features being stable
- Some features may require relay server updates
- UI components should follow existing design patterns
- Consider backward compatibility when implementing
