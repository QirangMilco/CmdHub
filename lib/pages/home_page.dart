import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:system_tray/system_tray.dart';
import 'package:window_manager/window_manager.dart';
import '../services/cmdhub_service.dart';
import '../services/theme_service.dart';
import '../src/rust/models.dart';
import '../theme/app_theme.dart';
import '../widgets/status_badge.dart';
import 'instance_detail_page.dart';
import 'pipeline_editor_page.dart';
import 'settings_page.dart';
import 'task_editor_page.dart';

/// 桌面端专用布局 — 左侧竖向导航 + 右侧内容区
/// 无 AppBar，顶部为 plain 标题栏
class HomePage extends StatefulWidget {
  final int initialIndex;

  const HomePage({super.key, this.initialIndex = 0});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> with WindowListener {
  final _service = CmdHubService();
  late int _selectedIndex;
  Timer? _pollTimer;

  List<Task> _tasks = [];
  List<Pipeline> _pipelines = [];
  List<TaskInstance> _instances = [];
  List<PipelineRunState> _runs = [];

  static const List<_NavItem> _navItems = [
    _NavItem(icon: Icons.terminal_outlined, label: '任务'),
    _NavItem(icon: Icons.account_tree_outlined, label: '编排'),
    _NavItem(icon: Icons.history_outlined, label: '实例'),
    _NavItem(icon: Icons.settings_outlined, label: '设置'),
  ];

  @override
  void initState() {
    super.initState();
    _selectedIndex = widget.initialIndex;
    _loadData();
    _pollTimer = Timer.periodic(const Duration(seconds: 1), (_) => _loadData());

    windowManager.addListener(this);
    _initTray();
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    windowManager.removeListener(this);
    super.dispose();
  }

  // --------------- 窗口管理 ---------------

  @override
  void onWindowClose() async {
    // 关闭窗口 → 隐藏到托盘
    await windowManager.hide();
    _updateDockVisibility(false);
  }

  @override
  void onWindowFocus() {
    // 窗口获得焦点时确保 Dock 图标可见
    _updateDockVisibility(true);
  }

  /// macOS 上控制 Dock 图标显示/隐藏
  static const _dockChannel = MethodChannel('com.cmdhub/dock');
  void _updateDockVisibility(bool visible) {
    if (Platform.isMacOS) {
      _dockChannel.invokeMethod('setDockVisibility', {'visible': visible});
    }
  }

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

  // --------------- 系统托盘 ---------------

  final SystemTray _systemTray = SystemTray();

  Future<void> _initTray() async {
    await _systemTray.initSystemTray(
      iconPath: 'assets/tray_icon.png',
      toolTip: 'CmdHub',
    );
    await _updateTrayMenu();

    _systemTray.registerSystemTrayEventHandler((eventName) {
      if (eventName == kSystemTrayEventClick) {
        if (Platform.isMacOS) {
          _systemTray.popUpContextMenu();
        } else {
          windowManager.show();
          windowManager.focus();
          _updateDockVisibility(true);
        }
      } else if (eventName == kSystemTrayEventRightClick) {
        if (Platform.isMacOS) {
          windowManager.show();
          windowManager.focus();
          _updateDockVisibility(true);
        } else {
          _systemTray.popUpContextMenu();
        }
      }
    });
  }

  Future<void> _updateTrayMenu() async {
    final menu = Menu();
    menu.buildFrom([
      MenuItemLabel(
        label: '显示窗口',
        onClicked: (_) async {
          await windowManager.show();
          await windowManager.focus();
          _updateDockVisibility(true);
        },
      ),
      MenuSeparator(),
      MenuItemLabel(
        label: '退出',
        onClicked: (_) async {
          await _quitApp();
        },
      ),
    ]);
    await _systemTray.setContextMenu(menu);
  }



  Future<void> _loadData() async {
    try {
      final results = await Future.wait([
        _service.listTasks(),
        _service.listPipelines(),
        _service.listInstances(),
        _service.listPipelineRuns(),
      ]);
      if (mounted) {
        setState(() {
          _tasks = results[0] as List<Task>;
          _pipelines = results[1] as List<Pipeline>;
          _instances = results[2] as List<TaskInstance>;
          _runs = results[3] as List<PipelineRunState>;
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

  void _addPipeline() {
    Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => const PipelineEditorPage()),
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
      final instance = await _service.spawnTask(task.id);
      _loadData();
      if (mounted) {
        // 自动跳转到实例详情页
        _viewInstance(instance);
      }
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

  void _editPipeline(Pipeline pipeline) {
    Navigator.push(
      context,
      MaterialPageRoute(builder: (_) => PipelineEditorPage(pipeline: pipeline)),
    ).then((_) => _loadData());
  }

  void _deletePipeline(Pipeline pipeline) {
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        title: Text('删除编排: ${pipeline.name}'),
        content: const Text('此操作不可撤销'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
          TextButton(
            onPressed: () { Navigator.pop(ctx); _service.deletePipeline(pipeline.id); _loadData(); },
            style: TextButton.styleFrom(foregroundColor: AppTheme.error),
            child: const Text('删除'),
          ),
        ],
      ),
    );
  }

  Future<void> _runPipeline(Pipeline pipeline) async {
    try {
      await _service.runPipeline(pipeline.id);
      if (mounted) {
        setState(() {
          _selectedIndex = 2; // 切换到实例 Tab
        });
        _loadData();
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
      width: 140,
      color: AppTheme.nav(isDark),
      child: SafeArea(
        child: Column(
          children: [
            const SizedBox(height: 20),
            // 简洁图标 Logo
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: AppTheme.accent,
                borderRadius: BorderRadius.circular(10),
              ),
              child: const Icon(Icons.terminal, color: Colors.white, size: 22),
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
            const SizedBox(height: 12),
          ],
        ),
      ),
    );
  }

  // --------------- 顶部标题栏 (48px) ---------------

  Widget _buildHeader(bool isDark) {
    final isSettings = _selectedIndex == 3;
    final titles = ['任务', '编排', '实例', '设置'];
    final counts = ['${_tasks.length}', '${_pipelines.length}', '${_instances.length}', ''];
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
          if (!isSettings) ...[
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
          ],
          const Spacer(),
          if (!isSettings && isTaskTab)
            _HeaderButton(
              icon: Icons.add,
              label: '新建',
              onTap: _addTask,
            ),
          if (!isSettings && _selectedIndex == 1)
            _HeaderButton(
              icon: Icons.add,
              label: '新建',
              onTap: _addPipeline,
            ),
          if (!isSettings)
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
        const SettingsPage(),
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
        onEdit: () => _editPipeline(_pipelines[i]),
        onDelete: () => _deletePipeline(_pipelines[i]),
      ),
    );
  }

  // --------------- 实例 Tab ---------------

  Widget _buildInstanceTab() {
    if (_instances.isEmpty && _runs.isEmpty) {
      return _EmptyState(
        icon: Icons.history_outlined,
        title: '暂无实例',
        subtitle: '运行任务后，实例会出现在这里',
      );
    }

    // 编排实例按 runId 分组
    final Map<String, List<TaskInstance>> runGroups = {};
    for (final i in _instances.where((i) => i.runId != null)) {
      runGroups.putIfAbsent(i.runId!, () => []).add(i);
    }

    // 运行记录索引
    final runMap = <String, PipelineRunState>{};
    for (final r in _runs) {
      runMap[r.runId] = r;
    }

    // 构建统一列表，每个元素有 startedAt 作为排序键
    final items = <_InstanceListItem>[];

    // 添加单独实例
    for (final inst in _instances.where((i) => i.runId == null)) {
      items.add(_InstanceListItem.solo(
        startedAt: inst.startedAt.toInt(),
        builder: (_) => _InstanceCard(
          instance: inst,
          onTap: () => _viewInstance(inst),
          onKill: inst.status is InstanceStatus_Running
              ? () => _killInstance(inst)
              : null,
          onClear: inst.status is! InstanceStatus_Running
              ? () => _removeInstance(inst)
              : null,
        ),
      ));
    }

    // 添加编排运行
    for (final run in _runs) {
      final runInstances = runGroups[run.runId] ?? [];
      runInstances.sort((a, b) => a.startedAt.toInt().compareTo(b.startedAt.toInt()));
      final runStartedAt = run.startedAt.toInt();
      items.add(_InstanceListItem.run(
        startedAt: runStartedAt,
        builder: (_) => _PipelineRunGroup(
          run: run,
          instances: runInstances,
          onView: _viewInstance,
          onKill: _killInstance,
          onClear: _removeInstance,
        ),
      ));
    }

    // 添加孤实例（有 runId 但无对应运行记录）
    for (final entry in runGroups.entries.where((e) => !runMap.containsKey(e.key))) {
      // 以该组最早实例的时间作为排序键
      final minStart = entry.value
          .map((i) => i.startedAt.toInt())
          .reduce((a, b) => a < b ? a : b);
      items.add(_InstanceListItem.solo(
        startedAt: minStart,
        builder: (_) => _PipelineInstanceGroup(
          runName: '未知运行',
          instances: entry.value,
          onView: _viewInstance,
          onKill: _killInstance,
          onClear: _removeInstance,
        ),
      ));
    }

    // 统一按 startedAt 倒序排列
    items.sort((a, b) => b.startedAt.compareTo(a.startedAt));

    return RefreshIndicator(
      onRefresh: _loadData,
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: items.map((item) => item.builder(context)).toList(),
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
                  size: 20,
                  color: isSelected
                      ? AppTheme.accent
                      : (isDark ? AppTheme.darkTextSecondary : AppTheme.lightTextSecondary),
                ),
                const SizedBox(width: 10),
                // 文字
                Text(
                  label,
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: isSelected ? FontWeight.w600 : FontWeight.w500,
                    color: isSelected
                        ? AppTheme.accent
                        : (isDark ? AppTheme.darkTextSecondary : AppTheme.lightTextSecondary),
                  ),
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
                          fontFamily: AppTheme.monoFont,
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
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  const _PipelineCard({
    required this.pipeline,
    required this.onRun,
    required this.onEdit,
    required this.onDelete,
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
                      Row(
                        children: [
                          _MetaLabel(
                            label: '开始',
                            value: _fmtTime(instance.startedAt.toInt()),
                          ),
                          const SizedBox(width: 12),
                          _MetaLabel(
                            label: '耗时',
                            value: _fmtDuration(
                              instance.startedAt.toInt(),
                              instance.endedAt?.toInt(),
                            ),
                          ),
                          const SizedBox(width: 12),
                          StatusBadge(status: instance.status),
                        ],
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

}


// ============================================================
// 通用辅助函数
// ============================================================

String _fmtDuration(int start, int? end) {
  final e = end ?? DateTime.now().millisecondsSinceEpoch ~/ 1000;
  final s = e - start;
  if (s < 60) return '${s}s';
  if (s < 3600) return '${s ~/ 60}m ${s % 60}s';
  return '${s ~/ 3600}h ${(s % 3600) ~/ 60}m';
}

String _fmtTime(int epoch) {
  final dt = DateTime.fromMillisecondsSinceEpoch(epoch * 1000);
  return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
}

// ============================================================
// 编排实例组 — 可展开
// ============================================================

class _PipelineRunGroup extends StatefulWidget {
  final PipelineRunState run;
  final List<TaskInstance> instances;
  final void Function(TaskInstance) onView;
  final void Function(TaskInstance) onKill;
  final void Function(TaskInstance) onClear;

  const _PipelineRunGroup({
    required this.run,
    required this.instances,
    required this.onView,
    required this.onKill,
    required this.onClear,
  });

  @override
  State<_PipelineRunGroup> createState() => _PipelineRunGroupState();
}

class _PipelineRunGroupState extends State<_PipelineRunGroup> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final run = widget.run;
    final runningCount = widget.instances.where((i) => i.status is InstanceStatus_Running).length;
    final totalCount = widget.instances.length;
    final runDuration = _fmtDuration(run.startedAt.toInt(), run.endedAt?.toInt());
    final runStatus = run.status == PipelineStatus.running
        ? '运行中'
        : (run.status == PipelineStatus.completed ? '已完成' : '失败');

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
      child: Column(
        children: [
          // 父项：点击展开/收起
          Material(
            color: Colors.transparent,
            borderRadius: BorderRadius.vertical(
              top: const Radius.circular(8),
              bottom: Radius.circular(_expanded ? 0 : 8),
            ),
            child: InkWell(
              onTap: () => setState(() => _expanded = !_expanded),
              borderRadius: BorderRadius.vertical(
                top: const Radius.circular(8),
                bottom: Radius.circular(_expanded ? 0 : 8),
              ),
              hoverColor: AppTheme.hover(isDark),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Row(
                  children: [
                    Icon(
                      _expanded ? Icons.expand_less : Icons.expand_more,
                      size: 20,
                      color: AppTheme.textSecondary(isDark),
                    ),
                    const SizedBox(width: 8),
                    Icon(
                      Icons.account_tree_outlined,
                      size: 18,
                      color: AppTheme.accent,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            run.pipelineName,
                            style: TextStyle(
                              fontWeight: FontWeight.w600,
                              fontSize: 14,
                              color: AppTheme.text(isDark),
                            ),
                          ),
                          const SizedBox(height: 4),
                          Row(
                            children: [
                              _MetaLabel(
                                label: '开始',
                                value: _fmtTime(run.startedAt.toInt()),
                              ),
                              const SizedBox(width: 12),
                              _MetaLabel(
                                label: '耗时',
                                value: runDuration,
                              ),
                              const SizedBox(width: 12),
                              _MetaLabel(
                                label: '状态',
                                value: runStatus,
                              ),
                            ],
                          ),
                        ],
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
                        '$totalCount',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w500,
                          color: AppTheme.textSecondary(isDark),
                        ),
                      ),
                    ),
                    if (runningCount > 0) ...[
                      const SizedBox(width: 6),
                      Container(
                        width: 8,
                        height: 8,
                        decoration: const BoxDecoration(
                          color: AppTheme.success,
                          shape: BoxShape.circle,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
          // 子项列表
          if (_expanded)
            Container(
              decoration: BoxDecoration(
                border: Border(
                  top: BorderSide(color: AppTheme.divider(isDark)),
                ),
              ),
              child: Column(
                children: widget.instances.map((inst) {
                  final instDuration = _fmtDuration(inst.startedAt.toInt(), inst.endedAt?.toInt());
                  return Container(
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                    decoration: BoxDecoration(
                      border: Border(
                        bottom: BorderSide(color: AppTheme.divider(isDark)),
                      ),
                    ),
                    child: Material(
                      color: Colors.transparent,
                      child: InkWell(
                        onTap: () => widget.onView(inst),
                        borderRadius: BorderRadius.circular(6),
                        hoverColor: AppTheme.hover(isDark),
                        child: Padding(
                          padding: const EdgeInsets.all(8),
                          child: Row(
                            children: [
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      inst.taskName,
                                      style: TextStyle(
                                        fontSize: 13,
                                        fontWeight: FontWeight.w500,
                                        color: AppTheme.text(isDark),
                                      ),
                                    ),
                                    const SizedBox(height: 4),
                                    Row(
                                      children: [
                                        _MetaLabel(
                                          label: '开始',
                                          value: _fmtTime(inst.startedAt.toInt()),
                                        ),
                                        const SizedBox(width: 12),
                                        _MetaLabel(
                                          label: '耗时',
                                          value: instDuration,
                                        ),
                                        const SizedBox(width: 12),
                                        StatusBadge(status: inst.status),
                                      ],
                                    ),
                                  ],
                                ),
                              ),
                              const SizedBox(width: 8),
                              if (inst.status is InstanceStatus_Running)
                                _IconAction(
                                  icon: Icons.stop,
                                  color: AppTheme.error,
                                  onTap: () => widget.onKill(inst),
                                  tooltip: '停止',
                                ),
                              if (inst.status is! InstanceStatus_Running)
                                _IconAction(
                                  icon: Icons.clear,
                                  color: AppTheme.textMuted(isDark),
                                  onTap: () => widget.onClear(inst),
                                  tooltip: '清除',
                                ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  );
                }).toList(),
              ),
            ),
        ],
      ),
    );
  }
}

class _PipelineInstanceGroup extends StatefulWidget {
  final String runName;
  final List<TaskInstance> instances;
  final void Function(TaskInstance) onView;
  final void Function(TaskInstance) onKill;
  final void Function(TaskInstance) onClear;

  const _PipelineInstanceGroup({
    required this.runName,
    required this.instances,
    required this.onView,
    required this.onKill,
    required this.onClear,
  });

  @override
  State<_PipelineInstanceGroup> createState() => _PipelineInstanceGroupState();
}

class _PipelineInstanceGroupState extends State<_PipelineInstanceGroup> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final runningCount = widget.instances.where((i) => i.status is InstanceStatus_Running).length;
    final totalCount = widget.instances.length;

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
      child: Column(
        children: [
          Material(
            color: Colors.transparent,
            borderRadius: BorderRadius.vertical(
              top: const Radius.circular(8),
              bottom: Radius.circular(_expanded ? 0 : 8),
            ),
            child: InkWell(
              onTap: () => setState(() => _expanded = !_expanded),
              borderRadius: BorderRadius.vertical(
                top: const Radius.circular(8),
                bottom: Radius.circular(_expanded ? 0 : 8),
              ),
              hoverColor: AppTheme.hover(isDark),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Row(
                  children: [
                    Icon(
                      _expanded ? Icons.expand_less : Icons.expand_more,
                      size: 20,
                      color: AppTheme.textSecondary(isDark),
                    ),
                    const SizedBox(width: 8),
                    Icon(
                      Icons.account_tree_outlined,
                      size: 18,
                      color: AppTheme.accent,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        widget.runName,
                        style: TextStyle(
                          fontWeight: FontWeight.w600,
                          fontSize: 14,
                          color: AppTheme.text(isDark),
                        ),
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
                        '$totalCount',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w500,
                          color: AppTheme.textSecondary(isDark),
                        ),
                      ),
                    ),
                    if (runningCount > 0) ...[
                      const SizedBox(width: 6),
                      Container(
                        width: 8,
                        height: 8,
                        decoration: const BoxDecoration(
                          color: AppTheme.success,
                          shape: BoxShape.circle,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
          if (_expanded)
            Container(
              decoration: BoxDecoration(
                border: Border(
                  top: BorderSide(color: AppTheme.divider(isDark)),
                ),
              ),
              child: Column(
                children: widget.instances.map((inst) {
                  return Container(
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                    decoration: BoxDecoration(
                      border: Border(
                        bottom: BorderSide(color: AppTheme.divider(isDark)),
                      ),
                    ),
                    child: Material(
                      color: Colors.transparent,
                      child: InkWell(
                        onTap: () => widget.onView(inst),
                        borderRadius: BorderRadius.circular(6),
                        hoverColor: AppTheme.hover(isDark),
                        child: Padding(
                          padding: const EdgeInsets.all(8),
                          child: Row(
                            children: [
                              Expanded(
                                child: Column(
                                  crossAxisAlignment: CrossAxisAlignment.start,
                                  children: [
                                    Text(
                                      inst.taskName,
                                      style: TextStyle(
                                        fontSize: 13,
                                        fontWeight: FontWeight.w500,
                                        color: AppTheme.text(isDark),
                                      ),
                                    ),
                                    const SizedBox(height: 4),
                                    StatusBadge(status: inst.status),
                                  ],
                                ),
                              ),
                              const SizedBox(width: 8),
                              if (inst.status is InstanceStatus_Running)
                                _IconAction(
                                  icon: Icons.stop,
                                  color: AppTheme.error,
                                  onTap: () => widget.onKill(inst),
                                  tooltip: '停止',
                                ),
                              if (inst.status is! InstanceStatus_Running)
                                _IconAction(
                                  icon: Icons.clear,
                                  color: AppTheme.textMuted(isDark),
                                  onTap: () => widget.onClear(inst),
                                  tooltip: '清除',
                                ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  );
                }).toList(),
              ),
            ),
        ],
      ),
    );
  }
}

// ============================================================
// 元标签
// ============================================================

class _MetaLabel extends StatelessWidget {
  final String label;
  final String value;

  const _MetaLabel({
    required this.label,
    required this.value,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

    return Text.rich(
      TextSpan(
        children: [
          TextSpan(
            text: '$label: ',
            style: TextStyle(
              fontSize: 11,
              color: AppTheme.textMuted(isDark),
            ),
          ),
          TextSpan(
            text: value,
            style: TextStyle(
              fontSize: 11,
              color: AppTheme.textSecondary(isDark),
            ),
          ),
        ],
      ),
    );
  }
}

// ============================================================
// 图标操作
// ============================================================

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
// 实例列表统一排序项
// ============================================================

class _InstanceListItem {
  final int startedAt;
  final Widget Function(BuildContext) builder;

  const _InstanceListItem._({
    required this.startedAt,
    required this.builder,
  });

  factory _InstanceListItem.solo({
    required int startedAt,
    required Widget Function(BuildContext) builder,
  }) =>
      _InstanceListItem._(startedAt: startedAt, builder: builder);

  factory _InstanceListItem.run({
    required int startedAt,
    required Widget Function(BuildContext) builder,
  }) =>
      _InstanceListItem._(startedAt: startedAt, builder: builder);
}

// ============================================================
// 导航项数据
// ============================================================

class _NavItem {
  final IconData icon;
  final String label;

  const _NavItem({required this.icon, required this.label});
}
