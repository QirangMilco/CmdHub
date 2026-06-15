import 'package:flutter/material.dart';
import '../services/cmdhub_service.dart';
import '../src/rust/models.dart';
import '../theme/app_theme.dart';
import '../widgets/status_badge.dart';
import 'instance_detail_page.dart';
import 'task_editor_page.dart';

/// 桌面端专用布局 — 左侧竖向导航 + 右侧内容区
/// 无 AppBar，顶部为 plain 标题栏
class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  final _service = CmdHubService();
  int _selectedIndex = 0;

  List<Task> _tasks = [];
  List<Pipeline> _pipelines = [];
  List<TaskInstance> _instances = [];

  static const List<_NavItem> _navItems = [
    _NavItem(icon: Icons.terminal_outlined, label: '任务'),
    _NavItem(icon: Icons.account_tree_outlined, label: '编排'),
    _NavItem(icon: Icons.history_outlined, label: '实例'),
  ];

  @override
  void initState() {
    super.initState();
    _loadData();
  }

  Future<void> _loadData() async {
    try {
      final results = await Future.wait([
        _service.listTasks(),
        _service.listPipelines(),
        _service.listInstances(),
      ]);
      if (mounted) {
        setState(() {
          _tasks = results[0] as List<Task>;
          _pipelines = results[1] as List<Pipeline>;
          _instances = results[2] as List<TaskInstance>;
        });
      }
    } catch (e) {
      debugPrint('load error: $e');
    }
  }

  void _onNavTap(int index) {
    setState(() => _selectedIndex = index);
    _loadData();
  }

  // --------------- 任务操作 ---------------

  void _addTask() {
    Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => const TaskEditorPage()),
    ).then((_) => _loadData());
  }

  void _editTask(Task task) {
    Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => TaskEditorPage(task: task)),
    ).then((_) => _loadData());
  }

  Future<void> _runTask(Task task) async {
    try {
      await _service.spawnTask(task.id);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('已启动: ${task.name}')),
        );
      }
      _loadData();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('启动失败: $e')),
        );
      }
    }
  }

  void _deleteTask(Task task) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        title: Text('删除任务: ${task.name}'),
        content: const Text('此操作不可撤销'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          TextButton(
            onPressed: () { Navigator.pop(ctx); _service.deleteTask(task.id); _loadData(); },
            style: TextButton.styleFrom(foregroundColor: AppTheme.error),
            child: const Text('删除'),
          ),
        ],
      ),
    );
  }

  // --------------- 编排操作 ---------------

  Future<void> _runPipeline(Pipeline pipeline) async {
    try {
      await _service.runPipeline(pipeline.id);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('已启动编排: ${pipeline.name}')),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('编排启动失败: $e')),
        );
      }
    }
  }

  // --------------- 实例操作 ---------------

  void _viewInstance(TaskInstance instance) {
    Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => InstanceDetailPage(instanceId: instance.id)),
    ).then((_) => _loadData());
  }

  Future<void> _killInstance(TaskInstance i) async {
    await _service.killInstance(i.id);
    _loadData();
  }

  Future<void> _removeInstance(TaskInstance i) async {
    await _service.removeInstance(i.id);
    _loadData();
  }

  // --------------- 构建 ---------------

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Scaffold(
      body: Row(
        children: [
          _buildSideNav(isDark),
          Expanded(
            child: Column(
              children: [
                _buildHeader(isDark),
                Expanded(child: _buildContent()),
              ],
            ),
          ),
        ],
      ),
    );
  }

  // --------------- 左侧导航栏 (80px) ---------------

  Widget _buildSideNav(bool isDark) {
    return Container(
      width: 80,
      color: AppTheme.nav(isDark),
      child: SafeArea(
        child: Column(
          children: [
            const SizedBox(height: 20),
            // 简洁图标 Logo
            Container(
              width: 32,
              height: 32,
              decoration: BoxDecoration(
                color: AppTheme.accent,
                borderRadius: BorderRadius.circular(8),
              ),
              child: const Icon(Icons.terminal, color: Colors.white, size: 18),
            ),
            const SizedBox(height: 28),
            // 导航项
            Expanded(
              child: ListView.builder(
                padding: const EdgeInsets.symmetric(horizontal: 10),
                itemCount: _navItems.length,
                itemBuilder: (context, index) {
                  final isSelected = _selectedIndex == index;
                  final item = _navItems[index];
                  return _NavButton(
                    icon: item.icon,
                    label: item.label,
                    isSelected: isSelected,
                    isDark: isDark,
                    onTap: () => _onNavTap(index),
                  );
                },
              ),
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }

  // --------------- 顶部标题栏 (48px) ---------------

  Widget _buildHeader(bool isDark) {
    final titles = ['任务', '编排', '实例'];
    final counts = ['${_tasks.length}', '${_pipelines.length}', '${_instances.length}'];
    final isTaskTab = _selectedIndex == 0;

    return Container(
      height: 48,
      padding: const EdgeInsets.symmetric(horizontal: 20),
      decoration: BoxDecoration(
        color: AppTheme.surface(isDark),
        border: Border(
          bottom: BorderSide(color: AppTheme.divider(isDark)),
        ),
      ),
      child: Row(
        children: [
          Text(
            titles[_selectedIndex],
            style: TextStyle(
              fontSize: 16,
              fontWeight: FontWeight.w600,
              color: AppTheme.text(isDark),
            ),
          ),
          const SizedBox(width: 8),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
            decoration: BoxDecoration(
              color: AppTheme.border(isDark),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Text(
              counts[_selectedIndex],
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w500,
                color: AppTheme.textSecondary(isDark),
              ),
            ),
          ),
          const Spacer(),
          if (isTaskTab)
            _HeaderButton(
              icon: Icons.add,
              label: '新建',
              onTap: _addTask,
            ),
          _HeaderButton(
            icon: Icons.refresh,
            onTap: _loadData,
          ),
        ],
      ),
    );
  }

  // --------------- 内容区域 ---------------

  Widget _buildContent() {
    return IndexedStack(
      index: _selectedIndex,
      children: [
        _buildTaskTab(),
        _buildPipelineTab(),
        _buildInstanceTab(),
      ],
    );
  }

  // --------------- 任务 Tab ---------------

  Widget _buildTaskTab() {
    if (_tasks.isEmpty) {
      return _EmptyState(
        icon: Icons.terminal_outlined,
        title: '暂无任务',
        subtitle: '点击左上角「新建」创建任务',
        onAction: _addTask,
        actionLabel: '创建任务',
      );
    }

    return RefreshIndicator(
      onRefresh: _loadData,
      child: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: _tasks.length,
        itemBuilder: (_, i) => _TaskCard(
          task: _tasks[i],
          onRun: () => _runTask(_tasks[i]),
          onEdit: () => _editTask(_tasks[i]),
          onDelete: () => _deleteTask(_tasks[i]),
        ),
      ),
    );
  }

  // --------------- 编排 Tab ---------------

  Widget _buildPipelineTab() {
    if (_pipelines.isEmpty) {
      return _EmptyState(
        icon: Icons.account_tree_outlined,
        title: '暂无编排',
        subtitle: '编排可以将多个任务按顺序自动执行',
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: _pipelines.length,
      itemBuilder: (_, i) => _PipelineCard(
        pipeline: _pipelines[i],
        onRun: () => _runPipeline(_pipelines[i]),
      ),
    );
  }

  // --------------- 实例 Tab ---------------

  Widget _buildInstanceTab() {
    if (_instances.isEmpty) {
      return _EmptyState(
        icon: Icons.history_outlined,
        title: '暂无实例',
        subtitle: '运行任务后，实例会出现在这里',
      );
    }

    return RefreshIndicator(
      onRefresh: _loadData,
      child: ListView.builder(
        padding: const EdgeInsets.all(16),
        itemCount: _instances.length,
        itemBuilder: (_, i) {
          final inst = _instances[i];
          return _InstanceCard(
            instance: inst,
            onTap: () => _viewInstance(inst),
            onKill: inst.status is InstanceStatus_Running
                ? () => _killInstance(inst)
                : null,
            onClear: inst.status is! InstanceStatus_Running
                ? () => _removeInstance(inst)
                : null,
          );
        },
      ),
    );
  }
}

// ============================================================
// 导航按钮 — 左侧竖线选中态
// ============================================================

class _NavButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool isSelected;
  final bool isDark;
  final VoidCallback onTap;

  const _NavButton({
    required this.icon,
    required this.label,
    required this.isSelected,
    required this.isDark,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(8),
          child: Container(
            padding: const EdgeInsets.symmetric(vertical: 10),
            child: Row(
              children: [
                // 左侧选中竖线
                Container(
                  width: 3,
                  height: 28,
                  decoration: BoxDecoration(
                    color: isSelected ? AppTheme.accent : Colors.transparent,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
                const SizedBox(width: 8),
                // 图标
                Icon(
                  icon,
                  size: 22,
                  color: isSelected
                      ? AppTheme.accent
                      : (isDark ? AppTheme.darkTextSecondary : AppTheme.lightTextSecondary),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ============================================================
// 顶部标题栏按钮
// ============================================================

class _HeaderButton extends StatelessWidget {
  final IconData icon;
  final String? label;
  final VoidCallback onTap;

  const _HeaderButton({
    required this.icon,
    this.label,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Tooltip(
      message: label ?? '',
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(6),
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(icon, size: 18, color: AppTheme.textSecondary(isDark)),
                if (label != null) ...[
                  const SizedBox(width: 4),
                  Text(
                    label!,
                    style: TextStyle(
                      fontSize: 13,
                      color: AppTheme.textSecondary(isDark),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }
}

// ============================================================
// 空状态
// ============================================================

class _EmptyState extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback? onAction;
  final String? actionLabel;

  const _EmptyState({
    required this.icon,
    required this.title,
    required this.subtitle,
    this.onAction,
    this.actionLabel,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 40, color: AppTheme.textMuted(isDark)),
          const SizedBox(height: 16),
          Text(
            title,
            style: TextStyle(
              fontSize: 15,
              fontWeight: FontWeight.w600,
              color: AppTheme.text(isDark),
            ),
          ),
          const SizedBox(height: 4),
          Text(
            subtitle,
            textAlign: TextAlign.center,
            style: TextStyle(
              fontSize: 13,
              color: AppTheme.textSecondary(isDark),
              height: 1.5,
            ),
          ),
          if (onAction != null && actionLabel != null) ...[
            const SizedBox(height: 20),
            ElevatedButton.icon(
              icon: const Icon(Icons.add, size: 16),
              label: Text(actionLabel!),
              onPressed: onAction,
            ),
          ],
        ],
      ),
    );
  }
}

// ============================================================
// 卡片
// ============================================================

class _TaskCard extends StatelessWidget {
  final Task task;
  final VoidCallback onRun;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _TaskCard({
    required this.task,
    required this.onRun,
    required this.onEdit,
    required this.onDelete,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final isInteractive = task.mode == TaskMode.interactive;

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: AppTheme.card(isDark),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: AppTheme.border(isDark),
          width: 1,
        ),
      ),
      child: Material(
        color: Colors.transparent,
        borderRadius: BorderRadius.circular(8),
        child: InkWell(
          onTap: onEdit,
          borderRadius: BorderRadius.circular(8),
          hoverColor: AppTheme.hover(isDark),
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Expanded(
                            child: Text(
                              task.name,
                              style: TextStyle(
                                fontWeight: FontWeight.w600,
                                fontSize: 14,
                                color: AppTheme.text(isDark),
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      Text(
                        task.command,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontFamily: 'monospace',
                          fontSize: 12,
                          height: 1.4,
                          color: AppTheme.textSecondary(isDark),
                        ),
                      ),
                      const SizedBox(height: 8),
                      _ModeTag(interactive: isInteractive),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                _IconAction(
                  icon: Icons.play_arrow,
                  color: AppTheme.success,
                  onTap: onRun,
                  tooltip: '运行',
                ),
                _IconAction(
                  icon: Icons.edit_outlined,
                  color: AppTheme.textSecondary(isDark),
                  onTap: onEdit,
                  tooltip: '编辑',
                ),
                _IconAction(
                  icon: Icons.delete_outline,
                  color: AppTheme.error,
                  onTap: onDelete,
                  tooltip: '删除',
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ModeTag extends StatelessWidget {
  final bool interactive;
  const _ModeTag({required this.interactive});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: interactive
            ? AppTheme.accent.withOpacity(0.08)
            : Colors.transparent,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(
          color: interactive
              ? AppTheme.accent.withOpacity(0.25)
              : AppTheme.border(isDark),
        ),
      ),
      child: Text(
        interactive ? '交互式' : '一次性',
        style: TextStyle(
          fontSize: 10,
          fontWeight: FontWeight.w500,
          color: interactive
              ? AppTheme.accent
              : AppTheme.textSecondary(isDark),
        ),
      ),
    );
  }
}

class _PipelineCard extends StatelessWidget {
  final Pipeline pipeline;
  final VoidCallback onRun;

  const _PipelineCard({required this.pipeline, required this.onRun});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: AppTheme.card(isDark),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: AppTheme.border(isDark),
          width: 1,
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    pipeline.name,
                    style: TextStyle(
                      fontWeight: FontWeight.w600,
                      fontSize: 14,
                      color: AppTheme.text(isDark),
                    ),
                  ),
                  const SizedBox(height: 6),
                  Text(
                    '${pipeline.steps.length} 个步骤',
                    style: TextStyle(
                      fontSize: 12,
                      color: AppTheme.textSecondary(isDark),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 12),
            _IconAction(
              icon: Icons.play_arrow,
              color: AppTheme.success,
              onTap: onRun,
              tooltip: '运行',
            ),
          ],
        ),
      ),
    );
  }
}

class _InstanceCard extends StatelessWidget {
  final TaskInstance instance;
  final VoidCallback onTap;
  final VoidCallback? onKill;
  final VoidCallback? onClear;

  const _InstanceCard({
    required this.instance,
    required this.onTap,
    this.onKill,
    this.onClear,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: AppTheme.card(isDark),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
          color: AppTheme.border(isDark),
          width: 1,
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
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        instance.taskName,
                        style: TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 14,
                          color: AppTheme.text(isDark),
                        ),
                      ),
                      const SizedBox(height: 6),
                      StatusBadge(status: instance.status),
                      const SizedBox(height: 4),
                      Text(
                        _formatDuration(
                          instance.startedAt.toInt(),
                          instance.endedAt?.toInt(),
                        ),
                        style: TextStyle(
                          fontSize: 11,
                          color: AppTheme.textSecondary(isDark),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                if (onKill != null)
                  _IconAction(
                    icon: Icons.stop,
                    color: AppTheme.error,
                    onTap: onKill!,
                    tooltip: '停止',
                  ),
                if (onClear != null)
                  _IconAction(
                    icon: Icons.clear,
                    color: AppTheme.textMuted(isDark),
                    onTap: onClear!,
                    tooltip: '清除',
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  String _formatDuration(int start, int? end) {
    final e = end ?? DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final s = e - start;
    if (s < 60) return '${s}s';
    if (s < 3600) return '${s ~/ 60}m ${s % 60}s';
    return '${s ~/ 3600}h ${(s % 3600) ~/ 60}m';
  }
}

class _IconAction extends StatelessWidget {
  final IconData icon;
  final Color color;
  final VoidCallback onTap;
  final String tooltip;

  const _IconAction({
    required this.icon,
    required this.color,
    required this.onTap,
    required this.tooltip,
  });

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: Material(
        color: Colors.transparent,
        shape: const CircleBorder(),
        child: InkWell(
          onTap: onTap,
          customBorder: const CircleBorder(),
          child: Padding(
            padding: const EdgeInsets.all(6),
            child: Icon(icon, size: 20, color: color),
          ),
        ),
      ),
    );
  }
}

// ============================================================
// 导航项数据
// ============================================================

class _NavItem {
  final IconData icon;
  final String label;

  const _NavItem({required this.icon, required this.label});
}
