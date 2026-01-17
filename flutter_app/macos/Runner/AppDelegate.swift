import Cocoa
import FlutterMacOS
import ApplicationServices

@main
class AppDelegate: FlutterAppDelegate {
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return true
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }

  override func applicationDidFinishLaunching(_ notification: Notification) {
    super.applicationDidFinishLaunching(notification)

    // Set up method channel for auto-start
    guard let controller = NSApplication.shared.windows.first?.contentViewController as? FlutterViewController else {
      return
    }

    let autoStartChannel = FlutterMethodChannel(
      name: "toss.app/auto_start",
      binaryMessenger: controller.engine.binaryMessenger
    )

    autoStartChannel.setMethodCallHandler { (call: FlutterMethodCall, result: @escaping FlutterResult) in
      switch call.method {
      case "enableAutoStart":
        let success = AutoStart.setAutoStart(true)
        result(success)
      case "disableAutoStart":
        let success = AutoStart.setAutoStart(false)
        result(success)
      case "isAutoStartEnabled":
        let enabled = AutoStart.isAutoStartEnabled()
        result(enabled)
      default:
        result(FlutterMethodNotImplemented)
      }
    }

    // Set up method channel for accessibility permissions
    let permissionsChannel = FlutterMethodChannel(
      name: "toss.app/permissions",
      binaryMessenger: controller.engine.binaryMessenger
    )

    permissionsChannel.setMethodCallHandler { (call: FlutterMethodCall, result: @escaping FlutterResult) in
      switch call.method {
      case "checkAccessibilityPermission":
        // Check if the app has accessibility permissions using AXIsProcessTrusted
        let isTrusted = AXIsProcessTrusted()
        result(isTrusted)

      case "requestAccessibilityPermission":
        // Request accessibility permission - this will prompt the user if not already granted
        // The options dictionary with kAXTrustedCheckOptionPrompt will show the system prompt
        let options = [kAXTrustedCheckOptionPrompt.takeRetainedValue() as String: true] as CFDictionary
        let isTrusted = AXIsProcessTrustedWithOptions(options)
        result(isTrusted)

      case "openAccessibilitySettings":
        // Open System Preferences/Settings to the Accessibility Privacy pane
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
          NSWorkspace.shared.open(url)
          result(true)
        } else {
          result(false)
        }

      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }
}
