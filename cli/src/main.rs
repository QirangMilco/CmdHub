use anyhow::{anyhow, Result};
use cmdhub_core::config::load_config_auto;
use cmdhub_core::pty::PtySession;
use cmdhub_core::session::{SessionStatus, SessionStore};
use cmdhub_core::template::render_command;
use crossterm::{
    cursor::MoveTo,
    event::{self, DisableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear as TermClear, ClearType},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use tokio::sync::{broadcast, oneshot};
use uuid::Uuid;
const DEFAULT_HISTORY_LIMIT: usize = 10;
const ATTACH_LOG_TAIL_BYTES: usize = 64 * 1024;

type RunningTaskId = u64;

#[derive(PartialEq, Copy, Clone)]
enum View {
    Selection,
    Terminal,
    Inputs,
}

struct RunningTask {
    id: RunningTaskId,
    task_index: usize,
    pid: Option<u32>,
    started_at: u64,
    session: PtySession,
    logs: String,
    scroll: u16,
}

struct App {
    tasks: Vec<cmdhub_core::models::Task>,
    display_items: Vec<DisplayItem>,
    list_state: ListState,
    running_tasks: HashMap<RunningTaskId, RunningTask>,
    active_run_id: Option<RunningTaskId>,
    next_run_id: RunningTaskId,
    session_id: Option<Uuid>,
    current_view: View,
    use_native_scrollback: bool,
    log_rx: mpsc::Receiver<(RunningTaskId, Vec<u8>)>,
    log_tx: mpsc::Sender<(RunningTaskId, Vec<u8>)>,
    input_state: Option<InputState>,
}

#[derive(Clone)]
enum DisplayItem {
    Header(String),
    Task(usize),
    Running { run_id: RunningTaskId, ordinal: usize },
}

enum SelectedItem {
    Task(usize),
    Running(RunningTaskId),
}

enum InputValue {
    Select {
        options: Vec<String>,
        selected: usize,
    },
    Text {
        value: String,
        placeholder: Option<String>,
    },
}

struct InputEntry {
    name: String,
    value: InputValue,
}

struct InputState {
    task_index: usize,
    entries: Vec<InputEntry>,
    selected: usize,
}

fn normalize_category(category: &Option<String>) -> String {
    match category {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => "Uncategorized".to_string(),
    }
}

fn build_display_items(
    tasks: &[cmdhub_core::models::Task],
    running_tasks: &HashMap<RunningTaskId, RunningTask>,
) -> Vec<DisplayItem> {
    let mut order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, Vec<usize>> = HashMap::new();

    for (index, task) in tasks.iter().enumerate() {
        let category = normalize_category(&task.category);
        if !grouped.contains_key(&category) {
            order.push(category.clone());
            grouped.insert(category.clone(), Vec::new());
        }
        if let Some(items) = grouped.get_mut(&category) {
            items.push(index);
        }
    }

    let mut items = Vec::new();
    for category in order {
        items.push(DisplayItem::Header(category.clone()));
        if let Some(task_indices) = grouped.get(&category) {
            for task_index in task_indices {
                items.push(DisplayItem::Task(*task_index));
                let mut runs: Vec<&RunningTask> = running_tasks
                    .values()
                    .filter(|run| run.task_index == *task_index)
                    .collect();
                runs.sort_by_key(|run| (run.started_at, run.id));
                for (ordinal, run) in runs.iter().enumerate() {
                    items.push(DisplayItem::Running {
                        run_id: run.id,
                        ordinal: ordinal + 1,
                    });
                }
            }
        }
    }

    items
}

fn first_selectable_index(items: &[DisplayItem]) -> Option<usize> {
    items
        .iter()
        .position(|item| !matches!(item, DisplayItem::Header(_)))
}

fn selected_item(app: &App) -> Option<SelectedItem> {
    let selected = app.list_state.selected()?;
    match app.display_items.get(selected) {
        Some(DisplayItem::Task(task_index)) => Some(SelectedItem::Task(*task_index)),
        Some(DisplayItem::Running { run_id, .. }) => Some(SelectedItem::Running(*run_id)),
        _ => None,
    }
}

fn find_selected_index(items: &[DisplayItem], selected: &SelectedItem) -> Option<usize> {
    items.iter().position(|item| match (item, selected) {
        (DisplayItem::Task(task_index), SelectedItem::Task(selected_index)) => {
            task_index == selected_index
        }
        (DisplayItem::Running { run_id, .. }, SelectedItem::Running(selected_run_id)) => {
            run_id == selected_run_id
        }
        _ => false,
    })
}

impl App {
    fn new(tasks: Vec<cmdhub_core::models::Task>, session_id: Option<Uuid>) -> App {
        let mut list_state = ListState::default();
        let running_tasks = HashMap::new();
        let display_items = build_display_items(&tasks, &running_tasks);
        let initial_selection = first_selectable_index(&display_items);
        list_state.select(initial_selection);
        let (tx, rx) = mpsc::channel(1000);
        App {
            tasks,
            display_items,
            list_state,
            running_tasks,
            active_run_id: None,
            next_run_id: 1,
            session_id,
            current_view: View::Selection,
            use_native_scrollback: true,
            log_rx: rx,
            log_tx: tx,
            input_state: None,
        }
    }

    fn next(&mut self) {
        let Some(selected) = self.list_state.selected() else {
            self.list_state
                .select(first_selectable_index(&self.display_items));
            return;
        };
        if self.display_items.is_empty() {
            return;
        }
        let mut i = selected;
        for _ in 0..self.display_items.len() {
            i = (i + 1) % self.display_items.len();
            if !matches!(self.display_items[i], DisplayItem::Header(_)) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    fn previous(&mut self) {
        let Some(selected) = self.list_state.selected() else {
            self.list_state
                .select(first_selectable_index(&self.display_items));
            return;
        };
        if self.display_items.is_empty() {
            return;
        }
        let mut i = selected;
        for _ in 0..self.display_items.len() {
            if i == 0 {
                i = self.display_items.len() - 1;
            } else {
                i -= 1;
            }
            if !matches!(self.display_items[i], DisplayItem::Header(_)) {
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    fn allocate_run_id(&mut self) -> RunningTaskId {
        let run_id = self.next_run_id;
        self.next_run_id = self.next_run_id.saturating_add(1);
        run_id
    }

    fn rebuild_display_items(&mut self) {
        let selected = selected_item(self);
        self.display_items = build_display_items(&self.tasks, &self.running_tasks);
        let next_selection = selected
            .as_ref()
            .and_then(|item| find_selected_index(&self.display_items, item))
            .or_else(|| first_selectable_index(&self.display_items));
        self.list_state.select(next_selection);
    }

    fn activate_run(&mut self, run_id: RunningTaskId) {
        if self.running_tasks.contains_key(&run_id) {
            self.active_run_id = Some(run_id);
            self.current_view = View::Terminal;
        }
    }

    fn sync_running_task_pids(&self) -> Result<()> {
        let Some(session_id) = self.session_id else {
            return Ok(());
        };
        let store = SessionStore::new()?;
        let mut info = store.load_session(session_id)?;
        let mut pids: Vec<u32> = self
            .running_tasks
            .values()
            .filter_map(|task| task.pid)
            .collect();
        pids.sort_unstable();
        info.running_task_pids = pids;
        store.write_session(&info)?;
        Ok(())
    }

    async fn start_task(&mut self, index: usize) -> Result<()> {
        let task = &self.tasks[index];
        let rendered = render_command(&task.command, &HashMap::new(), task.inputs.as_ref())?;
        let env_clear = task.env_clear.unwrap_or(false);
        let session = PtySession::new(&rendered, task.cwd.clone(), task.env.clone(), env_clear)?;
        let pid = session.child.process_id();
        let run_id = self.allocate_run_id();

        let tx = self.log_tx.clone();

        let (session_tx, mut session_rx) = mpsc::channel::<Vec<u8>>(100);
        session.run(session_tx).await?;

        tokio::spawn(async move {
            while let Some(data) = session_rx.recv().await {
                if tx.send((run_id, data)).await.is_err() {
                    break;
                }
            }
        });
        self.running_tasks.insert(
            run_id,
            RunningTask {
                id: run_id,
                task_index: index,
                pid,
                started_at: now_epoch(),
                session,
                logs: String::new(),
                scroll: 0,
            },
        );

        self.active_run_id = Some(run_id);
        self.current_view = View::Terminal;
        self.rebuild_display_items();
        let _ = self.sync_running_task_pids();
        Ok(())
    }

    async fn start_task_with_inputs(
        &mut self,
        index: usize,
        values: &HashMap<String, String>,
    ) -> Result<()> {
        let task = &self.tasks[index];
        let rendered = render_command(&task.command, values, task.inputs.as_ref())?;
        let env_clear = task.env_clear.unwrap_or(false);
        let session = PtySession::new(&rendered, task.cwd.clone(), task.env.clone(), env_clear)?;
        let pid = session.child.process_id();
        let run_id = self.allocate_run_id();

        let tx = self.log_tx.clone();

        let (session_tx, mut session_rx) = mpsc::channel::<Vec<u8>>(100);
        session.run(session_tx).await?;

        tokio::spawn(async move {
            while let Some(data) = session_rx.recv().await {
                if tx.send((run_id, data)).await.is_err() {
                    break;
                }
            }
        });
        self.running_tasks.insert(
            run_id,
            RunningTask {
                id: run_id,
                task_index: index,
                pid,
                started_at: now_epoch(),
                session,
                logs: String::new(),
                scroll: 0,
            },
        );

        self.active_run_id = Some(run_id);
        self.current_view = View::Terminal;
        self.rebuild_display_items();
        let _ = self.sync_running_task_pids();
        Ok(())
    }

    fn prepare_inputs(&mut self, index: usize) {
        let task = &self.tasks[index];
        let mut entries = Vec::new();

        if let Some(inputs) = &task.inputs {
            let mut keys: Vec<&String> = inputs.keys().collect();
            keys.sort();
            for key in keys {
                if let Some(config) = inputs.get(key) {
                    let value = match config {
                        cmdhub_core::models::InputConfig::Select { options, default } => {
                            let selected =
                                options.iter().position(|opt| opt == default).unwrap_or(0);
                            InputValue::Select {
                                options: options.clone(),
                                selected,
                            }
                        }
                        cmdhub_core::models::InputConfig::Text {
                            placeholder,
                            default,
                        } => InputValue::Text {
                            value: default.clone().unwrap_or_default(),
                            placeholder: placeholder.clone(),
                        },
                    };
                    entries.push(InputEntry {
                        name: key.clone(),
                        value,
                    });
                }
            }
        }

        self.input_state = Some(InputState {
            task_index: index,
            entries,
            selected: 0,
        });
        self.current_view = View::Inputs;
    }

    fn kill_active_task(&mut self) -> Result<()> {
        if let Some(run_id) = self.active_run_id.take() {
            if let Some(mut task) = self.running_tasks.remove(&run_id) {
                task.session.kill()?;
            }
            self.rebuild_display_items();
            let _ = self.sync_running_task_pids();
        }
        self.current_view = View::Selection;
        Ok(())
    }

    fn kill_all_tasks(&mut self) -> Result<()> {
        for (_, mut task) in self.running_tasks.drain() {
            let _ = task.session.kill();
        }
        self.active_run_id = None;
        self.rebuild_display_items();
        let _ = self.sync_running_task_pids();
        Ok(())
    }

    fn refresh_logs(&mut self, view_height: u16) -> Result<()> {
        while let Ok((idx, data)) = self.log_rx.try_recv() {
            if let Some(task) = self.running_tasks.get_mut(&idx) {
                let normalized = sanitize_log_chunk(&data);
                task.logs.push_str(&normalized);

                // Performance: Limit logs buffer to last 2000 lines
                let lines: Vec<&str> = task.logs.lines().collect();
                if lines.len() > 2000 {
                    task.logs = lines[lines.len() - 2000..].join("\n");
                }

                let line_count = task.logs.lines().count() as u16;
                let max_scroll = line_count.saturating_sub(view_height);
                if self.use_native_scrollback {
                    task.scroll = max_scroll;
                } else if task.scroll >= max_scroll {
                    task.scroll = max_scroll;
                }
            }
        }
        Ok(())
    }
}

fn sanitize_log_chunk(data: &[u8]) -> String {
    String::from_utf8_lossy(data)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

fn parse_ansi_text(input: &str) -> Text<'static> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default();
    let mut buffer = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' && matches!(chars.peek(), Some('[')) {
            chars.next();
            if !buffer.is_empty() {
                spans.push(Span::styled(buffer.clone(), style));
                buffer.clear();
            }
            let mut sequence = String::new();
            while let Some(next) = chars.next() {
                if next == 'm' {
                    apply_sgr(&sequence, &mut style);
                    break;
                }
                sequence.push(next);
            }
            continue;
        }

        if ch == '\n' {
            if !buffer.is_empty() {
                spans.push(Span::styled(buffer.clone(), style));
                buffer.clear();
            }
            lines.push(Line::from(spans));
            spans = Vec::new();
        } else {
            buffer.push(ch);
        }
    }

    if !buffer.is_empty() {
        spans.push(Span::styled(buffer, style));
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }

    Text::from(lines)
}

fn apply_sgr(sequence: &str, style: &mut Style) {
    let codes: Vec<i64> = if sequence.is_empty() {
        vec![0]
    } else {
        sequence
            .split(';')
            .filter_map(|value| value.parse::<i64>().ok())
            .collect()
    };

    let mut index = 0;
    while index < codes.len() {
        match codes[index] {
            0 => *style = Style::default(),
            1 => *style = style.add_modifier(Modifier::BOLD),
            2 => *style = style.add_modifier(Modifier::DIM),
            3 => *style = style.add_modifier(Modifier::ITALIC),
            4 => *style = style.add_modifier(Modifier::UNDERLINED),
            7 => *style = style.add_modifier(Modifier::REVERSED),
            9 => *style = style.add_modifier(Modifier::CROSSED_OUT),
            22 => *style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => *style = style.remove_modifier(Modifier::ITALIC),
            24 => *style = style.remove_modifier(Modifier::UNDERLINED),
            27 => *style = style.remove_modifier(Modifier::REVERSED),
            29 => *style = style.remove_modifier(Modifier::CROSSED_OUT),
            30..=37 | 90..=97 => style.fg = ansi_color(codes[index]),
            40..=47 | 100..=107 => {
                let fg_code = codes[index] - 10;
                style.bg = ansi_color(fg_code);
            }
            38 | 48 => {
                let is_fg = codes[index] == 38;
                if index + 1 < codes.len() {
                    match codes[index + 1] {
                        5 if index + 2 < codes.len() => {
                            let color = Color::Indexed(clamp_u8(codes[index + 2]));
                            if is_fg {
                                style.fg = Some(color);
                            } else {
                                style.bg = Some(color);
                            }
                            index += 3;
                            continue;
                        }
                        2 if index + 4 < codes.len() => {
                            let r = clamp_u8(codes[index + 2]);
                            let g = clamp_u8(codes[index + 3]);
                            let b = clamp_u8(codes[index + 4]);
                            let color = Color::Rgb(r, g, b);
                            if is_fg {
                                style.fg = Some(color);
                            } else {
                                style.bg = Some(color);
                            }
                            index += 5;
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            39 => style.fg = None,
            49 => style.bg = None,
            _ => {}
        }
        index += 1;
    }
}

fn ansi_color(code: i64) -> Option<Color> {
    match code {
        30 => Some(Color::Black),
        31 => Some(Color::Red),
        32 => Some(Color::Green),
        33 => Some(Color::Yellow),
        34 => Some(Color::Blue),
        35 => Some(Color::Magenta),
        36 => Some(Color::Cyan),
        37 => Some(Color::Gray),
        90 => Some(Color::DarkGray),
        91 => Some(Color::LightRed),
        92 => Some(Color::LightGreen),
        93 => Some(Color::LightYellow),
        94 => Some(Color::LightBlue),
        95 => Some(Color::LightMagenta),
        96 => Some(Color::LightCyan),
        97 => Some(Color::White),
        _ => None,
    }
}

fn clamp_u8(value: i64) -> u8 {
    if value < 0 {
        0
    } else if value > 255 {
        255
    } else {
        value as u8
    }
}

async fn load_history_limit() -> Result<usize> {
    let config = load_config_auto().await?;
    Ok(config.history_limit.unwrap_or(DEFAULT_HISTORY_LIMIT))
}

fn parse_start_args() -> Result<String> {
    let mut args = std::env::args().skip(2);
    let Some(first) = args.next() else {
        return Err(anyhow!("missing session name"));
    };
    if first == "--name" {
        let value = args
            .next()
            .ok_or_else(|| anyhow!("--name requires a value"))?;
        return Ok(value);
    }
    Ok(first)
}

fn parse_session_args(args: &[String]) -> Result<Option<Uuid>> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--session" {
            let value = iter
                .next()
                .ok_or_else(|| anyhow!("missing session id"))?;
            return Ok(Some(Uuid::parse_str(value)?));
        }
    }
    Ok(None)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|value| value.as_str()) {
        Some("ls") => {
            run_ls()?;
            return Ok(());
        }
        Some("history") => {
            run_history()?;
            return Ok(());
        }
        Some("kill") => {
            let session_id = args
                .get(2)
                .ok_or_else(|| anyhow!("missing session id"))?;
            run_kill(session_id).await?;
            return Ok(());
        }
        Some("start") => {
            run_start().await?;
            std::process::exit(0);
        }
        Some("attach") => {
            let session_id = args
                .get(2)
                .ok_or_else(|| anyhow!("missing session id"))?;
            run_attach(session_id).await?;
            std::process::exit(0);
        }
        Some("tui") => {
            let session_id = parse_session_args(&args[2..])?;
            run_tui(true, session_id).await?;
            return Ok(());
        }
        Some("session-host") => {
            let session_id = args
                .get(2)
                .ok_or_else(|| anyhow!("missing session id"))?;
            run_session_host(session_id).await?;
            return Ok(());
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_help();
            return Ok(());
        }
        _ => {}
    }

    run_tui(false, None).await
}

fn print_help() {
    println!("CmdHub CLI - Command Line Interface for CmdHub");
    println!();
    println!("Usage: cmdhub [COMMAND] [ARGS]");
    println!();
    println!("Commands:");
    println!("  ls             List active sessions");
    println!("  history        List session history");
    println!("  start          Start a new TUI session (optionally with --name <name>)");
    println!("  attach <id>    Attach to a running session (Ctrl+b to detach)");
    println!("  kill <id>      Kill a running session");
    println!("  tui            Open the TUI interface (default)");
    println!("  help           Show this help message");
    println!();
}
async fn run_tui(session_mode: bool, session_id: Option<Uuid>) -> Result<()> {
    let config = load_config_auto().await?;
    let app = App::new(config.tasks, session_id);

    let mut stdout = io::stdout();
    execute!(stdout, TermClear(ClearType::All), MoveTo(0, 0))?;
    enable_raw_mode()?;
    execute!(stdout, DisableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, app, !session_mode).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    terminal.show_cursor()?;
    if !session_mode {
        println!("--- CmdHub exited ---");
    }

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}
fn run_ls() -> Result<()> {
    let store = SessionStore::new()?;
    let sessions = store.list_sessions()?;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }
    for info in sessions {
        let display_name = info
            .session_name
            .as_deref()
            .unwrap_or(info.task_name.as_str());
        let process_count = info.running_task_pids.len();
        println!(
            "{}: {} processes (ID: {})",
            display_name, process_count, info.id
        );
    }
    Ok(())
}

fn run_history() -> Result<()> {
    let store = SessionStore::new()?;
    let sessions = store.list_history()?;
    if sessions.is_empty() {
        println!("No history found.");
        return Ok(());
    }
    for info in sessions {
        let display_name = info
            .session_name
            .as_deref()
            .unwrap_or(info.task_name.as_str());
        println!(
            "{}\t{}\t{:?}\t{}",
            info.id, display_name, info.status, info.started_at
        );
    }
    Ok(())
}

async fn run_kill(name_or_id: &str) -> Result<()> {
    let store = SessionStore::new()?;
    let id = resolve_session_id(&store, name_or_id)?;
    let history_limit = load_history_limit().await?;
    kill_session(&store, id, history_limit)?;
    println!("Killed session {}", name_or_id);
    Ok(())
}

async fn run_start() -> Result<()> {
    let session_name = parse_start_args()?;
    let store = SessionStore::new()?;
    let info = store.create_session(
        "tui".to_string(),
        "CmdHub".to_string(),
        Some(session_name),
        "tui".to_string(),
        None,
        None,
        false,
    )?;
    spawn_session_host(&store, info.id)?;
    println!("Session started: {}", info.id);

    let socket_path = store.session_dir(info.id).join("attach.sock");
    let start = std::time::Instant::now();
    while !socket_path.exists() {
        if start.elapsed().as_secs() > 5 {
            return Err(anyhow!("Timed out waiting for session to start"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    run_attach(&info.id.to_string()).await?;
    Ok(())
}

fn resolve_session_id(store: &SessionStore, name_or_id: &str) -> Result<Uuid> {
    if let Ok(id) = Uuid::parse_str(name_or_id) {
        return Ok(id);
    }

    let sessions = store.list_sessions()?;
    let matches: Vec<_> = sessions
        .iter()
        .filter(|info| info.session_name.as_deref() == Some(name_or_id))
        .collect();

    if matches.is_empty() {
        return Err(anyhow!("no active session named '{}'", name_or_id));
    }

    let latest = matches
        .iter()
        .max_by_key(|info| info.started_at)
        .ok_or_else(|| anyhow!("no active session named '{}'", name_or_id))?;
    Ok(latest.id)
}

async fn run_attach(name_or_id: &str) -> Result<()> {
    let store = SessionStore::new()?;
    let id = resolve_session_id(&store, name_or_id)?;
    let info = store.load_session(id)?;
    let needs_refresh = info.task_id == "tui" || info.command == "tui";
    let display_name = info
        .session_name
        .as_deref()
        .unwrap_or(info.task_name.as_str());
    let socket_path = info
        .socket_path
        .ok_or_else(|| anyhow!("session is not attachable yet"))?;

    println!("Attaching to session '{}'. Press Ctrl+b to detach.", name_or_id);

    let mut stdout = io::stdout();
    execute!(stdout, TermClear(ClearType::All), MoveTo(0, 0))?;
    enable_raw_mode()?;
    execute!(stdout, DisableMouseCapture)?;

    if !needs_refresh {
        if let Ok(mut log_file) = std::fs::File::open(store.session_log_path(id)) {
            let _ = log_file.seek(io::SeekFrom::End(0)).map(|size| {
                let start = size.saturating_sub(ATTACH_LOG_TAIL_BYTES as u64);
                let _ = log_file.seek(io::SeekFrom::Start(start));
            });
            let _ = io::copy(&mut log_file, &mut stdout);
            let _ = stdout.flush();
        }
    }

    let stream = tokio::net::UnixStream::connect(socket_path).await?;
    let (mut reader, mut writer) = stream.into_split();
    if needs_refresh {
        let _ = writer.write_all(&[0x0c]).await;
    }

    let mut stdout_async = tokio::io::stdout();
    let mut output_task = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut reader, &mut stdout_async).await;
    });

    let mut stdin_async = tokio::io::stdin();
    let mut buf = [0u8; 1024];
    loop {
        tokio::select! {
            _ = &mut output_task => {
                break;
            }
            res = stdin_async.read(&mut buf) => {
                let n = res?;
                if n == 0 {
                    break;
                }
                let mut out = Vec::new();
                let mut detach = false;
                for byte in &buf[..n] {
                    if *byte == 0x02 {
                        detach = true;
                        break;
                    }
                    out.push(*byte);
                }
                if !out.is_empty() {
                    if writer.write_all(&out).await.is_err() {
                        break;
                    }
                }
                if detach {
                    break;
                }
            }
        }
    }

    let _ = writer.shutdown().await;
    output_task.abort();
    disable_raw_mode()?;
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, crossterm::cursor::Show)?;
    if let Ok((_, rows)) = crossterm::terminal::size() {
        let row = rows.saturating_sub(1);
        let _ = execute!(stdout, MoveTo(0, row));
    }
    println!("\nSession detached: {} (ID: {})", display_name, id);
    Ok(())
}

async fn run_session_host(session_id: &str) -> Result<()> {
    let id = Uuid::parse_str(session_id)?;
    let store = SessionStore::new()?;
    let history_limit = load_history_limit().await?;
    let mut info = store.load_session(id)?;
    info.status = SessionStatus::Running;
    info.runner_pid = Some(std::process::id());

    let socket_path = store.session_dir(id).join("attach.sock");
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }
    let listener = UnixListener::bind(&socket_path)?;
    info.socket_path = Some(socket_path.clone());
    store.write_session(&info)?;

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let exe = std::env::current_exe()?;
    let cwd = std::env::current_dir()?;
    let mut cmd = CommandBuilder::new(exe);
    cmd.arg("tui");
    cmd.arg("--session");
    cmd.arg(id.to_string());
    cmd.cwd(cwd);
    let mut child = pair.slave.spawn_command(cmd)?;
    info.child_pid = child.process_id();
    store.write_session(&info)?;

    let log_path = store.session_log_path(info.id);
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let (output_tx, _) = broadcast::channel::<Vec<u8>>(100);
    let (input_tx, mut input_rx) = mpsc::channel::<Vec<u8>>(100);

    let mut reader = pair.master.try_clone_reader()?;
    let output_tx_clone = output_tx.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            let data = buf[..n].to_vec();
            let _ = log_file.write_all(&data);
            let _ = output_tx_clone.send(data);
        }
    });

    let mut writer = pair.master.take_writer()?;
    thread::spawn(move || {
        while let Some(data) = input_rx.blocking_recv() {
            let _ = writer.write_all(&data);
        }
    });

    let (exit_tx, mut exit_rx) = oneshot::channel::<portable_pty::ExitStatus>();
    thread::spawn(move || {
        if let Ok(status) = child.wait() {
            let _ = exit_tx.send(status);
        }
    });

    loop {
        tokio::select! {
            exit = &mut exit_rx => {
                if let Ok(status) = exit {
                    info.status = SessionStatus::Exited;
                    info.ended_at = Some(now_epoch());
                    info.exit_code = Some(status.exit_code());
                    let _ = store.write_session(&info);
                    let _ = store.move_to_history(info.id, history_limit);
                }
                let _ = fs::remove_file(&socket_path);
                break;
            }
            accept = listener.accept() => {
                let (stream, _) = accept?;
                let output_rx = output_tx.subscribe();
                let input_tx = input_tx.clone();
                tokio::spawn(handle_attach_stream(stream, output_rx, input_tx));
            }
        }
    }

    Ok(())
}

