import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/cmdhub_service.dart';
import '../src/rust/models.dart';
import '../theme/app_theme.dart';
import '../widgets/status_badge.dart';
import 'home_page.dart';

class InstanceDetailPage extends StatefulWidget {
  final String instanceId;
  const InstanceDetailPage({super.key, required this.instanceId});

  @override
  State<InstanceDetailPage> createState() => _InstanceDetailPageState();
}

class _InstanceDetailPageState extends State<InstanceDetailPage> {
  final _service = CmdHubService();
  final _inputController = TextEditingController();
  final _scrollController = ScrollController();
  bool _autoScroll = true;
  Timer? _pollTimer;
  TaskInstance? _instance;
  String _fullOutput = '';
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
    _pollTimer = Timer.periodic(const Duration(milliseconds: 500), (_) => _poll());
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    _inputController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final i = await _service.getInstance(widget.instanceId);
      if (mounted) setState(() { _instance = i; _loading = false; });
    } catch (_) {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _poll() async {
    try {
      final out = await _service.readOutput(widget.instanceId);
      if (out != _fullOutput) {
        _fullOutput = out;
        if (mounted) setState(() {});
        if (_autoScroll) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (_scrollController.hasClients) {
              _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
            }
          });
        }
      }
      _load();
    } catch (_) {}
  }

  void _sendInput() {
    final t = _inputController.text;
    if (t.isEmpty) return;
    _service.writeInput(widget.instanceId, '$t\n');
    _inputController.clear();
  }

  Future<void> _kill() async {
    await _service.killInstance(widget.instanceId);
    _load();
    _poll();
  }

  void _copy() async {
    final text = _fullOutput.endsWith('\n')
        ? _fullOutput.substring(0, _fullOutput.length - 1)
        : _fullOutput;
    await Clipboard.setData(ClipboardData(text: text));
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('已复制输出到剪贴板')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return Scaffold(
        body: _buildHeader(context, '实例详情', null)
      );
    }

    final inst = _instance;
    if (inst == null) {
      return Scaffold(
        body: _buildHeader(context, '实例详情', null)
      );
    }

    final isDark = Theme.of(context).brightness == Brightness.dark;
    final isRunning = inst.status is InstanceStatus_Running;
    final lines = _fullOutput.split('\n');

    return Scaffold(
      body: Column(
        children: [
          // 顶部：只返回 + 标题
          _buildHeader(context, inst.taskName, null),
          Expanded(
            child: Column(
              children: [
                // 信息栏：PID 左、命令中、操作右
                Container(
                  margin: const EdgeInsets.all(16),
                  padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                  decoration: BoxDecoration(
                    color: AppTheme.card(isDark),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: AppTheme.border(isDark)),
                  ),
                  child: Stack(
                    children: [
                      // 命令居中
                      Center(
                        child: Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 120),
                          child: Text(
                            _fmtCmd(inst.command),
                            textAlign: TextAlign.center,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              fontSize: 11,
                              fontFamily: AppTheme.monoFont,
                              color: AppTheme.textMuted(isDark),
                            ),
                          ),
                        ),
                      ),
                      // 左侧：状态 + PID + 时间
                      Positioned(
                        left: 0,
                        top: 0,
                        bottom: 0,
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            StatusBadge(status: inst.status),
                            const SizedBox(width: 8),
                            if (inst.childPid != null)
                              Padding(
                                padding: const EdgeInsets.only(right: 8),
                                child: Text(
                                  'PID: ${inst.childPid}',
                                  style: TextStyle(
                                    fontSize: 12,
                                    color: AppTheme.textSecondary(isDark),
                                  ),
                                ),
                              ),
                            _timeLabel('开始', _fmtTime(inst.startedAt.toInt()), isDark),
                            const SizedBox(width: 8),
                            _timeLabel('耗时', _fmtDuration(
                              inst.startedAt.toInt(),
                              inst.endedAt?.toInt(),
                            ), isDark),
                          ],
                        ),
                      ),
                      // 右侧：操作按钮
                      Positioned(
                        right: 0,
                        top: 0,
                        bottom: 0,
                        child: Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            if (isRunning)
                              _InfoAction(
                                icon: Icons.stop,
                                color: AppTheme.error,
                                onTap: _kill,
                                tooltip: '停止',
                              ),
                            _InfoAction(
                              icon: Icons.copy,
                              onTap: _copy,
                              tooltip: '复制输出',
                            ),
                            _InfoAction(
                              icon: Icons.refresh,
                              onTap: _poll,
                              tooltip: '刷新',
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
            ),
                // 输出
                Expanded(
                  child: Container(
                    margin: const EdgeInsets.symmetric(horizontal: 16),
                    decoration: BoxDecoration(
                      color: const Color(0xFF0D1117),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: const Color(0xFF30363D)),
                    ),
                    child: NotificationListener<ScrollNotification>(
                      onNotification: (n) {
                        if (n is ScrollEndNotification) {
                          final max = _scrollController.position.maxScrollExtent;
                          _autoScroll = _scrollController.position.pixels >= max - 20;
                        }
                        return false;
                      },
                      child: ListView.builder(
                        controller: _scrollController,
                        padding: const EdgeInsets.all(12),
                        itemCount: lines.length,
                        itemBuilder: (_, i) => Text(
                          lines[i],
                          style: const TextStyle(
                            fontFamily: 'monospace',
                            fontSize: 13,
                            height: 1.5,
                            color: Color(0xFFD4D4D4),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
                // 输入栏
                if (isRunning)
                  Container(
                    margin: const EdgeInsets.all(16),
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: AppTheme.card(isDark),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: AppTheme.border(isDark)),
                    ),
                    child: Row(
                      children: [
                        Expanded(
                          child: TextField(
                            controller: _inputController,
                            style: TextStyle(fontFamily: AppTheme.monoFont),
                            decoration: const InputDecoration(
                              hintText: '输入命令...',
                              border: OutlineInputBorder(),
                              isDense: true,
                              contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 10),
                            ),
                            onSubmitted: (_) => _sendInput(),
                          ),
                        ),
                        const SizedBox(width: 8),
                        IconButton(
                          icon: const Icon(Icons.send, color: AppTheme.accent),
                          onPressed: _sendInput,
                        ),
                      ],
                    ),
                  ),

              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHeader(BuildContext context, String title, String? subtitle) {
    final isDark = Theme.of(context).brightness == Brightness.dark;

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
          _HeaderAction(
            icon: Icons.list_alt,
            onTap: () {
              Navigator.pushAndRemoveUntil(
                context,
                MaterialPageRoute(builder: (_) => const HomePage(initialIndex: 2)),
                (route) => false,
              );
            },
            tooltip: '实例列表',
          ),
        ],
      ),
    );
  }

  String _fmtCmd(String c) => c.length > 50 ? '${c.substring(0, 50)}...' : c;
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

// ============================================================
// 信息栏操作按钮
// ============================================================

class _InfoAction extends StatelessWidget {
  final IconData icon;
  final VoidCallback onTap;
  final String? tooltip;
  final Color? color;

  const _InfoAction({
    required this.icon,
    required this.onTap,
    this.tooltip,
    this.color,
  });

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    return Tooltip(
      message: tooltip ?? '',
      child: Material(
        color: Colors.transparent,
        shape: const CircleBorder(),
        child: InkWell(
          onTap: onTap,
          customBorder: const CircleBorder(),
          child: Padding(
            padding: const EdgeInsets.all(6),
            child: Icon(icon, size: 18, color: color ?? AppTheme.textSecondary(isDark)),
          ),
        ),
      ),
    );
  }
}

// ============================================================
// 辅助函数
// ============================================================

Widget _timeLabel(String label, String value, bool isDark) {
  return Text.rich(
    TextSpan(
      children: [
        TextSpan(
          text: '$label: ',
          style: TextStyle(fontSize: 11, color: AppTheme.textMuted(isDark)),
        ),
        TextSpan(
          text: value,
          style: TextStyle(fontSize: 11, color: AppTheme.textSecondary(isDark)),
        ),
      ],
    ),
  );
}

String _fmtTime(int epoch) {
  final dt = DateTime.fromMillisecondsSinceEpoch(epoch * 1000);
  return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}:${dt.second.toString().padLeft(2, '0')}';
}

String _fmtDuration(int start, int? end) {
  final e = end ?? DateTime.now().millisecondsSinceEpoch ~/ 1000;
  final s = e - start;
  if (s < 60) return '${s}s';
  if (s < 3600) return '${s ~/ 60}m ${s % 60}s';
  return '${s ~/ 3600}h ${(s % 3600) ~/ 60}m';
}
