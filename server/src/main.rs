use anyhow::{anyhow, Result};
use cmdhub_core::config::{load_client_config_auto, load_server_config_auto};
use cmdhub_core::instance::{SessionManager, SpawnedInstance};
use cmdhub_core::models::{
    AppConfig, InstanceInfo, InstanceStatus, SessionInfo, SessionStatus, Task,
};
use cmdhub_core::rpc::{
    default_attach_uri, default_rpc_uri, parse_endpoint, CmdHubService, RpcEndpoint, RpcError,
};
use cmdhub_core::session::SessionStore;
use cmdhub_core::template::render_command;
use futures::StreamExt;
use portable_pty::MasterPty;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::{TcpListener, UnixListener};
use tarpc::context;
use tarpc::server::{self, Channel};
use tokio_serde::formats::Json;

const BUFFER_CAP: usize = 16 * 1024;
const DEFAULT_HISTORY_LIMIT: usize = 10;

#[derive(Clone)]
struct CmdHubServer {
    state: Arc<ServerState>,
}

struct ServerState {
    manager: SessionManager,
    config: AppConfig,
    session_store: SessionStore,
    history_limit: usize,
    io_map: Mutex<HashMap<String, InstanceIo>>,
}

struct InstanceIo {
    // Keep the PTY master alive for the instance lifetime.
    _master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    clients: Arc<Mutex<Vec<mpsc::Sender<Vec<u8>>>>>,
}

impl CmdHubService for CmdHubServer {
    async fn list_tasks(self, _: context::Context) -> Result<Vec<Task>, RpcError> {
        Ok(self.state.config.tasks.clone())
    }

    async fn list_instances(self, _: context::Context) -> Result<Vec<InstanceInfo>, RpcError> {
        self.state
            .manager
            .list_instances()
            .map_err(|err| RpcError::Internal(err.to_string()))
    }

    async fn get_history(self, _: context::Context) -> Result<Vec<SessionInfo>, RpcError> {
        self.state
            .session_store
            .list_history()
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

        let session = self
            .state
            .session_store
            .create_session(
                task.id.clone(),
                task.name.clone(),
                None,
                command.clone(),
                task.cwd.clone(),
                task.env.clone(),
                task.env_clear.unwrap_or(false),
            )
            .map_err(|err| RpcError::Internal(err.to_string()))?;
        let session_id = session.id;
        let spawned = match self
            .state
            .manager
            .spawn_raw_with_session(&task, &command, Some(session_id))
        {
            Ok(spawned) => spawned,
            Err(err) => {
                let _ = fs::remove_dir_all(self.state.session_store.session_dir(session_id));
                return Err(RpcError::Internal(err.to_string()));
            }
        };
        let instance_id = spawned.info.id.clone();

        let mut session = session;
        session.status = SessionStatus::Running;
        session.started_at = spawned.info.started_at;
        session.runner_pid = Some(std::process::id());
        session.child_pid = spawned.info.child_pid;
        self.state
            .session_store
            .write_session(&session)
            .map_err(|err| RpcError::Internal(err.to_string()))?;

        self.state
            .register_instance_io(spawned)
            .map_err(|err| RpcError::Internal(err.to_string()))?;
        Ok(instance_id)
    }

    async fn stop(self, _: context::Context, instance_id: String) -> Result<(), RpcError> {
        self.state
            .manager
            .kill(&instance_id)
            .map_err(|err| RpcError::Internal(err.to_string()))?;
        Ok(())
    }

    async fn remove_if_exited(
        self,
        _: context::Context,
        instance_id: String,
    ) -> Result<bool, RpcError> {
        let removed = self
            .state
            .manager
            .remove_if_exited(&instance_id)
            .map_err(|err| RpcError::Internal(err.to_string()))?;
        if removed {
            self.state.remove_instance_io(&instance_id);
        }
        Ok(removed)
    }

    async fn shutdown(self, _: context::Context) -> Result<(), RpcError> {
        let _ = self.state.manager.terminate_all(libc::SIGHUP);
        remove_pid_file();
        thread::spawn(|| {
            thread::sleep(std::time::Duration::from_millis(100));
            std::process::exit(0);
        });
        Ok(())
    }
}

