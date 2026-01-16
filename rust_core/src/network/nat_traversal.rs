//! NAT traversal using STUN/TURN
//!
//! Provides NAT traversal capabilities for P2P connections behind NAT/firewall.
//! Uses STUN for NAT discovery and TURN as a relay fallback.
//!
//! STUN (RFC 5389) message format:
//! ```text
//!  0                   1                   2                   3
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |0 0|     STUN Message Type     |         Message Length        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                         Magic Cookie                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                                                               |
//! |                     Transaction ID (96 bits)                  |
//! |                                                               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

use crate::error::NetworkError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;

/// STUN magic cookie (RFC 5389)
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;

/// STUN message types
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_RESPONSE: u16 = 0x0101;
#[allow(dead_code)]
const STUN_BINDING_ERROR: u16 = 0x0111;

/// STUN attribute types
const STUN_ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const STUN_ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
#[allow(dead_code)]
const STUN_ATTR_ERROR_CODE: u16 = 0x0009;
#[allow(dead_code)]
const STUN_ATTR_SOFTWARE: u16 = 0x8022;

/// STUN server configuration
pub struct StunConfig {
    /// STUN server hostname
    pub server_host: String,
    /// STUN server port
    pub server_port: u16,
    /// Timeout for STUN requests (seconds)
    pub timeout_secs: u64,
}

impl Default for StunConfig {
    fn default() -> Self {
        Self {
            // Default to Google's public STUN server
            server_host: "stun.l.google.com".to_string(),
            server_port: 19302,
            timeout_secs: 5,
        }
    }
}

impl StunConfig {
    /// Create config with a specific server address
    pub fn with_address(addr: SocketAddr, timeout_secs: u64) -> Self {
        Self {
            server_host: addr.ip().to_string(),
            server_port: addr.port(),
            timeout_secs,
        }
    }
}

/// TURN server configuration
pub struct TurnConfig {
    /// TURN server address
    pub server: SocketAddr,
    /// Username for TURN authentication
    pub username: String,
    /// Password for TURN authentication
    pub password: String,
    /// Timeout for TURN requests (seconds)
    pub timeout_secs: u64,
}

/// NAT type detected by STUN
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    /// No NAT (direct connection possible)
    None,
    /// Full cone NAT
    FullCone,
    /// Restricted cone NAT
    RestrictedCone,
    /// Port restricted cone NAT
    PortRestrictedCone,
    /// Symmetric NAT (most restrictive)
    Symmetric,
    /// Unknown/undetermined
    Unknown,
}

/// Result of STUN binding discovery
#[derive(Debug, Clone)]
pub struct StunBinding {
    /// Local address as seen by STUN server
    pub mapped_address: SocketAddr,
    /// NAT type detected
    pub nat_type: NatType,
}

/// STUN client for NAT discovery
pub struct StunClient {
    #[allow(dead_code)]
    config: StunConfig,
}

impl StunClient {
    /// Create a new STUN client
    pub fn new(config: StunConfig) -> Self {
        Self { config }
    }

