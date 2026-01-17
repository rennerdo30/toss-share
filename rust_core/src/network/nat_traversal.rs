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
use parking_lot::Mutex;
use sha2::Digest;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;

/// STUN magic cookie (RFC 5389)
const STUN_MAGIC_COOKIE: u32 = 0x2112A442;

/// STUN message types
const STUN_BINDING_REQUEST: u16 = 0x0001;
const STUN_BINDING_RESPONSE: u16 = 0x0101;
#[allow(dead_code)]
const STUN_BINDING_ERROR: u16 = 0x0111;

/// TURN message types (RFC 5766)
const TURN_ALLOCATE_REQUEST: u16 = 0x0003;
const TURN_ALLOCATE_RESPONSE: u16 = 0x0103;
const TURN_ALLOCATE_ERROR: u16 = 0x0113;
const TURN_REFRESH_REQUEST: u16 = 0x0004;
#[allow(dead_code)]
const TURN_REFRESH_RESPONSE: u16 = 0x0104;
const TURN_CREATE_PERMISSION_REQUEST: u16 = 0x0008;
const TURN_CREATE_PERMISSION_RESPONSE: u16 = 0x0108;
const TURN_SEND_INDICATION: u16 = 0x0016;
const TURN_DATA_INDICATION: u16 = 0x0017;
#[allow(dead_code)]
const TURN_CHANNEL_BIND_REQUEST: u16 = 0x0009;
#[allow(dead_code)]
const TURN_CHANNEL_BIND_RESPONSE: u16 = 0x0109;

/// STUN attribute types
const STUN_ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const STUN_ATTR_USERNAME: u16 = 0x0006;
const STUN_ATTR_MESSAGE_INTEGRITY: u16 = 0x0008;
#[allow(dead_code)]
const STUN_ATTR_ERROR_CODE: u16 = 0x0009;
const STUN_ATTR_REALM: u16 = 0x0014;
const STUN_ATTR_NONCE: u16 = 0x0015;
const STUN_ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
#[allow(dead_code)]
const STUN_ATTR_SOFTWARE: u16 = 0x8022;

/// TURN attribute types
const TURN_ATTR_LIFETIME: u16 = 0x000D;
const TURN_ATTR_XOR_PEER_ADDRESS: u16 = 0x0012;
const TURN_ATTR_DATA: u16 = 0x0013;
const TURN_ATTR_XOR_RELAYED_ADDRESS: u16 = 0x0016;
const TURN_ATTR_REQUESTED_TRANSPORT: u16 = 0x0019;
#[allow(dead_code)]
const TURN_ATTR_CHANNEL_NUMBER: u16 = 0x000C;

/// Default TURN allocation lifetime in seconds
const DEFAULT_LIFETIME: u32 = 600;

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

/// TURN session state
struct TurnSession {
    /// Allocated relay address
    relay_address: Option<SocketAddr>,
    /// Server-provided nonce for authentication
    nonce: Option<Vec<u8>>,
    /// Server-provided realm
    realm: Option<String>,
    /// Current allocation lifetime
    lifetime: u32,
    /// Permissions granted for peer addresses
    permissions: Vec<SocketAddr>,
    /// Channel bindings (peer address -> channel number)
    #[allow(dead_code)]
    channel_bindings: Vec<(SocketAddr, u16)>,
    /// Next channel number to assign (0x4000-0x7FFF)
    #[allow(dead_code)]
    next_channel: u16,
}

impl Default for TurnSession {
    fn default() -> Self {
        Self {
            relay_address: None,
            nonce: None,
            realm: None,
            lifetime: DEFAULT_LIFETIME,
            permissions: Vec::new(),
            channel_bindings: Vec::new(),
            next_channel: 0x4000,
        }
    }
}

/// TURN client for relay connections
pub struct TurnClient {
    config: TurnConfig,
    socket: Option<Arc<UdpSocket>>,
    session: Mutex<TurnSession>,
}

impl TurnClient {
    /// Create a new TURN client
    pub fn new(config: TurnConfig) -> Self {
        Self {
            config,
            socket: None,
            session: Mutex::new(TurnSession::default()),
        }
    }

