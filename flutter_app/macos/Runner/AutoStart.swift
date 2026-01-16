import Foundation
import ServiceManagement

class AutoStart {
    static let bundleIdentifier = Bundle.main.bundleIdentifier ?? "com.toss.app"

    static func setAutoStart(_ enabled: Bool) -> Bool {
        if #available(macOS 13.0, *) {
            // Use modern SMAppService API for macOS 13+
            do {
                if enabled {
                    try SMAppService.mainApp.register()
                } else {
                    try SMAppService.mainApp.unregister()
                }
                return true
            } catch {
                print("AutoStart: Failed to \(enabled ? "enable" : "disable"): \(error)")
                return false
            }
        } else {
            // Fallback for older macOS versions
            // Note: SMLoginItemSetEnabled is deprecated but still works on older systems
            return false
        }
    }

    static func isAutoStartEnabled() -> Bool {
        if #available(macOS 13.0, *) {
            return SMAppService.mainApp.status == .enabled
        } else {
            // Fallback for older macOS versions
            return false
        }
    }
}