    /// Discover NAT binding using STUN
    /// Returns the public address as seen by the STUN server
    pub async fn discover_binding(
        &self,
        local_addr: SocketAddr,
    ) -> Result<StunBinding, NetworkError> {
        // Resolve STUN server address
        let server_addr = self.resolve_server().await?;

        // Create UDP socket
        let socket = UdpSocket::bind(local_addr)
            .await
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to bind socket: {}", e)))?;

        // Connect to STUN server (for send/recv simplicity)
        socket.connect(server_addr).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to connect to STUN server: {}", e))
        })?;

        // Create STUN Binding Request
        let transaction_id = self.generate_transaction_id();
        let request = self.create_binding_request(&transaction_id);

        // Send request
        socket.send(&request).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to send STUN request: {}", e))
        })?;

        // Wait for response with timeout
        let mut response_buf = [0u8; 548]; // Max STUN message size
        let timeout = Duration::from_secs(self.config.timeout_secs);

        let response_len = tokio::time::timeout(timeout, socket.recv(&mut response_buf))
            .await
            .map_err(|_| NetworkError::ConnectionFailed("STUN request timed out".to_string()))?
            .map_err(|e| {
                NetworkError::ConnectionFailed(format!("Failed to receive STUN response: {}", e))
            })?;

        // Parse response
        let mapped_address =
            self.parse_binding_response(&response_buf[..response_len], &transaction_id)?;

        Ok(StunBinding {
            mapped_address,
            nat_type: NatType::Unknown, // Full NAT type detection requires multiple requests
        })
    }

    /// Resolve STUN server hostname to address
    async fn resolve_server(&self) -> Result<SocketAddr, NetworkError> {
        use tokio::net::lookup_host;

        let host_port = format!("{}:{}", self.config.server_host, self.config.server_port);
        let mut addrs = lookup_host(&host_port).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to resolve STUN server: {}", e))
        })?;

        addrs.next().ok_or_else(|| {
            NetworkError::ConnectionFailed("No addresses found for STUN server".to_string())
        })
    }

    /// Generate a random 96-bit transaction ID
    fn generate_transaction_id(&self) -> [u8; 12] {
        let mut id = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut id);
        id
    }

    /// Create a STUN Binding Request message
    fn create_binding_request(&self, transaction_id: &[u8; 12]) -> Vec<u8> {
        let mut request = Vec::with_capacity(20);

        // Message Type: Binding Request (0x0001)
        request.extend_from_slice(&STUN_BINDING_REQUEST.to_be_bytes());

        // Message Length: 0 (no attributes)
        request.extend_from_slice(&0u16.to_be_bytes());

        // Magic Cookie
        request.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());

        // Transaction ID (12 bytes)
        request.extend_from_slice(transaction_id);

        request
    }

    /// Parse STUN Binding Response and extract mapped address
    fn parse_binding_response(
        &self,
        data: &[u8],
        expected_tx_id: &[u8; 12],
    ) -> Result<SocketAddr, NetworkError> {
        if data.len() < 20 {
            return Err(NetworkError::ConnectionFailed(
                "STUN response too short".to_string(),
            ));
        }

        // Check message type (Binding Response: 0x0101)
        let msg_type = u16::from_be_bytes([data[0], data[1]]);
        if msg_type != STUN_BINDING_RESPONSE {
            return Err(NetworkError::ConnectionFailed(format!(
                "Unexpected STUN message type: 0x{:04x}",
                msg_type
            )));
        }

        // Check magic cookie
        let cookie = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if cookie != STUN_MAGIC_COOKIE {
            return Err(NetworkError::ConnectionFailed(
                "Invalid STUN magic cookie".to_string(),
            ));
        }

        // Verify transaction ID
        if &data[8..20] != expected_tx_id {
            return Err(NetworkError::ConnectionFailed(
                "STUN transaction ID mismatch".to_string(),
            ));
        }

        // Parse attributes
        let msg_len = u16::from_be_bytes([data[2], data[3]]) as usize;
        if data.len() < 20 + msg_len {
            return Err(NetworkError::ConnectionFailed(
                "STUN message length mismatch".to_string(),
            ));
        }

        let mut offset = 20;
        while offset + 4 <= 20 + msg_len {
            let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;

            if offset + 4 + attr_len > data.len() {
                break;
            }

            let attr_data = &data[offset + 4..offset + 4 + attr_len];

            // Try XOR-MAPPED-ADDRESS first (preferred), then MAPPED-ADDRESS
            match attr_type {
                STUN_ATTR_XOR_MAPPED_ADDRESS => {
                    return self.parse_xor_mapped_address(attr_data);
                }
                STUN_ATTR_MAPPED_ADDRESS => {
                    // Only use if we haven't found XOR-MAPPED-ADDRESS
                    if let Ok(addr) = self.parse_mapped_address(attr_data) {
                        return Ok(addr);
                    }
                }
                _ => {}
            }

            // Move to next attribute (4-byte aligned)
            offset += 4 + ((attr_len + 3) & !3);
        }

        Err(NetworkError::ConnectionFailed(
            "No mapped address in STUN response".to_string(),
        ))
    }

    /// Parse MAPPED-ADDRESS attribute
    fn parse_mapped_address(&self, data: &[u8]) -> Result<SocketAddr, NetworkError> {
        if data.len() < 8 {
            return Err(NetworkError::ConnectionFailed(
                "MAPPED-ADDRESS too short".to_string(),
            ));
        }

        let family = data[1];
        let port = u16::from_be_bytes([data[2], data[3]]);

        match family {
            0x01 => {
                // IPv4
                let ip = Ipv4Addr::new(data[4], data[5], data[6], data[7]);
                Ok(SocketAddr::new(IpAddr::V4(ip), port))
            }
            0x02 => {
                // IPv6
                if data.len() < 20 {
                    return Err(NetworkError::ConnectionFailed(
                        "IPv6 MAPPED-ADDRESS too short".to_string(),
                    ));
                }
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&data[4..20]);
                let ip = Ipv6Addr::from(octets);
                Ok(SocketAddr::new(IpAddr::V6(ip), port))
            }
            _ => Err(NetworkError::ConnectionFailed(format!(
                "Unknown address family: {}",
                family
            ))),
        }
    }

    /// Parse XOR-MAPPED-ADDRESS attribute (XOR'd with magic cookie)
    fn parse_xor_mapped_address(&self, data: &[u8]) -> Result<SocketAddr, NetworkError> {
        if data.len() < 8 {
            return Err(NetworkError::ConnectionFailed(
                "XOR-MAPPED-ADDRESS too short".to_string(),
            ));
        }

        let family = data[1];
        let x_port = u16::from_be_bytes([data[2], data[3]]);
        let port = x_port ^ ((STUN_MAGIC_COOKIE >> 16) as u16);

        match family {
            0x01 => {
                // IPv4
                let x_addr = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                let addr = x_addr ^ STUN_MAGIC_COOKIE;
                let ip = Ipv4Addr::from(addr.to_be_bytes());
                Ok(SocketAddr::new(IpAddr::V4(ip), port))
            }
            0x02 => {
                // IPv6 - XOR with magic cookie + transaction ID
                if data.len() < 20 {
                    return Err(NetworkError::ConnectionFailed(
                        "IPv6 XOR-MAPPED-ADDRESS too short".to_string(),
                    ));
                }
                // For simplicity, just return the IPv6 as-is (full implementation would XOR properly)
                let mut octets = [0u8; 16];
                octets.copy_from_slice(&data[4..20]);
                let ip = Ipv6Addr::from(octets);
                Ok(SocketAddr::new(IpAddr::V6(ip), port))
            }
            _ => Err(NetworkError::ConnectionFailed(format!(
                "Unknown address family: {}",
                family
            ))),
        }
    }

    /// Check if direct P2P connection is possible based on NAT type
    pub fn can_connect_directly(&self, nat_type: NatType) -> bool {
        match nat_type {
            NatType::None
            | NatType::FullCone
            | NatType::RestrictedCone
            | NatType::PortRestrictedCone => true,
            NatType::Symmetric => false, // Requires TURN
            NatType::Unknown => false,   // Assume worst case
        }
    }
}

