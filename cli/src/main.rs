use anyhow::{anyhow, Result};
use cmdhub_core::config::load_client_config_auto;
use cmdhub_core::models::{
    ClientConfig, InputConfig, InstanceInfo, InstanceStatus, KeyBindings, ServerConfig, Task,
    UiConfig,
};
use cmdhub_core::rpc::{
    default_attach_uri, default_rpc_uri, parse_endpoint, CmdHubServiceClient, RpcEndpoint,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::cursor::{MoveTo, RestorePosition, SavePosition, Show};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Read, Write};
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tarpc::client::Config as TarpcConfig;
use tarpc::context;
use tokio_serde::formats::Json;

fn main() -> Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    let list_only = args.iter().any(|arg| arg == "--servers");
    let runtime = Arc::new(tokio::runtime::Runtime::new()?);
    let config = runtime.block_on(load_client_config_auto())?;
    if list_only {
        let servers = server_entries_from_config(&config)?;
        let active_server = pick_default_server(&servers);
        let servers = refresh_servers_statuses(Arc::clone(&runtime), servers);
        print_servers(&servers, active_server);
        return Ok(());
    }

    let (servers, active_server, rpc) = build_servers(Arc::clone(&runtime), &config)?;
    let tasks = rpc.list_tasks().unwrap_or_default();
    run_ui(config, tasks, servers, active_server, rpc)?;
    Ok(())
}

fn setup_signal_handlers() -> Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;
    thread::spawn(move || {
        for _ in signals.forever() {
            std::process::exit(1);
        }
    });
    Ok(())
}

fn run_ui(
    config: ClientConfig,
    tasks: Vec<Task>,
    servers: Vec<ServerEntry>,
    active_server: usize,
    rpc: RpcHandle,
) -> Result<()> {
    setup_signal_handlers()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut app = App::new(config, tasks, servers, active_server, rpc);
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        app.refresh_instances();
        app.refresh_server_statuses();
        terminal.draw(|frame| app.draw(frame))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if app.handle_key(key)? {
                        break;
                    }
                }
                Event::Resize(_, _) => {
                    app.needs_redraw = true;
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if let Some(next) = app.take_passthrough() {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            let outcome = run_passthrough(next)?;
            if matches!(outcome, PassthroughOutcome::QuitClient) {
                return Ok(());
            }
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.clear()?; // Force full redraw
            enable_raw_mode()?;
            terminal.hide_cursor()?;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn print_servers(servers: &[ServerEntry], active_server: usize) {
    println!("Servers:");
    for (idx, server) in servers.iter().enumerate() {
        let status = match server.status {
            ServerStatus::Online => "online",
            ServerStatus::Offline => "offline",
            ServerStatus::Unknown => "unknown",
        };
        let active = if idx == active_server { "*" } else { " " };
        println!("{} {} [{}] {}", active, server.name, status, server.rpc_addr);
    }
}

fn resolve_server_status(
    runtime: &Arc<tokio::runtime::Runtime>,
    server: &ServerEntry,
) -> ServerStatus {
    if server.is_local {
        if let Ok(pid) = read_local_server_pid() {
            if is_process_alive(pid) {
                return ServerStatus::Online;
            }
        }
    }
    match check_server_status(runtime, &server.rpc_addr) {
        Ok(true) => ServerStatus::Online,
        Ok(false) => ServerStatus::Offline,
        Err(_) => ServerStatus::Offline,
    }
}

fn refresh_servers_statuses(
    runtime: Arc<tokio::runtime::Runtime>,
    mut servers: Vec<ServerEntry>,
) -> Vec<ServerEntry> {
    for server in servers.iter_mut() {
        server.status = resolve_server_status(&runtime, server);
    }
    servers
}

#[derive(Clone)]
struct RpcHandle {
    runtime: Arc<tokio::runtime::Runtime>,
    client: Option<Arc<CmdHubServiceClient>>,
}

impl RpcHandle {
    fn list_tasks(&self) -> Result<Vec<Task>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("server offline"))?;
        let result = self
            .runtime
            .block_on(client.list_tasks(context::current()))
            .map_err(|err| anyhow!("rpc transport: {}", err))?;
        result.map_err(|err| anyhow!("rpc error: {}", err))
    }

    fn list_instances(&self) -> Result<Vec<InstanceInfo>> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("server offline"))?;
        let result = self
            .runtime
            .block_on(client.list_instances(context::current()))
            .map_err(|err| anyhow!("rpc transport: {}", err))?;
        result.map_err(|err| anyhow!("rpc error: {}", err))
    }

    fn spawn(&self, task_id: String, params: HashMap<String, String>) -> Result<String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("server offline"))?;
        let result = self
            .runtime
            .block_on(client.spawn(context::current(), task_id, params))
            .map_err(|err| anyhow!("rpc transport: {}", err))?;
        result.map_err(|err| anyhow!("rpc error: {}", err))
    }

    fn stop(&self, instance_id: &str) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("server offline"))?;
        let result = self
            .runtime
            .block_on(client.stop(context::current(), instance_id.to_string()))
            .map_err(|err| anyhow!("rpc transport: {}", err))?;
        result.map_err(|err| anyhow!("rpc error: {}", err))
    }

    fn remove_if_exited(&self, instance_id: &str) -> Result<bool> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("server offline"))?;
        let result = self
            .runtime
            .block_on(
                client
                    .remove_if_exited(context::current(), instance_id.to_string()),
            )
            .map_err(|err| anyhow!("rpc transport: {}", err))?;
        result.map_err(|err| anyhow!("rpc error: {}", err))
    }

    fn shutdown(&self) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("server offline"))?;
        let result = self
            .runtime
            .block_on(client.shutdown(context::current()))
            .map_err(|err| anyhow!("rpc transport: {}", err))?;
        result.map_err(|err| anyhow!("rpc error: {}", err))
    }

    fn offline(runtime: Arc<tokio::runtime::Runtime>) -> Self {
        Self {
            runtime,
            client: None,
        }
    }
}

