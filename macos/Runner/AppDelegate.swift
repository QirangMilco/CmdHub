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

    // 键盘快捷键通道
    let channel = FlutterMethodChannel(
      name: "com.cmdhub/keyboard",
      binaryMessenger: controller.engine.binaryMessenger
    )
    _keyboardChannel = channel

    // 本地事件监视器：拦截 Ctrl 和 Cmd 组合键
    NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak channel] event in
      guard let channel = channel else { return event }

      // Ctrl+[A-Z] → 控制字符，吞掉不让 NSTextInputContext 拦截
      let ctrlPressed = event.modifierFlags.contains(.control)
      if ctrlPressed,
         let chars = event.charactersIgnoringModifiers,
         chars.count == 1,
         let char = chars.lowercased().first,
         char >= "a" && char <= "z" {
        let code = Int(char.asciiValue!) - 96
        channel.invokeMethod("ctrlSequence", arguments: code)
        return nil
      }

      // Cmd+C/V → 吞掉不让菜单系统拦截，转发到 Dart 统一处理（TextField + 终端）
      let metaPressed = event.modifierFlags.contains(.command)
      if metaPressed,
         let chars = event.charactersIgnoringModifiers?.lowercased(),
         chars == "c" || chars == "v" {
        channel.invokeMethod("cmdAction", arguments: chars == "c" ? "copy" : "paste")
        return nil
      }

      return event
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
