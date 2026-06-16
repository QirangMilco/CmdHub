import 'package:flutter/material.dart';
import '../services/cmdhub_service.dart';
import '../src/rust/models.dart';
import '../theme/app_theme.dart';

class TaskEditorPage extends StatefulWidget {
  final Task? task;
  const TaskEditorPage({super.key, this.task});

  @override
  State<TaskEditorPage> createState() => _TaskEditorPageState();
}

class _TaskEditorPageState extends State<TaskEditorPage> {
  final _service = CmdHubService();
  final _nameCtrl = TextEditingController();
  final _cmdCtrl = TextEditingController();
  final _cwdCtrl = TextEditingController();
  final _envKeys = <String>[];
  final _envValues = <String>[];
  bool _envInherit = true;
  TaskMode _mode = TaskMode.oneshot;
  bool _saving = false;

  bool get _editing => widget.task != null;

  @override
  void initState() {
    super.initState();
    if (widget.task != null) {
      final t = widget.task!;
      _nameCtrl.text = t.name;
      _cmdCtrl.text = t.command;
      _cwdCtrl.text = t.cwd ?? '';
      _envInherit = t.envInherit;
      _mode = t.mode;
      for (final e in t.env.entries) {
        _envKeys.add(e.key);
        _envValues.add(e.value);
      }
    }
  }

  @override
  void dispose() {
    _nameCtrl.dispose();
    _cmdCtrl.dispose();
    _cwdCtrl.dispose();
    super.dispose();
  }

  void _addEnv() => setState(() { _envKeys.add(''); _envValues.add(''); });
  void _removeEnv(int i) => setState(() { _envKeys.removeAt(i); _envValues.removeAt(i); });

  Future<void> _save() async {
    final name = _nameCtrl.text.trim();
    final cmd = _cmdCtrl.text.trim();
    if (name.isEmpty || cmd.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('名称和命令不能为空')),
      );
      return;
    }

    final env = <String, String>{};
    for (var i = 0; i < _envKeys.length; i++) {
      final k = _envKeys[i].trim();
      if (k.isNotEmpty) env[k] = _envValues[i];
    }

    final task = Task(
      id: widget.task?.id ?? '',
      name: name,
      command: cmd,
      cwd: _cwdCtrl.text.trim().isEmpty ? null : _cwdCtrl.text.trim(),
      env: env,
      envInherit: _envInherit,
      mode: _mode,
    );

    setState(() => _saving = true);
    try {
      if (_editing) {
        await _service.updateTask(task);
      } else {
        await _service.createTask(task);
      }
      if (mounted) Navigator.pop(context);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('保存失败: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final title = _editing ? '编辑任务' : '新建任务';

    return Scaffold(
      body: Column(
        children: [
          // 顶部标题行
          Container(
            height: 48,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            decoration: BoxDecoration(
              color: AppTheme.surface(isDark),
              border: Border(bottom: BorderSide(color: AppTheme.divider(isDark))),
            ),
            child: Row(
              children: [
                _HeaderAction(
                  icon: Icons.arrow_back,
                  onTap: () => Navigator.pop(context),
                  tooltip: '返回',
                ),
                const SizedBox(width: 8),
                Text(
                  title,
                  style: TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    color: AppTheme.text(isDark),
                  ),
                ),
                const Spacer(),
                if (_saving)
                  const SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                else
                  ElevatedButton(
                    onPressed: _save,
                    child: const Text('保存'),
                  ),
              ],
            ),
          ),
          // 内容
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _section(context, '基本信息'),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _nameCtrl,
                    decoration: const InputDecoration(
                      labelText: '名称',
                      hintText: '输入任务名称',
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _cmdCtrl,
                    maxLines: 3,
                    decoration: const InputDecoration(
                      labelText: '命令',
                      hintText: '输入要执行的命令',
                    ),
                    style: TextStyle(fontFamily: AppTheme.monoFont),
                  ),
                  const SizedBox(height: 12),
                  TextField(
                    controller: _cwdCtrl,
                    decoration: const InputDecoration(
                      labelText: '工作路径（可选）',
                      hintText: '留空使用当前目录',
                    ),
                  ),
                  const SizedBox(height: 24),
                  _section(context, '执行配置'),
                  const SizedBox(height: 8),
                  DropdownButtonFormField<TaskMode>(
                    initialValue: _mode,
                    decoration: const InputDecoration(
                      labelText: '执行模式',
                    ),
                    items: const [
                      DropdownMenuItem(
                        value: TaskMode.oneshot,
                        child: Text('一次性执行（命令结束即退出）'),
                      ),
                      DropdownMenuItem(
                        value: TaskMode.interactive,
                        child: Text('交互式（保持终端，可发送输入）'),
                      ),
                    ],
                    onChanged: (v) {
                      if (v != null) setState(() => _mode = v);
                    },
                  ),
                  const SizedBox(height: 8),
                  SwitchListTile(
                    title: const Text('继承系统环境变量'),
                    value: _envInherit,
                    onChanged: (v) => setState(() => _envInherit = v),
                    contentPadding: EdgeInsets.zero,
                  ),
                  const SizedBox(height: 24),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      _section(context, '环境变量'),
                      TextButton.icon(
                        icon: const Icon(Icons.add, size: 18),
                        label: const Text('添加'),
                        onPressed: _addEnv,
                      ),
                    ],
                  ),
                  if (_envKeys.isEmpty)
                    Padding(
                      padding: const EdgeInsets.only(top: 4),
                      child: Text(
                        '无自定义环境变量',
                        style: TextStyle(
                          color: AppTheme.textMuted(isDark),
                          fontSize: 13,
                        ),
                      ),
                    ),
                  ...List.generate(_envKeys.length, (i) {
                    return Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Row(
                        children: [
                          Expanded(
                            child: TextField(
                              controller: TextEditingController(text: _envKeys[i]),
                              decoration: const InputDecoration(
                                hintText: 'KEY',
                                isDense: true,
                              ),
                              onChanged: (v) => _envKeys[i] = v,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: TextField(
                              controller: TextEditingController(text: _envValues[i]),
                              decoration: const InputDecoration(
                                hintText: 'VALUE',
                                isDense: true,
                              ),
                              onChanged: (v) => _envValues[i] = v,
                            ),
                          ),
                          IconButton(
                            icon: const Icon(
                              Icons.remove_circle,
                              color: AppTheme.error,
                              size: 20,
                            ),
                            onPressed: () => _removeEnv(i),
                          ),
                        ],
                      ),
                    );
                  }),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _section(BuildContext context, String title) {
    return Text(
      title,
      style: TextStyle(
        fontWeight: FontWeight.w600,
        fontSize: 13,
        color: AppTheme.textMuted(Theme.of(context).brightness == Brightness.dark),
        letterSpacing: 0.5,
      ),
    );
  }
}

class _HeaderAction extends StatelessWidget {
  final IconData icon;
  final VoidCallback onTap;
  final String? tooltip;

  const _HeaderAction({
    required this.icon,
    required this.onTap,
    this.tooltip,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Tooltip(
      message: tooltip ?? '',
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(6),
          child: Container(
            padding: const EdgeInsets.all(8),
            child: Icon(icon, size: 20, color: AppTheme.textSecondary(isDark)),
          ),
        ),
      ),
    );
  }
}
