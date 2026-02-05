//! Compression support for clipboard content
//!
//! This module provides transparent compression/decompression for protocol messages
//! to optimize transfer speeds on slow networks.
//!
//! # Compression Strategy
//! - Uses zstd for high compression ratio with fast decompression
//! - Only compresses content above a configurable threshold (default 10KB)
//! - Maintains backwards compatibility with uncompressed messages
//! - Compression flag stored in the compressed payload itself

use crate::error::ProtocolError;
use std::io::{Read, Write};

/// Default compression threshold in bytes (10KB)
pub const DEFAULT_COMPRESSION_THRESHOLD: usize = 10 * 1024;

/// Minimum compression threshold (1KB)
pub const MIN_COMPRESSION_THRESHOLD: usize = 1024;

/// Maximum compression threshold (1MB)
pub const MAX_COMPRESSION_THRESHOLD: usize = 1024 * 1024;

/// Default zstd compression level (3 is a good balance of speed and ratio)
pub const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Magic bytes to identify compressed payload
/// Using a unique sequence unlikely to appear in serialized data
const COMPRESSION_MAGIC: &[u8; 4] = b"ZSTD";

/// Configuration for compression behavior
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Minimum size in bytes before compression is applied
    pub threshold: usize,
    /// Zstd compression level (1-22, higher = better compression but slower)
    pub level: i32,
    /// Whether compression is enabled
    pub enabled: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_COMPRESSION_THRESHOLD,
            level: DEFAULT_COMPRESSION_LEVEL,
            enabled: true,
        }
    }
}

impl CompressionConfig {
    /// Create a new compression config with the given threshold
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            threshold: threshold.clamp(MIN_COMPRESSION_THRESHOLD, MAX_COMPRESSION_THRESHOLD),
            ..Default::default()
        }
    }

    /// Create a disabled compression config
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Set the compression level
    pub fn with_level(mut self, level: i32) -> Self {
        self.level = level.clamp(1, 22);
        self
    }
}

/// Statistics about a compression operation
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    /// Original uncompressed size in bytes
    pub original_size: usize,
    /// Compressed size in bytes (same as original if not compressed)
    pub compressed_size: usize,
    /// Whether compression was applied
    pub was_compressed: bool,
}

impl CompressionStats {
    /// Calculate compression ratio (compressed_size / original_size)
    /// Returns 1.0 if not compressed or original_size is 0
    pub fn ratio(&self) -> f64 {
        if self.original_size == 0 || !self.was_compressed {
            1.0
        } else {
            self.compressed_size as f64 / self.original_size as f64
        }
    }

    /// Calculate space saved as a percentage
    /// Returns 0.0 if not compressed
    pub fn space_saved_percent(&self) -> f64 {
        if !self.was_compressed {
            0.0
        } else {
            (1.0 - self.ratio()) * 100.0
        }
    }
}

/// Compress payload if it exceeds the threshold
///
/// Returns the (possibly compressed) payload and compression statistics.
/// The compressed format is: [MAGIC: 4 bytes][compressed_data]
/// Uncompressed data is returned as-is without any prefix.
pub fn compress_payload(
    payload: &[u8],
    config: &CompressionConfig,
) -> Result<(Vec<u8>, CompressionStats), ProtocolError> {
    let original_size = payload.len();

    // Don't compress if disabled or below threshold
    if !config.enabled || original_size < config.threshold {
        return Ok((
            payload.to_vec(),
            CompressionStats {
                original_size,
                compressed_size: original_size,
                was_compressed: false,
            },
        ));
    }

    // Compress using zstd
    let compressed = zstd::encode_all(payload, config.level)
        .map_err(|e| ProtocolError::Serialization(format!("Compression failed: {}", e)))?;

    // Only use compressed data if it's actually smaller
    // Account for the magic bytes overhead
    let compressed_with_magic_size = COMPRESSION_MAGIC.len() + compressed.len();
    if compressed_with_magic_size >= original_size {
        return Ok((
            payload.to_vec(),
            CompressionStats {
                original_size,
                compressed_size: original_size,
                was_compressed: false,
            },
        ));
    }

    // Prepend magic bytes to indicate compressed data
    let mut result = Vec::with_capacity(compressed_with_magic_size);
    result.extend_from_slice(COMPRESSION_MAGIC);
    result.extend_from_slice(&compressed);

    let result_len = result.len();
    Ok((
        result,
        CompressionStats {
            original_size,
            compressed_size: result_len,
            was_compressed: true,
        },
    ))
}

