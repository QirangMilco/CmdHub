import 'package:flutter/material.dart';
import '../services/cmdhub_service.dart';
import '../src/rust/models.dart';
import '../theme/app_theme.dart';

/// 编排编辑器
///
/// 创建/编辑 Pipeline，步骤按顺序执行。
class PipelineEditorPage extends StatefulWidget {
  final Pipeline? pipeline;
  const PipelineEditorPage({super.key, this.pipeline});

  @override
  State<PipelineEditorPage> createState() => _PipelineEditorPageState();
}

class _PipelineEditorPageState extends State<PipelineEditorPage> {
  final _service = CmdHubService();
  final _nameCtrl = TextEditingController();
  final _steps = <_StepEditor>[];
  List<Task> _tasks = [];
  bool _loading = true;
  bool _saving = false;

  bool get _editing => widget.pipeline != null;

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  @override
  void dispose() {
    _nameCtrl.dispose();
    super.dispose();
  }

  Future<void> _loadData() async {
    try {
      final tasks = await _service.listTasks();
      setState(() {
        _tasks = tasks;
        _loading = false;
      });
    } catch (e) {
      setState(() => _loading = false);
    }

    if (widget.pipeline != null) {
      _nameCtrl.text = widget.pipeline!.name;
      for (final s in widget.pipeline!.steps) {
        _steps.add(_StepEditor(
          taskId: s.taskId,
          order: s.order,
          delayMs: s.delayMs.toInt(),
          condition: s.condition,
        ));
      }
    }
  }

  void _addStep() {
    setState(() {
      _steps.add(_StepEditor(
        taskId: _tasks.isNotEmpty ? _tasks.first.id : '',
        order: _steps.length + 1,
        delayMs: 0,
        condition: _steps.isEmpty ? StepCondition.onStart : StepCondition.afterPrevious,
      ));
    });
  }

  void _removeStep(int index) {
    setState(() {
      _steps.removeAt(index);
      // 重新编号
      for (var i = 0; i < _steps.length; i++) {
        _steps[i].order = i + 1;
      }
    });
  }

