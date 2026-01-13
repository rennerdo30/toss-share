//! QUIC transport for P2P connections

use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig, VarInt};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::crypto::KEY_SIZE;
use crate::error::NetworkError;
use crate::protocol::{Frame, Message};
use std::time::SystemTime;

/// Max idle timeout for connections
const IDLE_TIMEOUT_SECS: u64 = 30;

/// Keep-alive interval
const KEEP_ALIVE_SECS: u64 = 5;

/// QUIC transport layer
pub struct QuicTransport {
    endpoint: Endpoint,
    local_addr: SocketAddr,
}

impl QuicTransport {
    /// Create a new QUIC transport
    pub async fn new(bind_addr: SocketAddr) -> Result<Self, NetworkError> {
        // Generate self-signed certificate
        let (cert, key) =
            generate_self_signed_cert().map_err(|e| NetworkError::Tls(e.to_string()))?;

        // Configure server
        let server_config =
            configure_server(cert.clone(), key).map_err(|e| NetworkError::Tls(e.to_string()))?;

        // Configure client (accepts any certificate for P2P)
        let client_config = configure_client().map_err(|e| NetworkError::Tls(e.to_string()))?;

        // Create endpoint
        let mut endpoint = Endpoint::server(server_config, bind_addr)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        endpoint.set_default_client_config(client_config);

        let local_addr = endpoint
            .local_addr()
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        Ok(Self {
            endpoint,
            local_addr,
        })
    }

    /// Get local address
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Connect to a peer
    pub async fn connect(&self, addr: SocketAddr) -> Result<PeerConnection, NetworkError> {
        let connection = self
            .endpoint
            .connect(addr, "toss")
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?
            .await
            .map_err(|e| NetworkError::ConnectionFailed(e.to_string()))?;

        Ok(PeerConnection::new(connection, vec![addr], true))
    }

    /// Accept an incoming connection
    pub async fn accept(&self) -> Option<PeerConnection> {
        let incoming = self.endpoint.accept().await?;
        let addr = incoming.remote_address();
        let connection = incoming.await.ok()?;
        Some(PeerConnection::new(connection, vec![addr], true))
    }

    /// Close the endpoint
    pub fn close(&self) {
        self.endpoint.close(VarInt::from_u32(0), b"closing");
    }
}

/// Session tracking for key rotation
struct SessionTracker {
    created_at: SystemTime,
    message_count: u64,
}

impl SessionTracker {
    fn new() -> Self {
        Self {
            created_at: SystemTime::now(),
            message_count: 0,
        }
    }

    fn increment_message_count(&mut self) {
        self.message_count += 1;
    }

    fn should_rotate(&self) -> bool {
        const MAX_MESSAGES: u64 = 1000;
        const MAX_AGE_SECS: u64 = 24 * 60 * 60; // 24 hours

        if self.message_count >= MAX_MESSAGES {
            return true;
        }

        if let Ok(age) = SystemTime::now().duration_since(self.created_at) {
            if age.as_secs() >= MAX_AGE_SECS {
                return true;
            }
        }

        false
    }

    fn reset(&mut self) {
        self.created_at = SystemTime::now();
        self.message_count = 0;
    }
}

/// Connection to a peer
pub struct PeerConnection {
    connection: Connection,
    addresses: Vec<SocketAddr>,
    session_key: Mutex<Option<[u8; KEY_SIZE]>>,
    peer_device_id: Mutex<Option<[u8; 32]>>,
    peer_name: Mutex<Option<String>>,
    is_local: bool,
    session_tracker: Mutex<SessionTracker>,
}

impl PeerConnection {
    /// Create a new peer connection
    pub fn new(connection: Connection, addresses: Vec<SocketAddr>, is_local: bool) -> Self {
        Self {
            connection,
            addresses,
            session_key: Mutex::new(None),
            peer_device_id: Mutex::new(None),
            peer_name: Mutex::new(None),
            is_local,
            session_tracker: Mutex::new(SessionTracker::new()),
        }
    }

    /// Check if connection is still active
    pub fn is_connected(&self) -> bool {
        self.connection.close_reason().is_none()
    }