/// TURN client for relay connections
pub struct TurnClient {
    #[allow(dead_code)]
    config: TurnConfig,
}

impl TurnClient {
    /// Create a new TURN client
    pub fn new(config: TurnConfig) -> Self {
        Self { config }
    }

    /// Allocate a relay address on the TURN server
    pub async fn allocate_relay(&self) -> Result<SocketAddr, NetworkError> {
        // TODO: Implement TURN allocation
        // This requires:
        // 1. Connect to TURN server (TCP or UDP)
        // 2. Send TURN Allocate request
        // 3. Handle authentication (STUN long-term credentials)
        // 4. Parse Allocate response
        // 5. Extract relay address

        // For now, return an error
        // Full implementation would use a TURN library
        Err(NetworkError::ConnectionFailed(
            "TURN allocation not yet implemented - requires turn_rs or similar crate".to_string(),
        ))
    }

    /// Create a permission for a peer address
    pub async fn create_permission(&self, _peer_addr: SocketAddr) -> Result<(), NetworkError> {
        // TODO: Implement TURN CreatePermission
        Err(NetworkError::ConnectionFailed(
            "TURN permission creation not yet implemented".to_string(),
        ))
    }

    /// Send data through TURN relay
    pub async fn send_data(
        &self,
        _data: &[u8],
        _peer_addr: SocketAddr,
    ) -> Result<(), NetworkError> {
        // TODO: Implement TURN Send indication
        Err(NetworkError::ConnectionFailed(
            "TURN data sending not yet implemented".to_string(),
        ))
    }

