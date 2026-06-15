use crate::executor::Executor;
use crate::models::{
    InstanceStatus, Pipeline, PipelineRunState, PipelineStatus, StepCondition, StepState,
    Task,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use uuid::Uuid;

/// 编排运行器
pub struct PipelineRunner {
    active_runs: Arc<Mutex<HashMap<String, PipelineRunState>>>,
}

impl PipelineRunner {
    pub fn new() -> Self {
        Self {
            active_runs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 启动编排，返回 run_id
    ///
    /// 按 step.order 排序步骤，依次处理触发条件和延时。
    pub async fn run(
        &self,
        pipeline: &Pipeline,
        tasks: &[Task],
        executor: Arc<Executor>,
        rt: &Runtime,
    ) -> Result<String> {
        let run_id = Uuid::new_v4().to_string();

        // 克隆 tasks 以满足 tokio::spawn 的 'static 约束
        let tasks: Vec<Task> = tasks.to_vec();

        // 按 order 排序步骤
        let mut steps = pipeline.steps.clone();
        steps.sort_by_key(|s| s.order);

        // 初始化运行状态
        let step_states: Vec<StepState> =
            steps.iter().map(|_| StepState::Pending).collect();

        let run_state = PipelineRunState {
            pipeline_id: pipeline.id.clone(),
            pipeline_name: pipeline.name.clone(),
            step_states: step_states.clone(),
            status: PipelineStatus::Running,
        };

        {
            let mut guard = self
                .active_runs
                .lock()
                .map_err(|_| anyhow!("active runs lock poisoned"))?;
            guard.insert(run_id.clone(), run_state);
        }

        let active_runs = Arc::clone(&self.active_runs);
        let run_id_clone = run_id.clone();

        rt.spawn(async move {
            let mut current_instance_id: Option<String> = None;

            for (i, step) in steps.iter().enumerate() {
                // 检查是否已被取消
                {
                    let guard = active_runs.lock().unwrap();
                    if let Some(state) = guard.get(&run_id_clone) {
                        if state.status == PipelineStatus::Cancelled {
                            return;
                        }
                    }
                }

                let task = match tasks.iter().find(|t| t.id == step.task_id) {
                    Some(t) => t,
                    None => {
                        update_step_state(
                            &active_runs,
                            &run_id_clone,
                            i,
                            StepState::Failed {
                                error: format!("task not found: {}", step.task_id),
                            },
                        );
                        set_pipeline_status(
                            &active_runs,
                            &run_id_clone,
                            PipelineStatus::Failed,
                        );
                        return;
                    }
                };

                // 处理触发条件
                match step.condition {
                    StepCondition::OnStart => {
                        // 第一步，直接执行（i == 0 通常）
                    }
                    StepCondition::AfterPrevious => {
                        // 等待上一步实例退出
                        if let Some(ref instance_id) = current_instance_id {
                            wait_for_exit(&executor, instance_id).await;
                        }
                    }
                    StepCondition::AfterDelay => {
                        // 不等退出，直接走延时逻辑
                    }
                }

                // 步骤级延时
                if step.delay_ms > 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(step.delay_ms))
                        .await;
                }

                // 再次检查取消
                {
                    let guard = active_runs.lock().unwrap();
                    if let Some(state) = guard.get(&run_id_clone) {
                        if state.status == PipelineStatus::Cancelled {
                            return;
                        }
                    }
                }

                // 启动任务
                match executor.spawn(task) {
                    Ok(instance) => {
                        let instance_id = instance.id.clone();
                        update_step_state(
                            &active_runs,
                            &run_id_clone,
                            i,
                            StepState::Running {
                                instance_id: instance_id.clone(),
                            },
                        );
                        current_instance_id = Some(instance_id.clone());

                        // 等待该实例退出，记录结果
                        wait_for_exit(&executor, &instance_id).await;

                        let exit_code = match executor.get_instance(&instance_id) {
                            Ok(Some(inst)) => match inst.status {
                                InstanceStatus::Exited { code } => code,
                                InstanceStatus::Error { .. } => {
                                    update_step_state(
                                        &active_runs,
                                        &run_id_clone,
                                        i,
                                        StepState::Failed {
                                            error: "instance error".to_string(),
                                        },
                                    );
                                    set_pipeline_status(
                                        &active_runs,
                                        &run_id_clone,
                                        PipelineStatus::Failed,
                                    );
                                    return;
                                }
                                _ => 0,
                            },
                            _ => 0,
                        };

                        if exit_code != 0
                            && matches!(step.condition, StepCondition::AfterPrevious)
                        {
                            update_step_state(
                                &active_runs,
                                &run_id_clone,
                                i,
                                StepState::Failed {
                                    error: format!("exit code {}", exit_code),
                                },
                            );
                            set_pipeline_status(
                                &active_runs,
                                &run_id_clone,
                                PipelineStatus::Failed,
                            );
                            return;
                        }

                        update_step_state(
                            &active_runs,
                            &run_id_clone,
                            i,
                            StepState::Completed {
                                instance_id,
                                exit_code,
                            },
                        );
                    }
                    Err(e) => {
                        update_step_state(
                            &active_runs,
                            &run_id_clone,
                            i,
                            StepState::Failed {
                                error: e.to_string(),
                            },
                        );
                        set_pipeline_status(
                            &active_runs,
                            &run_id_clone,
                            PipelineStatus::Failed,
                        );
                        return;
                    }
                }
            }

            set_pipeline_status(&active_runs, &run_id_clone, PipelineStatus::Completed);
        });

        Ok(run_id)
    }

    /// 取消正在运行的编排
    pub fn cancel(&self, run_id: &str) -> Result<()> {
        let mut guard = self
            .active_runs
            .lock()
            .map_err(|_| anyhow!("active runs lock poisoned"))?;
        if let Some(state) = guard.get_mut(run_id) {
            state.status = PipelineStatus::Cancelled;
        }
        Ok(())
    }

    /// 获取编排运行状态
    pub fn get_state(&self, run_id: &str) -> Result<Option<PipelineRunState>> {
        let guard = self
            .active_runs
            .lock()
            .map_err(|_| anyhow!("active runs lock poisoned"))?;
        Ok(guard.get(run_id).cloned())
    }

    /// 列出所有活跃的编排运行
    pub fn list_runs(&self) -> Result<Vec<PipelineRunState>> {
        let guard = self
            .active_runs
            .lock()
            .map_err(|_| anyhow!("active runs lock poisoned"))?;
        Ok(guard.values().cloned().collect())
    }
}

/// 等待指定实例退出
async fn wait_for_exit(executor: &Executor, instance_id: &str) {
    loop {
        match executor.get_instance(instance_id) {
            Ok(Some(inst)) => {
                if !matches!(inst.status, InstanceStatus::Running) {
                    break;
                }
            }
            _ => break,
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}

fn update_step_state(
    active_runs: &Arc<Mutex<HashMap<String, PipelineRunState>>>,
    run_id: &str,
    step_index: usize,
    state: StepState,
) {
    if let Ok(mut guard) = active_runs.lock() {
        if let Some(run_state) = guard.get_mut(run_id) {
            if step_index < run_state.step_states.len() {
                run_state.step_states[step_index] = state;
            }
        }
    }
}

fn set_pipeline_status(
    active_runs: &Arc<Mutex<HashMap<String, PipelineRunState>>>,
    run_id: &str,
    status: PipelineStatus,
) {
    if let Ok(mut guard) = active_runs.lock() {
        if let Some(run_state) = guard.get_mut(run_id) {
            run_state.status = status;
        }
    }
}
