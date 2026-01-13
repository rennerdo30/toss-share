# Windows Platform Support

Windows-specific implementation details for Toss.

## Clipboard Format Handling

Windows supports multiple clipboard formats that need special handling:

- **CF_TEXT**: ANSI text (legacy)
- **CF_UNICODETEXT**: Unicode text (preferred)
- **CF_HDROP**: File list (HDROP)
- **CF_DIB**: Device-independent bitmap
- **CF_BITMAP**: Bitmap handle

**Implementation Status**: Format constants and structure created in `windows_formats.rs`. Full implementation requires Windows API integration.

**Files**:
- `rust_core/src/clipboard/windows_formats.rs`

**Priority**: Handle CF_UNICODETEXT for text, CF_HDROP for files, CF_DIB for images.

## Implementation Details

### Text Handling

Prefer CF_UNICODETEXT over CF_TEXT:

```rust
// Use CF_UNICODETEXT for text
let format = ClipboardFormat::UnicodeText;
```

### File Handling

Handle CF_HDROP for file lists:

```rust
// Parse HDROP format
let files = parse_hdrop(data);
```

### Image Handling

Support CF_DIB for images:

```rust
// Convert DIB to image format
let image = dib_to_image(data);
```

## System Integration

- **Credential Manager**: Secure storage for device keys
- **System Tray**: Taskbar integration
- **Notifications**: Toast notifications

## Next Steps

- [Platform Overview](overview.md) - Return to platform overview
- [Linux Implementation](linux.md) - Linux platform details
