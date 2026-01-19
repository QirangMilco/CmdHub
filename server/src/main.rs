use anyhow::{anyhow, Result};
use cmdhub_core::config::load_config_auto;
use cmdhub_core::instance::{SessionManager, SpawnedInstance};
use cmdhub_core::models::{AppConfig, InstanceInfo};
use cmdhub_core::rpc::{attach_socket_path, rpc_socket_path, CmdHubService, RpcError};
use cmdhub_core::template::render_command;
use futures::StreamExt;
use portable_pty::MasterPty;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use tokio::net::UnixListener;
use tarpc::context;
use tarpc::server::{self, Channel};
use tokio_serde::formats::Json;

const BUFFER_CAP: usize = 16 * 1024;

#[derive(Clone)]
struct CmdHubServer {
    state: Arc<ServerState>,
}

struct ServerState {
    manager: SessionManager,
    config: AppConfig,
    io_map: Mutex<HashMap<String, InstanceIo>>,
}

struct InstanceIo {
    // Keep the PTY master alive for the instance lifetime.
    _master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    clients: Arc<Mutex<Vec<mpsc::Sender<Vec<u8>>>>>,
}

impl CmdHubService for CmdHubServer {
    async fn list_instances(self, _: context::Context) -> Result<Vec<InstanceInfo>, RpcError> {
        self.state
            .manager
            .list_instances()
            .map_err(|err| RpcError::Internal(err.to_string()))
    }

    async fn spawn(
        self,
        _: context::Context,
        task_id: String,
        params: HashMap<String, String>,
    ) -> Result<String, RpcError> {
        let task = self
            .state
            .config
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .cloned()
            .ok_or_else(|| RpcError::NotFound(format!("task {}", task_id)))?;
        let command = render_command(&task.command, &params, task.inputs.as_ref())
            .map_err(|err| RpcError::Invalid(format!("render command: {}", err)))?;
        let spawned = self
            .state
            .manager
            .spawn_raw(&task, &command)
            .map_err(|err| RpcError::Internal(err.to_string()))?;
        let instance_id = spawned.info.id.clone();
        self.state
            .register_instance_io(spawned)
            .map_err(|err| RpcError::Internal(err.to_string()))?;
        Ok(instance_id)
    }

    async fn stop(self, _: context::Context, instance_id: String) -> Result<(), RpcError> {
        self.state
            .manager
            .kill_and_remove(&instance_id)
            .map_err(|err| RpcError::Internal(err.to_string()))?;
        self.state.remove_instance_io(&instance_id);
        Ok(())
    }

    async fn remove_if_exited(
        self,
        _: context::Context,
        instance_id: String,
    ) -> Result<bool, RpcError> {
        self.state
            .manager
            .remove_if_exited(&instance_id)
            .map_err(|err| RpcError::Internal(err.to_string()))
    }
}

