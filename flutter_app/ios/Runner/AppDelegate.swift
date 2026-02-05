import Flutter
import UIKit
import AVFoundation
import Network
import BackgroundTasks
import Intents

@main
@objc class AppDelegate: FlutterAppDelegate {
  private var backgroundChannel: FlutterMethodChannel?

  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    GeneratedPluginRegistrant.register(with: self)

    if let controller = window?.rootViewController as? FlutterViewController {
      // Set up method channel for iOS permissions
      setupPermissionsChannel(controller: controller)

      // Set up method channel for iOS background service
      setupBackgroundServiceChannel(controller: controller)
    }

    // Register background tasks
    registerBackgroundTasks()

    // Set minimum background fetch interval
    application.setMinimumBackgroundFetchInterval(UIApplication.backgroundFetchIntervalMinimum)

    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  // MARK: - Permissions Channel

  private func setupPermissionsChannel(controller: FlutterViewController) {
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

  // MARK: - Background Service Channel

  private func setupBackgroundServiceChannel(controller: FlutterViewController) {
    backgroundChannel = FlutterMethodChannel(
      name: "toss.ios.background",
      binaryMessenger: controller.binaryMessenger
    )

    backgroundChannel?.setMethodCallHandler { [weak self] (call: FlutterMethodCall, result: @escaping FlutterResult) in
      switch call.method {
      case "initialize":
        // Background service initialization
        result(true)

      case "registerShortcut":
        if let args = call.arguments as? [String: Any],
           let actionId = args["actionId"] as? String,
           let title = args["title"] as? String {
          self?.registerShortcut(actionId: actionId, title: title, result: result)
        } else {
          result(false)
        }

      case "shortcutActionCompleted":
        // Acknowledge shortcut completion
        result(true)

      case "updateWidget":
        // Widget update would go here (requires WidgetKit extension)
        result(true)

      case "setupExtension":
        // App extension setup would go here
        result(true)

      case "getIOSVersion":
        let version = ProcessInfo.processInfo.operatingSystemVersion.majorVersion
        result(version)

      case "requestClipboardAccess":
        // Reading clipboard triggers the iOS notification automatically
        // Just return true as access is always granted
        result(true)

      case "configureBackgroundFetch":
        if let args = call.arguments as? [String: Any],
           let seconds = args["minimumIntervalSeconds"] as? Int {
          let interval = TimeInterval(seconds)
          UIApplication.shared.setMinimumBackgroundFetchInterval(interval)
          result(true)
        } else {
          result(false)
        }

      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }

  // MARK: - Background Tasks

  private func registerBackgroundTasks() {
    // Register background task for clipboard refresh
    BGTaskScheduler.shared.register(
      forTaskWithIdentifier: "com.toss.clipboard.refresh",
      using: nil
    ) { [weak self] task in
      self?.handleBackgroundRefresh(task: task as! BGAppRefreshTask)
    }
  }

  private func handleBackgroundRefresh(task: BGAppRefreshTask) {
    // Schedule next background refresh
    scheduleBackgroundRefresh()

    // Notify Flutter side about background fetch
    DispatchQueue.main.async { [weak self] in
      self?.backgroundChannel?.invokeMethod("onBackgroundFetch", arguments: nil) { result in
        task.setTaskCompleted(success: result != nil)
      }
    }

    // Handle task expiration
    task.expirationHandler = {
      task.setTaskCompleted(success: false)
    }
  }

  private func scheduleBackgroundRefresh() {
    let request = BGAppRefreshTaskRequest(identifier: "com.toss.clipboard.refresh")
    request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 minutes

    do {
      try BGTaskScheduler.shared.submit(request)
    } catch {
      print("Failed to schedule background refresh: \(error)")
    }
  }

  // MARK: - Shortcuts

  private func registerShortcut(actionId: String, title: String, result: @escaping FlutterResult) {
    // Create user activity for Siri Shortcuts
    let activity = NSUserActivity(activityType: "com.toss.clipboard.\(actionId)")
    activity.title = title
    activity.isEligibleForSearch = true
    activity.isEligibleForPrediction = true
    activity.persistentIdentifier = actionId

    // Suggest the shortcut to Siri
    activity.suggestedInvocationPhrase = title

    // Make the activity current
    activity.becomeCurrent()

    result(true)
  }

  // MARK: - App Lifecycle

  override func applicationDidBecomeActive(_ application: UIApplication) {
    super.applicationDidBecomeActive(application)

    // Notify Flutter that app became active
    backgroundChannel?.invokeMethod("onAppDidBecomeActive", arguments: nil)
  }

  override func applicationWillResignActive(_ application: UIApplication) {
    super.applicationWillResignActive(application)

    // Notify Flutter that app will resign active
    backgroundChannel?.invokeMethod("onAppWillResignActive", arguments: nil)

    // Schedule background refresh when going to background
    scheduleBackgroundRefresh()
  }

  // MARK: - Background Fetch (Legacy)

  override func application(
    _ application: UIApplication,
    performFetchWithCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void
  ) {
    // Legacy background fetch support
    backgroundChannel?.invokeMethod("onBackgroundFetch", arguments: nil) { result in
      if let dict = result as? [String: Any],
         let success = dict["success"] as? Bool,
         let newData = dict["newData"] as? Bool {
        if success {
          completionHandler(newData ? .newData : .noData)
        } else {
          completionHandler(.failed)
        }
      } else {
        completionHandler(.noData)
      }
    }
  }

  // MARK: - User Activity (Shortcuts)

  override func application(
    _ application: UIApplication,
    continue userActivity: NSUserActivity,
    restorationHandler: @escaping ([UIUserActivityRestoring]?) -> Void
  ) -> Bool {
    // Handle Siri Shortcut invocation
    if userActivity.activityType.hasPrefix("com.toss.clipboard.") {
      let actionId = userActivity.persistentIdentifier ??
                     userActivity.activityType.replacingOccurrences(of: "com.toss.clipboard.", with: "")

      backgroundChannel?.invokeMethod("onShortcutAction", arguments: ["actionId": actionId])
      return true
    }

    return super.application(application, continue: userActivity, restorationHandler: restorationHandler)
  }
}