fn build_servers(
    runtime: Arc<tokio::runtime::Runtime>,
    config: &ClientConfig,
) -> Result<(Vec<ServerEntry>, usize, RpcHandle)> {
    let mut servers = server_entries_from_config(config)?;

    let active_server = pick_default_server(&servers);
    let (rpc, status) = match connect_to_server(Arc::clone(&runtime), &servers[active_server]) {
        Ok(handle) => (handle, ServerStatus::Online),
        Err(_) => {
            if servers[active_server].is_local {
                (RpcHandle::offline(Arc::clone(&runtime)), ServerStatus::Offline)
            } else {
                (RpcHandle::offline(Arc::clone(&runtime)), ServerStatus::Offline)
            }
        }
    };

    if let Some(server) = servers.get_mut(active_server) {
        server.status = status;
    }

    Ok((servers, active_server, rpc))
}

fn server_entries_from_config(config: &ClientConfig) -> Result<Vec<ServerEntry>> {
    let mut servers = if let Some(configured) = config.servers.as_ref() {
        configured
            .iter()
            .map(server_entry_from_config)
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    if servers.is_empty() {
        servers.push(default_server_entry()?);
    }
    Ok(servers)
}

fn pick_default_server(servers: &[ServerEntry]) -> usize {
    servers
        .iter()
        .position(|server| server.is_default)
        .or_else(|| servers.iter().position(|server| server.auto_launch))
        .unwrap_or(0)
}

fn default_server_entry() -> Result<ServerEntry> {
    Ok(ServerEntry {
        id: "local".to_string(),
        name: "Local".to_string(),
        rpc_addr: default_rpc_uri()?,
        attach_addr: Some(default_attach_uri()?),
        auto_launch: true,
        is_default: true,
        is_local: true,
        status: ServerStatus::Unknown,
    })
}

fn server_entry_from_config(config: &ServerConfig) -> Result<ServerEntry> {
    let name = config.name.clone().unwrap_or_else(|| config.id.clone());
    let mut is_local = config.id == "local";
    if !is_local {
        if let Ok(default_rpc) = default_rpc_uri() {
            is_local = config.rpc == default_rpc;
        }
    }
    Ok(ServerEntry {
        id: config.id.clone(),
        name,
        rpc_addr: config.rpc.clone(),
        attach_addr: config.attach.clone(),
        auto_launch: config.auto_launch.unwrap_or(false),
        is_default: config.is_default.unwrap_or(false),
        is_local,
        status: ServerStatus::Unknown,
    })
}

fn connect_to_server(
    runtime: Arc<tokio::runtime::Runtime>,
    server: &ServerEntry,
) -> Result<RpcHandle> {
    let try_connect = |runtime: &Arc<tokio::runtime::Runtime>,
                       addr: &str|
     -> Result<CmdHubServiceClient> {
        runtime
            .block_on(connect_client(addr))
            .map_err(|err| anyhow!("connect client: {}", err))
    };

    if let Ok(client) = try_connect(&runtime, &server.rpc_addr) {
        return Ok(RpcHandle {
            runtime,
            client: Some(Arc::new(client)),
        });
    }

    if server.auto_launch && server.is_local {
        launch_server()?;
        for _ in 0..15 {
            thread::sleep(Duration::from_millis(200));
            if let Ok(client) = try_connect(&runtime, &server.rpc_addr) {
                return Ok(RpcHandle {
                    runtime,
                    client: Some(Arc::new(client)),
                });
            }
        }
    }

    Err(anyhow!("failed to connect to {}", server.rpc_addr))
}

async fn connect_client(addr: &str) -> Result<CmdHubServiceClient> {
    match parse_endpoint(addr)? {
        RpcEndpoint::Unix(path) => {
            let transport = tarpc::serde_transport::unix::connect(path, Json::default).await?;
            Ok(CmdHubServiceClient::new(TarpcConfig::default(), transport).spawn())
        }
        RpcEndpoint::Tcp(host) => {
            let transport = tarpc::serde_transport::tcp::connect(host, Json::default).await?;
            Ok(CmdHubServiceClient::new(TarpcConfig::default(), transport).spawn())
        }
    }
}

fn check_server_status(runtime: &Arc<tokio::runtime::Runtime>, addr: &str) -> Result<bool> {
    let client = runtime
        .block_on(connect_client(addr))
        .map_err(|err| anyhow!("connect client: {}", err))?;
    let handle = RpcHandle {
        runtime: Arc::clone(runtime),
        client: Some(Arc::new(client)),
    };
    Ok(handle.list_instances().is_ok())
}

fn launch_server() -> Result<()> {
    let log_path = server_log_path()?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let file_err = file.try_clone()?;

    let mut tried = Vec::new();
    let mut last_err: Option<anyhow::Error> = None;
    for candidate in server_command_candidates() {
        let result = Command::new(&candidate)
            .stdout(file.try_clone()?)
            .stderr(file_err.try_clone()?)
            .spawn()
            .map_err(|err| anyhow!("spawn {}: {}", candidate.display(), err));
        match result {
            Ok(_) => return Ok(()),
            Err(err) => {
                tried.push(candidate);
                last_err = Some(err);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow!("spawn cmdhub-server failed")))
        .map_err(|err| {
            let list = tried
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow!("{}; tried: {}", err, list)
        })?;

    Ok(())
}

fn server_command_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    candidates.push(PathBuf::from("cmdhub-server"));
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            candidates.push(dir.join("cmdhub-server"));
        }
    }
    candidates
}

fn read_local_server_pid() -> Result<i32> {
    let path = server_pid_path()?;
    let content = fs::read_to_string(path)?;
    let pid = content.trim().parse::<i32>()?;
    Ok(pid)
}

fn is_process_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

