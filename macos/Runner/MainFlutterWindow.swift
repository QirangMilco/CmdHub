import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  private var keyMonitor: Any?
  private var textEditChannel: FlutterMethodChannel?

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)

    super.awakeFromNib()

    textEditChannel = FlutterMethodChannel(
      name: "com.cmdhub/text_edit",
      binaryMessenger: flutterViewController.engine.binaryMessenger
    )

    // 在 app 层拦截 Cmd+C/V/X/A。
    //
    // Flutter macOS 上 NSTextInputContext 会拦截这些 keyEquivalent 事件但不正确
    // 处理它们（FlutterTextInputPlugin 不实现 copy:/paste: selector），导致菜单
    // 项变灰、快捷键完全失效。
    //
    // 只在 firstResponder 是 NSTextInputClient（即 TextField 获得焦点）时拦截，
    // 通过 MethodChannel 通知 Dart 侧直接操作 EditableTextState。
    // 终端页面时 firstResponder 不是 NSTextInputClient，事件正常放行给 Flutter。
    keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
      guard let self = self else { return event }

      // 只在有文本输入焦点时拦截
      guard self.firstResponder is NSTextInputClient else { return event }

      let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
      guard flags.contains(.command) else { return event }

      // 不带其他修饰键的 Cmd+C/V/X/A 才拦截
      let otherModifiers: NSEvent.ModifierFlags = [.shift, .control, .option]
      guard flags.intersection(otherModifiers).isEmpty else { return event }

      let key = event.charactersIgnoringModifiers ?? ""

      switch key {
      case "c":
        self.textEditChannel?.invokeMethod("copy", arguments: nil)
        return nil
      case "v":
        self.textEditChannel?.invokeMethod("paste", arguments: nil)
        return nil
      case "x":
        self.textEditChannel?.invokeMethod("cut", arguments: nil)
        return nil
      case "a":
        self.textEditChannel?.invokeMethod("selectAll", arguments: nil)
        return nil
      default:
        return event
      }
    }
  }

  deinit {
    if let monitor = keyMonitor {
      NSEvent.removeMonitor(monitor)
    }
  }
}
