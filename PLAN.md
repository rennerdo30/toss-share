# Plan: Fix Windows App Startup Issue

## Problem
The app doesn't open at all on Windows after adding stdout logging. This is because Windows GUI applications (non-console apps) don't have stdout connected, and writing to it can cause issues or silent failures.

## Root Cause
In `rust_core/src/api/mod.rs` lines 134-138:
```rust
.with(
    tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true),
)
```

On Windows GUI apps, `std::io::stdout()` writes to nothing (or causes issues) because there's no console attached.

## Solution
Make stdout logging conditional - only enable it on non-Windows platforms, OR when running from a console.

### Option A: Platform-conditional stdout (Recommended)
Only add stdout layer on non-Windows platforms:

```rust
let registry = tracing_subscriber::registry()
    .with(env_filter)
    .with(
        tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false),
    );

#[cfg(not(target_os = "windows"))]
let registry = registry.with(
    tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true),
);

let _ = registry.try_init();
```

### Option B: Use stderr instead
stderr is sometimes more reliably available:
```rust
.with_writer(std::io::stderr)
```

### Option C: Check if console is attached (Windows-specific)
Use Windows API to check if a console is attached before adding stdout layer.

## Recommended Approach
**Option A** is the simplest and safest. It ensures:
- File logging always works on all platforms
- Console logging works on macOS/Linux when running from terminal
- Windows builds work without modification

## Files to Modify
| File | Change |
|------|--------|
| `rust_core/src/api/mod.rs` | Make stdout layer conditional with `#[cfg(not(target_os = "windows"))]` |

## Implementation Steps
1. Modify `init_toss()` to conditionally add stdout layer
2. Test that Rust code compiles
3. Build and verify on macOS
4. Commit and push
5. CI will verify Windows build works
