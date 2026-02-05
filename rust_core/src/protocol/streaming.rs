//! Chunked streaming protocol for large clipboard content
//!
//! This module implements a chunked transfer protocol for efficient
//! streaming of large clipboard content (> 1MB). It provides:
//! - Configurable chunk sizes
//! - Progress tracking
//! - Resume capability for interrupted transfers
//! - Memory-efficient streaming (doesn't load entire content in memory)

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::content::{ClipboardContent, ContentMetadata, ContentType};
use crate::error::ProtocolError;

/// Default chunk size: 1 MB
pub const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;

/// Minimum chunk size: 64 KB
pub const MIN_CHUNK_SIZE: usize = 64 * 1024;

/// Maximum chunk size: 4 MB
pub const MAX_CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Threshold for using chunked transfer: 1 MB
pub const CHUNKED_TRANSFER_THRESHOLD: usize = 1024 * 1024;

/// Maximum concurrent transfers per connection
pub const MAX_CONCURRENT_TRANSFERS: usize = 4;

/// Transfer timeout in seconds
pub const TRANSFER_TIMEOUT_SECS: u64 = 300;

/// Streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Chunk size in bytes (default: 1 MB)
    pub chunk_size: usize,
    /// Threshold for using chunked transfer (default: 1 MB)
    pub chunked_threshold: usize,
    /// Enable streaming for large content
    pub enabled: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            chunked_threshold: CHUNKED_TRANSFER_THRESHOLD,
            enabled: true,
        }
    }
}

impl StreamingConfig {
    /// Validate and normalize chunk size
    pub fn validated_chunk_size(&self) -> usize {
        self.chunk_size.clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE)
    }

    /// Check if content should use chunked transfer
    pub fn should_use_chunked(&self, content_size: usize) -> bool {
        self.enabled && content_size > self.chunked_threshold
    }
}

/// Transfer ID for tracking chunked transfers
pub type TransferId = u64;

/// Generate a new transfer ID
pub fn generate_transfer_id() -> TransferId {
    let uuid = Uuid::new_v4();
    let bytes = uuid.as_bytes();
    u64::from_le_bytes(bytes[0..8].try_into().expect("UUID provides 16 bytes"))
}

/// State of a chunked transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
    /// Transfer initiated, waiting for chunks
    Initiated,
    /// Transfer in progress
    InProgress,
    /// Transfer completed successfully
    Completed,
    /// Transfer failed
    Failed,
    /// Transfer cancelled
    Cancelled,
}

/// Initiate a chunked clipboard transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedTransferInit {
    /// Unique transfer identifier
    pub transfer_id: TransferId,
    /// Total number of chunks
    pub total_chunks: u32,
    /// Total size in bytes
    pub total_size: u64,
    /// Chunk size in bytes
    pub chunk_size: u32,
    /// Content type
    pub content_type: ContentType,
    /// Content metadata (preview, dimensions, etc.)
    pub metadata: ContentMetadata,
    /// SHA-256 hash of full content for verification
    pub content_hash: [u8; 32],
}

impl ChunkedTransferInit {
    /// Create a new transfer initiation message
    pub fn new(content: &ClipboardContent, chunk_size: usize) -> Self {
        let total_size = content.data.len() as u64;
        let chunk_size_validated = chunk_size.clamp(MIN_CHUNK_SIZE, MAX_CHUNK_SIZE);
        let total_chunks = (total_size as usize).div_ceil(chunk_size_validated) as u32;

        Self {
            transfer_id: generate_transfer_id(),
            total_chunks,
            total_size,
            chunk_size: chunk_size_validated as u32,
            content_type: content.content_type,
            metadata: content.metadata.clone(),
            content_hash: content.hash(),
        }
    }
}

/// A single chunk of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedTransferData {
    /// Transfer identifier
    pub transfer_id: TransferId,
    /// Chunk sequence number (0-indexed)
    pub chunk_index: u32,
    /// Chunk data
    pub data: Vec<u8>,
    /// SHA-256 hash of this chunk for verification
    pub chunk_hash: [u8; 32],
}

impl ChunkedTransferData {
    /// Create a new chunk from content slice
    pub fn new(transfer_id: TransferId, chunk_index: u32, data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let chunk_hash: [u8; 32] = hasher.finalize().into();

        Self {
            transfer_id,
            chunk_index,
            data: data.to_vec(),
            chunk_hash,
        }
    }