/// Decompress payload if it was compressed
///
/// Automatically detects whether the payload is compressed by checking
/// for the magic bytes prefix. Returns the decompressed data and stats.
pub fn decompress_payload(payload: &[u8]) -> Result<(Vec<u8>, CompressionStats), ProtocolError> {
    // Check if payload starts with compression magic
    if payload.len() > COMPRESSION_MAGIC.len() && payload.starts_with(COMPRESSION_MAGIC) {
        let compressed_data = &payload[COMPRESSION_MAGIC.len()..];

        let decompressed = zstd::decode_all(compressed_data)
            .map_err(|e| ProtocolError::Deserialization(format!("Decompression failed: {}", e)))?;

        let stats = CompressionStats {
            original_size: decompressed.len(),
            compressed_size: payload.len(),
            was_compressed: true,
        };

        Ok((decompressed, stats))
    } else {
        // Not compressed, return as-is
        Ok((
            payload.to_vec(),
            CompressionStats {
                original_size: payload.len(),
                compressed_size: payload.len(),
                was_compressed: false,
            },
        ))
    }
}

/// Compress data using streaming for large payloads
///
/// This is useful when dealing with very large content where
/// we want to avoid loading everything into memory at once.
pub fn compress_streaming<R: Read, W: Write>(
    reader: R,
    writer: W,
    level: i32,
) -> Result<u64, ProtocolError> {
    let mut encoder = zstd::stream::Encoder::new(writer, level)
        .map_err(|e| ProtocolError::Serialization(format!("Failed to create compressor: {}", e)))?;

    let bytes_written =
        std::io::copy(&mut std::io::BufReader::new(reader), &mut encoder).map_err(|e| {
            ProtocolError::Serialization(format!("Compression streaming failed: {}", e))
        })?;

    encoder.finish().map_err(|e| {
        ProtocolError::Serialization(format!("Failed to finish compression: {}", e))
    })?;

    Ok(bytes_written)
}

/// Decompress data using streaming for large payloads
pub fn decompress_streaming<R: Read, W: Write>(
    reader: R,
    mut writer: W,
) -> Result<u64, ProtocolError> {
    let decoder = zstd::stream::Decoder::new(reader).map_err(|e| {
        ProtocolError::Deserialization(format!("Failed to create decompressor: {}", e))
    })?;

    let bytes_written =
        std::io::copy(&mut std::io::BufReader::new(decoder), &mut writer).map_err(|e| {
            ProtocolError::Deserialization(format!("Decompression streaming failed: {}", e))
        })?;

    Ok(bytes_written)
}