fn spawn_session_host(store: &SessionStore, session_id: Uuid) -> Result<()> {
    let exe = std::env::current_exe()?;
    let child = Command::new(exe)
        .arg("session-host")
        .arg(session_id.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Ok(mut info) = store.load_session(session_id) {
        info.runner_pid = Some(child.id());
        let _ = store.write_session(&info);
    }
    Ok(())
}

async fn handle_attach_stream(
    stream: tokio::net::UnixStream,
    mut output_rx: broadcast::Receiver<Vec<u8>>,
    input_tx: mpsc::Sender<Vec<u8>>,
) {
    let (mut reader, mut writer) = stream.into_split();

    let write_task = tokio::spawn(async move {
        while let Ok(data) = output_rx.recv().await {
            if writer.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    let mut buf = [0u8; 1024];
    loop {
        let n = match reader.read(&mut buf).await {
            Ok(n) => n,
            Err(_) => break,
        };
        if n == 0 {
            break;
        }
        if input_tx.send(buf[..n].to_vec()).await.is_err() {
            break;
        }
    }

    write_task.abort();
}

fn kill_session(store: &SessionStore, session_id: Uuid, history_limit: usize) -> Result<()> {
    let info = store.load_session(session_id)?;
    if let Some(pid) = info.runner_pid.or(info.child_pid) {
        terminate_pid(pid)?;
    }
    store.move_to_history(session_id, history_limit)?;
    Ok(())
}

fn terminate_pid(pid: u32) -> Result<()> {
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status()?;
        if !status.success() {
            return Err(anyhow!("failed to terminate pid {}", pid));
        }
    }
    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()?;
        if !status.success() {
            return Err(anyhow!("failed to terminate pid {}", pid));
        }
    }
    Ok(())
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

async fn run_app<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    mut app: App,
    _allow_exit: bool,
) -> Result<()> {
    let mut force_redraw = false;
    loop {
        if force_redraw {
            terminal.clear()?;
            force_redraw = false;
        }
        let view_height = terminal.size()?.height.saturating_sub(2);
        app.refresh_logs(view_height)?;
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('l')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        force_redraw = true;
                        continue;
                    }
                    match app.current_view {
                        View::Selection => match key.code {
                        KeyCode::Char('q') => {
                            app.kill_all_tasks()?;
                            std::process::exit(0);
                        }
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        KeyCode::Enter => {
                            match selected_item(&app) {
                                Some(SelectedItem::Task(index)) => {
                                    let has_inputs = app.tasks[index]
                                        .inputs
                                        .as_ref()
                                        .map_or(false, |v| !v.is_empty());
                                    if has_inputs {
                                        app.prepare_inputs(index);
                                    } else {
                                        app.start_task(index).await?;
                                    }
                                }
                                Some(SelectedItem::Running(run_id)) => {
                                    app.activate_run(run_id);
                                }
                                None => {}
                            }
                        }
                        _ => {}
                    },
                    View::Terminal => match key.code {
                        KeyCode::Esc => {
                            app.kill_active_task()?;
                        }
                        KeyCode::Char('q') | KeyCode::Backspace => {
                            app.current_view = View::Selection;
                        }
                        KeyCode::PageDown => {
                            if app.use_native_scrollback {
                                continue;
                            }
                            if let Some(run_id) = app.active_run_id {
                                if let Some(task) = app.running_tasks.get_mut(&run_id) {
                                    let line_count = task.logs.lines().count() as u16;
                                    let height = terminal.size()?.height.saturating_sub(2);
                                    let max_scroll = line_count.saturating_sub(height);
                                    task.scroll = task.scroll.saturating_add(5).min(max_scroll);
                                }
                            }
                        }
                        KeyCode::PageUp => {
                            if app.use_native_scrollback {
                                continue;
                            }
                            if let Some(run_id) = app.active_run_id {
                                if let Some(task) = app.running_tasks.get_mut(&run_id) {
                                    task.scroll = task.scroll.saturating_sub(5);
                                }
                            }
                        }
                        _ => {}
                    },
                    View::Inputs => match key.code {
                        KeyCode::Esc => {
                            app.input_state = None;
                            app.current_view = View::Selection;
                        }
                        KeyCode::Up => {
                            if let Some(state) = app.input_state.as_mut() {
                                if state.selected > 0 {
                                    state.selected -= 1;
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Tab => {
                            if let Some(state) = app.input_state.as_mut() {
                                if state.selected + 1 < state.entries.len() {
                                    state.selected += 1;
                                }
                            }
                        }
                        KeyCode::Left => {
                            if let Some(state) = app.input_state.as_mut() {
                                if let Some(entry) = state.entries.get_mut(state.selected) {
                                    if let InputValue::Select { options, selected } =
                                        &mut entry.value
                                    {
                                        if !options.is_empty() {
                                            if *selected == 0 {
                                                *selected = options.len() - 1;
                                            } else {
                                                *selected -= 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Right => {
                            if let Some(state) = app.input_state.as_mut() {
                                if let Some(entry) = state.entries.get_mut(state.selected) {
                                    if let InputValue::Select { options, selected } =
                                        &mut entry.value
                                    {
                                        if !options.is_empty() {
                                            *selected = (*selected + 1) % options.len();
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            if let Some(state) = app.input_state.as_mut() {
                                if let Some(entry) = state.entries.get_mut(state.selected) {
                                    if let InputValue::Text { value, .. } = &mut entry.value {
                                        value.pop();
                                    }
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(state) = app.input_state.take() {
                                let mut values = HashMap::new();
                                for entry in state.entries {
                                    let value = match entry.value {
                                        InputValue::Select { options, selected } => {
                                            options.get(selected).cloned().unwrap_or_default()
                                        }
                                        InputValue::Text { value, .. } => value,
                                    };
                                    values.insert(entry.name, value);
                                }
                                app.start_task_with_inputs(state.task_index, &values)
                                    .await?;
                            }
                        }
                        KeyCode::Char(ch) => {
                            if let Some(state) = app.input_state.as_mut() {
                                if let Some(entry) = state.entries.get_mut(state.selected) {
                                    if let InputValue::Text { value, .. } = &mut entry.value {
                                        value.push(ch);
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                }
                },
                Event::Mouse(mouse_event) => {
                    if app.use_native_scrollback {
                        continue;
                    }
                    if app.current_view == View::Terminal {
                        if let Some(run_id) = app.active_run_id {
                            if let Some(task) = app.running_tasks.get_mut(&run_id) {
                                match mouse_event.kind {
                                    MouseEventKind::ScrollUp => {
                                        task.scroll = task.scroll.saturating_sub(2);
                                    }
                                    MouseEventKind::ScrollDown => {
                                        let line_count = task.logs.lines().count() as u16;
                                        let height = terminal.size()?.height.saturating_sub(2);
                                        let max_scroll = line_count.saturating_sub(height);
                                        task.scroll = task.scroll.saturating_add(2).min(max_scroll);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.size();
    f.render_widget(Clear, area);
    match app.current_view {
        View::Selection => {
            let items: Vec<ListItem> = app
                .display_items
                .iter()
                .enumerate()
                .map(|(_, item)| match item {
                    DisplayItem::Header(title) => ListItem::new(format!("== {} ==", title)).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    DisplayItem::Task(task_index) => {
                        let run_count = app
                            .running_tasks
                            .values()
                            .filter(|run| run.task_index == *task_index)
                            .count();
                        let status = if run_count > 0 {
                            format!(" [Running {}]", run_count)
                        } else {
                            String::new()
                        };
                        let task = &app.tasks[*task_index];
                        ListItem::new(format!("{}{}", task.name, status))
                    }
                    DisplayItem::Running { run_id, ordinal } => {
                        let pid = app
                            .running_tasks
                            .get(run_id)
                            .and_then(|run| run.pid)
                            .map(|pid| pid.to_string())
                            .unwrap_or_else(|| "?".to_string());
                        ListItem::new(format!("  {}. PID {}", ordinal, pid))
                    }
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::default().title(
                        "Select Command (Enter: Run/View, q: Exit) | Ctrl+b: Detach",
                    )
                        .borders(Borders::ALL),
                )
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, area, &mut app.list_state);
        }
        View::Terminal => {
            if let Some(run_id) = app.active_run_id {
                if let Some(task) = app.running_tasks.get(&run_id) {
                    let line_count = task.logs.lines().count();
                    let height = area.height.saturating_sub(2) as usize;
                    let task_name = app.tasks[task.task_index].name.as_str();
                    let pid_display = task
                        .pid
                        .map(|pid| pid.to_string())
                        .unwrap_or_else(|| "?".to_string());

                    let title = if line_count > height {
                        let top = task.scroll as usize + 1;
                        let bottom = (task.scroll as usize + height).min(line_count);
                        let percent = if line_count == 0 {
                            0
                        } else {
                            (bottom * 100) / line_count
                        };
                        format!(
                            "Logs: {} (PID: {}) [Lines {}-{} / {} ({}%)] (Esc: Kill, q/Backspace: Back) | Ctrl+b: Detach",
                            task_name, pid_display, top, bottom, line_count, percent
                        )
                    } else {
                        format!(
                            "Logs: {} (PID: {}) (Esc: Kill, q/Backspace: Back) | Ctrl+b: Detach",
                            task_name, pid_display
                        )
                    };

                    let log_block = Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(
                            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
                        );

                    let logs = Paragraph::new(parse_ansi_text(task.logs.as_str()))
                        .block(log_block)
                        .wrap(Wrap { trim: false })
                        .scroll((task.scroll, 0));

                    f.render_widget(logs, area);

                    if line_count > height && !app.use_native_scrollback {
                        let mut scrollbar_state =
                            ScrollbarState::new(line_count).position(task.scroll as usize);
                        let scrollbar = Scrollbar::default()
                            .orientation(ScrollbarOrientation::VerticalRight)
                            .begin_symbol(None)
                            .end_symbol(None);
                        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
                    }
                }
            }
        }
        View::Inputs => {
            let block = Block::default().title(
                "Task Inputs (Enter: Run, Esc: Cancel, Tab/Up/Down: Select, Left/Right: Option) | Ctrl+b: Detach",
            )
            .borders(Borders::ALL);

            let mut items = Vec::new();
            if let Some(state) = &app.input_state {
                for entry in &state.entries {
                    let value = match &entry.value {
                        InputValue::Select { options, selected } => {
                            let selected_value =
                                options.get(*selected).cloned().unwrap_or_default();
                            format!("< {} >", selected_value)
                        }
                        InputValue::Text { value, placeholder } => {
                            if value.is_empty() {
                                placeholder.clone().unwrap_or_default()
                            } else {
                                value.clone()
                            }
                        }
                    };
                    let line = format!("{}: {}", entry.name, value);
                    items.push(ListItem::new(line));
                }
            }

            let list = List::new(items)
                .block(block)
                .highlight_style(
                    ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            if let Some(state) = &mut app.input_state {
                let mut list_state = ListState::default();
                list_state.select(Some(state.selected));
                f.render_stateful_widget(list, area, &mut list_state);
            } else {
                f.render_widget(list, area);
            }
        }
    }
}
