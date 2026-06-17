import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
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
