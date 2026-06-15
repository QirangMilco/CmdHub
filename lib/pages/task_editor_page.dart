import 'package:flutter/material.dart';
import '../services/cmdhub_service.dart';
import '../src/rust/models.dart';
import '../widgets/custom_app_bar.dart';

class TaskEditorPage extends StatefulWidget {
  final Task? task;

  const TaskEditorPage({super.key, this.task});

  @override
  State<TaskEditorPage> createState() => _TaskEditorPageState();
}

class _TaskEditorPageState extends State<TaskEditorPage> {
  final _service = CmdHubService();
  final _nameController = TextEditingController();
  final _commandController = TextEditingController();
  final _cwdController = TextEditingController();
  final _envKeys = <String>[];
  final _envValues = <String>[];
  bool _envInherit = true;
  TaskMode _mode = TaskMode.oneshot;
  bool _saving = false;

  bool get _isEditing => widget.task != null;

  @override
  void initState() {
    super.initState();
    if (widget.task != null) {
      _nameController.text = widget.task!.name;
      _commandController.text = widget.task!.command;
      _cwdController.text = widget.task!.cwd ?? '';
      _envInherit = widget.task!.envInherit;
      _mode = widget.task!.mode;
      for (final entry in widget.task!.env.entries) {
        _envKeys.add(entry.key);
        _envValues.add(entry.value);
      }
    }
  }

  @override
  void dispose() {
    _nameController.dispose();
    _commandController.dispose();
    _cwdController.dispose();
    super.dispose();
  }

  void _addEnvRow() {
    setState(() {
      _envKeys.add('');
      _envValues.add('');
    });
  }

  void _removeEnvRow(int index) {
    setState(() {
      _envKeys.removeAt(index);
      _envValues.removeAt(index);
    });
  }

  Future<void> _save() async {
    final name = _nameController.text.trim();
    final command = _commandController.text.trim();
    if (name.isEmpty || command.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('名称和命令不能为空')),
      );
      return;
    }

    final env = <String, String>{};
    for (var i = 0; i < _envKeys.length; i++) {
      final k = _envKeys[i].trim();
      if (k.isNotEmpty) {
        env[k] = _envValues[i];
      }
    }

    final task = Task(
      id: widget.task?.id ?? '',
      name: name,
      command: command,
      cwd: _cwdController.text.trim().isEmpty
          ? null
          : _cwdController.text.trim(),
      env: env,
      envInherit: _envInherit,
      mode: _mode,
    );

    setState(() => _saving = true);
    try {
      if (_isEditing) {
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
    return Scaffold(
      appBar: CustomAppBar(
        title: Text(_isEditing ? '编辑任务' : '新建任务'),
        actions: [
          _saving
              ? const Padding(
                  padding: EdgeInsets.all(12),
                  child: SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: Colors.white,
                    ),
                  ),
                )
              : IconButton(
                  icon: const Icon(Icons.save, color: Colors.white),
                  onPressed: _save,
                  tooltip: '保存',
                ),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            TextField(
              controller: _nameController,
              decoration: const InputDecoration(
                labelText: '名称',
                hintText: '输入任务名称',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: _commandController,
              maxLines: 3,
              decoration: const InputDecoration(
                labelText: '命令',
                hintText: '输入要执行的命令',
                border: OutlineInputBorder(),
              ),
              style: const TextStyle(fontFamily: 'monospace'),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: _cwdController,
              decoration: const InputDecoration(
                labelText: '工作路径（可选）',
                hintText: '留空使用当前目录',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 16),
            DropdownButtonFormField<TaskMode>(
              initialValue: _mode,
              decoration: const InputDecoration(
                labelText: '执行模式',
                border: OutlineInputBorder(),
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
              onChanged: (val) {
                if (val != null) setState(() => _mode = val);
              },
            ),
            const SizedBox(height: 16),
            SwitchListTile(
              title: const Text('继承系统环境变量'),
              value: _envInherit,
              onChanged: (val) => setState(() => _envInherit = val),
              contentPadding: EdgeInsets.zero,
            ),
            const SizedBox(height: 16),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text('环境变量',
                    style: TextStyle(fontWeight: FontWeight.w500)),
                TextButton.icon(
                  icon: const Icon(Icons.add, size: 18),
                  label: const Text('添加'),
                  onPressed: _addEnvRow,
                ),
              ],
            ),
            if (_envKeys.isEmpty)
              Text(
                '无自定义环境变量',
                style: TextStyle(color: Colors.grey[500], fontSize: 13),
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
                          border: OutlineInputBorder(),
                          isDense: true,
                        ),
                        onChanged: (v) => _envKeys[i] = v,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: TextField(
                        controller:
                            TextEditingController(text: _envValues[i]),
                        decoration: const InputDecoration(
                          hintText: 'VALUE',
                          border: OutlineInputBorder(),
                          isDense: true,
                        ),
                        onChanged: (v) => _envValues[i] = v,
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.remove_circle,
                          color: Colors.red, size: 20),
                      onPressed: () => _removeEnvRow(i),
                    ),
                  ],
                ),
              );
            }),
          ],
        ),
      ),
    );
  }
}
