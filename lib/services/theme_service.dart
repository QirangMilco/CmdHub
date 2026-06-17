import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

/// 主题模式管理 —— 持久化到 shared_preferences
class ThemeService extends ChangeNotifier {
  static const _key = 'theme_mode';
  static ThemeService? _instance;

  ThemeMode _mode = ThemeMode.system;

  ThemeMode get mode => _mode;

  /// 初始化后可通过此静态 getter 全局访问
  static ThemeService get instance {
    assert(_instance != null, 'ThemeService.init() must be called first');
    return _instance!;
  }

  static Future<ThemeService> init() async {
    if (_instance != null) return _instance!;
    final prefs = await SharedPreferences.getInstance();
    final value = prefs.getString(_key) ?? 'system';
    final mode = _parse(value);
    _instance = ThemeService._(mode);
    return _instance!;
  }

  ThemeService._(this._mode);

  static ThemeMode _parse(String v) {
    switch (v) {
      case 'light': return ThemeMode.light;
      case 'dark':  return ThemeMode.dark;
      default:      return ThemeMode.system;
    }
  }

  Future<void> setMode(ThemeMode mode) async {
    if (_mode == mode) return;
    _mode = mode;
    notifyListeners();

    final prefs = await SharedPreferences.getInstance();
    final value = switch (mode) {
      ThemeMode.light  => 'light',
      ThemeMode.dark   => 'dark',
      ThemeMode.system => 'system',
    };
    await prefs.setString(_key, value);
  }

  /// 下一个模式（循环）
  ThemeMode nextMode() => switch (_mode) {
    ThemeMode.light  => ThemeMode.dark,
    ThemeMode.dark   => ThemeMode.light,
    ThemeMode.system => ThemeMode.light,
  };

  String get modeLabel => switch (_mode) {
    ThemeMode.light  => '浅色',
    ThemeMode.dark   => '深色',
    ThemeMode.system => '跟随系统',
  };
}