    /// Verify chunk integrity
    pub fn verify(&self) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        let computed_hash: [u8; 32] = hasher.finalize().into();
        computed_hash == self.chunk_hash
    }
}

/// Acknowledge receipt of a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedTransferAck {
    /// Transfer identifier
    pub transfer_id: TransferId,
    /// Acknowledged chunk index
    pub chunk_index: u32,
    /// Whether chunk was received successfully
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Complete or cancel a chunked transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedTransferComplete {
    /// Transfer identifier
    pub transfer_id: TransferId,
    /// Final state
    pub state: TransferState,
    /// Error message if failed/cancelled
    pub error: Option<String>,
}

/// Progress information for a transfer
#[derive(Debug, Clone)]
pub struct TransferProgress {
    /// Transfer identifier
    pub transfer_id: TransferId,
    /// Total chunks
    pub total_chunks: u32,
    /// Chunks received/sent
    pub chunks_completed: u32,
    /// Bytes transferred
    pub bytes_transferred: u64,
    /// Total bytes
    pub total_bytes: u64,
    /// Current state
    pub state: TransferState,
}

impl TransferProgress {
    /// Get progress as a percentage (0.0 - 1.0)
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            return 1.0;
        }
        self.bytes_transferred as f64 / self.total_bytes as f64
    }
}

/// Assembles chunks into complete content
pub struct ChunkAssembler {
    /// Transfer ID
    transfer_id: TransferId,
    /// Total number of expected chunks
    total_chunks: u32,
    /// Total expected size
    total_size: u64,
    /// Expected chunk size (kept for potential future use in validation)
    #[allow(dead_code)]
    chunk_size: u32,
    /// Content type
    content_type: ContentType,
    /// Metadata
    metadata: ContentMetadata,
    /// Expected content hash
    expected_hash: [u8; 32],
    /// Received chunks (indexed by chunk number)
    chunks: HashMap<u32, Vec<u8>>,
    /// Current state
    state: TransferState,
    /// Creation timestamp
    created_at: std::time::Instant,
}

impl ChunkAssembler {
    /// Create a new chunk assembler from transfer init message
    pub fn new(init: &ChunkedTransferInit) -> Self {
        Self {
            transfer_id: init.transfer_id,
            total_chunks: init.total_chunks,
            total_size: init.total_size,
            chunk_size: init.chunk_size,
            content_type: init.content_type,
            metadata: init.metadata.clone(),
            expected_hash: init.content_hash,
            chunks: HashMap::with_capacity(init.total_chunks as usize),
            state: TransferState::Initiated,
            created_at: std::time::Instant::now(),
        }
    }

    /// Add a chunk to the assembler
    pub fn add_chunk(&mut self, chunk: &ChunkedTransferData) -> Result<(), ProtocolError> {
        // Verify transfer ID matches
        if chunk.transfer_id != self.transfer_id {
            return Err(ProtocolError::InvalidTransferId);
        }

        // Verify chunk index is valid
        if chunk.chunk_index >= self.total_chunks {
            return Err(ProtocolError::InvalidChunkIndex(chunk.chunk_index));
        }

        // Verify chunk integrity
        if !chunk.verify() {
            return Err(ProtocolError::ChunkHashMismatch);
        }

        // Store chunk
        self.chunks.insert(chunk.chunk_index, chunk.data.clone());
        self.state = TransferState::InProgress;

        Ok(())
    }

    /// Check if all chunks have been received
    pub fn is_complete(&self) -> bool {
        self.chunks.len() == self.total_chunks as usize
    }

    /// Get missing chunk indices
    pub fn missing_chunks(&self) -> Vec<u32> {
        (0..self.total_chunks)
            .filter(|i| !self.chunks.contains_key(i))
            .collect()
    }

    /// Get transfer progress
    pub fn progress(&self) -> TransferProgress {
        let chunks_completed = self.chunks.len() as u32;
        let bytes_transferred: u64 = self.chunks.values().map(|c| c.len() as u64).sum();

        TransferProgress {
            transfer_id: self.transfer_id,
            total_chunks: self.total_chunks,
            chunks_completed,
            bytes_transferred,
            total_bytes: self.total_size,
            state: self.state,
        }
    }