  Future<void> _save() async {
    final name = _nameCtrl.text.trim();
    if (name.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('名称不能为空')),
      );
      return;
    }
    if (_steps.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('至少添加一个步骤')),
      );
      return;
    }

    final steps = _steps.map((s) => PipelineStep(
      taskId: s.taskId,
      order: s.order,
      delayMs: BigInt.from(s.delayMs),
      condition: s.condition,
    )).toList();

    final pipeline = Pipeline(
      id: widget.pipeline?.id ?? '',
      name: name,
      steps: steps,
    );

    setState(() => _saving = true);
    try {
      if (_editing) {
        await _service.updatePipeline(pipeline);
      } else {
        await _service.createPipeline(pipeline);
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
    final title = _editing ? '编辑编排' : '新建编排';

    if (_loading) {
      return Scaffold(
        body: Column(
          children: [
            _buildHeader(isDark, title),
            const Expanded(
              child: Center(child: CircularProgressIndicator()),
            ),
          ],
        ),
      );
    }

    if (_tasks.isEmpty) {
      return Scaffold(
        body: Column(
          children: [
            _buildHeader(isDark, title),
            const Expanded(
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.warning_amber, size: 32, color: AppTheme.warning),
                    SizedBox(height: 12),
                    Text('没有可用任务，先创建任务再编排'),
                  ],
                ),
              ),
            ),
          ],
        ),
      );
    }

    return Scaffold(
      body: Column(
        children: [
          _buildHeader(isDark, title),
          Expanded(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // 名称
                  _section(context, '基本信息'),
                  const SizedBox(height: 8),
                  TextField(
                    controller: _nameCtrl,
                    decoration: const InputDecoration(
                      labelText: '编排名称',
                      hintText: '输入编排名称',
                    ),
                  ),
                  const SizedBox(height: 24),
                  // 步骤
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      _section(context, '步骤'),
                      TextButton.icon(
                        icon: const Icon(Icons.add, size: 18),
                        label: const Text('添加步骤'),
                        onPressed: _addStep,
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  if (_steps.isEmpty)
                    Text(
                      '暂无步骤，点击「添加步骤」',
                      style: TextStyle(
                        color: AppTheme.textMuted(isDark),
                        fontSize: 13,
                      ),
                    ),
                  ...List.generate(_steps.length, (i) {
                    return _StepCard(
                      index: i,
                      step: _steps[i],
                      tasks: _tasks,
                      onRemove: () => _removeStep(i),
                    );
                  }),
                  const SizedBox(height: 24),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHeader(bool isDark, String title) {
    return Container(
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

// ============================================================
// 步骤卡片
// ============================================================

class _StepCard extends StatefulWidget {
  final int index;
  final _StepEditor step;
  final List<Task> tasks;
  final VoidCallback onRemove;

  const _StepCard({
    required this.index,
    required this.step,
    required this.tasks,
    required this.onRemove,
  });

  @override
  State<_StepCard> createState() => _StepCardState();
}

class _StepCardState extends State<_StepCard> {
  late TextEditingController _delayCtrl;

  @override
  void initState() {
    super.initState();
    _delayCtrl = TextEditingController(text: widget.step.delayMs.toString());
  }

  @override
  void dispose() {
    _delayCtrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final step = widget.step;

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: AppTheme.card(isDark),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: AppTheme.border(isDark)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                decoration: BoxDecoration(
                  color: AppTheme.accent.withOpacity(0.08),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Text(
                  '步骤 ${widget.index + 1}',
                  style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    color: AppTheme.accent,
                  ),
                ),
              ),
              const Spacer(),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 18, color: AppTheme.error),
                onPressed: widget.onRemove,
                tooltip: '删除步骤',
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
              ),
            ],
          ),
          const SizedBox(height: 12),
          // 任务选择
          DropdownButtonFormField<String>(
            value: widget.tasks.any((t) => t.id == step.taskId) ? step.taskId : null,
            decoration: const InputDecoration(
              labelText: '选择任务',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            ),
            items: widget.tasks.map((t) {
              return DropdownMenuItem(
                value: t.id,
                child: Text(t.name, overflow: TextOverflow.ellipsis),
              );
            }).toList(),
            onChanged: (v) {
              if (v != null) setState(() => step.taskId = v);
            },
          ),
          const SizedBox(height: 12),
          // 触发条件
          DropdownButtonFormField<StepCondition>(
            value: step.condition,
            decoration: const InputDecoration(
              labelText: '触发条件',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            ),
            items: const [
              DropdownMenuItem(
                value: StepCondition.onStart,
                child: Text('编排启动时立即执行'),
              ),
              DropdownMenuItem(
                value: StepCondition.afterPrevious,
                child: Text('上一步退出后执行'),
              ),
              DropdownMenuItem(
                value: StepCondition.afterDelay,
                child: Text('上一步启动后延时执行'),
              ),
            ],
            onChanged: (v) {
              if (v != null) setState(() => step.condition = v);
            },
          ),
          const SizedBox(height: 12),
          // 延时
          TextField(
            controller: _delayCtrl,
            keyboardType: TextInputType.number,
            decoration: const InputDecoration(
              labelText: '延时（毫秒）',
              hintText: '0',
              isDense: true,
              contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 10),
            ),
            onChanged: (v) {
              step.delayMs = int.tryParse(v) ?? 0;
            },
          ),
        ],
      ),
    );
  }
}

// ============================================================
// 步骤编辑器数据
// ============================================================

class _StepEditor {
  String taskId;
  int order;
  int delayMs;
  StepCondition condition;

  _StepEditor({
    required this.taskId,
    required this.order,
    required this.delayMs,
    required this.condition,
  });
}

// ============================================================
// 顶部按钮
// ============================================================

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