/// Check if payload appears to be compressed
pub fn is_compressed(payload: &[u8]) -> bool {
    payload.len() > COMPRESSION_MAGIC.len() && payload.starts_with(COMPRESSION_MAGIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.threshold, DEFAULT_COMPRESSION_THRESHOLD);
        assert_eq!(config.level, DEFAULT_COMPRESSION_LEVEL);
        assert!(config.enabled);
    }

    #[test]
    fn test_compression_config_with_threshold() {
        let config = CompressionConfig::with_threshold(5000);
        assert_eq!(config.threshold, 5000);

        // Test clamping to minimum
        let config = CompressionConfig::with_threshold(100);
        assert_eq!(config.threshold, MIN_COMPRESSION_THRESHOLD);

        // Test clamping to maximum
        let config = CompressionConfig::with_threshold(10_000_000);
        assert_eq!(config.threshold, MAX_COMPRESSION_THRESHOLD);
    }

    #[test]
    fn test_compression_disabled() {
        let config = CompressionConfig::disabled();
        assert!(!config.enabled);

        let data = vec![b'A'; 20000]; // 20KB of data
        let (result, stats) = compress_payload(&data, &config).unwrap();

        assert_eq!(result.len(), data.len());
        assert!(!stats.was_compressed);
    }

    #[test]
    fn test_no_compression_below_threshold() {
        let config = CompressionConfig::default();
        let data = vec![b'A'; 1000]; // 1KB, below default 10KB threshold

        let (result, stats) = compress_payload(&data, &config).unwrap();

        assert_eq!(result, data);
        assert!(!stats.was_compressed);
        assert_eq!(stats.original_size, 1000);
        assert_eq!(stats.compressed_size, 1000);
    }

    #[test]
    fn test_compression_above_threshold() {
        let config = CompressionConfig::with_threshold(1024);
        // Create highly compressible data (repeating pattern)
        let data = vec![b'A'; 20000]; // 20KB of repeating 'A'

        let (result, stats) = compress_payload(&data, &config).unwrap();

        assert!(stats.was_compressed);
        assert!(result.len() < data.len());
        assert!(result.starts_with(COMPRESSION_MAGIC));
        assert_eq!(stats.original_size, 20000);
        assert!(stats.compressed_size < stats.original_size);
    }

    #[test]
    fn test_compression_decompression_roundtrip() {
        let config = CompressionConfig::with_threshold(1024);
        let original_data =
            b"Hello, this is a test message that should be compressed! ".repeat(500);

        let (compressed, compress_stats) = compress_payload(&original_data, &config).unwrap();
        assert!(compress_stats.was_compressed);

        let (decompressed, decompress_stats) = decompress_payload(&compressed).unwrap();
        assert!(decompress_stats.was_compressed);
        assert_eq!(decompressed, original_data);
    }

    #[test]
    fn test_decompress_uncompressed_data() {
        let data = b"This is uncompressed data";
        let (result, stats) = decompress_payload(data).unwrap();

        assert_eq!(result, data);
        assert!(!stats.was_compressed);
    }

    #[test]
    fn test_compression_stats_ratio() {
        let stats = CompressionStats {
            original_size: 1000,
            compressed_size: 250,
            was_compressed: true,
        };
        assert!((stats.ratio() - 0.25).abs() < 0.001);
        assert!((stats.space_saved_percent() - 75.0).abs() < 0.1);

        // Test uncompressed stats
        let uncompressed_stats = CompressionStats {
            original_size: 1000,
            compressed_size: 1000,
            was_compressed: false,
        };
        assert!((uncompressed_stats.ratio() - 1.0).abs() < 0.001);
        assert!((uncompressed_stats.space_saved_percent() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_incompressible_data_not_compressed() {
        let config = CompressionConfig::with_threshold(100);
        // Random-ish data that doesn't compress well
        let data: Vec<u8> = (0..500).map(|i| (i * 17 + 13) as u8).collect();

        let (result, stats) = compress_payload(&data, &config).unwrap();

        // If compression doesn't help, original data should be returned
        // (either compressed is larger, or compression ratio is negligible)
        if stats.was_compressed {
            // If it was compressed, it should be smaller
            assert!(result.len() < data.len());
        } else {
            // If not compressed, it should be identical
            assert_eq!(result, data);
        }
    }

    #[test]
    fn test_is_compressed() {
        // Compressed data starts with magic
        let mut compressed = COMPRESSION_MAGIC.to_vec();
        compressed.extend_from_slice(b"some compressed data");
        assert!(is_compressed(&compressed));

        // Uncompressed data doesn't start with magic
        let uncompressed = b"regular data";
        assert!(!is_compressed(uncompressed));

        // Empty data is not compressed
        assert!(!is_compressed(&[]));

        // Data shorter than magic is not compressed
        assert!(!is_compressed(b"ZST"));
    }

    #[test]
    fn test_compression_with_different_levels() {
        let data = b"Test data for compression ".repeat(1000);

        for level in [1, 3, 9, 15] {
            let config = CompressionConfig::with_threshold(1024).with_level(level);
            let (compressed, stats) = compress_payload(&data, &config).unwrap();

            assert!(stats.was_compressed);

            let (decompressed, _) = decompress_payload(&compressed).unwrap();
            assert_eq!(decompressed, data);
        }
    }

    #[test]
    fn test_streaming_compression() {
        let data = b"Streaming test data ".repeat(1000);
        let mut compressed = Vec::new();

        compress_streaming(data.as_slice(), &mut compressed, DEFAULT_COMPRESSION_LEVEL).unwrap();

        let mut decompressed = Vec::new();
        decompress_streaming(compressed.as_slice(), &mut decompressed).unwrap();

        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_empty_payload() {
        let config = CompressionConfig::default();
        let (result, stats) = compress_payload(&[], &config).unwrap();

        assert!(result.is_empty());
        assert!(!stats.was_compressed);
        assert_eq!(stats.original_size, 0);
    }

    #[test]
    fn test_large_payload_compression() {
        let config = CompressionConfig::with_threshold(1024);
        // 5MB of compressible data
        let data = vec![b'X'; 5 * 1024 * 1024];

        let (compressed, stats) = compress_payload(&data, &config).unwrap();

        assert!(stats.was_compressed);
        assert!(stats.ratio() < 0.1); // Should compress very well (< 10% of original)

        let (decompressed, _) = decompress_payload(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_various_content_types() {
        let config = CompressionConfig::with_threshold(1024);

        // Text content (should compress well)
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(500);
        let (_, text_stats) = compress_payload(text.as_bytes(), &config).unwrap();
        assert!(text_stats.was_compressed);
        assert!(text_stats.ratio() < 0.5);

        // JSON-like content (should compress well)
        let json = r#"{"key": "value", "number": 12345, "array": [1, 2, 3]}"#.repeat(200);
        let (compressed_json, json_stats) = compress_payload(json.as_bytes(), &config).unwrap();
        assert!(json_stats.was_compressed);
        let (decompressed_json, _) = decompress_payload(&compressed_json).unwrap();
        assert_eq!(decompressed_json, json.as_bytes());
    }
}