fn stop_local_server_via_pid() -> Result<()> {
    let pid = read_local_server_pid()?;
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    for _ in 0..10 {
        if !is_process_alive(pid) {
            if let Ok(path) = server_pid_path() {
                let _ = fs::remove_file(path);
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    unsafe {
        libc::kill(pid, libc::SIGKILL);
    }
    if is_process_alive(pid) {
        return Err(anyhow!("failed to stop local server (pid {})", pid));
    }
    if let Ok(path) = server_pid_path() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

fn server_log_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow!("HOME not set"))?;
    Ok(PathBuf::from(home).join(".cmdhub").join("server.log"))
}

fn server_pid_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").map_err(|_| anyhow!("HOME not set"))?;
    Ok(PathBuf::from(home).join(".cmdhub").join("server.pid"))
}

#[derive(Clone)]
struct ServerEntry {
    #[allow(dead_code)]
    id: String,
    name: String,
    rpc_addr: String,
    attach_addr: Option<String>,
    auto_launch: bool,
    is_default: bool,
    is_local: bool,
    status: ServerStatus,
}

#[derive(Clone, Copy)]
enum ServerStatus {
    Unknown,
    Online,
    Offline,
}

struct App {
    config: ClientConfig,
    tasks: Vec<Task>,
    rpc: RpcHandle,
    servers: Vec<ServerEntry>,
    active_server: usize,
    expanded: HashSet<String>,
    entries: Vec<Entry>,
    selected: usize,
    list_state: ListState,
    instances: Vec<InstanceInfo>,
    mode: AppMode,
    last_error: Option<String>,
    needs_redraw: bool,
    next_passthrough: Option<PassthroughRequest>,
    key_bindings: KeyBindings,
    last_server_check: Instant,
}

enum AppMode {
    List,
    InputForm(InputFormState),
}

enum InputResult {
    Stay,
    ExitToList,
}

struct InputFormState {
    task_index: usize,
    fields: Vec<InputField>,
    selected: usize,
}

struct InputField {
    name: String,
    config: InputConfig,
    value: String,
    cursor: usize,
    options: Vec<String>,
    option_index: usize,
}

impl App {
    fn new(
        config: ClientConfig,
        tasks: Vec<Task>,
        servers: Vec<ServerEntry>,
        active_server: usize,
        rpc: RpcHandle,
    ) -> Self {
        let expanded = tasks.iter().map(|task| task.id.clone()).collect();
        
        let mut key_bindings = KeyBindings::default();
        if let Some(user_keys) = &config.keys {
            for (k, v) in &user_keys.global {
                key_bindings.global.insert(k.clone(), v.clone());
            }
            for (k, v) in &user_keys.task_list {
                key_bindings.task_list.insert(k.clone(), v.clone());
            }
            for (k, v) in &user_keys.task_running {
                key_bindings.task_running.insert(k.clone(), v.clone());
            }
        }

        Self {
            config,
            tasks,
            rpc,
            servers,
            active_server,
            expanded,
            entries: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
            instances: Vec::new(),
            mode: AppMode::List,
            last_error: None,
            needs_redraw: true,
            next_passthrough: None,
            key_bindings,
            last_server_check: Instant::now(),
        }
    }

    fn refresh_instances(&mut self) {
        match self.rpc.list_instances() {
            Ok(instances) => {
                self.instances = instances;
                if let Some(server) = self.servers.get_mut(self.active_server) {
                    server.status = ServerStatus::Online;
                }
                if self.tasks.is_empty() {
                    if let Ok(tasks) = self.rpc.list_tasks() {
                        self.tasks = tasks;
                        self.expanded = self.tasks.iter().map(|task| task.id.clone()).collect();
                    }
                }
            }
            Err(_) => {
                self.instances.clear();
                self.tasks.clear();
                self.expanded.clear();
                if let Some(server) = self.servers.get_mut(self.active_server) {
                    server.status = ServerStatus::Offline;
                }
            }
        }
        self.rebuild_entries();
    }

    fn refresh_server_statuses(&mut self) {
        if self.last_server_check.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_server_check = Instant::now();
        for (idx, server) in self.servers.iter_mut().enumerate() {
            if idx == self.active_server {
                continue;
            }
            server.status = resolve_server_status(&self.rpc.runtime, server);
        }
    }

    fn rebuild_entries(&mut self) {
        let mut entries = Vec::new();
        if !self.servers.is_empty() {
            entries.push(Entry::ServerCategory {
                name: "Servers".to_string(),
            });
            for idx in 0..self.servers.len() {
                entries.push(Entry::Server { index: idx });
            }
        }
        let mut by_task: HashMap<String, Vec<InstanceInfo>> = HashMap::new();
        for instance in &self.instances {
            by_task.entry(instance.task_id.clone()).or_default().push(instance.clone());
        }

        let mut by_category: HashMap<String, Vec<&Task>> = HashMap::new();
        for task in &self.tasks {
            let category = task.category.clone().unwrap_or_else(|| "Default".to_string());
            by_category.entry(category).or_default().push(task);
        }

        let mut categories: Vec<String> = by_category.keys().cloned().collect();
        categories.sort();
        for category in categories {
            entries.push(Entry::Category { name: category.clone() });
            if let Some(tasks) = by_category.get(&category) {
                for task in tasks {
                    entries.push(Entry::Task { task_id: task.id.clone() });
                    if self.expanded.contains(&task.id) {
                        if let Some(list) = by_task.get_mut(&task.id) {
                            list.sort_by_key(|info| info.started_at);
                            for instance in list {
                                entries.push(Entry::Instance {
                                    instance_id: instance.id.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
        self.entries = entries;
        if self.selected >= self.entries.len() {
            self.selected = self.entries.len().saturating_sub(1);
        }
        if self.entries.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(self.selected));
        }
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        match &self.mode {
            AppMode::InputForm(form) => {
                let area = frame.size();
                let block = Block::default().borders(Borders::ALL).title("Inputs");
                frame.render_widget(block, area);
                self.render_input_form(frame, area, form);
            }
            AppMode::List => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
                    .split(frame.size());
                let items = self.list_items();
                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("CmdHub"))
                    .highlight_style(
                        Style::default()
                            .bg(Color::Blue)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");
                frame.render_stateful_widget(list, chunks[0], &mut self.list_state);
                let help = self.build_help();
                frame.render_widget(help, chunks[1]);
            }
        }
    }

    fn list_items(&self) -> Vec<ListItem<'static>> {
        let mut items = Vec::new();
        for entry in &self.entries {
            match entry {
                Entry::ServerCategory { name } => {
                    let line = Line::from(vec![Span::styled(
                        name.clone(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )]);
                    items.push(ListItem::new(line));
                }
                Entry::Server { index } => {
                    let line = self.server_line(*index);
                    items.push(ListItem::new(line));
                }
                Entry::Category { name } => {
                    let line = Line::from(vec![Span::styled(
                        name.clone(),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )]);
                    items.push(ListItem::new(line));
                }
                Entry::Task { task_id } => {
                    let task = self.task_by_id(task_id);
                    let name = task.map(|t| t.name.as_str()).unwrap_or(task_id);
                    let marker = if self.expanded.contains(task_id) { "-" } else { "+" };
                    let line = Line::from(vec![
                        Span::styled(marker.to_string(), Style::default().fg(Color::Gray)),
                        Span::raw(" "),
                        Span::styled(name.to_string(), Style::default().add_modifier(Modifier::BOLD)),
                    ]);
                    items.push(ListItem::new(line));
                }
                Entry::Instance { instance_id } => {
                    let instance = self.instances.iter().find(|i| &i.id == instance_id);
                    let line = if let Some(info) = instance {
                        instance_line(info)
                    } else {
                        Line::from(vec![Span::raw("  (missing)")])
                    };
                    items.push(ListItem::new(line));
                }
            }
        }

        items
    }

    fn server_line(&self, index: usize) -> Line<'static> {
        let server = match self.servers.get(index) {
            Some(server) => server,
            None => return Line::from(vec![Span::raw("  (missing server)")]),
        };
        let status_label = match server.status {
            ServerStatus::Online => "online",
            ServerStatus::Offline => "offline",
            ServerStatus::Unknown => "unknown",
        };
        let status_color = match server.status {
            ServerStatus::Online => Color::Green,
            ServerStatus::Offline => Color::Red,
            ServerStatus::Unknown => Color::Gray,
        };
        let active_marker = if index == self.active_server { "*" } else { " " };
        Line::from(vec![
            Span::styled(
                format!("{} {}", active_marker, server.name),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}]", status_label),
                Style::default().fg(status_color),
            ),
        ])
    }

    fn build_help(&self) -> Paragraph<'_> {
        let mut text = Vec::new();
        match self.mode {
            AppMode::List => {
                text.push(Line::from(
                    "Enter: run/attach/switch  Tab: fold  d: delete  k: kill  Q: quit",
                ));
            }
            AppMode::InputForm(_) => {
                text.push(Line::from("Enter: next/submit  Esc: cancel  Up/Down: select  Left/Right: option"));
            }
        }
        if let Some(err) = &self.last_error {
            text.push(Line::from(Span::styled(
                err.clone(),
                Style::default().fg(Color::Red),
            )));
        }
        Paragraph::new(text).wrap(Wrap { trim: true })
    }

    fn render_input_form(&self, frame: &mut ratatui::Frame, area: Rect, form: &InputFormState) {
        let inner = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };
        let mut lines = Vec::new();
        for (idx, field) in form.fields.iter().enumerate() {
            let title = format!("{}:", field.name);
            let mut spans = vec![Span::styled(title, Style::default().fg(Color::Yellow))];
            spans.push(Span::raw(" "));
            let value = field.value.clone();
            let style = if idx == form.selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            spans.push(Span::styled(value, style));
            lines.push(Line::from(spans));
        }
        let content = Paragraph::new(lines).wrap(Wrap { trim: true });
        let content_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };
        frame.render_widget(content, content_area);

        let help_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        let help = Paragraph::new(Line::from(
            "Enter: next/submit  Esc: cancel  Up/Down: select  Left/Right: option",
        ));
        frame.render_widget(help, help_area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        let mode = std::mem::replace(&mut self.mode, AppMode::List);
        match mode {
            AppMode::List => {
                self.mode = AppMode::List;
                self.handle_list_key(key)
            }
            AppMode::InputForm(mut form) => {
                let result = self.handle_input_key(key, &mut form)?;
                match result {
                    InputResult::Stay => {
                        self.mode = AppMode::InputForm(form);
                    }
                    InputResult::ExitToList => {
                        self.mode = AppMode::List;
                    }
                }
                Ok(false)
            }
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> Result<bool> {
        self.last_error = None;
        let keys = &self.key_bindings.task_list;
        
        // Helper to check key
        let check = |action: &str, k: &KeyEvent| -> bool {
            if let Some(binding) = keys.get(action) {
                matches_key(k, binding)
            } else {
                false
            }
        };

        if check("quit", &key) {
             return Ok(true);
        } else if check("down", &key) {
             if self.selected + 1 < self.entries.len() {
                 self.selected += 1;
             }
        } else if check("up", &key) {
             if self.selected > 0 {
                 self.selected -= 1;
             }
        } else if check("fold_task", &key) {
             if let Some(Entry::Task { task_id }) = self.entries.get(self.selected) {
                 if self.expanded.contains(task_id) {
                     self.expanded.remove(task_id);
                 } else {
                     self.expanded.insert(task_id.clone());
                 }
             }
        } else if check("delete_instance", &key) {
             if let Some(Entry::Instance { instance_id }) = self.entries.get(self.selected) {
                 let _ = self.rpc.remove_if_exited(instance_id);
             }
        } else if check("kill_instance", &key) {
             if let Some(Entry::Instance { instance_id }) = self.entries.get(self.selected) {
                 let _ = self.rpc.stop(instance_id);
             }
        } else if check("select", &key) {
             if let Some(entry) = self.entries.get(self.selected).cloned() {
                 match entry {
                     Entry::ServerCategory { .. } => {}
                     Entry::Server { index } => {
                         if self
                             .servers
                             .get(index)
                             .map(|server| server.is_local)
                             .unwrap_or(false)
                         {
                             self.toggle_local_server(index)?;
                         } else {
                             self.switch_server(index)?;
                         }
                     }
                     Entry::Category { .. } => {}
                     Entry::Task { task_id } => {
                         let task = self.task_by_id(&task_id).cloned();
                         if let Some(task) = task {
                             self.start_task(task)?;
                         }
                     }
                     Entry::Instance { instance_id } => {
                         self.attach_instance(&instance_id)?;
                     }
                 }
             }
        }
        
        if self.entries.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(self.selected));
        }
        Ok(false)
    }

    fn handle_input_key(&mut self, key: KeyEvent, form: &mut InputFormState) -> Result<InputResult> {
        match key.code {
            KeyCode::Esc => {
                return Ok(InputResult::ExitToList);
            }
            KeyCode::Down => {
                if form.selected + 1 < form.fields.len() {
                    form.selected += 1;
                }
            }
            KeyCode::Up => {
                if form.selected > 0 {
                    form.selected -= 1;
                }
            }
            KeyCode::Left => {
                if let Some(field) = form.fields.get_mut(form.selected) {
                    field.cycle_option(false);
                }
            }
            KeyCode::Right => {
                if let Some(field) = form.fields.get_mut(form.selected) {
                    field.cycle_option(true);
                }
            }
            KeyCode::Backspace => {
                if let Some(field) = form.fields.get_mut(form.selected) {
                    field.backspace();
                }
            }
            KeyCode::Char(ch) => {
                if let Some(field) = form.fields.get_mut(form.selected) {
                    field.insert_char(ch);
                }
            }
            KeyCode::Enter => {
                if form.selected + 1 < form.fields.len() {
                    form.selected += 1;
                } else {
                    let task_index = form.task_index;
                    let values = form.collect_values();
                    let task = self.tasks.get(task_index).cloned();
                    if let Some(task) = task {
                        self.spawn_from_values(task, values)?;
                    }
                    return Ok(InputResult::ExitToList);
                }
            }
            _ => {}
        }
        Ok(InputResult::Stay)
    }

    fn start_task(&mut self, task: Task) -> Result<()> {
        if let Some(inputs) = task.inputs.as_ref() {
            let mut fields = Vec::new();
            for (name, config) in inputs {
                fields.push(InputField::from_config(name, config));
            }
            let state = InputFormState {
                task_index: self
                    .tasks
                    .iter()
                    .position(|t| t.id == task.id)
                    .ok_or_else(|| anyhow!("task not found"))?,
                fields,
                selected: 0,
            };
            self.mode = AppMode::InputForm(state);
            Ok(())
        } else {
            self.spawn_from_values(task, HashMap::new())
        }
    }

    fn spawn_from_values(&mut self, task: Task, values: HashMap<String, String>) -> Result<()> {
        let instance_id = self.rpc.spawn(task.id.clone(), values)?;
        let info = self
            .rpc
            .list_instances()
            .ok()
            .and_then(|items| items.into_iter().find(|item| item.id == instance_id));
        self.next_passthrough = Some(PassthroughRequest {
            instance_id,
            task_name: task.name.clone(),
            instance_info: info,
            rpc: self.rpc.clone(),
            attach_addr: self
                .servers
                .get(self.active_server)
                .and_then(|server| server.attach_addr.clone()),
            ui_config: self.config.ui.clone().unwrap_or_default(),
            key_config: self.key_bindings.clone(),
        });
        Ok(())
    }

    fn attach_instance(&mut self, instance_id: &str) -> Result<()> {
        if let Some(info) = self.instances.iter().find(|info| info.id == instance_id).cloned() {
            self.next_passthrough = Some(PassthroughRequest {
                instance_id: instance_id.to_string(),
                task_name: info.task_name.clone(),
                instance_info: Some(info),
                rpc: self.rpc.clone(),
                attach_addr: self
                    .servers
                    .get(self.active_server)
                    .and_then(|server| server.attach_addr.clone()),
                ui_config: self.config.ui.clone().unwrap_or_default(),
                key_config: self.key_bindings.clone(),
            });
        } else {
            self.last_error = Some("Instance not found".to_string());
        }
        Ok(())
    }

    fn take_passthrough(&mut self) -> Option<PassthroughRequest> {
        self.next_passthrough.take()
    }

    fn task_by_id(&self, task_id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == task_id)
    }

    fn switch_server(&mut self, index: usize) -> Result<()> {
        if index >= self.servers.len() {
            return Ok(());
        }
        if index == self.active_server {
            return Ok(());
        }
        let server = self.servers[index].clone();
        match connect_to_server(Arc::clone(&self.rpc.runtime), &server) {
            Ok(rpc) => {
                let tasks = rpc.list_tasks().unwrap_or_default();
                self.tasks = tasks;
                self.expanded = self.tasks.iter().map(|task| task.id.clone()).collect();
                self.rpc = rpc;
                self.active_server = index;
                if let Some(server) = self.servers.get_mut(index) {
                    server.status = ServerStatus::Online;
                }
                self.refresh_instances();
            }
            Err(err) => {
                if let Some(server) = self.servers.get_mut(index) {
                    server.status = ServerStatus::Offline;
                }
                self.tasks.clear();
                self.expanded.clear();
                self.instances.clear();
                self.last_error = Some(format!("Server connect failed: {}", err));
            }
        }
        Ok(())
    }

    fn toggle_local_server(&mut self, index: usize) -> Result<()> {
        if index >= self.servers.len() {
            return Ok(());
        }
        let server = self.servers[index].clone();
        if !server.is_local {
            self.last_error = Some("Only local server can be managed".to_string());
            return Ok(());
        }
        match server.status {
            ServerStatus::Online => {
                let rpc_result = if self.active_server == index {
                    self.rpc.shutdown()
                } else {
                    connect_to_server(Arc::clone(&self.rpc.runtime), &server)
                        .and_then(|rpc| rpc.shutdown())
                };
                let pid_result = stop_local_server_via_pid();
                if check_server_status(&self.rpc.runtime, &server.rpc_addr).unwrap_or(false) {
                    if let Some(entry) = self.servers.get_mut(index) {
                        entry.status = ServerStatus::Online;
                    }
                    self.last_error = Some(format!(
                        "Server is still online (rpc={}, pid={})",
                        rpc_result
                            .err()
                            .map(|err| err.to_string())
                            .unwrap_or_else(|| "ok".to_string()),
                        pid_result
                            .err()
                            .map(|err| err.to_string())
                            .unwrap_or_else(|| "ok".to_string())
                    ));
                    return Ok(());
                }
                if let Some(entry) = self.servers.get_mut(index) {
                    entry.status = ServerStatus::Offline;
                }
                if self.active_server == index {
                    self.rpc = RpcHandle::offline(Arc::clone(&self.rpc.runtime));
                    self.tasks.clear();
                    self.expanded.clear();
                    self.instances.clear();
                }
            }
            ServerStatus::Offline | ServerStatus::Unknown => {
                if let Err(err) = launch_server() {
                    self.last_error = Some(format!("Server start failed: {}", err));
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(200));
                match connect_to_server(Arc::clone(&self.rpc.runtime), &server) {
                    Ok(rpc) => {
                        let tasks = rpc.list_tasks().unwrap_or_default();
                        self.tasks = tasks;
                        self.expanded = self.tasks.iter().map(|task| task.id.clone()).collect();
                        self.rpc = rpc;
                        self.active_server = index;
                        if let Some(entry) = self.servers.get_mut(index) {
                            entry.status = ServerStatus::Online;
                        }
                        self.refresh_instances();
                    }
                    Err(err) => {
                        self.last_error = Some(format!("Server connect failed: {}", err));
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
enum Entry {
    ServerCategory { name: String },
    Server { index: usize },
    Category { name: String },
    Task { task_id: String },
    Instance { instance_id: String },
}

impl InputField {
    fn from_config(name: &str, config: &InputConfig) -> Self {
        match config {
            InputConfig::Select { options, default } => {
                let mut option_index = 0;
                if let Some(pos) = options.iter().position(|opt| opt == default) {
                    option_index = pos;
                }
                let value = options.get(option_index).cloned().unwrap_or_default();
                Self {
                    name: name.to_string(),
                    config: config.clone(),
                    value,
                    cursor: 0,
                    options: options.clone(),
                    option_index,
                }
            }
            InputConfig::Text { default, .. } => {
                let value = default.clone().unwrap_or_default();
                let cursor = value.len();
                Self {
                    name: name.to_string(),
                    config: config.clone(),
                    value,
                    cursor,
                    options: Vec::new(),
                    option_index: 0,
                }
            }
        }
    }

    fn insert_char(&mut self, ch: char) {
        if matches!(self.config, InputConfig::Select { .. }) {
            return;
        }
        self.value.insert(self.cursor, ch);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if matches!(self.config, InputConfig::Select { .. }) {
            return;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    fn cycle_option(&mut self, forward: bool) {
        if self.options.is_empty() {
            return;
        }
        if forward {
            self.option_index = (self.option_index + 1) % self.options.len();
        } else {
            self.option_index = if self.option_index == 0 {
                self.options.len() - 1
            } else {
                self.option_index - 1
            };
        }
        self.value = self.options[self.option_index].clone();
    }
}

impl InputFormState {
    fn collect_values(&self) -> HashMap<String, String> {
        self.fields
            .iter()
            .map(|field| (field.name.clone(), field.value.clone()))
            .collect()
    }
}

struct PassthroughRequest {
    instance_id: String,
    task_name: String,
    instance_info: Option<InstanceInfo>,
    rpc: RpcHandle,
    attach_addr: Option<String>,
    ui_config: UiConfig,
    key_config: KeyBindings,
}

enum PassthroughOutcome {
    BackToList,
    QuitClient,
}

enum AttachStream {
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl AttachStream {
    fn connect(addr: &str) -> Result<Self> {
        match parse_endpoint(addr)? {
            RpcEndpoint::Unix(path) => Ok(Self::Unix(UnixStream::connect(path)?)),
            RpcEndpoint::Tcp(host) => Ok(Self::Tcp(TcpStream::connect(host)?)),
        }
    }

    fn try_clone(&self) -> io::Result<Self> {
        match self {
            AttachStream::Unix(stream) => Ok(AttachStream::Unix(stream.try_clone()?)),
            AttachStream::Tcp(stream) => Ok(AttachStream::Tcp(stream.try_clone()?)),
        }
    }

    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        match self {
            AttachStream::Unix(stream) => stream.set_read_timeout(timeout),
            AttachStream::Tcp(stream) => stream.set_read_timeout(timeout),
        }
    }

    fn shutdown(&self, how: std::net::Shutdown) -> io::Result<()> {
        match self {
            AttachStream::Unix(stream) => stream.shutdown(how),
            AttachStream::Tcp(stream) => stream.shutdown(how),
        }
    }
}

impl Read for AttachStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            AttachStream::Unix(stream) => stream.read(buf),
            AttachStream::Tcp(stream) => stream.read(buf),
        }
    }
}

impl Write for AttachStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            AttachStream::Unix(stream) => stream.write(buf),
            AttachStream::Tcp(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            AttachStream::Unix(stream) => stream.flush(),
            AttachStream::Tcp(stream) => stream.flush(),
        }
    }
}

fn run_passthrough(request: PassthroughRequest) -> Result<PassthroughOutcome> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;

    let attach_addr = request
        .attach_addr
        .as_ref()
        .ok_or_else(|| anyhow!("attach endpoint not configured for server"))?;
    let mut stream = AttachStream::connect(attach_addr)?;
    stream.write_all(format!("{}\n", request.instance_id).as_bytes())?;
    stream.flush()?;

    let reader_stream = stream.try_clone()?;
    reader_stream.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut reader = io::BufReader::new(reader_stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    if !response.trim_start().starts_with("OK") {
        return Err(anyhow!("attach failed: {}", response.trim()));
    }

    let size = crossterm::terminal::size()?;
    set_scroll_region(size.1)?;
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), Show)?;

    let header = format!(
        "\r\n\x1b[1;36m=== CmdHub Task Session ===\x1b[0m\r\n\
         Task: \x1b[1m{}\x1b[0m (ID: {})\r\n\
         Started: {}\r\n\
         \x1b[1;36m===========================\x1b[0m\r\n\r\n",
        request.task_name,
        request.instance_id,
        format_start_time(request.instance_info.as_ref())
    );
    let status_cache = Arc::new(Mutex::new(request.instance_info.clone()));
    stdout.write_all(header.as_bytes())?;
    draw_status_bar(&mut stdout, size.0, size.1, &request, &status_cache, false)?;
    let stop = Arc::new(Mutex::new(false));

    let reader_stop = Arc::clone(&stop);
    let reader_handle = {
        let mut reader = reader;
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut out = io::stdout();
            loop {
                let stopped = reader_stop.lock().map(|lock| *lock).unwrap_or(true);
                if stopped {
                    break;
                }
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let _ = out.write_all(&buf[..n]);
                        let _ = out.flush();
                    }
                    Err(err)
                        if err.kind() == io::ErrorKind::WouldBlock
                            || err.kind() == io::ErrorKind::TimedOut =>
                    {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        })
    };

    let status_stop = Arc::clone(&stop);
    let status_cache_clone = Arc::clone(&status_cache);
    let rpc = request.rpc.clone();
    let instance_id = request.instance_id.clone();
    thread::spawn(move || {
        while !status_stop.lock().map(|lock| *lock).unwrap_or(true) {
            if let Ok(list) = rpc.list_instances() {
                if let Some(info) = list.into_iter().find(|info| info.id == instance_id) {
                    if let Ok(mut guard) = status_cache_clone.lock() {
                        *guard = Some(info);
                    }
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });

    let mut command_mode = false;
    let mut last_status_update = Instant::now();

    let mut outcome = PassthroughOutcome::BackToList;
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let toggle_key = request
                    .key_config
                    .task_running
                    .get("toggle_command_mode")
                    .map(|s| s.as_str())
                    .unwrap_or("ctrl+p");
                if matches_key(&key, toggle_key) {
                    command_mode = !command_mode;
                    let size = crossterm::terminal::size()?;
                    draw_status_bar(&mut stdout, size.0, size.1, &request, &status_cache, command_mode)?;
                    continue;
                }

                if command_mode {
                    let kill_key = request
                        .key_config
                        .task_running
                        .get("kill_task")
                        .map(|s| s.as_str())
                        .unwrap_or("k");
                    let back_key = request
                        .key_config
                        .task_running
                        .get("back_to_list")
                        .map(|s| s.as_str())
                        .unwrap_or("b");
                    let quit_key = request
                        .key_config
                        .task_running
                        .get("quit_client")
                        .map(|s| s.as_str())
                        .unwrap_or("q");
                    let legacy_quit_task = request
                        .key_config
                        .task_running
                        .get("quit_task")
                        .map(|s| s.as_str());

                    if matches_key(&key, back_key)
                        || legacy_quit_task.map(|value| matches_key(&key, value)).unwrap_or(false)
                    {
                        break;
                    } else if matches_key(&key, quit_key) {
                        outcome = PassthroughOutcome::QuitClient;
                        break;
                    } else if matches_key(&key, kill_key) {
                        let _ = request.rpc.stop(&request.instance_id);
                        break;
                    }
                } else if let Some(bytes) = key_event_to_bytes(&key) {
                    let _ = stream.write_all(&bytes);
                    let _ = stream.flush();
                }
            } else if let Event::Resize(cols, rows) = event::read()? {
                set_scroll_region(rows)?;
                draw_status_bar(&mut stdout, cols, rows, &request, &status_cache, command_mode)?;
            }
        }

        if last_status_update.elapsed() > Duration::from_millis(500) {
            last_status_update = Instant::now();
            let size = crossterm::terminal::size()?;
            draw_status_bar(&mut stdout, size.0, size.1, &request, &status_cache, command_mode)?;
        }

        let is_running = status_cache
            .lock()
            .ok()
            .and_then(|info| info.as_ref().map(|i| matches!(i.status, InstanceStatus::Running)))
            .unwrap_or(false);
        if !is_running && !command_mode {
            let size = crossterm::terminal::size()?;
            draw_status_bar(&mut stdout, size.0, size.1, &request, &status_cache, command_mode)?;
        }
    }

    if let Ok(mut lock) = stop.lock() {
        *lock = true;
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
    let _ = reader_handle.join();
    reset_scroll_region(&mut stdout)?;
    disable_raw_mode()?;
    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    Ok(outcome)
}

fn matches_key(event: &KeyEvent, binding: &str) -> bool {
    let binding = binding.trim().to_lowercase();
    let mut parts: Vec<&str> = binding.split('+').collect();
    let code_str = parts.pop().unwrap_or("").trim();
    
    let mut modifiers = KeyModifiers::empty();
    for mod_str in parts {
        let mod_str = mod_str.trim();
        match mod_str {
            "ctrl" => modifiers.insert(KeyModifiers::CONTROL),
            "alt" => modifiers.insert(KeyModifiers::ALT),
            "shift" => modifiers.insert(KeyModifiers::SHIFT),
            _ => {}
        }
    }
    
    if modifiers.contains(KeyModifiers::CONTROL)
        && !event.modifiers.contains(KeyModifiers::CONTROL)
    {
        return false;
    }
    if modifiers.contains(KeyModifiers::ALT) && !event.modifiers.contains(KeyModifiers::ALT) {
        return false;
    }
    if modifiers.contains(KeyModifiers::SHIFT) && !event.modifiers.contains(KeyModifiers::SHIFT) {
        return false;
    }

    match code_str {
        "enter" => event.code == KeyCode::Enter,
        "tab" => event.code == KeyCode::Tab,
        "esc" => event.code == KeyCode::Esc,
        "backspace" => event.code == KeyCode::Backspace,
        "up" => event.code == KeyCode::Up,
        "down" => event.code == KeyCode::Down,
        "left" => event.code == KeyCode::Left,
        "right" => event.code == KeyCode::Right,
        "home" => event.code == KeyCode::Home,
        "end" => event.code == KeyCode::End,
        "pageup" => event.code == KeyCode::PageUp,
        "pagedown" => event.code == KeyCode::PageDown,
        "delete" => event.code == KeyCode::Delete,
        "insert" => event.code == KeyCode::Insert,
        c if c.len() == 1 => {
             if let KeyCode::Char(ch) = event.code {
                 ch.to_ascii_lowercase() == c.chars().next().unwrap_or('\0')
             } else {
                 false
             }
        }
        _ => false,
    }
}

fn key_event_to_bytes(key: &KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                Some(vec![ctrl_byte(ch)])
            } else {
                Some(vec![ch as u8])
            }
        }
        KeyCode::Enter => Some(b"\r".to_vec()),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        _ => None,
    }
}

