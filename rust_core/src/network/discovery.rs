//! mDNS-SD device discovery

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::SocketAddr;
use std::time::Duration;

use super::interfaces;
use crate::error::NetworkError;

/// Service type for Toss discovery
const SERVICE_TYPE: &str = "_toss._udp.local.";

/// Protocol version for discovery
const DISCOVERY_VERSION: &str = "1";

/// Default mDNS browse timeout in seconds
const DEFAULT_BROWSE_TIMEOUT_SECS: u64 = 15;

/// Default number of retries for mDNS registration
const DEFAULT_REGISTER_RETRIES: u32 = 3;

/// Delay between registration retries
const REGISTER_RETRY_DELAY: Duration = Duration::from_secs(2);

/// Discovered peer information
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    /// Device ID (from TXT record)
    pub device_id: String,
    /// Device name (from TXT record)
    pub device_name: String,
    /// Network addresses
    pub addresses: Vec<SocketAddr>,
    /// Protocol version
    pub version: String,
}

/// mDNS-SD discovery service
pub struct MdnsDiscovery {
    daemon: ServiceDaemon,
    device_id: String,
    device_name: String,
    port: u16,
    service_fullname: Option<String>,
}

impl MdnsDiscovery {
    /// Create a new discovery service
    pub fn new(device_id: &str, device_name: &str, port: u16) -> Result<Self, NetworkError> {
        let daemon = ServiceDaemon::new().map_err(|e| NetworkError::Discovery(e.to_string()))?;

        Ok(Self {
            daemon,
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            port,
            service_fullname: None,
        })
    }

    /// Register this device on the network using real LAN IP addresses
    pub fn register(&self) -> Result<(), NetworkError> {
        let host_name = format!("toss-{}.local.", &self.device_id[..8]);

        // Create TXT record properties
        let properties = [
            ("v", DISCOVERY_VERSION),
            ("id", &self.device_id[..16]), // Truncated ID
            ("name", &self.device_name),
        ];

        // Get real LAN IPv4 addresses for mDNS announcement
        let lan_addrs = interfaces::get_lan_ipv4_addresses();
        let ip_str = if lan_addrs.is_empty() {
            tracing::warn!("No LAN IPv4 addresses found — mDNS will register without explicit IPs");
            String::new()
        } else {
            let addr_str = lan_addrs
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join(",");
            tracing::info!("Registering mDNS with LAN addresses: {}", addr_str);
            addr_str
        };

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &self.device_name,
            &host_name,
            ip_str.as_str(),
            self.port,
            &properties[..],
        )
        .map_err(|e| NetworkError::Discovery(format!("Failed to create service info: {}", e)))?;

        self.daemon
            .register(service_info)
            .map_err(|e| NetworkError::Discovery(format!("Failed to register service: {}", e)))?;

        Ok(())
    }

    /// Register with retry — retries `register()` on failure
    pub fn register_with_retry(&self) -> Result<(), NetworkError> {
        let mut last_err = None;
        for attempt in 1..=DEFAULT_REGISTER_RETRIES {
            match self.register() {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(
                        "mDNS registration attempt {}/{} failed: {}",
                        attempt,
                        DEFAULT_REGISTER_RETRIES,
                        e
                    );
                    last_err = Some(e);
                    if attempt < DEFAULT_REGISTER_RETRIES {
                        std::thread::sleep(REGISTER_RETRY_DELAY);
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            NetworkError::Discovery("mDNS registration failed after retries".to_string())
        }))
    }

    /// Unregister this device
    pub fn unregister(&self) {
        if let Some(ref fullname) = self.service_fullname {
            let _ = self.daemon.unregister(fullname);
        }
    }

    /// Start browsing for other devices with a configurable timeout
    pub fn browse(&self) -> Result<mdns_sd::Receiver<ServiceEvent>, NetworkError> {
        self.daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| NetworkError::Discovery(format!("Failed to browse: {}", e)))
    }

    /// Get the default browse timeout
    pub fn default_browse_timeout() -> Duration {
        Duration::from_secs(DEFAULT_BROWSE_TIMEOUT_SECS)
    }

    /// Parse a discovered service into peer info, filtering out non-LAN addresses
    pub fn parse_service(info: &ServiceInfo) -> Option<DiscoveredPeer> {
        let properties = info.get_properties();

        let device_id = properties.get("id").map(|v| v.val_str().to_string())?;
        let device_name = properties
            .get("name")
            .map(|v| v.val_str().to_string())
            .unwrap_or_else(|| info.get_fullname().to_string());
        let version = properties
            .get("v")
            .map(|v| v.val_str().to_string())
            .unwrap_or_else(|| "1".to_string());

        // Get addresses, filtering out loopback, link-local, and unspecified
        let port = info.get_port();
        let mut addresses: Vec<SocketAddr> = info
            .get_addresses()
            .iter()
            .map(|addr| SocketAddr::new(*addr, port))
            .filter(|sock_addr| interfaces::is_lan_suitable(&sock_addr.ip()))
            .collect();

        // Prioritize IPv4 over IPv6 for LAN connections
        addresses.sort_by_key(|a| if a.is_ipv4() { 0 } else { 1 });

        if addresses.is_empty() {
            tracing::debug!(
                "Discovered peer {} has no suitable LAN addresses after filtering",
                device_id
            );
            return None;
        }

        Some(DiscoveredPeer {
            device_id,
            device_name,
            addresses,
            version,
        })
    }

    /// Check if this is our own service
    pub fn is_own_service(&self, info: &ServiceInfo) -> bool {
        if let Some(id) = info.get_properties().get("id") {
            id.val_str().starts_with(&self.device_id[..16])
        } else {
            false
        }
    }
}

impl Drop for MdnsDiscovery {
    fn drop(&mut self) {
        self.unregister();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_creation() {
        let result = MdnsDiscovery::new("test-device-id", "Test Device", 12345);
        assert!(result.is_ok());
    }

    #[test]
    fn test_service_type() {
        assert_eq!(SERVICE_TYPE, "_toss._udp.local.");
    }

    #[test]
    fn test_default_browse_timeout() {
        let timeout = MdnsDiscovery::default_browse_timeout();
        assert_eq!(timeout, Duration::from_secs(15));
    }
}
