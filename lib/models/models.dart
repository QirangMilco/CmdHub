/// 任务执行模式
enum TaskMode {
  oneshot,
  interactive,
}

/// 实例状态
enum InstanceStatusType {
  running,
  exited,
  error,
}

/// 任务模型
class TaskModel {
  final String id;
  String name;
  String command;
  String? cwd;
  Map<String, String> env;
  bool envInherit;
  TaskMode mode;

  TaskModel({
    required this.id,
    required this.name,
    required this.command,
    this.cwd,
    Map<String, String>? env,
    this.envInherit = true,
    this.mode = TaskMode.oneshot,
  }) : env = env ?? {};

  TaskModel copyWith({
    String? name,
    String? command,
    String? cwd,
    Map<String, String>? env,
    bool? envInherit,
    TaskMode? mode,
  }) {
    return TaskModel(
      id: id,
      name: name ?? this.name,
      command: command ?? this.command,
      cwd: cwd ?? this.cwd,
      env: env ?? Map<String, String>.from(this.env),
      envInherit: envInherit ?? this.envInherit,
      mode: mode ?? this.mode,
    );
  }
}

/// 编排步骤触发条件
enum StepConditionType {
  onStart,
  afterPrevious,
  afterDelay,
}

/// 编排步骤
class PipelineStepModel {
  String taskId;
  int order;
  int delayMs;
  StepConditionType condition;

  PipelineStepModel({
    required this.taskId,
    required this.order,
    this.delayMs = 0,
    this.condition = StepConditionType.onStart,
  });
}

/// 编排模型
class PipelineModel {
  final String id;
  String name;
  List<PipelineStepModel> steps;

  PipelineModel({
    required this.id,
    required this.name,
    List<PipelineStepModel>? steps,
  }) : steps = steps ?? [];
}

/// 任务实例模型
class TaskInstanceModel {
  final String id;
  final String taskId;
  final String taskName;
  final String command;
  final InstanceStatusType status;
  final int? exitCode;
  final String? errorMessage;
  final int startedAt;
  final int? endedAt;
  final int? childPid;

  TaskInstanceModel({
    required this.id,
    required this.taskId,
    required this.taskName,
    required this.command,
    required this.status,
    this.exitCode,
    this.errorMessage,
    required this.startedAt,
    this.endedAt,
    this.childPid,
  });

  bool get isRunning => status == InstanceStatusType.running;

  String get statusText {
    switch (status) {
      case InstanceStatusType.running:
        return '运行中';
      case InstanceStatusType.exited:
        return '已退出 (${exitCode ?? 0})';
      case InstanceStatusType.error:
        return '错误: ${errorMessage ?? "未知"}';
    }
  }

  String get durationText {
    final end = endedAt ?? DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final seconds = end - startedAt;
    if (seconds < 60) return '${seconds}s';
    if (seconds < 3600) return '${seconds ~/ 60}m ${seconds % 60}s';
    return '${seconds ~/ 3600}h ${(seconds % 3600) ~/ 60}m';
  }
}
