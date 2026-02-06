//! Network interface enumeration and classification
//!
//! Provides utilities to enumerate network interfaces, classify them by type
//! (LAN, VPN, virtual, loopback, link-local), and filter for addresses suitable
//! for local network discovery and QUIC connections.

use std::net::{IpAddr, Ipv4Addr};

/// Classification of a network interface
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// Standard LAN interface (Ethernet, WiFi)
    Lan,
    /// VPN tunnel interface (tun/tap, utun, wg)
    Vpn,
    /// Virtual/container interface (Docker, WSL, VMware, Parallels)
    Virtual,
    /// Loopback (127.x.x.x, ::1)
    Loopback,
    /// Link-local (169.254.x.x, fe80::)
    LinkLocal,
}

/// A classified network interface
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: IpAddr,
    pub interface_type: InterfaceType,
}

/// VPN interface name prefixes
const VPN_PREFIXES: &[&str] = &[
    "tun",
    "tap",
    "utun",
    "wg",
    "ppp",
    "vpn",
    "tailscale",
    "proton",
    "nord",
];

/// Virtual/container interface name prefixes
const VIRTUAL_PREFIXES: &[&str] = &[
    "docker",
    "br-",
    "veth",
    "vmnet",
    "virbr",
    "vbox",
    "wsl",
    "lxd",
    "parallels",
];

/// Classify an interface by its name and IP address
fn classify_interface(name: &str, ip: IpAddr) -> InterfaceType {
    // Check IP-based classifications first
    if ip.is_loopback() {
        return InterfaceType::Loopback;
    }

    if is_link_local(ip) {
        return InterfaceType::LinkLocal;
    }

    let name_lower = name.to_lowercase();

    // Check VPN patterns
    for prefix in VPN_PREFIXES {
        if name_lower.starts_with(prefix) {
            return InterfaceType::Vpn;
        }
    }

    // Check virtual/container patterns
    for prefix in VIRTUAL_PREFIXES {
        if name_lower.starts_with(prefix) {
            return InterfaceType::Virtual;
        }
    }

    InterfaceType::Lan
}