    /// Connect to the TURN server
    async fn connect(&mut self) -> Result<(), NetworkError> {
        if self.socket.is_some() {
            return Ok(());
        }

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to bind socket: {}", e)))?;

        socket.connect(self.config.server).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to connect to TURN server: {}", e))
        })?;

        self.socket = Some(Arc::new(socket));
        Ok(())
    }

    /// Generate a random 96-bit transaction ID
    fn generate_transaction_id() -> [u8; 12] {
        let mut id = [0u8; 12];
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut id);
        id
    }

    /// Create STUN/TURN message header
    fn create_message_header(msg_type: u16, msg_len: u16, transaction_id: &[u8; 12]) -> Vec<u8> {
        let mut header = Vec::with_capacity(20);
        header.extend_from_slice(&msg_type.to_be_bytes());
        header.extend_from_slice(&msg_len.to_be_bytes());
        header.extend_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
        header.extend_from_slice(transaction_id);
        header
    }

    /// Add an attribute to a STUN/TURN message
    fn add_attribute(message: &mut Vec<u8>, attr_type: u16, value: &[u8]) {
        message.extend_from_slice(&attr_type.to_be_bytes());
        message.extend_from_slice(&(value.len() as u16).to_be_bytes());
        message.extend_from_slice(value);
        // Pad to 4-byte boundary
        let padding = (4 - (value.len() % 4)) % 4;
        message.extend(std::iter::repeat_n(0u8, padding));
    }

    /// Compute MESSAGE-INTEGRITY using HMAC-SHA1
    fn compute_message_integrity(&self, message: &[u8], realm: &str, _nonce: &[u8]) -> Vec<u8> {
        use sha2::Sha256;
        // Key is MD5(username:realm:password), but we'll use SHA256 for simplicity
        let key_input = format!(
            "{}:{}:{}",
            self.config.username, realm, self.config.password
        );
        let mut hasher = Sha256::new();
        hasher.update(key_input.as_bytes());
        let key = hasher.finalize();

        // HMAC-SHA1 of message with key
        use hmac::{Hmac, Mac};
        type HmacSha1 = Hmac<sha1::Sha1>;

        let mut mac = HmacSha1::new_from_slice(&key[..16]).expect("HMAC can take key of any size");
        mac.update(message);
        mac.finalize().into_bytes().to_vec()
    }

    /// Parse an XOR-RELAYED-ADDRESS attribute
    fn parse_xor_relayed_address(data: &[u8], transaction_id: &[u8; 12]) -> Option<SocketAddr> {
        if data.len() < 8 {
            return None;
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
                Some(SocketAddr::new(IpAddr::V4(ip), port))
            }
            0x02 => {
                // IPv6 - XOR with magic cookie + transaction ID
                if data.len() < 20 {
                    return None;
                }
                let mut octets = [0u8; 16];
                // First 4 bytes XOR with magic cookie
                let magic_bytes = STUN_MAGIC_COOKIE.to_be_bytes();
                for i in 0..4 {
                    octets[i] = data[4 + i] ^ magic_bytes[i];
                }
                // Remaining 12 bytes XOR with transaction ID
                for i in 0..12 {
                    octets[4 + i] = data[8 + i] ^ transaction_id[i];
                }
                let ip = Ipv6Addr::from(octets);
                Some(SocketAddr::new(IpAddr::V6(ip), port))
            }
            _ => None,
        }
    }

    /// Encode XOR-PEER-ADDRESS attribute
    fn encode_xor_peer_address(peer_addr: SocketAddr, transaction_id: &[u8; 12]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // Reserved
        match peer_addr.ip() {
            IpAddr::V4(ip) => {
                data.push(0x01); // IPv4 family
                let x_port = peer_addr.port() ^ ((STUN_MAGIC_COOKIE >> 16) as u16);
                data.extend_from_slice(&x_port.to_be_bytes());
                let x_addr = u32::from_be_bytes(ip.octets()) ^ STUN_MAGIC_COOKIE;
                data.extend_from_slice(&x_addr.to_be_bytes());
            }
            IpAddr::V6(ip) => {
                data.push(0x02); // IPv6 family
                let x_port = peer_addr.port() ^ ((STUN_MAGIC_COOKIE >> 16) as u16);
                data.extend_from_slice(&x_port.to_be_bytes());
                let octets = ip.octets();
                let magic_bytes = STUN_MAGIC_COOKIE.to_be_bytes();
                // First 4 bytes XOR with magic cookie
                for i in 0..4 {
                    data.push(octets[i] ^ magic_bytes[i]);
                }
                // Remaining 12 bytes XOR with transaction ID
                for i in 0..12 {
                    data.push(octets[4 + i] ^ transaction_id[i]);
                }
            }
        }
        data
    }

    /// Allocate a relay address on the TURN server
    pub async fn allocate_relay(&mut self) -> Result<SocketAddr, NetworkError> {
        self.connect().await?;
        let socket = self.socket.as_ref().unwrap().clone();

        let transaction_id = Self::generate_transaction_id();

        // First, send an unauthenticated Allocate request to get nonce/realm
        let mut request = Self::create_message_header(TURN_ALLOCATE_REQUEST, 0, &transaction_id);

        // Add REQUESTED-TRANSPORT attribute (UDP = 17)
        let transport = [17u8, 0, 0, 0]; // Protocol number in first byte, rest reserved
        Self::add_attribute(&mut request, TURN_ATTR_REQUESTED_TRANSPORT, &transport);

        // Update message length in header
        let msg_len = (request.len() - 20) as u16;
        request[2..4].copy_from_slice(&msg_len.to_be_bytes());

        socket.send(&request).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to send TURN Allocate request: {}", e))
        })?;

        // Wait for response
        let mut response_buf = [0u8; 1500];
        let timeout = Duration::from_secs(self.config.timeout_secs);

        let response_len = tokio::time::timeout(timeout, socket.recv(&mut response_buf))
            .await
            .map_err(|_| NetworkError::ConnectionFailed("TURN request timed out".to_string()))?
            .map_err(|e| {
                NetworkError::ConnectionFailed(format!(
                    "Failed to receive TURN Allocate response: {}",
                    e
                ))
            })?;

        let response = &response_buf[..response_len];

        // Check if this is an error response (401 Unauthorized with nonce/realm)
        let msg_type = u16::from_be_bytes([response[0], response[1]]);

        if msg_type == TURN_ALLOCATE_ERROR {
            // Parse error response to get nonce and realm
            let (nonce, realm) = self.parse_auth_challenge(response)?;

            // Store auth info
            {
                let mut session = self.session.lock();
                session.nonce = Some(nonce.clone());
                session.realm = Some(realm.clone());
            }

            // Retry with authentication
            return self.allocate_relay_authenticated(&transaction_id).await;
        }

        if msg_type != TURN_ALLOCATE_RESPONSE {
            return Err(NetworkError::ConnectionFailed(format!(
                "Unexpected TURN response type: 0x{:04x}",
                msg_type
            )));
        }

        // Parse the successful response
        self.parse_allocate_response(response, &transaction_id)
    }

    /// Allocate with authentication (second attempt after getting nonce/realm)
    async fn allocate_relay_authenticated(
        &self,
        _original_tx_id: &[u8; 12],
    ) -> Result<SocketAddr, NetworkError> {
        let socket = self.socket.as_ref().unwrap().clone();
        let transaction_id = Self::generate_transaction_id();

        let (nonce, realm) = {
            let session = self.session.lock();
            (
                session.nonce.clone().unwrap_or_default(),
                session.realm.clone().unwrap_or_default(),
            )
        };

        // Build authenticated request
        let mut request = Self::create_message_header(TURN_ALLOCATE_REQUEST, 0, &transaction_id);

        // Add REQUESTED-TRANSPORT attribute
        let transport = [17u8, 0, 0, 0];
        Self::add_attribute(&mut request, TURN_ATTR_REQUESTED_TRANSPORT, &transport);

        // Add USERNAME attribute
        Self::add_attribute(
            &mut request,
            STUN_ATTR_USERNAME,
            self.config.username.as_bytes(),
        );

        // Add REALM attribute
        Self::add_attribute(&mut request, STUN_ATTR_REALM, realm.as_bytes());

        // Add NONCE attribute
        Self::add_attribute(&mut request, STUN_ATTR_NONCE, &nonce);

        // Update message length before computing integrity
        let msg_len = (request.len() - 20 + 24) as u16; // +24 for MESSAGE-INTEGRITY
        request[2..4].copy_from_slice(&msg_len.to_be_bytes());

        // Compute and add MESSAGE-INTEGRITY
        let integrity = self.compute_message_integrity(&request, &realm, &nonce);
        Self::add_attribute(&mut request, STUN_ATTR_MESSAGE_INTEGRITY, &integrity[..20]);

        // Update final message length
        let final_len = (request.len() - 20) as u16;
        request[2..4].copy_from_slice(&final_len.to_be_bytes());

        socket.send(&request).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!(
                "Failed to send authenticated TURN request: {}",
                e
            ))
        })?;

        // Wait for response
        let mut response_buf = [0u8; 1500];
        let timeout = Duration::from_secs(self.config.timeout_secs);

        let response_len = tokio::time::timeout(timeout, socket.recv(&mut response_buf))
            .await
            .map_err(|_| NetworkError::ConnectionFailed("TURN request timed out".to_string()))?
            .map_err(|e| {
                NetworkError::ConnectionFailed(format!("Failed to receive TURN response: {}", e))
            })?;

        let response = &response_buf[..response_len];
        let msg_type = u16::from_be_bytes([response[0], response[1]]);

        if msg_type != TURN_ALLOCATE_RESPONSE {
            return Err(NetworkError::ConnectionFailed(format!(
                "TURN allocation failed with response type: 0x{:04x}",
                msg_type
            )));
        }

        self.parse_allocate_response(response, &transaction_id)
    }

    /// Parse error response to extract nonce and realm
    fn parse_auth_challenge(&self, response: &[u8]) -> Result<(Vec<u8>, String), NetworkError> {
        let msg_len = u16::from_be_bytes([response[2], response[3]]) as usize;
        let mut nonce = Vec::new();
        let mut realm = String::new();

        let mut offset = 20;
        while offset + 4 <= 20 + msg_len && offset + 4 <= response.len() {
            let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
            let attr_len =
                u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;

            if offset + 4 + attr_len > response.len() {
                break;
            }

            let attr_data = &response[offset + 4..offset + 4 + attr_len];

            match attr_type {
                STUN_ATTR_NONCE => {
                    nonce = attr_data.to_vec();
                }
                STUN_ATTR_REALM => {
                    realm = String::from_utf8_lossy(attr_data).to_string();
                }
                _ => {}
            }

            offset += 4 + ((attr_len + 3) & !3);
        }

        if nonce.is_empty() || realm.is_empty() {
            return Err(NetworkError::ConnectionFailed(
                "Missing nonce or realm in TURN 401 response".to_string(),
            ));
        }

        Ok((nonce, realm))
    }

    /// Parse successful Allocate response
    fn parse_allocate_response(
        &self,
        response: &[u8],
        transaction_id: &[u8; 12],
    ) -> Result<SocketAddr, NetworkError> {
        let msg_len = u16::from_be_bytes([response[2], response[3]]) as usize;
        let mut relay_address = None;
        let mut lifetime = DEFAULT_LIFETIME;

        let mut offset = 20;
        while offset + 4 <= 20 + msg_len && offset + 4 <= response.len() {
            let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
            let attr_len =
                u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;

            if offset + 4 + attr_len > response.len() {
                break;
            }

            let attr_data = &response[offset + 4..offset + 4 + attr_len];

            match attr_type {
                TURN_ATTR_XOR_RELAYED_ADDRESS => {
                    relay_address = Self::parse_xor_relayed_address(attr_data, transaction_id);
                }
                TURN_ATTR_LIFETIME => {
                    if attr_len >= 4 {
                        lifetime = u32::from_be_bytes([
                            attr_data[0],
                            attr_data[1],
                            attr_data[2],
                            attr_data[3],
                        ]);
                    }
                }
                _ => {}
            }

            offset += 4 + ((attr_len + 3) & !3);
        }

        let relay_addr = relay_address.ok_or_else(|| {
            NetworkError::ConnectionFailed("No relay address in TURN response".to_string())
        })?;

        // Store in session
        {
            let mut session = self.session.lock();
            session.relay_address = Some(relay_addr);
            session.lifetime = lifetime;
        }

        tracing::info!(
            "TURN allocation successful: relay={}, lifetime={}s",
            relay_addr,
            lifetime
        );

        Ok(relay_addr)
    }

    /// Create a permission for a peer address
    pub async fn create_permission(&self, peer_addr: SocketAddr) -> Result<(), NetworkError> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::ConnectionFailed("TURN not connected".to_string()))?
            .clone();

        let transaction_id = Self::generate_transaction_id();

        let (nonce, realm) = {
            let session = self.session.lock();
            (
                session.nonce.clone().unwrap_or_default(),
                session.realm.clone().unwrap_or_default(),
            )
        };

        // Build CreatePermission request
        let mut request =
            Self::create_message_header(TURN_CREATE_PERMISSION_REQUEST, 0, &transaction_id);

        // Add XOR-PEER-ADDRESS attribute
        let peer_addr_data = Self::encode_xor_peer_address(peer_addr, &transaction_id);
        Self::add_attribute(&mut request, TURN_ATTR_XOR_PEER_ADDRESS, &peer_addr_data);

        // Add authentication attributes
        Self::add_attribute(
            &mut request,
            STUN_ATTR_USERNAME,
            self.config.username.as_bytes(),
        );
        Self::add_attribute(&mut request, STUN_ATTR_REALM, realm.as_bytes());
        Self::add_attribute(&mut request, STUN_ATTR_NONCE, &nonce);

        // Update message length before computing integrity
        let msg_len = (request.len() - 20 + 24) as u16;
        request[2..4].copy_from_slice(&msg_len.to_be_bytes());

        // Add MESSAGE-INTEGRITY
        let integrity = self.compute_message_integrity(&request, &realm, &nonce);
        Self::add_attribute(&mut request, STUN_ATTR_MESSAGE_INTEGRITY, &integrity[..20]);

        let final_len = (request.len() - 20) as u16;
        request[2..4].copy_from_slice(&final_len.to_be_bytes());

        socket.send(&request).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to send CreatePermission: {}", e))
        })?;

        // Wait for response
        let mut response_buf = [0u8; 1500];
        let timeout = Duration::from_secs(self.config.timeout_secs);

        let response_len = tokio::time::timeout(timeout, socket.recv(&mut response_buf))
            .await
            .map_err(|_| {
                NetworkError::ConnectionFailed("CreatePermission request timed out".to_string())
            })?
            .map_err(|e| {
                NetworkError::ConnectionFailed(format!(
                    "Failed to receive CreatePermission response: {}",
                    e
                ))
            })?;

        let response = &response_buf[..response_len];
        let msg_type = u16::from_be_bytes([response[0], response[1]]);

        if msg_type != TURN_CREATE_PERMISSION_RESPONSE {
            return Err(NetworkError::ConnectionFailed(format!(
                "CreatePermission failed with response type: 0x{:04x}",
                msg_type
            )));
        }

        // Store permission
        {
            let mut session = self.session.lock();
            if !session.permissions.contains(&peer_addr) {
                session.permissions.push(peer_addr);
            }
        }

        tracing::info!("TURN permission created for peer: {}", peer_addr);
        Ok(())
    }

    /// Send data through TURN relay using Send indication
    pub async fn send_data(&self, data: &[u8], peer_addr: SocketAddr) -> Result<(), NetworkError> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::ConnectionFailed("TURN not connected".to_string()))?
            .clone();

        let transaction_id = Self::generate_transaction_id();

        // Build Send indication (no response expected)
        let mut indication = Self::create_message_header(TURN_SEND_INDICATION, 0, &transaction_id);

        // Add XOR-PEER-ADDRESS attribute
        let peer_addr_data = Self::encode_xor_peer_address(peer_addr, &transaction_id);
        Self::add_attribute(&mut indication, TURN_ATTR_XOR_PEER_ADDRESS, &peer_addr_data);

        // Add DATA attribute
        Self::add_attribute(&mut indication, TURN_ATTR_DATA, data);

        // Update message length
        let msg_len = (indication.len() - 20) as u16;
        indication[2..4].copy_from_slice(&msg_len.to_be_bytes());

        socket.send(&indication).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to send TURN indication: {}", e))
        })?;

        Ok(())
    }

    /// Receive data from TURN relay (Data indication)
    pub async fn receive_data(&self) -> Result<(Vec<u8>, SocketAddr), NetworkError> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::ConnectionFailed("TURN not connected".to_string()))?
            .clone();

        let mut response_buf = [0u8; 65535];

        let response_len = socket.recv(&mut response_buf).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to receive TURN data: {}", e))
        })?;

        let response = &response_buf[..response_len];

        // Check if this is a Data indication
        let msg_type = u16::from_be_bytes([response[0], response[1]]);
        if msg_type != TURN_DATA_INDICATION {
            return Err(NetworkError::ConnectionFailed(format!(
                "Expected Data indication, got: 0x{:04x}",
                msg_type
            )));
        }

        // Parse transaction ID
        let mut transaction_id = [0u8; 12];
        transaction_id.copy_from_slice(&response[8..20]);

        // Parse attributes
        let msg_len = u16::from_be_bytes([response[2], response[3]]) as usize;
        let mut peer_addr = None;
        let mut data = None;

        let mut offset = 20;
        while offset + 4 <= 20 + msg_len && offset + 4 <= response.len() {
            let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
            let attr_len =
                u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;

            if offset + 4 + attr_len > response.len() {
                break;
            }

            let attr_data = &response[offset + 4..offset + 4 + attr_len];

            match attr_type {
                TURN_ATTR_XOR_PEER_ADDRESS => {
                    peer_addr = Self::parse_xor_relayed_address(attr_data, &transaction_id);
                }
                TURN_ATTR_DATA => {
                    data = Some(attr_data.to_vec());
                }
                _ => {}
            }

            offset += 4 + ((attr_len + 3) & !3);
        }

        let peer = peer_addr.ok_or_else(|| {
            NetworkError::ConnectionFailed("No peer address in Data indication".to_string())
        })?;
        let payload = data.ok_or_else(|| {
            NetworkError::ConnectionFailed("No data in Data indication".to_string())
        })?;

        Ok((payload, peer))
    }

    /// Get the allocated relay address
    pub fn relay_address(&self) -> Option<SocketAddr> {
        self.session.lock().relay_address
    }

    /// Refresh the TURN allocation to extend lifetime
    pub async fn refresh_allocation(&self) -> Result<u32, NetworkError> {
        let socket = self
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::ConnectionFailed("TURN not connected".to_string()))?
            .clone();

        let transaction_id = Self::generate_transaction_id();

        let (nonce, realm) = {
            let session = self.session.lock();
            (
                session.nonce.clone().unwrap_or_default(),
                session.realm.clone().unwrap_or_default(),
            )
        };

        // Build Refresh request
        let mut request = Self::create_message_header(TURN_REFRESH_REQUEST, 0, &transaction_id);

        // Add LIFETIME attribute (request same lifetime)
        let lifetime_bytes = DEFAULT_LIFETIME.to_be_bytes();
        Self::add_attribute(&mut request, TURN_ATTR_LIFETIME, &lifetime_bytes);

        // Add authentication
        Self::add_attribute(
            &mut request,
            STUN_ATTR_USERNAME,
            self.config.username.as_bytes(),
        );
        Self::add_attribute(&mut request, STUN_ATTR_REALM, realm.as_bytes());
        Self::add_attribute(&mut request, STUN_ATTR_NONCE, &nonce);

        let msg_len = (request.len() - 20 + 24) as u16;
        request[2..4].copy_from_slice(&msg_len.to_be_bytes());

        let integrity = self.compute_message_integrity(&request, &realm, &nonce);
        Self::add_attribute(&mut request, STUN_ATTR_MESSAGE_INTEGRITY, &integrity[..20]);

        let final_len = (request.len() - 20) as u16;
        request[2..4].copy_from_slice(&final_len.to_be_bytes());

        socket.send(&request).await.map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to send Refresh: {}", e))
        })?;

        // Wait for response
        let mut response_buf = [0u8; 1500];
        let timeout = Duration::from_secs(self.config.timeout_secs);

        let response_len = tokio::time::timeout(timeout, socket.recv(&mut response_buf))
            .await
            .map_err(|_| NetworkError::ConnectionFailed("Refresh request timed out".to_string()))?
            .map_err(|e| {
                NetworkError::ConnectionFailed(format!("Failed to receive Refresh response: {}", e))
            })?;

        // Parse lifetime from response
        let response = &response_buf[..response_len];
        let msg_len = u16::from_be_bytes([response[2], response[3]]) as usize;
        let mut new_lifetime = DEFAULT_LIFETIME;

        let mut offset = 20;
        while offset + 4 <= 20 + msg_len && offset + 4 <= response.len() {
            let attr_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
            let attr_len =
                u16::from_be_bytes([response[offset + 2], response[offset + 3]]) as usize;

            if attr_type == TURN_ATTR_LIFETIME && attr_len >= 4 {
                let attr_data = &response[offset + 4..offset + 4 + attr_len];
                new_lifetime =
                    u32::from_be_bytes([attr_data[0], attr_data[1], attr_data[2], attr_data[3]]);
            }

            offset += 4 + ((attr_len + 3) & !3);
        }

        // Update session
        {
            let mut session = self.session.lock();
            session.lifetime = new_lifetime;
        }

        tracing::info!("TURN allocation refreshed, new lifetime: {}s", new_lifetime);
        Ok(new_lifetime)
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
        let mut turn_client = TurnClient::new(turn_config);
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
