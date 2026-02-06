//! Firewall integration for network access
//!
//! On Windows, uses the COM API to manage Windows Firewall rules for QUIC UDP traffic.
//! On other platforms, provides no-op stubs (macOS/Linux handle firewall via OS prompts or iptables).

use crate::error::NetworkError;

/// The firewall rule name used for identification
#[cfg(target_os = "windows")]
const FIREWALL_RULE_NAME: &str = "Toss Clipboard Share";

/// Check if a UDP port has a firewall exemption
///
/// On non-Windows platforms, this always returns true.
#[cfg(not(target_os = "windows"))]
pub fn is_port_allowed(_port: u16) -> bool {
    true // macOS/Linux handle this via OS prompts or iptables
}

/// Request firewall exemption for Toss UDP traffic
///
/// On non-Windows platforms, this is a no-op.
#[cfg(not(target_os = "windows"))]
pub fn ensure_firewall_exemption(_port: u16, _app_name: &str) -> Result<(), NetworkError> {
    Ok(())
}

/// Remove firewall rule when shutting down (cleanup)
///
/// On non-Windows platforms, this is a no-op.
#[cfg(not(target_os = "windows"))]
pub fn remove_firewall_exemption(_app_name: &str) -> Result<(), NetworkError> {
    Ok(())
}

// Windows-specific implementation
#[cfg(target_os = "windows")]
pub fn is_port_allowed(port: u16) -> bool {
    match check_firewall_rule(port) {
        Ok(allowed) => allowed,
        Err(e) => {
            tracing::warn!("Failed to check firewall status: {}", e);
            false
        }
    }
}

#[cfg(target_os = "windows")]
pub fn ensure_firewall_exemption(port: u16, app_name: &str) -> Result<(), NetworkError> {
    use windows::core::BSTR;
    use windows::Win32::NetworkManagement::WindowsFirewall::*;
    use windows::Win32::System::Com::*;

    // Initialize COM
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED)
            .map_err(|e| NetworkError::ConnectionFailed(format!("COM init failed: {}", e)))?;
    }

    // Create firewall policy
    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER).map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to create firewall policy: {}", e))
        })?
    };

    // Create firewall rule
    let rule: INetFwRule = unsafe {
        CoCreateInstance(&NetFwRule, None, CLSCTX_INPROC_SERVER).map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to create firewall rule: {}", e))
        })?
    };

    let rule_name = format!("{} - {}", FIREWALL_RULE_NAME, app_name);

    unsafe {
        rule.SetName(&BSTR::from(&rule_name)).map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to set rule name: {}", e))
        })?;
        rule.SetDescription(&BSTR::from("Allow Toss clipboard sharing UDP traffic"))
            .map_err(|e| {
                NetworkError::ConnectionFailed(format!("Failed to set rule description: {}", e))
            })?;
        rule.SetProtocol(NET_FW_IP_PROTOCOL_UDP.0 as i32)
            .map_err(|e| {
                NetworkError::ConnectionFailed(format!("Failed to set protocol: {}", e))
            })?;
        rule.SetLocalPorts(&BSTR::from(port.to_string()))
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to set port: {}", e)))?;
        rule.SetAction(NET_FW_ACTION_ALLOW)
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to set action: {}", e)))?;
        rule.SetEnabled(windows::Win32::Foundation::VARIANT_TRUE)
            .map_err(|e| NetworkError::ConnectionFailed(format!("Failed to enable rule: {}", e)))?;

        // Add rule to firewall
        let rules = policy.Rules().map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to get firewall rules: {}", e))
        })?;
        rules.Add(&rule).map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to add firewall rule: {}", e))
        })?;
    }

    tracing::info!("Firewall rule created for UDP port {}", port);
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn remove_firewall_exemption(app_name: &str) -> Result<(), NetworkError> {
    use windows::core::BSTR;
    use windows::Win32::NetworkManagement::WindowsFirewall::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER).map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to create firewall policy: {}", e))
        })?
    };

    let rule_name = format!("{} - {}", FIREWALL_RULE_NAME, app_name);

    unsafe {
        let rules = policy.Rules().map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to get firewall rules: {}", e))
        })?;
        rules.Remove(&BSTR::from(&rule_name)).map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to remove firewall rule: {}", e))
        })?;
    }

    tracing::info!("Firewall rule removed for {}", app_name);
    Ok(())
}

#[cfg(target_os = "windows")]
fn check_firewall_rule(port: u16) -> Result<bool, NetworkError> {
    use windows::Win32::NetworkManagement::WindowsFirewall::*;
    use windows::Win32::System::Com::*;

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }

    let policy: INetFwPolicy2 = unsafe {
        CoCreateInstance(&NetFwPolicy2, None, CLSCTX_INPROC_SERVER).map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to create firewall policy: {}", e))
        })?
    };

    let port_str = port.to_string();

    unsafe {
        let rules = policy.Rules().map_err(|e| {
            NetworkError::ConnectionFailed(format!("Failed to get firewall rules: {}", e))
        })?;

        // Check if any rule exists that allows our port
        for rule in &rules {
            if let Ok(local_ports) = rule.LocalPorts() {
                if local_ports.to_string() == port_str {
                    if let Ok(action) = rule.Action() {
                        if action == NET_FW_ACTION_ALLOW {
                            if let Ok(protocol) = rule.Protocol() {
                                if protocol == NET_FW_IP_PROTOCOL_UDP.0 as i32 {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_firewall_exemption_noop() {
        // On non-Windows, this should be a no-op that succeeds
        #[cfg(not(target_os = "windows"))]
        {
            assert!(ensure_firewall_exemption(12345, "Toss").is_ok());
        }
    }

    #[test]
    fn test_remove_firewall_exemption_noop() {
        #[cfg(not(target_os = "windows"))]
        {
            assert!(remove_firewall_exemption("Toss").is_ok());
        }
    }

    #[test]
    fn test_is_port_allowed_noop() {
        #[cfg(not(target_os = "windows"))]
        {
            assert!(is_port_allowed(12345));
        }
    }
}