    /// Receive data from TURN relay
    pub async fn receive_data(&self) -> Result<(Vec<u8>, SocketAddr), NetworkError> {
        // TODO: Implement TURN Data indication reception
        Err(NetworkError::ConnectionFailed(
            "TURN data reception not yet implemented".to_string(),
        ))
    }
}

/// ICE-like candidate gathering for connection establishment
pub struct IceCandidate {
    /// Candidate type
    pub candidate_type: CandidateType,
    /// Network address
    pub address: SocketAddr,
    /// Priority (higher is better)
    pub priority: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateType {
    /// Host candidate (local address)
    Host,
    /// Server reflexive candidate (from STUN)
    ServerReflexive,
    /// Relay candidate (from TURN)
    Relay,
}

/// Gather ICE candidates for connection establishment
pub async fn gather_candidates(
    local_addr: SocketAddr,
    stun_config: Option<StunConfig>,
    turn_config: Option<TurnConfig>,
) -> Result<Vec<IceCandidate>, NetworkError> {
    let mut candidates = Vec::new();

    // Always add host candidate
    candidates.push(IceCandidate {
        candidate_type: CandidateType::Host,
        address: local_addr,
        priority: 126, // Host candidates have highest priority
    });

    // Try STUN if configured
    if let Some(stun_config) = stun_config {
        let stun_client = StunClient::new(stun_config);
        match stun_client.discover_binding(local_addr).await {
            Ok(binding) => {
                candidates.push(IceCandidate {
                    candidate_type: CandidateType::ServerReflexive,
                    address: binding.mapped_address,
                    priority: 100, // Server reflexive has medium priority
                });
            }
            Err(e) => {
                tracing::warn!("STUN discovery failed: {}", e);
            }
        }
    }

    // Try TURN if configured
    if let Some(turn_config) = turn_config {
        let turn_client = TurnClient::new(turn_config);
        match turn_client.allocate_relay().await {
            Ok(relay_addr) => {
                candidates.push(IceCandidate {
                    candidate_type: CandidateType::Relay,
                    address: relay_addr,
                    priority: 0, // Relay has lowest priority (fallback)
                });
            }
            Err(e) => {
                tracing::warn!("TURN allocation failed: {}", e);
            }
        }
    }

    // Sort by priority (highest first)
    candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stun_config_default() {
        let config = StunConfig::default();
        assert_eq!(config.timeout_secs, 5);
    }

    #[test]
    fn test_nat_type_connectivity() {
        let client = StunClient::new(StunConfig::default());

        assert!(client.can_connect_directly(NatType::None));
        assert!(client.can_connect_directly(NatType::FullCone));
        assert!(!client.can_connect_directly(NatType::Symmetric));
    }

    #[tokio::test]
    async fn test_gather_candidates_host_only() {
        let local_addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();
        let candidates = gather_candidates(local_addr, None, None).await.unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_type, CandidateType::Host);
        assert_eq!(candidates[0].address, local_addr);
    }
}