fn instance_line(info: &InstanceInfo) -> Line<'static> {
    let status = match &info.status {
        InstanceStatus::Running => ("Running".to_string(), Color::Green),
        InstanceStatus::Exited { code } => (format!("Exited({})", code), Color::Gray),
        InstanceStatus::Error { .. } => ("Error".to_string(), Color::Red),
    };
    let runtime = format_duration(info.started_at, info.ended_at);
    let pid = info
        .child_pid
        .map(|pid| format!("pid:{}", pid))
        .unwrap_or_else(|| "pid:-".to_string());
    Line::from(vec![
        Span::raw("  "),
        Span::styled("*", Style::default().fg(status.1)),
        Span::raw(" "),
        Span::styled(info.id.clone(), Style::default().fg(Color::Cyan)),
        Span::raw(" "),
        Span::styled(status.0, Style::default().fg(status.1)),
        Span::raw(" "),
        Span::styled(pid, Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(runtime, Style::default().fg(Color::DarkGray)),
    ])
}

fn format_duration(started_at: u64, ended_at: Option<u64>) -> String {
    let now = ended_at.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default()
    });
    let secs = now.saturating_sub(started_at);
    let minutes = secs / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn ctrl_byte(ch: char) -> u8 {
    (ch as u8) & 0x1f
}