impl ServerState {
    fn register_instance_io(&self, spawned: SpawnedInstance) -> Result<()> {
        let instance_id = spawned.info.id.clone();
        let mut reader = spawned
            .master
            .try_clone_reader()
            .map_err(|err| anyhow!("clone reader: {}", err))?;
        let writer = Arc::new(Mutex::new(spawned.writer));
        let clients = Arc::new(Mutex::new(Vec::<mpsc::Sender<Vec<u8>>>::new()));

        let manager = self.manager.clone();
        let clients_clone = Arc::clone(&clients);
        let instance_id_clone = instance_id.clone();

        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = manager.append_output(&instance_id_clone, &buf[..n]);
                        let mut to_remove = Vec::new();
                        if let Ok(mut guard) = clients_clone.lock() {
                            for (idx, sender) in guard.iter().enumerate() {
                                if sender.send(buf[..n].to_vec()).is_err() {
                                    to_remove.push(idx);
                                }
                            }
                            for idx in to_remove.into_iter().rev() {
                                guard.remove(idx);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let mut guard = self
            .io_map
            .lock()
            .map_err(|_| anyhow!("io map lock poisoned"))?;
        guard.insert(
            instance_id,
            InstanceIo {
                _master: spawned.master,
                writer,
                clients,
            },
        );
        Ok(())
    }

    fn get_instance_io(
        &self,
        instance_id: &str,
    ) -> Option<(Arc<Mutex<Box<dyn Write + Send>>>, Arc<Mutex<Vec<mpsc::Sender<Vec<u8>>>>>)> {
        let guard = self.io_map.lock().ok()?;
        let io = guard.get(instance_id)?;
        Some((Arc::clone(&io.writer), Arc::clone(&io.clients)))
    }

    fn remove_instance_io(&self, instance_id: &str) {
        if let Ok(mut guard) = self.io_map.lock() {
            guard.remove(instance_id);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = load_config_auto().await?;
    let manager = SessionManager::new(BUFFER_CAP);
    let state = Arc::new(ServerState {
        manager,
        config,
        io_map: Mutex::new(HashMap::new()),
    });

    setup_signal_handlers(Arc::clone(&state))?;

    let rpc_path = rpc_socket_path().ok_or_else(|| anyhow!("HOME not set"))?;
    let attach_path = attach_socket_path().ok_or_else(|| anyhow!("HOME not set"))?;
    prepare_socket(&rpc_path)?;
    prepare_socket(&attach_path)?;

    let rpc_listener = tarpc::serde_transport::unix::listen(rpc_path, Json::default)
        .await?;
    let attach_listener = UnixListener::bind(attach_path)?;

    let attach_state = Arc::clone(&state);
    tokio::spawn(async move {
        let _ = run_attach_listener(attach_listener, attach_state).await;
    });

    let incoming = rpc_listener
        .filter_map(|conn| async move { conn.ok() })
        .map(server::BaseChannel::with_defaults);

    incoming
        .for_each(|channel| {
            let server = CmdHubServer {
                state: Arc::clone(&state),
            };
            async move {
                let fut = channel.execute(server.serve()).for_each(spawn_task);
                tokio::spawn(fut);
            }
        })
        .await;

    Ok(())
}

async fn spawn_task(fut: impl std::future::Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}

fn setup_signal_handlers(state: Arc<ServerState>) -> Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;
    thread::spawn(move || {
        for _ in signals.forever() {
            let _ = state.manager.terminate_all(libc::SIGHUP);
            std::process::exit(1);
        }
    });
    Ok(())
}

fn prepare_socket(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

async fn run_attach_listener(listener: UnixListener, state: Arc<ServerState>) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let std_stream = stream.into_std()?;
        std_stream.set_nonblocking(false)?;
        let state_clone = Arc::clone(&state);
        thread::spawn(move || {
            let _ = handle_attach(std_stream, state_clone);
        });
    }
}

fn handle_attach(mut stream: std::os::unix::net::UnixStream, state: Arc<ServerState>) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut instance_id = String::new();
    let bytes = reader.read_line(&mut instance_id)?;
    if bytes == 0 {
        return Ok(());
    }
    let instance_id = instance_id.trim().to_string();
    let (writer, clients) = match state.get_instance_io(&instance_id) {
        Some(io) => io,
        None => {
            let _ = stream.write_all(b"ERR instance not found\n");
            return Ok(());
        }
    };

    stream.write_all(b"OK\n")?;
    let snapshot = state.manager.buffer_snapshot(&instance_id)?;
    if !snapshot.is_empty() {
        stream.write_all(&snapshot)?;
    }

    let (tx, rx) = mpsc::channel();
    if let Ok(mut guard) = clients.lock() {
        guard.push(tx);
    }

    let mut stream_out = stream.try_clone()?;
    let forwarder = thread::spawn(move || {
        for chunk in rx.iter() {
            if stream_out.write_all(&chunk).is_err() {
                break;
            }
            let _ = stream_out.flush();
        }
    });

    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if let Ok(mut guard) = writer.lock() {
                    let _ = guard.write_all(&buf[..n]);
                    let _ = guard.flush();
                }
            }
            Err(_) => break,
        }
    }

    let _ = forwarder.join();
    Ok(())
}
