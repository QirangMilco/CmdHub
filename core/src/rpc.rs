use crate::models::{InstanceInfo, Task};
use anyhow::{anyhow, Result};
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
    async fn list_tasks() -> Result<Vec<Task>, RpcError>;
    async fn list_instances() -> Result<Vec<InstanceInfo>, RpcError>;
    async fn spawn(task_id: String, params: HashMap<String, String>) -> Result<String, RpcError>;
    async fn stop(instance_id: String) -> Result<(), RpcError>;
    async fn remove_if_exited(instance_id: String) -> Result<bool, RpcError>;
    async fn shutdown() -> Result<(), RpcError>;
}

pub fn rpc_socket_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cmdhub").join("cmdhub.sock"))
}

pub fn attach_socket_path() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".cmdhub").join("cmdhub-attach.sock"))
}

#[derive(Debug, Clone)]
pub enum RpcEndpoint {
    Unix(PathBuf),
    Tcp(String),
}

pub fn parse_endpoint(addr: &str) -> Result<RpcEndpoint> {
    if let Some(path) = addr.strip_prefix("unix://") {
        if path.is_empty() {
            return Err(anyhow!("unix endpoint missing path"));
        }
        return Ok(RpcEndpoint::Unix(PathBuf::from(path)));
    }
    if let Some(host) = addr.strip_prefix("tcp://") {
        if host.is_empty() {
            return Err(anyhow!("tcp endpoint missing address"));
        }
        return Ok(RpcEndpoint::Tcp(host.to_string()));
    }
    Err(anyhow!("unsupported endpoint scheme: {}", addr))
}

pub fn default_rpc_uri() -> Result<String> {
    let path = rpc_socket_path().ok_or_else(|| anyhow!("HOME not set"))?;
    Ok(format!("unix://{}", path.display()))
}

pub fn default_attach_uri() -> Result<String> {
    let path = attach_socket_path().ok_or_else(|| anyhow!("HOME not set"))?;
    Ok(format!("unix://{}", path.display()))
}
