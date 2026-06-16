import 'package:flutter/material.dart';

/// CmdHub 主题 — 桌面端开发者工具风格
/// 主色：靛蓝 accent，灰白背景，克制、安静、专业
class AppTheme {
  // 强调色 — 靛蓝，用于按钮、选中态、图标高亮
  static const Color accent = Color(0xFF6366F1);
  static const Color accentLight = Color(0xFF818CF8);
  static const Color accentDark = Color(0xFF4F46E5);

  // 字体
  static const String uiFont = 'NotoSansSC';
  static const String monoFont = 'JetBrainsMono';

  // 状态色 — 低饱和度，融入灰调
  static const Color success = Color(0xFF10B981);
  static const Color warning = Color(0xFFF59E0B);
  static const Color error = Color(0xFFEF4444);

  // 日间模式
  static const Color lightBg = Color(0xFFF8FAFC);       // 页面背景
  static const Color lightSurface = Color(0xFFFFFFFF);   // 内容区
  static const Color lightNav = Color(0xFFF1F5F9);     // 导航栏
  static const Color lightCard = Color(0xFFFFFFFF);      // 卡片
  static const Color lightHover = Color(0xFFF1F5F9);   // 悬停背景
  static const Color lightText = Color(0xFF1E293B);     // 主文字
  static const Color lightTextSecondary = Color(0xFF64748B); // 次要文字
  static const Color lightTextMuted = Color(0xFF94A3B8);     // 更弱文字
  static const Color lightDivider = Color(0xFFE2E8F0); // 分隔线
  static const Color lightBorder = Color(0xFFE2E8F0);  // 边框

  // 暗黑模式
  static const Color darkBg = Color(0xFF0F172A);
  static const Color darkSurface = Color(0xFF1E293B);
  static const Color darkNav = Color(0xFF1E293B);
  static const Color darkCard = Color(0xFF334155);
  static const Color darkHover = Color(0xFF334155);
  static const Color darkText = Color(0xFFF1F5F9);
  static const Color darkTextSecondary = Color(0xFF94A3B8);
  static const Color darkTextMuted = Color(0xFF64748B);
  static const Color darkDivider = Color(0xFF334155);
  static const Color darkBorder = Color(0xFF334155);

  static ThemeData lightTheme() => _buildTheme(Brightness.light);
  static ThemeData darkTheme() => _buildTheme(Brightness.dark);

  static ThemeData _buildTheme(Brightness brightness) {
    final isDark = brightness == Brightness.dark;

    return ThemeData(
      useMaterial3: true,
      brightness: brightness,
      fontFamily: 'NotoSansSC',
      scaffoldBackgroundColor: isDark ? darkBg : lightBg,
      cardColor: isDark ? darkCard : lightCard,
      dividerColor: isDark ? darkDivider : lightDivider,
      splashColor: Colors.transparent,
      highlightColor: Colors.transparent,

      appBarTheme: const AppBarTheme(
        backgroundColor: Colors.transparent,
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: false,
      ),

      cardTheme: CardThemeData(
        color: isDark ? darkCard : lightCard,
        elevation: 0,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(8),
        ),
      ),

      elevatedButtonTheme: ElevatedButtonThemeData(
        style: ElevatedButton.styleFrom(
          backgroundColor: accent,
          foregroundColor: Colors.white,
          elevation: 0,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
          textStyle: const TextStyle(fontWeight: FontWeight.w500, fontSize: 13),
        ),
      ),

      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          foregroundColor: accent,
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
        ),
      ),

      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: isDark ? darkSurface : lightSurface,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: isDark ? darkBorder : lightBorder),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: BorderSide(color: isDark ? darkBorder : lightBorder),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(6),
          borderSide: const BorderSide(color: accent, width: 1.5),
        ),
        contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        hintStyle: TextStyle(
          fontSize: 13,
          color: isDark ? darkTextMuted : lightTextMuted,
        ),
      ),

      colorScheme: ColorScheme.fromSeed(
        seedColor: accent,
        brightness: brightness,
        surface: isDark ? darkSurface : lightSurface,
      ),
    );
  }

  // 便捷方法
  static Color bg(bool isDark) => isDark ? darkBg : lightBg;
  static Color surface(bool isDark) => isDark ? darkSurface : lightSurface;
  static Color nav(bool isDark) => isDark ? darkNav : lightNav;
  static Color card(bool isDark) => isDark ? darkCard : lightCard;
  static Color text(bool isDark) => isDark ? darkText : lightText;
  static Color textSecondary(bool isDark) => isDark ? darkTextSecondary : lightTextSecondary;
  static Color textMuted(bool isDark) => isDark ? darkTextMuted : lightTextMuted;
  static Color divider(bool isDark) => isDark ? darkDivider : lightDivider;
  static Color border(bool isDark) => isDark ? darkBorder : lightBorder;
  static Color hover(bool isDark) => isDark ? darkHover : lightHover;

  // 导航栏阴影
  static List<BoxShadow> navShadow(bool isDark) => [
    BoxShadow(
      color: Colors.black.withOpacity(isDark ? 0.2 : 0.04),
      blurRadius: 8,
      offset: const Offset(2, 0),
    ),
  ];
}