    /// Get remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.connection.remote_address()
    }

    /// Get all addresses
    pub fn addresses(&self) -> &[SocketAddr] {
        &self.addresses
    }

    /// Check if local connection
    pub fn is_local(&self) -> bool {
        self.is_local
    }

    /// Set session key
    pub async fn set_session_key(&self, key: [u8; KEY_SIZE]) {
        *self.session_key.lock().await = Some(key);
    }

    /// Set peer device ID
    pub async fn set_peer_device_id(&self, id: [u8; 32]) {
        *self.peer_device_id.lock().await = Some(id);
    }

    /// Get peer device ID
    pub fn peer_device_id(&self) -> Option<[u8; 32]> {
        // Use try_lock for non-async context
        self.peer_device_id.try_lock().ok().and_then(|guard| *guard)
    }

    /// Set peer name
    pub async fn set_peer_name(&self, name: String) {
        *self.peer_name.lock().await = Some(name);
    }

    /// Get peer name
    pub fn peer_name(&self) -> Option<String> {
        self.peer_name
            .try_lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Send raw bytes
    pub async fn send_raw(&self, data: &[u8]) -> Result<(), NetworkError> {
        let mut send = self
            .connection
            .open_uni()
            .await
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        send.write_all(data)
            .await
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        send.finish()
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        Ok(())
    }

    /// Receive raw bytes
    pub async fn receive_raw(&self) -> Result<Vec<u8>, NetworkError> {
        let mut recv = self
            .connection
            .accept_uni()
            .await
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        let data = recv
            .read_to_end(crate::protocol::MAX_MESSAGE_SIZE)
            .await
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        Ok(data)
    }

    /// Send an encrypted message
    pub async fn send_message(&self, message: &Message) -> Result<(), NetworkError> {
        // Increment message count (only for non-rotation messages)
        if !matches!(message, Message::KeyRotation(_)) {
            let mut tracker = self.session_tracker.lock().await;
            tracker.increment_message_count();
        }

        let key = self.session_key.lock().await;
        let key = key.as_ref().ok_or(NetworkError::NotAuthenticated)?;

        let header = message.header();
        let payload = message
            .serialize()
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        let frame = Frame::encrypt(&header, &payload, key)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        self.send_raw(&frame.to_bytes()).await
    }

    /// Check if session key should be rotated
    pub async fn should_rotate_key(&self) -> bool {
        let tracker = self.session_tracker.lock().await;
        tracker.should_rotate()
    }

    /// Reset session tracker after rotation
    pub async fn reset_session_tracker(&self) {
        let mut tracker = self.session_tracker.lock().await;
        tracker.reset();
    }

    /// Receive and decrypt a message
    pub async fn receive_message(&self) -> Result<Message, NetworkError> {
        let key = self.session_key.lock().await;
        let key = key.as_ref().ok_or(NetworkError::NotAuthenticated)?;

        let data = self.receive_raw().await?;

        let frame = Frame::from_bytes(&data).map_err(|e| NetworkError::Transport(e.to_string()))?;

        let (header, payload) = frame
            .decrypt(key)
            .map_err(|e| NetworkError::Transport(e.to_string()))?;

        Message::deserialize(&header, &payload).map_err(|e| NetworkError::Transport(e.to_string()))
    }

    /// Close the connection
    pub fn close(&self) {
        self.connection.close(VarInt::from_u32(0), b"closing");
    }
}

/// Generate a self-signed certificate for QUIC
fn generate_self_signed_cert(
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), Box<dyn std::error::Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["toss".to_string()])?;
    let key_der = cert.signing_key.serialize_der();
    let key = PrivatePkcs8KeyDer::from(key_der).into();
    let cert_der = cert.cert.der().to_vec();
    let cert = CertificateDer::from(cert_der);
    Ok((cert, key))
}

/// Configure QUIC server
fn configure_server(
    cert: CertificateDer<'static>,
    key: PrivateKeyDer<'static>,
) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let mut server_config = ServerConfig::with_single_cert(vec![cert], key)?;

    let mut transport = TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(IDLE_TIMEOUT_SECS).try_into()?));
    transport.keep_alive_interval(Some(Duration::from_secs(KEEP_ALIVE_SECS)));

    server_config.transport_config(Arc::new(transport));

    Ok(server_config)
}

/// Configure QUIC client (skip certificate verification for P2P)
fn configure_client() -> Result<ClientConfig, Box<dyn std::error::Error>> {
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();

    let mut client_config = ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(crypto)?,
    ));

    let mut transport = TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(IDLE_TIMEOUT_SECS).try_into()?));
    transport.keep_alive_interval(Some(Duration::from_secs(KEEP_ALIVE_SECS)));

    client_config.transport_config(Arc::new(transport));

    Ok(client_config)
}

/// Skip server certificate verification (for P2P where we verify via pairing)
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_certificate() {
        let result = generate_self_signed_cert();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transport_creation() {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = QuicTransport::new(addr).await;
        assert!(transport.is_ok());

        let transport = transport.unwrap();
        assert_ne!(transport.local_addr().port(), 0);
    }
}