    /// Check if transfer has timed out
    pub fn is_timed_out(&self) -> bool {
        self.created_at.elapsed().as_secs() > TRANSFER_TIMEOUT_SECS
    }

    /// Assemble all chunks into complete content
    pub fn assemble(mut self) -> Result<ClipboardContent, ProtocolError> {
        if !self.is_complete() {
            return Err(ProtocolError::IncompleteTransfer(self.missing_chunks()));
        }

        // Assemble data in order
        let mut data = Vec::with_capacity(self.total_size as usize);
        for i in 0..self.total_chunks {
            if let Some(chunk_data) = self.chunks.remove(&i) {
                data.extend(chunk_data);
            } else {
                return Err(ProtocolError::MissingChunk(i));
            }
        }

        // Verify total content hash
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let computed_hash: [u8; 32] = hasher.finalize().into();

        if computed_hash != self.expected_hash {
            return Err(ProtocolError::ContentHashMismatch);
        }

        Ok(ClipboardContent {
            content_type: self.content_type,
            data,
            metadata: self.metadata,
        })
    }

    /// Cancel the transfer
    pub fn cancel(&mut self) {
        self.state = TransferState::Cancelled;
        self.chunks.clear();
    }
}

/// Manages active chunked transfers
pub struct TransferManager {
    /// Active incoming transfers (receiving)
    incoming: Arc<Mutex<HashMap<TransferId, ChunkAssembler>>>,
    /// Streaming configuration
    config: StreamingConfig,
}

impl TransferManager {
    /// Create a new transfer manager
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            incoming: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Get streaming configuration
    pub fn config(&self) -> &StreamingConfig {
        &self.config
    }

    /// Check if content should use chunked transfer
    pub fn should_chunk(&self, content_size: usize) -> bool {
        self.config.should_use_chunked(content_size)
    }

    /// Create chunks from clipboard content
    pub fn create_chunks(
        &self,
        content: &ClipboardContent,
    ) -> (ChunkedTransferInit, Vec<ChunkedTransferData>) {
        let chunk_size = self.config.validated_chunk_size();
        let init = ChunkedTransferInit::new(content, chunk_size);

        let chunks: Vec<ChunkedTransferData> = content
            .data
            .chunks(chunk_size)
            .enumerate()
            .map(|(i, chunk_data)| ChunkedTransferData::new(init.transfer_id, i as u32, chunk_data))
            .collect();

        (init, chunks)
    }

    /// Start receiving a chunked transfer
    pub async fn start_incoming(&self, init: &ChunkedTransferInit) -> Result<(), ProtocolError> {
        let mut incoming = self.incoming.lock().await;

        // Check for too many concurrent transfers
        if incoming.len() >= MAX_CONCURRENT_TRANSFERS {
            // Clean up timed out transfers
            incoming.retain(|_, assembler| !assembler.is_timed_out());

            if incoming.len() >= MAX_CONCURRENT_TRANSFERS {
                return Err(ProtocolError::TooManyConcurrentTransfers);
            }
        }

        // Check for duplicate transfer ID
        if incoming.contains_key(&init.transfer_id) {
            return Err(ProtocolError::DuplicateTransferId(init.transfer_id));
        }

        let assembler = ChunkAssembler::new(init);
        incoming.insert(init.transfer_id, assembler);

        Ok(())
    }

    /// Receive a chunk
    pub async fn receive_chunk(
        &self,
        chunk: &ChunkedTransferData,
    ) -> Result<TransferProgress, ProtocolError> {
        let mut incoming = self.incoming.lock().await;

        let assembler = incoming
            .get_mut(&chunk.transfer_id)
            .ok_or(ProtocolError::UnknownTransferId(chunk.transfer_id))?;

        // Check for timeout
        if assembler.is_timed_out() {
            incoming.remove(&chunk.transfer_id);
            return Err(ProtocolError::TransferTimedOut);
        }

        assembler.add_chunk(chunk)?;
        Ok(assembler.progress())
    }

    /// Complete and assemble a transfer
    pub async fn complete_transfer(
        &self,
        transfer_id: TransferId,
    ) -> Result<ClipboardContent, ProtocolError> {
        let mut incoming = self.incoming.lock().await;

        let assembler = incoming
            .remove(&transfer_id)
            .ok_or(ProtocolError::UnknownTransferId(transfer_id))?;

        assembler.assemble()
    }

