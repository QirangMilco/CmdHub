import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';
import '../services/cmdhub_service.dart';

/// macOS 快捷键路由（Ctrl + Cmd 组合键）
///
/// 当 hardwareKeyboardOnly = false 时（IME 启用），Ctrl 和 Cmd 快捷键会被
/// NSTextInputContext / 菜单系统拦截，不到达 Flutter 的按键事件系统。
///
/// 本 Service 通过 AppDelegate 的本地事件监视器拦截这些组合键，
/// 经由 MethodChannel 转发到 Dart，然后路由到当前激活的终端或 TextField。
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
    // 焦点在 TextField 上 → 交由 EditableText 处理
    final editable = _findEditableTextState();
    if (editable != null) {
      _handleTextEditAction(editable, action);
      return;
    }

    // 焦点在终端上
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
        break;
      case 'paste':
        final data = await Clipboard.getData(Clipboard.kTextPlain);
        final text = data?.text;
        if (text != null && text.isNotEmpty) {
          await _service!.writeInput(_activeInstanceId!, text);
        }
    }
  }

  void _handleTextEditAction(EditableTextState state, String action) {
    final controller = state.widget.controller;

    switch (action) {
      case 'copy':
        final value = controller.value;
        if (value.selection.isValid && !value.selection.isCollapsed) {
          final sel = value.selection;
          final selected = value.text.substring(sel.start, sel.end);
          Clipboard.setData(ClipboardData(text: selected));
        }
        break;
      case 'paste':
        Clipboard.getData(Clipboard.kTextPlain).then((data) {
          final text = data?.text;
          if (text == null || text.isEmpty) return;
          final value = controller.value;
          final sel = value.selection;
          final newText = value.text.substring(0, sel.start) +
              text +
              value.text.substring(sel.end);
          controller.value = TextEditingValue(
            text: newText,
            selection: TextSelection.collapsed(
              offset: sel.start + text.length,
            ),
          );
        });
    }
  }

  EditableTextState? _findEditableTextState() {
    final primary = FocusManager.instance.primaryFocus;
    if (primary == null) return null;
    return primary.context?.findAncestorStateOfType<EditableTextState>();
  }

  void dispose() {
    _channel.setMethodCallHandler(null);
    _activeInstanceId = null;
    _service = null;
    _terminalController = null;
    _terminal = null;
  }
}
