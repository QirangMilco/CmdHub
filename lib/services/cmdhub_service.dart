import 'dart:convert';
import '../src/rust/api/cmdhub_api.dart' as api;
import '../src/rust/models.dart';

/// CmdHub 服务层
///
/// 封装 flutter_rust_bridge 生成的所有 API 调用。
class CmdHubService {
  static final CmdHubService _instance = CmdHubService._();
  factory CmdHubService() => _instance;
  CmdHubService._();

  // ========================================
  // 任务管理
  // ========================================

  Future<List<Task>> listTasks() => api.listTasks();

  Future<Task> createTask(Task task) => api.createTask(task: task);

  Future<Task> updateTask(Task task) => api.updateTask(task: task);

  Future<void> deleteTask(String id) => api.deleteTask(id: id);

  Future<Task?> getTask(String id) => api.getTask(id: id);

  // ========================================
  // 实例管理
  // ========================================

  Future<TaskInstance> spawnTask(String taskId) =>
      api.spawnTask(taskId: taskId);

  Future<void> killInstance(String instanceId) =>
      api.killInstance(instanceId: instanceId);

  Future<List<TaskInstance>> listInstances() => api.listInstances();

  Future<TaskInstance?> getInstance(String instanceId) =>
      api.getInstance(instanceId: instanceId);

  Future<String> readOutput(String instanceId) async {
    final bytes = await api.readOutput(instanceId: instanceId);
    return utf8.decode(bytes);
  }

  Future<void> writeInput(String instanceId, String input) =>
      api.writeInput(
        instanceId: instanceId,
        data: utf8.encode(input),
      );

  Future<bool> removeInstance(String instanceId) =>
      api.removeInstance(instanceId: instanceId);

  Future<int> runningCount() => api.runningCount();

  Future<List<InstanceEvent>> getEventsSince(int lastIndex) =>
      api.getInstanceEventsSince(lastEventIndex: lastIndex);

  // ========================================
  // 编排管理
  // ========================================

  Future<List<Pipeline>> listPipelines() => api.listPipelines();

  Future<Pipeline> createPipeline(Pipeline pipeline) =>
      api.createPipeline(pipeline: pipeline);

  Future<Pipeline> updatePipeline(Pipeline pipeline) =>
      api.updatePipeline(pipeline: pipeline);

  Future<void> deletePipeline(String id) => api.deletePipeline(id: id);

  Future<Pipeline?> getPipeline(String id) => api.getPipeline(id: id);

  Future<String> runPipeline(String id) => api.runPipeline(id: id);

  Future<void> cancelPipeline(String runId) =>
      api.cancelPipeline(runId: runId);

  Future<List<PipelineRunState>> listPipelineRuns() => api.listPipelineRuns();
}
