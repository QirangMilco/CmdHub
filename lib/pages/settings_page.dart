import 'dart:io';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import '../services/cmdhub_service.dart';
import '../services/theme_service.dart';
import '../src/rust/models.dart';
import '../theme/app_theme.dart';

/// 设置页面
class SettingsPage extends StatefulWidget {
  const SettingsPage({super.key});

  @override
  State<SettingsPage> createState() => _SettingsPageState();
}

class _SettingsPageState extends State<SettingsPage> {
  final _service = CmdHubService();

  Future<void> _quitApp() async {
    try {
      final instances = await _service.listInstances();
      for (final inst in instances) {
        if (inst.status is InstanceStatus_Running) {
          await _service.killInstance(inst.id);
        }
      }
    } catch (_) {}
    await windowManager.destroy();
    exit(0);
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final theme = ThemeService.instance;

    return ListenableBuilder(
      listenable: theme,
      builder: (context, _) => ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // ---------- 外观 ----------
          _SectionHeader(title: '外观'),
          const SizedBox(height: 8),
          _ThemeOption(
            icon: Icons.light_mode_outlined,
            label: '浅色',
            selected: theme.mode == ThemeMode.light,
            onTap: () => theme.setMode(ThemeMode.light),
          ),
          _ThemeOption(
            icon: Icons.dark_mode_outlined,
            label: '深色',
            selected: theme.mode == ThemeMode.dark,
            onTap: () => theme.setMode(ThemeMode.dark),
          ),
          _ThemeOption(
            icon: Icons.settings_brightness_outlined,
            label: '跟随系统',
            selected: theme.mode == ThemeMode.system,
            onTap: () => theme.setMode(ThemeMode.system),
          ),

          const SizedBox(height: 24),
          // ---------- 应用 ----------
          _SectionHeader(title: '应用'),
          const SizedBox(height: 8),
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
            decoration: BoxDecoration(
              color: AppTheme.card(isDark),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: AppTheme.border(isDark)),
            ),
            child: Material(
              color: Colors.transparent,
              borderRadius: BorderRadius.circular(8),
              child: InkWell(
                onTap: _quitApp,
                borderRadius: BorderRadius.circular(8),
                hoverColor: AppTheme.hover(isDark),
                child: const Padding(
                  padding: EdgeInsets.symmetric(horizontal: 16, vertical: 14),
                  child: Row(
                    children: [
                      Icon(Icons.power_settings_new, size: 20, color: AppTheme.error),
                      SizedBox(width: 12),
                      Text(
                        '退出应用',
                        style: TextStyle(
                          fontSize: 14,
                          fontWeight: FontWeight.w500,
                          color: AppTheme.error,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
      child: Text(
        title,
        style: TextStyle(
          fontWeight: FontWeight.w600,
          fontSize: 13,
          color: AppTheme.textMuted(isDark),
          letterSpacing: 0.5,
        ),
      ),
    );
  }
}

class _ThemeOption extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  const _ThemeOption({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      decoration: BoxDecoration(
        color: selected
            ? AppTheme.accent.withValues(alpha: 0.08)
            : Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: selected
              ? AppTheme.accent.withValues(alpha: 0.25)
              : Colors.transparent,
        ),
      ),
      child: Material(
        color: Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(8),
          hoverColor: AppTheme.hover(isDark),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
            child: Row(
              children: [
                Icon(
                  icon,
                  size: 20,
                  color: selected ? AppTheme.accent : AppTheme.textSecondary(isDark),
                ),
                const SizedBox(width: 12),
                Text(
                  label,
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w500,
                    color: selected ? AppTheme.accent : AppTheme.text(isDark),
                  ),
                ),
                const Spacer(),
                if (selected)
                  Icon(
                    Icons.check,
                    size: 18,
                    color: AppTheme.accent,
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
