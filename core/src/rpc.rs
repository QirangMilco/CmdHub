use crate::models::InstanceInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Error, Clone)]
pub enum RpcError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[tarpc::service]
pub trait CmdHubService {
    async fn list_instances() -> Result<Vec<InstanceInfo>, RpcError>;
    async fn spawn(task_id: String, params: HashMap<String, String>) -> Result<String, RpcError>;
    async fn stop(instance_id: String) -> Result<(), RpcError>;
    async fn remove_if_exited(instance_id: String) -> Result<bool, RpcError>;
}

pub fn rpc_socket_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cmdhub").join("cmdhub.sock"))
}

pub fn attach_socket_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cmdhub").join("cmdhub-attach.sock"))
}
