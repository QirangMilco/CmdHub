import 'package:flutter/material.dart';

/// ANSI 转义序列解析器
/// 将包含 ANSI 颜色码的文本转换为带样式的 TextSpan 列表

class AnsiParser {
  // 标准 ANSI 前景色 30-37
  static const _fgColors = {
    30: Color(0xFF1E1E1E), // 黑
    31: Color(0xFFCD3131), // 红
    32: Color(0xFF0DBC79), // 绿
    33: Color(0xFFE5E510), // 黄
    34: Color(0xFF2472C8), // 蓝
    35: Color(0xFFBC3FBC), // 品红
    36: Color(0xFF11A8CD), // 青
    37: Color(0xFFE5E5E5), // 白
  };

  // 亮色前景 90-97
  static const _brightFg = {
    90: Color(0xFF666666),
    91: Color(0xFFF14C4C),
    92: Color(0xFF23D18B),
    93: Color(0xFFF5F543),
    94: Color(0xFF3B8EEA),
    95: Color(0xFFD670D6),
    96: Color(0xFF29B8DB),
    97: Color(0xFFFFFFFF),
  };

  /// 解析文本，生成带样式的 TextSpan 列表
  static List<TextSpan> parse(String text, {required bool dark}) {
    if (!text.contains('\x1b[')) {
      return [TextSpan(text: text)];
    }

    final spans = <TextSpan>[];
    final buffer = StringBuffer();
    int i = 0;
    Color? fg;
    Color? bg;
    bool bold = false;

    while (i < text.length) {
      if (text[i] == '\x1b' && i + 1 < text.length && text[i + 1] == '[') {
        // 提取积累的普通文本
        if (buffer.isNotEmpty) {
          spans.add(_makeSpan(buffer.toString(), fg, bg, bold, dark));
          buffer.clear();
        }

        // 找到结束符
        int j = i + 2;
        while (j < text.length) {
          if (text[j] == 'm' || text[j] == 'K' || text[j] == 'J') {
            break;
          }
          j++;
        }

        if (j < text.length && text[j] == 'm') {
          // 解析 CSI 参数，如 "31;1;42"
          final params = text.substring(i + 2, j);
          for (final code in params.split(';')) {
            final n = int.tryParse(code);
            if (n == null) continue;
            switch (n) {
              case 0:
                fg = null; bg = null; bold = false;
              case 1:
                bold = true;
              case 22:
                bold = false;
              case 30: case 31: case 32: case 33:
              case 34: case 35: case 36: case 37:
                fg = _fgColors[n];
              case 90: case 91: case 92: case 93:
              case 94: case 95: case 96: case 97:
                fg = _brightFg[n];
              case 40: case 41: case 42: case 43:
              case 44: case 45: case 46: case 47:
                bg = _ansiBg(n, dark);
              case 100: case 101: case 102: case 103:
              case 104: case 105: case 106: case 107:
                bg = _brightBg(n);
            }
          }
        }
        // 跳过 CSI 序列
        i = j + 1;
      } else if (text[i] == '\r') {
        // 回车：忽略（在终端中表示回到行首，我们直接追加）
        i++;
      } else {
        buffer.write(text[i]);
        i++;
      }
    }

    // 剩余文本
    if (buffer.isNotEmpty) {
      spans.add(_makeSpan(buffer.toString(), fg, bg, bold, dark));
    }

    return spans;
  }

  static TextSpan _makeSpan(
    String text, Color? fg, Color? bg, bool bold, bool dark) {
    return TextSpan(
      text: text,
      style: TextStyle(
        color: fg ?? (dark ? const Color(0xFFD4D4D4) : const Color(0xFF1E1E1E)),
        backgroundColor: bg,
        fontWeight: bold ? FontWeight.bold : FontWeight.normal,
        fontFamily: 'monospace',
        fontSize: 13,
        height: 1.5,
      ),
    );
  }

  static Color _ansiBg(int n, bool dark) {
    // 简单的暗/亮背景映射
    const colors = [
      Color(0xFF1E1E1E),
      Color(0xFF5F3737),
      Color(0xFF315C31),
      Color(0xFF5C5C31),
      Color(0xFF31516C),
      Color(0xFF5C315C),
      Color(0xFF215D6E),
      Color(0xFF606060),
    ];
    final idx = n - 40;
    return idx >= 0 && idx < colors.length ? colors[idx] : Colors.transparent;
  }

  static Color _brightBg(int n) {
    const colors = [
      Color(0xFF4A4A4A),
      Color(0xFF6B2A2A),
      Color(0xFF2D6B2D),
      Color(0xFF6B6B2D),
      Color(0xFF2D4D7F),
      Color(0xFF6B2D6B),
      Color(0xFF2D6B7F),
      Color(0xFF8A8A8A),
    ];
    final idx = n - 100;
    return idx >= 0 && idx < colors.length ? colors[idx] : Colors.transparent;
  }
}