fn set_scroll_region(rows: u16) -> Result<()> {
    let mut stdout = io::stdout();
    if rows > 1 {
        let region = format!("\x1b[1;{}r", rows - 1);
        stdout.write_all(region.as_bytes())?;
        stdout.flush()?;
    }
    Ok(())
}

fn reset_scroll_region(stdout: &mut impl Write) -> Result<()> {
    stdout.write_all(b"\x1b[r")?;
    stdout.flush()?;
    Ok(())
}

fn format_start_time(info: Option<&InstanceInfo>) -> String {
    if let Some(info) = info {
        return format!("{}s ago", format_duration(info.started_at, None));
    }
    "Unknown".to_string()
}

fn instance_status_details(info: Option<&InstanceInfo>) -> (String, String, String, String) {
    let mut title = String::new();
    let mut pid = "-".to_string();
    let mut status_str = "Unknown".to_string();
    let mut status_color = "0"; // Default

    if let Some(info) = info {
        title = info.title.clone().unwrap_or_default();
        pid = info.child_pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
        match &info.status {
            InstanceStatus::Running => {
                status_str = "Running".to_string();
                status_color = "32"; // Green
            }
            InstanceStatus::Exited { code } => {
                status_str = format!("Exited({})", code);
                status_color = "90"; // Dark Gray
            }
            InstanceStatus::Error { .. } => {
                status_str = "Error".to_string();
                status_color = "31"; // Red
            }
        }
    }
    (title, pid, status_str, status_color.to_string())
}

