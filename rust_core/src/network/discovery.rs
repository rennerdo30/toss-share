//! mDNS-SD device discovery

use std::net::SocketAddr;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

use crate::error::NetworkError;

/// Service type for Toss discovery
const SERVICE_TYPE: &str = "_toss._udp.local.";

/// Protocol version for discovery
const DISCOVERY_VERSION: &str = "1";

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
        let daemon = ServiceDaemon::new()
            .map_err(|e| NetworkError::Discovery(e.to_string()))?;

        Ok(Self {
            daemon,
            device_id: device_id.to_string(),
            device_name: device_name.to_string(),
            port,
            service_fullname: None,
        })
    }

    /// Register this device on the network
    pub fn register(&self) -> Result<(), NetworkError> {
        let host_name = format!("toss-{}.local.", &self.device_id[..8]);

        // Create TXT record properties
        let properties = [
            ("v", DISCOVERY_VERSION),
            ("id", &self.device_id[..16]), // Truncated ID
            ("name", &self.device_name),
        ];

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &self.device_name,
            &host_name,
            "",
            self.port,
            &properties[..],
        )
        .map_err(|e| NetworkError::Discovery(format!("Failed to create service info: {}", e)))?;

        self.daemon
            .register(service_info)
            .map_err(|e| NetworkError::Discovery(format!("Failed to register service: {}", e)))?;

        Ok(())
    }

    /// Unregister this device
    pub fn unregister(&self) {
        if let Some(ref fullname) = self.service_fullname {
            let _ = self.daemon.unregister(fullname);
        }
    }

    /// Start browsing for other devices
    pub fn browse(&self) -> Result<mdns_sd::Receiver<ServiceEvent>, NetworkError> {
        self.daemon
            .browse(SERVICE_TYPE)
            .map_err(|e| NetworkError::Discovery(format!("Failed to browse: {}", e)))
    }

    /// Parse a discovered service into peer info
    pub fn parse_service(info: &ServiceInfo) -> Option<DiscoveredPeer> {
        let properties = info.get_properties();

        let device_id = properties
            .get("id")
            .map(|v| v.val_str().to_string())?;
        let device_name = properties
            .get("name")
            .map(|v| v.val_str().to_string())
            .unwrap_or_else(|| info.get_fullname().to_string());
        let version = properties
            .get("v")
            .map(|v| v.val_str().to_string())
            .unwrap_or_else(|| "1".to_string());

        // Get addresses
        let addresses: Vec<SocketAddr> = info
            .get_addresses()
            .iter()
            .map(|addr| SocketAddr::new(*addr, info.get_port()))
            .collect();

        if addresses.is_empty() {
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
}
