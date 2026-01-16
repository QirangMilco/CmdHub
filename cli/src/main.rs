use anyhow::{anyhow, Result};
use cmdhub_core::config::load_config_auto;
use cmdhub_core::instance::{InstanceInfo, InstanceStatus, SessionManager, SpawnedInstance};
use cmdhub_core::models::{AppConfig, InputConfig, Task, UiConfig, KeyBindings};
use cmdhub_core::template::render_command;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::cursor::{MoveTo, RestorePosition, SavePosition, Show};
use portable_pty::PtySize;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::iterator::Signals;
use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const BUFFER_CAP: usize = 16 * 1024;

fn main() -> Result<()> {
    env_logger::init();
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_main())
}

async fn async_main() -> Result<()> {
    let config = load_config_auto().await?;
    let manager = SessionManager::new(BUFFER_CAP);
    setup_signal_handlers(manager.clone())?;
    run_ui(config, manager)?;
    Ok(())
}

fn setup_signal_handlers(manager: SessionManager) -> Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM, SIGQUIT])?;
    thread::spawn(move || {
        for _ in signals.forever() {
            let _ = manager.terminate_all(libc::SIGHUP);
            std::process::exit(1);
        }
    });
    Ok(())
}

fn run_ui(config: AppConfig, manager: SessionManager) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut app = App::new(config, manager);
    let tick_rate = Duration::from_millis(200);
    let mut last_tick = Instant::now();

    loop {
        app.refresh_instances();
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
            let _outcome = run_passthrough(next, &app.manager)?;
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

struct App {
    config: AppConfig,
    manager: SessionManager,
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
    fn new(config: AppConfig, manager: SessionManager) -> Self {
        let expanded = config.tasks.iter().map(|task| task.id.clone()).collect();
        
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
            manager,
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
        }
    }

    fn refresh_instances(&mut self) {
        if let Ok(instances) = self.manager.list_instances() {
            self.instances = instances;
            self.rebuild_entries();
        }
    }

    fn rebuild_entries(&mut self) {
        let mut entries = Vec::new();
        let mut by_task: HashMap<String, Vec<InstanceInfo>> = HashMap::new();
        for instance in &self.instances {
            by_task.entry(instance.task_id.clone()).or_default().push(instance.clone());
        }

        let mut by_category: HashMap<String, Vec<&Task>> = HashMap::new();
        for task in &self.config.tasks {
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

    fn build_help(&self) -> Paragraph<'_> {
        let mut text = Vec::new();
        match self.mode {
            AppMode::List => {
                text.push(Line::from("Enter: run/attach  Tab: fold  d: delete  X: kill  Q: quit"));
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
             let _ = self.manager.terminate_all(libc::SIGTERM);
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
                 let _ = self.manager.remove_if_exited(instance_id);
             }
        } else if check("kill_instance", &key) {
             if let Some(Entry::Instance { instance_id }) = self.entries.get(self.selected) {
                 let _ = self.manager.kill_and_remove(instance_id);
             }
        } else if check("select", &key) {
             if let Some(entry) = self.entries.get(self.selected).cloned() {
                 match entry {
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
                    let task = self.config.tasks.get(task_index).cloned();
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
                    .config
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
        let command = render_command(&task.command, &values, task.inputs.as_ref())
            .map_err(|err| anyhow!("render command: {}", err))?;
        let spawned = self.manager.spawn_raw(&task, &command)?;
        self.attach_spawned(spawned)
    }

    fn attach_spawned(&mut self, spawned: SpawnedInstance) -> Result<()> {
        self.next_passthrough = Some(PassthroughRequest {
            instance_id: spawned.info.id.clone(),
            task_name: spawned.info.task_name.clone(),
            master: spawned.master,
            writer: spawned.writer,
            ui_config: self.config.ui.clone().unwrap_or_default(),
            key_config: self.key_bindings.clone(),
        });
        Ok(())
    }

    fn attach_instance(&mut self, instance_id: &str) -> Result<()> {
        let result = self.manager.take_master(instance_id)?;
        if let Some((master, writer)) = result {
            let task_name = self
                .instances
                .iter()
                .find(|info| info.id == instance_id)
                .map(|info| info.task_name.clone())
                .unwrap_or_else(|| instance_id.to_string());
            self.next_passthrough = Some(PassthroughRequest {
                instance_id: instance_id.to_string(),
                task_name,
                master,
                writer,
                ui_config: self.config.ui.clone().unwrap_or_default(),
                key_config: self.key_bindings.clone(),
            });
        } else {
            let status = self.manager.get_status(instance_id).ok().flatten();
            if status.is_none() {
                self.last_error = Some("Instance not found".to_string());
            } else {
                self.last_error = Some("Instance is already attached".to_string());
            }
        }
        Ok(())
    }

    fn take_passthrough(&mut self) -> Option<PassthroughRequest> {
        self.next_passthrough.take()
    }

    fn task_by_id(&self, task_id: &str) -> Option<&Task> {
        self.config.tasks.iter().find(|task| task.id == task_id)
    }
}

#[derive(Clone)]
enum Entry {
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
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn std::io::Write + Send>,
    ui_config: UiConfig,
    key_config: KeyBindings,
}

enum PassthroughOutcome {
    BackToList,
}

fn run_passthrough(mut request: PassthroughRequest, manager: &SessionManager) -> Result<PassthroughOutcome> {
    let outcome = run_passthrough_inner(&mut request, manager);
    // Always return the master to the session manager, even if inner failed
    if let Err(e) = manager.return_master(&request.instance_id, request.master, request.writer) {
        // If we can't return the master, it's likely the instance was removed or something severe happened.
        // We log it but don't overwrite the original error if there was one.
        eprintln!("Failed to return master: {}", e);
    }
    outcome
}

fn run_passthrough_inner(request: &mut PassthroughRequest, manager: &SessionManager) -> Result<PassthroughOutcome> {
    let mut stdout = io::stdout();
    enable_raw_mode()?;

    let size = crossterm::terminal::size()?;
    set_scroll_region(size.1)?;

    execute!(stdout, Clear(ClearType::All), MoveTo(0, 0), Show)?;

    // Print Task Header
    let header = format!(
        "\r\n\x1b[1;36m=== CmdHub Task Session ===\x1b[0m\r\n\
         Task: \x1b[1m{}\x1b[0m (ID: {})\r\n\
         Started: {}\r\n\
         \x1b[1;36m===========================\x1b[0m\r\n\r\n",
         request.task_name,
         request.instance_id,
         format_start_time(manager, &request.instance_id)
    );
    stdout.write_all(header.as_bytes())?;
    
    // Draw initial status bar
    draw_status_bar(&mut stdout, size.0, size.1, request, manager, false)?;
    
    // Move cursor back to top-left for output
    // But we printed a header, so we shouldn't move to (0,0) blindly if we want to keep the header visible
    // Wait, the buffer replay will just write from current cursor position?
    // If we move to 0,0, we overwrite the header.
    // We should NOT move to 0,0 after printing header.
    // But we need to make sure we are not at the bottom (status bar).
    // The header printing ends with newlines, so cursor is below header.
    // Just ensure we are not overwriting status bar.
    
    // Remove: execute!(stdout, MoveTo(0, 0))?; 

    let replay = manager.buffer_snapshot(&request.instance_id)?;
    if !replay.is_empty() {
        stdout.write_all(&replay)?;
        stdout.flush()?;
    }

    // Use existing writer instead of taking it
    // let mut writer = request.master.take_writer()?; 
    #[cfg(unix)]
    {
        if let Some(fd) = request.master.as_raw_fd() {
            // Make reads non-blocking so we can stop the reader thread on detach.
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFL);
                if flags != -1 {
                    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
                }
            }
        }
    }

    let mut reader = request.master.try_clone_reader()?;
    let stop = Arc::new(Mutex::new(false));
    let stop_reader = Arc::clone(&stop);
    let manager_clone = manager.clone();
    let instance_id = request.instance_id.clone();

    let reader_handle = thread::spawn(move || {
        let mut buf = [0u8; 8192];
        let mut out = io::stdout();
        loop {
            let stopped = stop_reader.lock().map(|lock| *lock).unwrap_or(true);
            if stopped {
                break;
            }
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = out.write_all(&buf[..n]);
                    let _ = out.flush();
                    let _ = manager_clone.append_output(&instance_id, &buf[..n]);
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if stop_reader.lock().map(|lock| *lock).unwrap_or(true) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => break,
            }
        }
    });

    let mut command_mode = false;
    let mut last_status_running = true;

    let exit = loop {
        let is_running = matches!(manager.get_status(&request.instance_id), Ok(Some(InstanceStatus::Running)));
        // status_label logic moved to draw_status_bar

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let toggle_key = request.key_config.task_running.get("toggle_command_mode").map(|s| s.as_str()).unwrap_or("ctrl+p");
                    
                    if matches_key(&key, toggle_key) {
                        if command_mode {
                            command_mode = false;
                        } else {
                            command_mode = true;
                        }
                        // Redraw status bar immediately
                        let size = crossterm::terminal::size()?;
                        draw_status_bar(&mut stdout, size.0, size.1, request, manager, command_mode)?;
                        continue;
                    }

                    if command_mode {
                        let quit_key = request
                            .key_config
                            .task_running
                            .get("quit_task")
                            .map(|s| s.as_str())
                            .unwrap_or("q");
                        let back_key = request
                            .key_config
                            .task_running
                            .get("back_to_list")
                            .map(|s| s.as_str())
                            .unwrap_or("b");
                        let kill_key = request
                            .key_config
                            .task_running
                            .get("kill_task")
                            .map(|s| s.as_str())
                            .unwrap_or("k");

                        if matches_key(&key, quit_key) || matches_key(&key, back_key) {
                            break PassthroughOutcome::BackToList;
                        } else if matches_key(&key, kill_key) {
                            let _ = manager.kill_and_remove(&request.instance_id);
                            break PassthroughOutcome::BackToList;
                        }
                    } else if let Some(bytes) = key_event_to_bytes(&key) {
                        let _ = request.writer.write_all(&bytes);
                        let _ = request.writer.flush();
                    }
                    
                    let size = crossterm::terminal::size()?;
                    draw_status_bar(&mut stdout, size.0, size.1, request, manager, command_mode)?;
                }
                Event::Resize(cols, rows) => {
                    if is_running {
                        let _ = request.master.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        });
                    }
                    set_scroll_region(rows)?;
                    draw_status_bar(&mut stdout, cols, rows, request, manager, command_mode)?;
                }
                _ => {}
            }
        } else if last_status_running != is_running {
             let size = crossterm::terminal::size()?;
             draw_status_bar(&mut stdout, size.0, size.1, request, manager, command_mode)?;
        }
        last_status_running = is_running;
    };

    if let Ok(mut lock) = stop.lock() {
        *lock = true;
    }
    let _ = reader_handle.join();
    reset_scroll_region(&mut stdout)?;
    disable_raw_mode()?;
    Ok(exit)
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
        InstanceStatus::Exited(code) => (format!("Exited({})", code), Color::Gray),
        InstanceStatus::Error(_) => ("Error".to_string(), Color::Red),
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

