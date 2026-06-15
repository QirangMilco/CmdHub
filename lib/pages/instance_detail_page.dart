import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/cmdhub_service.dart';
import '../src/rust/models.dart';
import '../widgets/custom_app_bar.dart';
import '../widgets/status_badge.dart';

class InstanceDetailPage extends StatefulWidget {
  final String instanceId;

  const InstanceDetailPage({super.key, required this.instanceId});

  @override
  State<InstanceDetailPage> createState() => _InstanceDetailPageState();
}

class _InstanceDetailPageState extends State<InstanceDetailPage> {
  final _service = CmdHubService();
  final _inputController = TextEditingController();
  final _outputLines = <String>[];
  final _scrollController = ScrollController();
  bool _autoScroll = true;
  Timer? _pollTimer;
  TaskInstance? _instance;
  String _fullOutput = '';
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _loadInstance();
    _pollTimer = Timer.periodic(
      const Duration(milliseconds: 500),
      (_) => _pollOutput(),
    );
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    _inputController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  Future<void> _loadInstance() async {
    try {
      final instance = await _service.getInstance(widget.instanceId);
      if (mounted) {
        setState(() {
          _instance = instance;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  Future<void> _pollOutput() async {
    try {
      final output = await _service.readOutput(widget.instanceId);
      if (output != _fullOutput) {
        _fullOutput = output;
        _outputLines.clear();
        _outputLines.addAll(output.split('\n'));
        if (mounted) setState(() {});
        if (_autoScroll) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (_scrollController.hasClients) {
              _scrollController.jumpTo(
                _scrollController.position.maxScrollExtent,
              );
            }
          });
        }
      }
      // 也刷新实例状态
      _loadInstance();
    } catch (_) {}
  }

  void _sendInput() {
    final text = _inputController.text;
    if (text.isEmpty) return;
    _service.writeInput(widget.instanceId, '$text\n');
    _inputController.clear();
  }

  Future<void> _kill() async {
    await _service.killInstance(widget.instanceId);
    _loadInstance();
    _pollOutput();
  }

  Future<void> _copyOutput() async {
    await Clipboard.setData(ClipboardData(text: _fullOutput));
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
        appBar: const CustomAppBar(title: Text('实例详情')),
        body: const Center(child: CircularProgressIndicator()),
      );
    }

    if (_instance == null) {
      return Scaffold(
        appBar: const CustomAppBar(title: Text('实例详情')),
        body: const Center(child: Text('实例未找到')),
      );
    }

    final inst = _instance!;
    final isRunning = inst.status is InstanceStatus_Running;

    return Scaffold(
      appBar: CustomAppBar(
        title: Text(inst.taskName),
        actions: [
          IconButton(
            icon: const Icon(Icons.copy, color: Colors.white),
            onPressed: _copyOutput,
            tooltip: '复制输出',
          ),
          IconButton(
            icon: const Icon(Icons.refresh, color: Colors.white),
            onPressed: () {
              _loadInstance();
              _pollOutput();
            },
            tooltip: '刷新',
          ),
        ],
      ),
      body: Column(
        children: [
          // 信息栏
          Container(
            color: Colors.grey[100],
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                StatusBadge(status: inst.status),
                const SizedBox(width: 12),
                if (inst.childPid != null) ...[
                  Text('PID: ${inst.childPid}',
                      style: const TextStyle(fontSize: 12)),
                  const SizedBox(width: 12),
                ],
                Text(_formatTime(inst.startedAt.toInt()),
                    style: TextStyle(fontSize: 12, color: Colors.grey[600])),
                const Spacer(),
                Text(
                  _formatCommand(inst.command),
                  style: TextStyle(
                    fontSize: 11,
                    fontFamily: 'monospace',
                    color: Colors.grey[600],
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),

          // 输出区域
          Expanded(
            child: NotificationListener<ScrollNotification>(
              onNotification: (notification) {
                if (notification is ScrollEndNotification) {
                  final max = _scrollController.position.maxScrollExtent;
                  final current = _scrollController.position.pixels;
                  _autoScroll = current >= max - 20;
                }
                return false;
              },
              child: ListView.builder(
                controller: _scrollController,
                padding: const EdgeInsets.all(12),
                itemCount: _outputLines.length,
                itemBuilder: (context, index) {
                  return Text(
                    _outputLines[index],
                    style: const TextStyle(
                      fontFamily: 'monospace',
                      fontSize: 13,
                      height: 1.4,
                    ),
                  );
                },
              ),
            ),
          ),

          // 输入栏（仅交互式任务且运行中可用）
          if (isRunning)
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: Colors.grey[100],
                border: Border(top: BorderSide(color: Colors.grey[300]!)),
              ),
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      controller: _inputController,
                      style: const TextStyle(fontFamily: 'monospace'),
                      decoration: const InputDecoration(
                        hintText: '输入命令...',
                        border: OutlineInputBorder(),
                        isDense: true,
                        contentPadding: EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 8,
                        ),
                      ),
                      onSubmitted: (_) => _sendInput(),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton(
                    icon: const Icon(Icons.send, color: Colors.teal),
                    onPressed: _sendInput,
                  ),
                ],
              ),
            ),

          // 底部操作栏
          Container(
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: Colors.grey[100],
              border: Border(top: BorderSide(color: Colors.grey[300]!)),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                if (isRunning)
                  ElevatedButton.icon(
                    icon: const Icon(Icons.stop),
                    label: const Text('停止'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Colors.red,
                      foregroundColor: Colors.white,
                    ),
                    onPressed: _kill,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  String _formatTime(int epoch) {
    final dt =
        DateTime.fromMillisecondsSinceEpoch(epoch * 1000);
    return '${dt.month}/${dt.day} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  String _formatCommand(String cmd) {
    return cmd.length > 40 ? '${cmd.substring(0, 40)}...' : cmd;
  }
}
