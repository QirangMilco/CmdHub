import 'package:flutter/material.dart';
import '../services/cmdhub_service.dart';
import '../src/rust/models.dart';
import '../widgets/custom_app_bar.dart';
import '../widgets/status_badge.dart';
import 'instance_detail_page.dart';
import 'task_editor_page.dart';

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  final _service = CmdHubService();

  List<Task> _tasks = [];
  List<Pipeline> _pipelines = [];
  List<TaskInstance> _instances = [];

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    _loadData();
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  Future<void> _loadData() async {
    try {
      final tasks = await _service.listTasks();
      final pipelines = await _service.listPipelines();
      final instances = await _service.listInstances();
      if (mounted) {
        setState(() {
          _tasks = tasks;
          _pipelines = pipelines;
          _instances = instances;
        });
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('加载失败: $e')),
        );
      }
    }
  }

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
        title: Text('删除任务: ${task.name}'),
        content: const Text('此操作不可撤销'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('取消'),
          ),
          TextButton(
            onPressed: () async {
              Navigator.pop(ctx);
              await _service.deleteTask(task.id);
              _loadData();
            },
            style: TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('删除'),
          ),
        ],
      ),
    );
  }

  Future<void> _killInstance(TaskInstance instance) async {
    await _service.killInstance(instance.id);
    _loadData();
  }

  Future<void> _removeInstance(TaskInstance instance) async {
    await _service.removeInstance(instance.id);
    _loadData();
  }

  void _viewInstance(TaskInstance instance) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => InstanceDetailPage(instanceId: instance.id),
      ),
    ).then((_) => _loadData());
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: CustomAppBar(
        title: const Text('CmdHub', style: TextStyle(fontSize: 18)),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _loadData,
            tooltip: '刷新',
          ),
          if (_tabController.index == 0)
            IconButton(
              icon: const Icon(Icons.add),
              onPressed: _addTask,
              tooltip: '新建任务',
            ),
        ],
      ),
      body: Column(
        children: [
          TabBar(
            controller: _tabController,
            labelColor: Colors.teal,
            unselectedLabelColor: Colors.grey,
            onTap: (_) => _loadData(),
            tabs: const [
              Tab(text: '任务'),
              Tab(text: '编排'),
              Tab(text: '实例'),
            ],
          ),
          Expanded(
            child: TabBarView(
              controller: _tabController,
              children: [
                _buildTaskList(),
                _buildPipelineList(),
                _buildInstanceList(),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTaskList() {
    if (_tasks.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.terminal, size: 48, color: Colors.grey[400]),
            const SizedBox(height: 12),
            Text(
              '暂无任务\n点击右上角 + 创建',
              textAlign: TextAlign.center,
              style: TextStyle(color: Colors.grey[500], fontSize: 14),
            ),
          ],
        ),
      );
    }

    return ListView.builder(
      itemCount: _tasks.length,
      itemBuilder: (context, index) {
        final task = _tasks[index];
        return Container(
          color: index % 2 == 0 ? Colors.grey[100] : Colors.white,
          child: ListTile(
            title: Text(
              task.name,
              style: const TextStyle(fontWeight: FontWeight.w500),
            ),
            subtitle: Row(
              children: [
                Flexible(
                  child: Text(
                    task.command,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontFamily: 'monospace',
                      fontSize: 11,
                      color: Colors.grey[600],
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 6,
                    vertical: 2,
                  ),
                  decoration: BoxDecoration(
                    color: task.mode == TaskMode.interactive
                        ? Colors.teal[50]
                        : Colors.grey[200],
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    task.mode == TaskMode.interactive ? '交互' : '一次性',
                    style: TextStyle(
                      fontSize: 10,
                      color: task.mode == TaskMode.interactive
                          ? Colors.teal
                          : Colors.grey[600],
                    ),
                  ),
                ),
              ],
            ),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                IconButton(
                  icon: const Icon(Icons.play_arrow, color: Colors.green),
                  onPressed: () => _runTask(task),
                  tooltip: '运行',
                ),
                IconButton(
                  icon: const Icon(Icons.edit, color: Colors.teal),
                  onPressed: () => _editTask(task),
                  tooltip: '编辑',
                ),
                IconButton(
                  icon: const Icon(Icons.delete, color: Colors.red),
                  onPressed: () => _deleteTask(task),
                  tooltip: '删除',
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  Widget _buildPipelineList() {
    if (_pipelines.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.account_tree, size: 48, color: Colors.grey[400]),
            const SizedBox(height: 12),
            Text(
              '暂无编排',
              textAlign: TextAlign.center,
              style: TextStyle(color: Colors.grey[500], fontSize: 14),
            ),
          ],
        ),
      );
    }

    return ListView.builder(
      itemCount: _pipelines.length,
      itemBuilder: (context, index) {
        final pipeline = _pipelines[index];
        return Container(
          color: index % 2 == 0 ? Colors.grey[100] : Colors.white,
          child: ListTile(
            title: Text(
              pipeline.name,
              style: const TextStyle(fontWeight: FontWeight.w500),
            ),
            subtitle: Text(
              '${pipeline.steps.length} 个步骤',
              style: TextStyle(color: Colors.grey[600], fontSize: 12),
            ),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                IconButton(
                  icon: const Icon(Icons.play_arrow, color: Colors.green),
                  onPressed: () async {
                    try {
                      await _service.runPipeline(pipeline.id);
                      if (mounted) {
                        // ignore: use_build_context_synchronously
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            content: Text('已启动编排: ${pipeline.name}'),
                          ),
                        );
                      }
                    } catch (e) {
                      if (mounted) {
                        // ignore: use_build_context_synchronously
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text('编排启动失败: $e')),
                        );
                      }
                    }
                  },
                  tooltip: '运行',
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  Widget _buildInstanceList() {
    if (_instances.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.history, size: 48, color: Colors.grey[400]),
            const SizedBox(height: 12),
            Text(
              '暂无实例记录',
              textAlign: TextAlign.center,
              style: TextStyle(color: Colors.grey[500], fontSize: 14),
            ),
          ],
        ),
      );
    }

    return ListView.builder(
      itemCount: _instances.length,
      itemBuilder: (context, index) {
        final instance = _instances[index];
        final isRunning = instance.status is InstanceStatus_Running;

        return Container(
          color: index % 2 == 0 ? Colors.grey[100] : Colors.white,
          child: ListTile(
            title: Text(instance.taskName),
            subtitle: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                StatusBadge(status: instance.status),
                Text(
                  _formatDuration(instance.startedAt.toInt(),
                      instance.endedAt?.toInt()),
                  style: TextStyle(fontSize: 11, color: Colors.grey[500]),
                ),
              ],
            ),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                IconButton(
                  icon: const Icon(Icons.article, color: Colors.teal),
                  onPressed: () => _viewInstance(instance),
                  tooltip: '查看输出',
                ),
                if (isRunning)
                  IconButton(
                    icon: const Icon(Icons.stop, color: Colors.red),
                    onPressed: () => _killInstance(instance),
                    tooltip: '停止',
                  )
                else
                  IconButton(
                    icon: const Icon(Icons.clear, color: Colors.grey),
                    onPressed: () => _removeInstance(instance),
                    tooltip: '清除',
                  ),
              ],
            ),
          ),
        );
      },
    );
  }

  String _formatDuration(int startedAt, int? endedAt) {
    final end = endedAt ??
        DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final seconds = end - startedAt;
    if (seconds < 60) return '${seconds}s';
    if (seconds < 3600) return '${seconds ~/ 60}m ${seconds % 60}s';
    return '${seconds ~/ 3600}h ${(seconds % 3600) ~/ 60}m';
  }
}