fn draw_status_bar(
    stdout: &mut impl Write,
    cols: u16,
    rows: u16,
    request: &PassthroughRequest,
    status_cache: &Arc<Mutex<Option<InstanceInfo>>>,
    command_mode: bool,
) -> Result<()> {
    let info = status_cache.lock().ok().and_then(|info| info.clone());
    let (title, pid, status, _status_color) = instance_status_details(info.as_ref());
    
    // Construct the status line
    // Format: [TaskName] | ID | PID: 123 | Status: Running | Title: bash
    let mut parts = vec![
        format!("[{}]", request.task_name),
        request.instance_id.clone(),
        format!("PID: {}", pid),
        format!("Status: {}", status),
    ];
    if !title.is_empty() {
        parts.push(format!("Title: {}", title));
    }
    if command_mode {
        // Show available shortcuts
        parts.clear(); // Clear status info
        parts.push("CMD MODE".to_string());
        
        let mut shortcuts: Vec<(String, String)> = request.key_config.task_running.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        // Sort for consistent display order? Or just iterate.
        shortcuts.sort_by(|a, b| a.0.cmp(&b.0));
        
        for (action, key) in shortcuts {
            parts.push(format!("[{}]: {}", key, action));
        }
    } else {
        // Show status info
        let toggle_key = request.key_config.task_running.get("toggle_command_mode")
            .map(|s| s.as_str())
            .unwrap_or("ctrl+p");
        parts.push(format!("{}: Cmd Mode", toggle_key));
    }

    let line_content = parts.join(" | ");
    
    let mut padded = line_content;
    if padded.len() < cols as usize {
        padded.push_str(&" ".repeat(cols as usize - padded.len()));
    } else {
        padded.truncate(cols as usize);
    }

    let row = rows;
    
    // Use colors from UI config
    let (fg_str, bg_str) = if command_mode {
        (
            request.ui_config.command_mode_fg.as_deref().unwrap_or("white bold"),
            request.ui_config.command_mode_bg.as_deref().unwrap_or("red"),
        )
    } else {
        (
            request.ui_config.status_bar_fg.as_deref().unwrap_or("white bold"),
            request.ui_config.status_bar_bg.as_deref().unwrap_or("blue"),
        )
    };

    let fg = UiConfig::parse_style(fg_str, false);
    let bg = UiConfig::parse_style(bg_str, true);

    let seq = format!("\x1b[{};1H\x1b[{};{}m{}\x1b[0m", row, bg, fg, padded);
    
    execute!(stdout, SavePosition)?;
    stdout.write_all(seq.as_bytes())?;
    stdout.flush()?;
    execute!(stdout, RestorePosition)?;
    Ok(())
}