impl ServerState {
    fn register_instance_io(&self, spawned: SpawnedInstance) -> Result<()> {
        let instance_id = spawned.info.id.clone();
        let mut reader = spawned
            .master
            .try_clone_reader()
            .map_err(|err| anyhow!("clone reader: {}", err))?;
        let mut log_file = if let Some(session_id) = spawned.info.session_id {
            let log_path = self.session_store.session_log_path(session_id);
            Some(OpenOptions::new().create(true).append(true).open(log_path)?)
        } else {
            None
        };
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
                        if let Some(file) = log_file.as_mut() {
                            let _ = file.write_all(&buf[..n]);
                            let _ = file.flush();
                        }
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

    fn update_session_on_exit(&self, info: InstanceInfo) {
        let Some(session_id) = info.session_id else {
            return;
        };
        let mut session = match self.session_store.load_session(session_id) {
            Ok(session) => session,
            Err(_) => return,
        };
        session.status = SessionStatus::Exited;
        session.ended_at = info.ended_at.or_else(|| Some(now_epoch()));
        session.exit_code = match info.status {
            InstanceStatus::Exited { code } => Some(code),
            _ => None,
        };
        session.child_pid = info.child_pid;
        let _ = self.session_store.write_session(&session);
        let _ = self
            .session_store
            .move_to_history(session_id, self.history_limit);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let config = load_server_config_auto().await?;
    let history_limit = load_client_config_auto()
        .await
        .ok()
        .and_then(|cfg| cfg.history_limit)
        .unwrap_or(DEFAULT_HISTORY_LIMIT);
    let session_store = SessionStore::new()?;
    recover_orphaned_sessions(&session_store, history_limit);
    let server_cfg = config.server.clone();
    let manager = SessionManager::new(BUFFER_CAP);
    let state = Arc::new(ServerState {
        manager,
        config,
        session_store,
        history_limit,
        io_map: Mutex::new(HashMap::new()),
    });
    let state_for_hook = Arc::clone(&state);
    state.manager.set_exit_hook(Arc::new(move |info| {
        state_for_hook.update_session_on_exit(info);
    }));

    setup_signal_handlers(Arc::clone(&state))?;
    write_pid_file()?;
    let rpc_uri = server_cfg
        .as_ref()
        .and_then(|cfg| cfg.rpc_listen.clone())
        .unwrap_or(default_rpc_uri()?);
    let attach_uri = server_cfg
        .as_ref()
        .and_then(|cfg| cfg.attach_listen.clone())
        .unwrap_or(default_attach_uri()?);

    let rpc_endpoint = parse_endpoint(&rpc_uri)?;
    let attach_endpoint = parse_endpoint(&attach_uri)?;

    let attach_state = Arc::clone(&state);
    match attach_endpoint {
        RpcEndpoint::Unix(path) => {
            prepare_socket(&path)?;
            let listener = UnixListener::bind(path)?;
            tokio::spawn(async move {
                let _ = run_attach_listener_unix(listener, attach_state).await;
            });
        }
        RpcEndpoint::Tcp(addr) => {
            let listener = TcpListener::bind(addr).await?;
            tokio::spawn(async move {
                let _ = run_attach_listener_tcp(listener, attach_state).await;
            });
        }
    }

    match rpc_endpoint {
        RpcEndpoint::Unix(path) => {
            prepare_socket(&path)?;
            let rpc_listener = tarpc::serde_transport::unix::listen(path, Json::default).await?;
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
        }
        RpcEndpoint::Tcp(addr) => {
            let rpc_listener = tarpc::serde_transport::tcp::listen(addr, Json::default).await?;
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
        }
    }

    Ok(())
}

fn pid_file_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow!("HOME not set"))?;
    Ok(PathBuf::from(home).join(".cmdhub").join("server.pid"))
}

fn write_pid_file() -> Result<()> {
    let path = pid_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let pid = std::process::id();
    fs::write(path, format!("{}\n", pid))?;
    Ok(())
}

fn remove_pid_file() {
    if let Ok(path) = pid_file_path() {
        let _ = fs::remove_file(path);
    }
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

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn recover_orphaned_sessions(store: &SessionStore, history_limit: usize) {
    let sessions = match store.list_sessions() {
        Ok(sessions) => sessions,
        Err(_) => return,
    };
    for mut session in sessions {
        if session.status != SessionStatus::Exited {
            session.status = SessionStatus::Exited;
            session.ended_at = Some(now_epoch());
            let _ = store.write_session(&session);
        }
        let _ = store.move_to_history(session.id, history_limit);
    }
}

async fn run_attach_listener_unix(listener: UnixListener, state: Arc<ServerState>) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let std_stream = stream.into_std()?;
        std_stream.set_nonblocking(false)?;
        let state_clone = Arc::clone(&state);
        thread::spawn(move || {
            let _ = handle_attach_unix(std_stream, state_clone);
        });
    }
}

async fn run_attach_listener_tcp(listener: TcpListener, state: Arc<ServerState>) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        let std_stream = stream.into_std()?;
        std_stream.set_nonblocking(false)?;
        let state_clone = Arc::clone(&state);
        thread::spawn(move || {
            let _ = handle_attach_tcp(std_stream, state_clone);
        });
    }
}

fn handle_attach_unix(
    mut stream: std::os::unix::net::UnixStream,
    state: Arc<ServerState>,
) -> Result<()> {
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

fn handle_attach_tcp(mut stream: std::net::TcpStream, state: Arc<ServerState>) -> Result<()> {
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
