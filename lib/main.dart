import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:window_manager/window_manager.dart';
import 'pages/home_page.dart';
import 'services/theme_service.dart';
import 'src/rust/frb_generated.dart';
import 'theme/app_theme.dart';

// ============================================================
// 简单日志系统 —— 带级别控制
// ============================================================

enum _LogLevel { debug, info, warn, error }

_LogLevel _currentLogLevel() {
  final env = Platform.environment['CMDHUB_LOG'];
  if (env != null) {
    switch (env.toLowerCase()) {
      case 'debug': return _LogLevel.debug;
      case 'info':  return _LogLevel.info;
      case 'warn':  return _LogLevel.warn;
      case 'error': return _LogLevel.error;
    }
  }
  if (kDebugMode) return _LogLevel.debug;
  return _LogLevel.warn;
}

bool _logEnabled(_LogLevel level) => level.index >= _currentLogLevel().index;

String _logPrefix(_LogLevel level) {
  switch (level) {
    case _LogLevel.debug: return '[DEBUG]';
    case _LogLevel.info:  return '[INFO]';
    case _LogLevel.warn:  return '[WARN]';
    case _LogLevel.error: return '[ERROR]';
  }
}

String _logDir() {
  if (Platform.isWindows) {
    final exePath = Platform.resolvedExecutable;
    final exeDir = File(exePath).parent.path;
    return '$exeDir\\data';
  } else {
    return '${Platform.environment['HOME'] ?? '.'}/.cmdhub';
  }
}

void _writeLog(_LogLevel level, String msg) {
  if (!_logEnabled(level)) return;
  try {
    final dir = _logDir();
    final file = File('$dir${Platform.pathSeparator}cmdhub.log');
    file.parent.createSync(recursive: true);
    file.writeAsStringSync(
      '${_logPrefix(level)} $msg\n',
      mode: FileMode.append,
    );
  } catch (_) {}
}

String _cmdhubDllPath() {
  final exeDir = File(Platform.resolvedExecutable).parent.path;
  return '$exeDir${Platform.pathSeparator}cmdhub_core.dll';
}

// ============================================================
// macOS 文本编辑快捷键通道
// ============================================================

const _textEditChannel = MethodChannel('com.cmdhub/text_edit');

void _initTextEditChannel() {
  _textEditChannel.setMethodCallHandler((call) async {
    // 先找 EditableTextState（TextField/TextFormField 的状态）
    final editableState = _findEditableTextState();

    // 如果当前焦点在 EditableText 上，执行文本编辑操作
    if (editableState != null) {
      switch (call.method) {
        case 'copy':
          await _handleTextEditCopy(editableState);
        case 'paste':
          await _handleTextEditPaste(editableState);
        case 'cut':
          await _handleTextEditCut(editableState);
        case 'selectAll':
          _handleTextEditSelectAll(editableState);
      }
      return;
    }

    // 焦点不在 EditableText 上，事件已被 Swift 消费。
    // 如果当前焦点在终端上，重新发送对应的事件给终端。
    // 终端的快捷键处理在 InstanceDetailPage 中，通过全局标记传递。
    // 这里不做任何操作，终端的 onKeyEvent 会在 Flutter key event 系统中
    // 独立处理（但 Swift 拦截了 keyDown，终端收不到）。
    //
    // 解决方案：Swift 侧只在有文本输入焦点时才消费事件。
  });
}

void _handleTextEditSelectAll(EditableTextState state) {
  final controller = state.widget.controller!;
  controller.selection = TextSelection(
    baseOffset: 0,
    extentOffset: controller.text.length,
  );
}

Future<void> _handleTextEditCopy(EditableTextState state) async {
  final controller = state.widget.controller!;
  final value = controller.value;
  if (value.selection.isValid && !value.selection.isCollapsed) {
    final sel = value.selection;
    final selected = value.text.substring(sel.start, sel.end);
    await Clipboard.setData(ClipboardData(text: selected));
  }
}

Future<void> _handleTextEditCut(EditableTextState state) async {
  final controller = state.widget.controller!;
  final value = controller.value;
  if (value.selection.isValid && !value.selection.isCollapsed) {
    final sel = value.selection;
    final selected = value.text.substring(sel.start, sel.end);
    await Clipboard.setData(ClipboardData(text: selected));
    final newText = value.text.substring(0, sel.start) + value.text.substring(sel.end);
    controller.value = TextEditingValue(
      text: newText,
      selection: TextSelection.collapsed(offset: sel.start),
    );
  }
}

Future<void> _handleTextEditPaste(EditableTextState state) async {
  final data = await Clipboard.getData(Clipboard.kTextPlain);
  if (data?.text == null) return;
  final controller = state.widget.controller!;
  final value = controller.value;
  final sel = value.selection;
  final newText = value.text.substring(0, sel.start) +
      data!.text! +
      value.text.substring(sel.end);
  controller.value = TextEditingValue(
    text: newText,
    selection: TextSelection.collapsed(offset: sel.start + data.text!.length),
  );
}

EditableTextState? _findEditableTextState() {
  final primary = FocusManager.instance.primaryFocus;
  if (primary == null) return null;
  return primary.context?.findAncestorStateOfType<EditableTextState>();
}

// ============================================================
// App 入口
// ============================================================

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  _writeLog(_LogLevel.info, 'CmdHub starting...');
  _writeLog(_LogLevel.debug, 'log dir: ${_logDir()}');

  // 初始化窗口管理器
  await windowManager.ensureInitialized();
  _writeLog(_LogLevel.debug, 'window_manager init ok');

  final windowOptions = WindowOptions(
    size: const Size(1280, 800),
    center: true,
    minimumSize: const Size(900, 600),
    skipTaskbar: false,
    titleBarStyle: TitleBarStyle.normal,
  );
  windowManager.waitUntilReadyToShow(windowOptions, () async {
    await windowManager.show();
    await windowManager.focus();
    await windowManager.setPreventClose(true);
  });
  _writeLog(_LogLevel.debug, 'window options set');

  // 初始化 Rust 桥接
  try {
    final library = Platform.isWindows
        ? ExternalLibrary.open(_cmdhubDllPath())
        : ExternalLibrary.process(iKnowHowToUseIt: true);
    await RustLib.init(externalLibrary: library);
  } catch (e) {
    _writeLog(_LogLevel.error, 'Rust init failed: $e');
    rethrow;
  }
  _writeLog(_LogLevel.info, 'Rust init OK');

  // 初始化主题服务
  final themeService = await ThemeService.init();
  _writeLog(_LogLevel.debug, 'theme: ${themeService.modeLabel}');

  // 初始化 macOS 文本编辑快捷键通道
  if (Platform.isMacOS) {
    _initTextEditChannel();
  }

  runApp(const CmdHubApp());
}

class CmdHubApp extends StatefulWidget {
  const CmdHubApp({super.key});

  @override
  State<CmdHubApp> createState() => _CmdHubAppState();
}

class _CmdHubAppState extends State<CmdHubApp> {
  late final ThemeService _themeService = ThemeService.instance;

  @override
  void initState() {
    super.initState();
    _themeService.addListener(_onThemeChanged);
  }

  @override
  void dispose() {
    _themeService.removeListener(_onThemeChanged);
    super.dispose();
  }

  void _onThemeChanged() => setState(() {});

  @override
  Widget build(BuildContext context) {
    _writeLog(_LogLevel.debug, 'Flutter app build');
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'CmdHub',
      theme: AppTheme.lightTheme(),
      darkTheme: AppTheme.darkTheme(),
      themeMode: _themeService.mode,
      home: const HomePage(),
    );
  }
}
