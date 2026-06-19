import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';
import '../services/cmdhub_service.dart';

/// macOS 原生 Ctrl 快捷键路由
///
/// 当 hardwareKeyboardOnly = false 时（IME 启用），Ctrl+C/D/Z 等被
/// NSTextInputContext 拦截，不会到达 Flutter 的按键事件系统。
///
/// 本 Service 在原生层（AppDelegate）通过 localEventMonitor 拦截 Ctrl+[A-Z]，
/// 通过 MethodChannel 转发控制码，然后写入当前激活的终端实例。
class KeyboardService {
  static final KeyboardService _instance = KeyboardService._();
  factory KeyboardService() => _instance;
  KeyboardService._();

  static const _channel = MethodChannel('com.cmdhub/keyboard');

  String? _activeInstanceId;
  CmdHubService? _service;
  TerminalController? _terminalController;
  Terminal? _terminal;

  /// 注册当前在前台的终端实例。页面 initState 时调用。
  void register(
    String instanceId,
    CmdHubService service, {
    TerminalController? terminalController,
    Terminal? terminal,
  }) {
    _activeInstanceId = instanceId;
    _service = service;
    _terminalController = terminalController;
    _terminal = terminal;
  }

  /// 取消注册。页面 dispose 时调用。
  void unregister() {
    _activeInstanceId = null;
    _service = null;
    _terminalController = null;
    _terminal = null;
  }

  /// 初始化 MethodChannel 监听（全局调用一次）
  void init() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'ctrlSequence') {
        final code = call.arguments as int;
        if (_activeInstanceId != null && _service != null) {
          await _service!.writeInput(_activeInstanceId!, String.fromCharCode(code));
        }
      } else if (call.method == 'cmdAction') {
        await _handleCmdAction(call.arguments as String);
      }
    });
  }

  Future<void> _handleCmdAction(String action) async {
    if (_activeInstanceId == null || _service == null) return;

    switch (action) {
      case 'copy':
        final sel = _terminalController?.selection;
        if (sel != null && _terminal != null) {
          final text = _terminal!.buffer.getText(sel);
          if (text.isNotEmpty) {
            await Clipboard.setData(ClipboardData(text: text));
            return;
          }
        }
        // 无选区时发送 SIGINT
        await _service!.writeInput(_activeInstanceId!, '\x03');
      case 'paste':
        final data = await Clipboard.getData(Clipboard.kTextPlain);
        if (data?.text != null) {
          await _service!.writeInput(_activeInstanceId!, data!.text!);
        }
    }
  }

  void dispose() {
    _channel.setMethodCallHandler(null);
    _activeInstanceId = null;
    _service = null;
    _terminalController = null;
    _terminal = null;
  }
}
