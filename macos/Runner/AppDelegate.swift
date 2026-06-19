import Cocoa
import FlutterMacOS

@main
class AppDelegate: FlutterAppDelegate {
  var _keyboardChannel: FlutterMethodChannel?

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

    // 键盘快捷键通道（Ctrl+[A-Z] → Dart）
    let channel = FlutterMethodChannel(
      name: "com.cmdhub/keyboard",
      binaryMessenger: controller.engine.binaryMessenger
    )
    _keyboardChannel = channel

    // 本地事件监视器：在 NSTextInputContext 之前拦截 Ctrl 组合键
    NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak channel] event in
      guard let channel = channel else { return event }

      let ctrlPressed = event.modifierFlags.contains(.control)
      if ctrlPressed,
         let chars = event.charactersIgnoringModifiers,
         chars.count == 1,
         let char = chars.lowercased().first,
         char >= "a" && char <= "z" {

        // Ctrl+A = 1, Ctrl+B = 2, ..., Ctrl+Z = 26
        let code = Int(char.asciiValue!) - 96
        channel.invokeMethod("ctrlSequence", arguments: code)
        return nil // 吞掉事件，不让它进入 NSTextInputContext
      }

      // Cmd+C → 通知 Dart 端（不吞事件，菜单系统也会处理）
      let metaPressed = event.modifierFlags.contains(.command)
      if metaPressed {
        if let chars = event.charactersIgnoringModifiers {
          if chars == "c" {
            channel.invokeMethod("cmdAction", arguments: "copy")
            return event
          }
          if chars == "v" {
            channel.invokeMethod("cmdAction", arguments: "paste")
            return event
          }
        }
      }

      return event // 非 Ctrl 事件正常传递
    }

    // Dock 可见性通道
    let dockChannel = FlutterMethodChannel(
      name: "com.cmdhub/dock",
      binaryMessenger: controller.engine.binaryMessenger
    )

    dockChannel.setMethodCallHandler { (call: FlutterMethodCall, result: @escaping FlutterResult) in
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