fn format_start_time(manager: &SessionManager, instance_id: &str) -> String {
    if let Ok(infos) = manager.list_instances() {
        if let Some(info) = infos.iter().find(|i| i.id == instance_id) {
            // Simple formatting since chrono is not in dependencies, just use raw timestamp or relative time if preferred
            // But let's stick to format_duration logic for simplicity or just a simple string
            return format!("{}s ago", format_duration(info.started_at, None));
        }
    }
    "Unknown".to_string()
}

fn instance_status_details(manager: &SessionManager, instance_id: &str) -> (String, String, String, String) {
    let mut title = String::new();
    let mut pid = "-".to_string();
    let mut status_str = "Unknown".to_string();
    let mut status_color = "0"; // Default

    if let Ok(infos) = manager.list_instances() {
        if let Some(info) = infos.iter().find(|i| i.id == instance_id) {
             title = info.title.clone().unwrap_or_default();
             pid = info.child_pid.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
             match &info.status {
                InstanceStatus::Running => {
                    status_str = "Running".to_string();
                    status_color = "32"; // Green
                }
                InstanceStatus::Exited(code) => {
                    status_str = format!("Exited({})", code);
                    status_color = "90"; // Dark Gray
                }
                InstanceStatus::Error(_) => {
                    status_str = "Error".to_string();
                    status_color = "31"; // Red
                }
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
    manager: &SessionManager,
    command_mode: bool,
) -> Result<()> {
    let (title, pid, status, _status_color) = instance_status_details(manager, &request.instance_id);
    
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