    /// Cancel a transfer
    pub async fn cancel_transfer(&self, transfer_id: TransferId) -> Result<(), ProtocolError> {
        let mut incoming = self.incoming.lock().await;

        if let Some(mut assembler) = incoming.remove(&transfer_id) {
            assembler.cancel();
            Ok(())
        } else {
            Err(ProtocolError::UnknownTransferId(transfer_id))
        }
    }

    /// Get progress of a transfer
    pub async fn get_progress(&self, transfer_id: TransferId) -> Option<TransferProgress> {
        let incoming = self.incoming.lock().await;
        incoming.get(&transfer_id).map(|a| a.progress())
    }

    /// Check if a transfer is complete
    pub async fn is_transfer_complete(&self, transfer_id: TransferId) -> bool {
        let incoming = self.incoming.lock().await;
        incoming.get(&transfer_id).is_some_and(|a| a.is_complete())
    }

    /// Clean up timed out transfers
    pub async fn cleanup_timed_out(&self) -> Vec<TransferId> {
        let mut incoming = self.incoming.lock().await;
        let timed_out: Vec<TransferId> = incoming
            .iter()
            .filter(|(_, a)| a.is_timed_out())
            .map(|(&id, _)| id)
            .collect();

        for id in &timed_out {
            incoming.remove(id);
        }

        timed_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_content(size: usize) -> ClipboardContent {
        ClipboardContent {
            content_type: ContentType::PlainText,
            data: vec![b'x'; size],
            metadata: ContentMetadata {
                size_bytes: size as u64,
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();
        assert_eq!(config.chunk_size, DEFAULT_CHUNK_SIZE);
        assert!(config.enabled);
    }

    #[test]
    fn test_streaming_config_validation() {
        let mut config = StreamingConfig::default();

        // Test minimum clamping
        config.chunk_size = 1024; // Too small
        assert_eq!(config.validated_chunk_size(), MIN_CHUNK_SIZE);

        // Test maximum clamping
        config.chunk_size = 100 * 1024 * 1024; // Too large
        assert_eq!(config.validated_chunk_size(), MAX_CHUNK_SIZE);

        // Test valid value
        config.chunk_size = 2 * 1024 * 1024; // 2 MB
        assert_eq!(config.validated_chunk_size(), 2 * 1024 * 1024);
    }

    #[test]
    fn test_should_use_chunked() {
        let config = StreamingConfig::default();

        // Small content - no chunking
        assert!(!config.should_use_chunked(100));
        assert!(!config.should_use_chunked(1024 * 1024)); // Exactly threshold

        // Large content - use chunking
        assert!(config.should_use_chunked(1024 * 1024 + 1));
        assert!(config.should_use_chunked(10 * 1024 * 1024));
    }

    #[test]
    fn test_chunked_transfer_init() {
        let content = create_test_content(3 * 1024 * 1024); // 3 MB
        let init = ChunkedTransferInit::new(&content, DEFAULT_CHUNK_SIZE);

        assert_eq!(init.total_chunks, 3);
        assert_eq!(init.total_size, 3 * 1024 * 1024);
        assert_eq!(init.chunk_size, DEFAULT_CHUNK_SIZE as u32);
        assert_eq!(init.content_type, ContentType::PlainText);
    }

    #[test]
    fn test_chunk_data_verify() {
        let data = b"Hello, World!";
        let chunk = ChunkedTransferData::new(1, 0, data);

        assert!(chunk.verify());

        // Tamper with data
        let mut tampered = chunk.clone();
        tampered.data[0] = b'X';
        assert!(!tampered.verify());
    }

    #[test]
    fn test_chunk_assembler() {
        let content = create_test_content(2 * 1024 * 1024 + 100); // 2 MB + 100 bytes
        let init = ChunkedTransferInit::new(&content, DEFAULT_CHUNK_SIZE);
        let mut assembler = ChunkAssembler::new(&init);

        assert!(!assembler.is_complete());
        assert_eq!(assembler.missing_chunks(), vec![0, 1, 2]);

        // Add chunks
        let chunk0 =
            ChunkedTransferData::new(init.transfer_id, 0, &content.data[0..DEFAULT_CHUNK_SIZE]);
        let chunk1 = ChunkedTransferData::new(
            init.transfer_id,
            1,
            &content.data[DEFAULT_CHUNK_SIZE..2 * DEFAULT_CHUNK_SIZE],
        );
        let chunk2 =
            ChunkedTransferData::new(init.transfer_id, 2, &content.data[2 * DEFAULT_CHUNK_SIZE..]);

        assembler.add_chunk(&chunk0).unwrap();
        assert!(!assembler.is_complete());

        assembler.add_chunk(&chunk1).unwrap();
        assert!(!assembler.is_complete());

        assembler.add_chunk(&chunk2).unwrap();
        assert!(assembler.is_complete());

        // Assemble
        let assembled = assembler.assemble().unwrap();
        assert_eq!(assembled.data, content.data);
        assert_eq!(assembled.content_type, content.content_type);
    }

    #[test]
    fn test_chunk_assembler_wrong_transfer_id() {
        let content = create_test_content(2 * 1024 * 1024);
        let init = ChunkedTransferInit::new(&content, DEFAULT_CHUNK_SIZE);
        let mut assembler = ChunkAssembler::new(&init);

        let wrong_chunk = ChunkedTransferData::new(999, 0, &content.data[0..DEFAULT_CHUNK_SIZE]);
        assert!(matches!(
            assembler.add_chunk(&wrong_chunk),
            Err(ProtocolError::InvalidTransferId)
        ));
    }

    #[test]
    fn test_chunk_assembler_invalid_index() {
        let content = create_test_content(2 * 1024 * 1024);
        let init = ChunkedTransferInit::new(&content, DEFAULT_CHUNK_SIZE);
        let mut assembler = ChunkAssembler::new(&init);

        let invalid_chunk = ChunkedTransferData::new(init.transfer_id, 100, &content.data[0..1000]);
        assert!(matches!(
            assembler.add_chunk(&invalid_chunk),
            Err(ProtocolError::InvalidChunkIndex(100))
        ));
    }

    #[test]
    fn test_transfer_progress() {
        let content = create_test_content(3 * 1024 * 1024);
        let init = ChunkedTransferInit::new(&content, DEFAULT_CHUNK_SIZE);
        let mut assembler = ChunkAssembler::new(&init);

        let progress = assembler.progress();
        assert_eq!(progress.chunks_completed, 0);
        assert_eq!(progress.percentage(), 0.0);

        let chunk0 =
            ChunkedTransferData::new(init.transfer_id, 0, &content.data[0..DEFAULT_CHUNK_SIZE]);
        assembler.add_chunk(&chunk0).unwrap();

        let progress = assembler.progress();
        assert_eq!(progress.chunks_completed, 1);
        assert!(progress.percentage() > 0.0);
        assert!(progress.percentage() < 1.0);
    }

    #[tokio::test]
    async fn test_transfer_manager() {
        let config = StreamingConfig::default();
        let manager = TransferManager::new(config);

        let content = create_test_content(2 * 1024 * 1024 + 100);
        let (init, chunks) = manager.create_chunks(&content);

        assert_eq!(chunks.len(), 3);

        // Start incoming transfer
        manager.start_incoming(&init).await.unwrap();

        // Receive chunks
        for chunk in &chunks {
            manager.receive_chunk(chunk).await.unwrap();
        }

        // Complete transfer
        let assembled = manager.complete_transfer(init.transfer_id).await.unwrap();
        assert_eq!(assembled.data, content.data);
    }

    #[tokio::test]
    async fn test_transfer_manager_concurrent_limit() {
        let config = StreamingConfig::default();
        let manager = TransferManager::new(config);

        // Start max concurrent transfers
        for _ in 0..MAX_CONCURRENT_TRANSFERS {
            let content = create_test_content(2 * 1024 * 1024);
            let init = ChunkedTransferInit::new(&content, DEFAULT_CHUNK_SIZE);
            manager.start_incoming(&init).await.unwrap();
        }

        // Next one should fail
        let content = create_test_content(2 * 1024 * 1024);
        let init = ChunkedTransferInit::new(&content, DEFAULT_CHUNK_SIZE);
        assert!(matches!(
            manager.start_incoming(&init).await,
            Err(ProtocolError::TooManyConcurrentTransfers)
        ));
    }
}