/// Check if an IP address is link-local
fn is_link_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            // 169.254.0.0/16
            octets[0] == 169 && octets[1] == 254
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            // fe80::/10
            (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Get all network interfaces, classified by type
pub fn get_all_interfaces() -> Vec<NetworkInterface> {
    let ifaces = match if_addrs::get_if_addrs() {
        Ok(ifaces) => ifaces,
        Err(e) => {
            tracing::warn!("Failed to enumerate network interfaces: {}", e);
            return Vec::new();
        }
    };

    ifaces
        .into_iter()
        .map(|iface| {
            let ip = iface.ip();
            let interface_type = classify_interface(&iface.name, ip);
            NetworkInterface {
                name: iface.name,
                ip,
                interface_type,
            }
        })
        .collect()
}

/// Get only LAN interfaces (suitable for mDNS/QUIC)
pub fn get_lan_interfaces() -> Vec<NetworkInterface> {
    get_all_interfaces()
        .into_iter()
        .filter(|iface| iface.interface_type == InterfaceType::Lan)
        .collect()
}

/// Get LAN IPv4 addresses (most common for local discovery)
pub fn get_lan_ipv4_addresses() -> Vec<Ipv4Addr> {
    get_lan_interfaces()
        .iter()
        .filter_map(|iface| match iface.ip {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        })
        .collect()
}

/// Check if a VPN is likely active
pub fn is_vpn_active() -> bool {
    get_all_interfaces()
        .iter()
        .any(|iface| iface.interface_type == InterfaceType::Vpn)
}

/// Check if an IP address is suitable for LAN connections
/// (not loopback, not link-local, not unspecified)
pub fn is_lan_suitable(ip: &IpAddr) -> bool {
    !ip.is_loopback() && !ip.is_unspecified() && !is_link_local(*ip)
}

/// Log a diagnostic summary of all interfaces
pub fn log_interface_diagnostics() {
    let interfaces = get_all_interfaces();

    if interfaces.is_empty() {
        tracing::warn!("No network interfaces found");
        return;
    }

    tracing::info!(
        "Network interface diagnostics ({} interfaces):",
        interfaces.len()
    );
    for iface in &interfaces {
        tracing::info!(
            "  {} ({:?}): {}",
            iface.name,
            iface.interface_type,
            iface.ip
        );
    }

    let lan_addrs = get_lan_ipv4_addresses();
    if lan_addrs.is_empty() {
        tracing::warn!("No LAN IPv4 addresses found — local discovery may not work");
    } else {
        tracing::info!("LAN IPv4 addresses for discovery: {:?}", lan_addrs);
    }

    if is_vpn_active() {
        tracing::warn!("VPN detected — local network discovery may be affected");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_classify_loopback() {
        assert_eq!(
            classify_interface("lo", IpAddr::V4(Ipv4Addr::LOCALHOST)),
            InterfaceType::Loopback
        );
        assert_eq!(
            classify_interface("lo0", IpAddr::V6(Ipv6Addr::LOCALHOST)),
            InterfaceType::Loopback
        );
    }

    #[test]
    fn test_classify_link_local_v4() {
        let ip = IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1));
        assert_eq!(classify_interface("en0", ip), InterfaceType::LinkLocal);
    }

    #[test]
    fn test_classify_link_local_v6() {
        let ip = IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1));
        assert_eq!(classify_interface("en0", ip), InterfaceType::LinkLocal);
    }

    #[test]
    fn test_classify_vpn_interfaces() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(classify_interface("tun0", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("tap0", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("utun3", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("wg0", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("ppp0", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("tailscale0", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("nordlynx", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("proton0", ip), InterfaceType::Vpn);
    }

    #[test]
    fn test_classify_virtual_interfaces() {
        let ip = IpAddr::V4(Ipv4Addr::new(172, 17, 0, 1));
        assert_eq!(classify_interface("docker0", ip), InterfaceType::Virtual);
        assert_eq!(classify_interface("br-abc123", ip), InterfaceType::Virtual);
        assert_eq!(classify_interface("veth1234", ip), InterfaceType::Virtual);
        assert_eq!(classify_interface("vmnet8", ip), InterfaceType::Virtual);
        assert_eq!(classify_interface("virbr0", ip), InterfaceType::Virtual);
        assert_eq!(classify_interface("vboxnet0", ip), InterfaceType::Virtual);
    }

    #[test]
    fn test_classify_lan_interfaces() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        assert_eq!(classify_interface("en0", ip), InterfaceType::Lan);
        assert_eq!(classify_interface("eth0", ip), InterfaceType::Lan);
        assert_eq!(classify_interface("wlan0", ip), InterfaceType::Lan);
        assert_eq!(classify_interface("Wi-Fi", ip), InterfaceType::Lan);
    }

    #[test]
    fn test_is_link_local() {
        assert!(is_link_local(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
        assert!(is_link_local(IpAddr::V4(Ipv4Addr::new(169, 254, 255, 255))));
        assert!(!is_link_local(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_link_local(IpAddr::V6(Ipv6Addr::new(
            0xfe80, 0, 0, 0, 0, 0, 0, 1
        ))));
        assert!(!is_link_local(IpAddr::V6(Ipv6Addr::new(
            0x2001, 0xdb8, 0, 0, 0, 0, 0, 1
        ))));
    }

    #[test]
    fn test_is_lan_suitable() {
        assert!(is_lan_suitable(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(!is_lan_suitable(&IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(!is_lan_suitable(&IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
        assert!(!is_lan_suitable(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
    }

    #[test]
    fn test_get_all_interfaces_returns_results() {
        // On any machine, we should have at least a loopback interface
        let interfaces = get_all_interfaces();
        assert!(!interfaces.is_empty(), "Should have at least one interface");
        assert!(
            interfaces
                .iter()
                .any(|i| i.interface_type == InterfaceType::Loopback),
            "Should have at least a loopback interface"
        );
    }

    #[test]
    fn test_get_lan_interfaces_excludes_loopback() {
        let lan = get_lan_interfaces();
        assert!(
            !lan.iter()
                .any(|i| i.interface_type == InterfaceType::Loopback),
            "LAN interfaces should not include loopback"
        );
    }

    #[test]
    fn test_get_lan_interfaces_excludes_vpn() {
        let lan = get_lan_interfaces();
        assert!(
            !lan.iter().any(|i| i.interface_type == InterfaceType::Vpn),
            "LAN interfaces should not include VPN"
        );
    }

    #[test]
    fn test_get_lan_interfaces_excludes_virtual() {
        let lan = get_lan_interfaces();
        assert!(
            !lan.iter()
                .any(|i| i.interface_type == InterfaceType::Virtual),
            "LAN interfaces should not include virtual interfaces"
        );
    }

    #[test]
    fn test_classify_is_case_insensitive() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        assert_eq!(classify_interface("TUN0", ip), InterfaceType::Vpn);
        assert_eq!(classify_interface("Docker0", ip), InterfaceType::Virtual);
    }
}
