//! Pairing coordinator for device pairing via mDNS and relay server

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::sync::RwLock;

use crate::error::NetworkError;

/// Service type for pairing discovery
const PAIRING_SERVICE_TYPE: &str = "_toss-pair._udp.local.";

/// Device info discovered during pairing
#[derive(Debug, Clone)]
pub struct PairingDeviceInfo {
    /// The pairing code
    pub code: String,
    /// The device's public key (32 bytes)
    pub public_key: [u8; 32],
    /// The device's name
    pub device_name: String,
    /// Network addresses (if discovered via mDNS)
    pub addresses: Vec<SocketAddr>,
    /// Whether discovered via relay or mDNS
    pub via_relay: bool,
    /// When the pairing session expires (Unix timestamp)
    pub expires_at: Option<u64>,
}

/// Request to register pairing on relay server
#[derive(Debug, Serialize)]
struct RegisterPairingRequest {
    code: String,
    public_key: String,
    device_name: String,
    expires_in_secs: Option<u64>,
}

/// Response from registering pairing
#[derive(Debug, Deserialize)]
struct RegisterPairingResponse {
    code: String,
    expires_at: u64,
}

/// Response from finding pairing
#[derive(Debug, Deserialize)]
struct FindPairingResponse {
    code: String,
    public_key: String,
    device_name: String,
    expires_at: u64,
}

/// Pairing coordinator that handles both mDNS and relay-based pairing
pub struct PairingCoordinator {
    mdns_daemon: Option<ServiceDaemon>,
    relay_url: Option<String>,
    device_name: String,
    current_code: RwLock<Option<String>>,
    http_client: reqwest::Client,
}

