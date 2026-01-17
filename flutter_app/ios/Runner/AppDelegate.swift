import Flutter
import UIKit
import AVFoundation
import Network

@main
@objc class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    GeneratedPluginRegistrant.register(with: self)

    // Set up method channel for iOS permissions
    if let controller = window?.rootViewController as? FlutterViewController {
      let permissionsChannel = FlutterMethodChannel(
        name: "toss.app/permissions",
        binaryMessenger: controller.binaryMessenger
      )

      permissionsChannel.setMethodCallHandler { [weak self] (call: FlutterMethodCall, result: @escaping FlutterResult) in
        switch call.method {
        case "checkCameraPermission":
          let status = AVCaptureDevice.authorizationStatus(for: .video)
          result(status == .authorized)

        case "requestCameraPermission":
          AVCaptureDevice.requestAccess(for: .video) { granted in
            DispatchQueue.main.async {
              result(granted)
            }
          }

        case "checkLocalNetworkPermission":
          // iOS local network permission is checked via NWBrowser
          // Return true as actual check happens when using network
          result(true)

        case "openSettings":
          if let settingsUrl = URL(string: UIApplication.openSettingsURLString) {
            UIApplication.shared.open(settingsUrl) { success in
              result(success)
            }
          } else {
            result(false)
          }

        case "checkClipboardAccess":
          // iOS clipboard access is always available (unlike macOS)
          result(true)

        case "requestClipboardAccess":
          // No special permission needed for clipboard on iOS
          result(true)

        case "openAccessibilitySettings":
          // iOS doesn't have accessibility permissions for clipboard
          result(false)

        default:
          result(FlutterMethodNotImplemented)
        }
      }
    }

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }
}
