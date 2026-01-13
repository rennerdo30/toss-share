//! Toss Core Library
//!
//! This library provides the core functionality for Toss, including:
//! - End-to-end encryption using X25519 and AES-256-GCM
//! - Device identity and pairing
//! - Clipboard operations
//! - P2P networking with mDNS discovery
//! - Relay server client

mod frb_generated; /* AUTO INJECTED BY flutter_rust_bridge. This line may not be accurate, and you can change it according to your needs. */

pub mod api;
pub mod clipboard;
pub mod crypto;
pub mod error;
pub mod network;
pub mod protocol;
pub mod storage;

pub use error::{CryptoError, NetworkError, ProtocolError, TossError};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Protocol version for wire format compatibility
pub const PROTOCOL_VERSION: u16 = 1;
