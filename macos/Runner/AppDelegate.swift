import Cocoa
import FlutterMacOS

@main
class AppDelegate: FlutterAppDelegate {
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return false
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }

  override func applicationDidFinishLaunching(_ notification: Notification) {
    guard let window = NSApplication.shared.windows.first(where: { $0.contentViewController is FlutterViewController }),
          let controller = window.contentViewController as? FlutterViewController else {
      return
    }

    let channel = FlutterMethodChannel(
      name: "com.cmdhub/dock",
      binaryMessenger: controller.engine.binaryMessenger
    )

    channel.setMethodCallHandler { (call: FlutterMethodCall, result: @escaping FlutterResult) in
      if call.method == "setDockVisibility" {
        if let args = call.arguments as? [String: Bool],
           let visible = args["visible"] {
          if visible {
            NSApp.setActivationPolicy(.regular)
          } else {
            NSApp.setActivationPolicy(.accessory)
          }
        }
        result(nil)
      } else {
        result(FlutterMethodNotImplemented)
      }
    }
  }
}