impl PairingCoordinator {
    /// Create a new pairing coordinator
    pub fn new(device_name: &str, relay_url: Option<String>) -> Result<Self, NetworkError> {
        let mdns_daemon = match ServiceDaemon::new() {
            Ok(daemon) => Some(daemon),
            Err(e) => {
                tracing::warn!("Failed to create mDNS daemon for pairing: {}", e);
                None
            }
        };

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| NetworkError::Discovery(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            mdns_daemon,
            relay_url,
            device_name: device_name.to_string(),
            current_code: RwLock::new(None),
            http_client,
        })
    }

    /// Start advertising this device for pairing with the given code and public key
    pub async fn start_advertisement(
        &self,
        code: &str,
        public_key: &[u8; 32],
    ) -> Result<(), NetworkError> {
        // Store the current code
        *self.current_code.write().await = Some(code.to_string());

        // Encode public key as base64
        let public_key_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, public_key);

        // Try mDNS advertisement
        if let Some(ref daemon) = self.mdns_daemon {
            let host_name = format!("toss-pair-{}.local.", code);
            let truncated_pk = &public_key_b64[..43.min(public_key_b64.len())]; // Max TXT record value size

            let properties: HashMap<&str, &str> = [
                ("code", code),
                ("pk", truncated_pk),
                ("name", &self.device_name),
            ]
            .into_iter()
            .collect();

            let properties_vec: Vec<(&str, &str)> = properties.into_iter().collect();

            match ServiceInfo::new(
                PAIRING_SERVICE_TYPE,
                &format!("toss-pair-{}", code),
                &host_name,
                "",
                12345, // Arbitrary port for pairing advertisement
                &properties_vec[..],
            ) {
                Ok(service_info) => {
                    if let Err(e) = daemon.register(service_info) {
                        tracing::warn!("Failed to register mDNS pairing service: {}", e);
                    } else {
                        tracing::info!("mDNS pairing service registered with code: {}", code);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to create mDNS service info: {}", e);
                }
            }
        }

        // Try relay server registration
        if let Some(ref relay_url) = self.relay_url {
            let request = RegisterPairingRequest {
                code: code.to_string(),
                public_key: public_key_b64,
                device_name: self.device_name.clone(),
                expires_in_secs: Some(300), // 5 minutes
            };

            let url = format!("{}/api/v1/pairing/register", relay_url);
            match self.http_client.post(&url).json(&request).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<RegisterPairingResponse>().await {
                            Ok(reg_response) => {
                                tracing::info!(
                                    "Pairing registered on relay server with code: {}, expires_at: {}",
                                    reg_response.code,
                                    reg_response.expires_at
                                );
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse registration response: {}", e);
                            }
                        }
                    } else {
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_default();
                        tracing::warn!(
                            "Failed to register pairing on relay: {} - {}",
                            status,
                            error_text
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to contact relay server for pairing: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Find a device by pairing code
    pub async fn find_device(&self, code: &str) -> Result<PairingDeviceInfo, NetworkError> {
        // First try mDNS discovery
        if let Some(ref daemon) = self.mdns_daemon {
            match self.find_via_mdns(daemon, code).await {
                Ok(Some(info)) => {
                    tracing::info!("Found device via mDNS with code: {}", code);
                    return Ok(info);
                }
                Ok(None) => {
                    tracing::debug!("Device not found via mDNS, trying relay...");
                }
                Err(e) => {
                    tracing::warn!("mDNS search failed: {}, trying relay...", e);
                }
            }
        }

        // Fall back to relay server
        if let Some(ref relay_url) = self.relay_url {
            return self.find_via_relay(relay_url, code).await;
        }

        Err(NetworkError::Discovery(
            "Device not found via mDNS or relay".to_string(),
        ))
    }

    /// Find device via mDNS
    async fn find_via_mdns(
        &self,
        daemon: &ServiceDaemon,
        code: &str,
    ) -> Result<Option<PairingDeviceInfo>, NetworkError> {
        let receiver = daemon
            .browse(PAIRING_SERVICE_TYPE)
            .map_err(|e| NetworkError::Discovery(format!("Failed to browse mDNS: {}", e)))?;

        // Listen for a short time for pairing services
        let timeout = tokio::time::timeout(Duration::from_secs(3), async {
            while let Ok(event) = receiver.recv() {
                if let ServiceEvent::ServiceResolved(info) = event {
                    if let Some(found_code) = info.get_properties().get("code") {
                        if found_code.val_str() == code {
                            // Found matching device
                            let pk_str = info
                                .get_properties()
                                .get("pk")
                                .map(|v| v.val_str().to_string())
                                .unwrap_or_default();

                            let device_name = info
                                .get_properties()
                                .get("name")
                                .map(|v| v.val_str().to_string())
                                .unwrap_or_else(|| "Unknown".to_string());

                            // Decode public key
                            if let Ok(pk_bytes) = base64::Engine::decode(
                                &base64::engine::general_purpose::STANDARD,
                                &pk_str,
                            ) {
                                if pk_bytes.len() == 32 {
                                    let mut public_key = [0u8; 32];
                                    public_key.copy_from_slice(&pk_bytes);

                                    let addresses: Vec<SocketAddr> = info
                                        .get_addresses()
                                        .iter()
                                        .filter_map(|scoped_ip| {
                                            let ip: IpAddr = match scoped_ip {
                                                mdns_sd::ScopedIp::V4(v4) => IpAddr::V4(*v4.addr()),
                                                mdns_sd::ScopedIp::V6(v6) => IpAddr::V6(*v6.addr()),
                                                _ => return None, // Handle future variants
                                            };
                                            Some(SocketAddr::new(ip, info.get_port()))
                                        })
                                        .collect();

                                    return Some(PairingDeviceInfo {
                                        code: code.to_string(),
                                        public_key,
                                        device_name,
                                        addresses,
                                        via_relay: false,
                                        expires_at: None, // mDNS doesn't provide expiration
                                    });
                                }
                            }
                        }
                    }
                }
            }
            None
        });

        match timeout.await {
            Ok(result) => Ok(result),
            Err(_) => Ok(None), // Timeout, device not found via mDNS
        }
    }

    /// Find device via relay server
    async fn find_via_relay(
        &self,
        relay_url: &str,
        code: &str,
    ) -> Result<PairingDeviceInfo, NetworkError> {
        let url = format!("{}/api/v1/pairing/find/{}", relay_url, code);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| NetworkError::Relay(format!("Failed to contact relay: {}", e)))?;

        if response.status().is_success() {
            let pairing: FindPairingResponse = response
                .json()
                .await
                .map_err(|e| NetworkError::Relay(format!("Invalid response: {}", e)))?;

            // Decode public key
            let pk_bytes = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &pairing.public_key,
            )
            .map_err(|e| NetworkError::Relay(format!("Invalid public key encoding: {}", e)))?;

            if pk_bytes.len() != 32 {
                return Err(NetworkError::Relay("Invalid public key length".to_string()));
            }

            let mut public_key = [0u8; 32];
            public_key.copy_from_slice(&pk_bytes);

            Ok(PairingDeviceInfo {
                code: pairing.code,
                public_key,
                device_name: pairing.device_name,
                addresses: vec![], // No direct addresses from relay
                via_relay: true,
                expires_at: Some(pairing.expires_at),
            })
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            Err(NetworkError::Discovery(
                "Pairing code not found or expired".to_string(),
            ))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(NetworkError::Relay(format!("Relay error: {}", error_text)))
        }
    }

    /// Stop advertising
    pub async fn stop_advertisement(&self) {
        if let Some(code) = self.current_code.write().await.take() {
            // Unregister from mDNS
            if let Some(ref daemon) = self.mdns_daemon {
                let fullname = format!("toss-pair-{}.{}", code, PAIRING_SERVICE_TYPE);
                let _ = daemon.unregister(&fullname);
            }

            // Cancel on relay server
            if let Some(ref relay_url) = self.relay_url {
                let url = format!("{}/api/v1/pairing/{}", relay_url, code);
                let _ = self.http_client.delete(&url).send().await;
            }
        }
    }

    /// Check if we have a relay URL configured
    pub fn has_relay(&self) -> bool {
        self.relay_url.is_some()
    }

    /// Check if we have mDNS available
    pub fn has_mdns(&self) -> bool {
        self.mdns_daemon.is_some()
    }
}

impl Drop for PairingCoordinator {
    fn drop(&mut self) {
        // Note: async cleanup would need to be handled by the caller
        // The stop_advertisement method should be called before dropping
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_coordinator_creation() {
        let coordinator = PairingCoordinator::new("Test Device", None);
        assert!(coordinator.is_ok());
    }

    #[test]
    fn test_pairing_coordinator_with_relay() {
        let coordinator =
            PairingCoordinator::new("Test Device", Some("http://localhost:8080".to_string()));
        assert!(coordinator.is_ok());
        assert!(coordinator.unwrap().has_relay());
    }
}
