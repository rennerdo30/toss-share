import Foundation
import ServiceManagement

class AutoStart {
    static let bundleIdentifier = Bundle.main.bundleIdentifier ?? "com.toss.app"
    
    static func setAutoStart(_ enabled: Bool) -> Bool {
        let appURL = Bundle.main.bundleURL
        
        if enabled {
            // Register login item
            return SMLoginItemSetEnabled(bundleIdentifier as CFString, true)
        } else {
            // Unregister login item
            return SMLoginItemSetEnabled(bundleIdentifier as CFString, false)
        }
    }
    
    static func isAutoStartEnabled() -> Bool {
        // Check if login item is registered
        var snapshot: Unmanaged<CFArray>?
        let status = SMCopyAllJobDictionaries(kSMDomainUserLaunchd, &snapshot)
        
        guard status == errSecSuccess, let jobs = snapshot?.takeRetainedValue() as? [[String: Any]] else {
            return false
        }
        
        for job in jobs {
            if let label = job["Label"] as? String, label == bundleIdentifier {
                return true
            }
        }
        
        return false
    }
}
