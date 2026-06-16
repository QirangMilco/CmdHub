use crate::executor::Executor;
use crate::models::{
    InstanceEvent, Pipeline, PipelineRunState, Task, TaskInstance,
};
use crate::pipeline::PipelineRunner;
use crate::storage;
use anyhow::Result;
use flutter_rust_bridge::frb;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::runtime::Runtime;

// 全局单例
static EXECUTOR: OnceLock<Arc<Executor>> = OnceLock::new();
static PIPELINE_RUNNER: OnceLock<Arc<PipelineRunner>> = OnceLock::new();
static TOKIO_RT: OnceLock<Runtime> = OnceLock::new();

fn executor() -> &'static Arc<Executor> {
    EXECUTOR.get_or_init(|| Arc::new(Executor::new(64 * 1024)))
}

fn pipeline_runner() -> &'static Arc<PipelineRunner> {
    PIPELINE_RUNNER.get_or_init(|| Arc::new(PipelineRunner::new()))
}

fn tokio_runtime() -> &'static Runtime {
    TOKIO_RT.get_or_init(|| {
        Runtime::new().expect("failed to create Tokio runtime")
    })
}

// ============================================================
// 初始化
// ============================================================

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
    // 预初始化 Tokio runtime
    let _ = tokio_runtime();
}

// ============================================================
// 任务管理
// ============================================================

#[frb]
pub fn create_task(task: Task) -> Result<Task> {
    let mut tasks = storage::load_tasks()?;
    // 如果 id 为空则自动生成
    let mut task = task;
    if task.id.is_empty() {
        task.id = uuid::Uuid::new_v4().to_string();
    }
    // 检查 id 是否重复
    if tasks.iter().any(|t| t.id == task.id) {
        return Err(anyhow::anyhow!("task id already exists: {}", task.id));
    }
    tasks.push(task.clone());
    storage::save_tasks(&tasks)?;
    Ok(task)
}

#[frb]
pub fn update_task(task: Task) -> Result<Task> {
    let mut tasks = storage::load_tasks()?;
    let pos = tasks
        .iter()
        .position(|t| t.id == task.id)
        .ok_or_else(|| anyhow::anyhow!("task not found: {}", task.id))?;
    tasks[pos] = task.clone();
    storage::save_tasks(&tasks)?;
    Ok(task)
}

#[frb]
pub fn delete_task(id: String) -> Result<()> {
    let mut tasks = storage::load_tasks()?;
    tasks.retain(|t| t.id != id);
    storage::save_tasks(&tasks)?;
    Ok(())
}

#[frb]
pub fn list_tasks() -> Result<Vec<Task>> {
    storage::load_tasks()
}

#[frb]
pub fn get_task(id: String) -> Result<Option<Task>> {
    let tasks = storage::load_tasks()?;
    Ok(tasks.into_iter().find(|t| t.id == id))
}

// ============================================================
// 编排管理
// ============================================================

#[frb]
pub fn create_pipeline(pipeline: Pipeline) -> Result<Pipeline> {
    let mut pipelines = storage::load_pipelines()?;
    let mut pipeline = pipeline;
    if pipeline.id.is_empty() {
        pipeline.id = uuid::Uuid::new_v4().to_string();
    }
    if pipelines.iter().any(|p| p.id == pipeline.id) {
        return Err(anyhow::anyhow!(
            "pipeline id already exists: {}",
            pipeline.id
        ));
    }
    pipelines.push(pipeline.clone());
    storage::save_pipelines(&pipelines)?;
    Ok(pipeline)
}

#[frb]
pub fn update_pipeline(pipeline: Pipeline) -> Result<Pipeline> {
    let mut pipelines = storage::load_pipelines()?;
    let pos = pipelines
        .iter()
        .position(|p| p.id == pipeline.id)
        .ok_or_else(|| anyhow::anyhow!("pipeline not found: {}", pipeline.id))?;
    pipelines[pos] = pipeline.clone();
    storage::save_pipelines(&pipelines)?;
    Ok(pipeline)
}

#[frb]
pub fn delete_pipeline(id: String) -> Result<()> {
    let mut pipelines = storage::load_pipelines()?;
    pipelines.retain(|p| p.id != id);
    storage::save_pipelines(&pipelines)?;
    Ok(())
}

#[frb]
pub fn list_pipelines() -> Result<Vec<Pipeline>> {
    storage::load_pipelines()
}

#[frb]
pub fn get_pipeline(id: String) -> Result<Option<Pipeline>> {
    let pipelines = storage::load_pipelines()?;
    Ok(pipelines.into_iter().find(|p| p.id == id))
}

#[frb]
pub async fn run_pipeline(id: String) -> Result<String> {
    let pipeline = get_pipeline(id)?
        .ok_or_else(|| anyhow::anyhow!("pipeline not found"))?;
    let tasks = storage::load_tasks()?;
    let runner = pipeline_runner();
    let exec = executor();
    runner.run(&pipeline, &tasks, Arc::clone(exec), tokio_runtime()).await
}

#[frb]
pub fn cancel_pipeline(run_id: String) -> Result<()> {
    pipeline_runner().cancel(&run_id)
}

#[frb]
pub fn get_pipeline_state(run_id: String) -> Result<Option<PipelineRunState>> {
    pipeline_runner().get_state(&run_id)
}

#[frb]
pub fn list_pipeline_runs() -> Result<Vec<PipelineRunState>> {
    pipeline_runner().list_runs()
}

// ============================================================
// 实例管理
// ============================================================

#[frb]
pub fn spawn_task(task_id: String) -> Result<TaskInstance> {
    let task = get_task(task_id)?
        .ok_or_else(|| anyhow::anyhow!("task not found"))?;
    executor().spawn(&task, None)
}

#[frb]
pub fn kill_instance(instance_id: String) -> Result<()> {
    executor().kill(&instance_id)
}

#[frb]
pub fn list_instances() -> Result<Vec<TaskInstance>> {
    executor().list_instances()
}

#[frb]
pub fn get_instance(instance_id: String) -> Result<Option<TaskInstance>> {
    executor().get_instance(&instance_id)
}

#[frb]
pub fn read_output(instance_id: String) -> Result<Vec<u8>> {
    executor().read_output(&instance_id)
}

#[frb]
pub fn write_input(instance_id: String, data: Vec<u8>) -> Result<()> {
    executor().write_input(&instance_id, &data)
}

#[frb]
pub fn remove_instance(instance_id: String) -> Result<bool> {
    executor().remove_instance(&instance_id)
}

#[frb]
pub fn running_count() -> Result<i32> {
    executor().running_count().map(|c| c as i32)
}

// ============================================================
// 事件流
// ============================================================

/// 订阅实例事件流
///
/// 返回一个接收端，Dart 侧通过 Stream 消费。
/// 当前返回的事件是批量轮询模式：Flutter 端定时调用获取增量。
#[frb]
pub fn get_instance_events_since(
    _last_event_index: i32,
) -> Result<Vec<InstanceEvent>> {
    // 简化实现：返回空列表。事件推送通过单独的流式 API 或轮询实现。
    // 实际使用中，Flutter 端轮询 list_instances() + read_output() 即可。
    Ok(Vec::new())
}
