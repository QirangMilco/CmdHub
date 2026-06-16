import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'pages/home_page.dart';
import 'src/rust/frb_generated.dart';
import 'theme/app_theme.dart';

// ============================================================
// 简单日志系统 —— 带级别控制
// ============================================================

/// debug 编译输出 Info 及以上，release 只输出 Warn 及以上
/// 环境变量 CMDBUB_LOG=debug/info/warn/error 可覆盖
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
  } catch (_) {
    // 日志不可用时不干扰主流程
  }
}

// ============================================================
// App 入口
// ============================================================

String _cmdhubDllPath() {
  // cmdhub_core.dll 应与 exe 同级
  final exeDir = File(Platform.resolvedExecutable).parent.path;
  return '$exeDir${Platform.pathSeparator}cmdhub_core.dll';
}

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  _writeLog(_LogLevel.info, 'CmdHub starting...');
  _writeLog(_LogLevel.debug, 'log dir: ${_logDir()}');

  try {
    // macOS/Linux：静态链接，从进程查找符号
    // Windows：cdylib 生成 cmdhub_core.dll，从文件加载
    final library = Platform.isWindows
        ? ExternalLibrary.open(_cmdhubDllPath())
        : ExternalLibrary.process(iKnowHowToUseIt: true);
    await RustLib.init(externalLibrary: library);
  } catch (e) {
    _writeLog(_LogLevel.error, 'Rust init failed: $e');
    rethrow;
  }

  _writeLog(_LogLevel.info, 'Rust init OK');
  runApp(const CmdHubApp());
}

class CmdHubApp extends StatelessWidget {
  const CmdHubApp({super.key});

  @override
  Widget build(BuildContext context) {
    _writeLog(_LogLevel.debug, 'Flutter app build');
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      title: 'CmdHub',
      theme: AppTheme.lightTheme(),
      darkTheme: AppTheme.darkTheme(),
      home: const HomePage(),
    );
  }
}
