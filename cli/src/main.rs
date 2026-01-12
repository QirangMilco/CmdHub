use anyhow::Result;
use cmdhub_core::config::load_config;
use cmdhub_core::pty::PtySession;
use cmdhub_core::template::render_command;
use crossterm::{
    cursor::MoveTo,
    event::{self, DisableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear as TermClear, ClearType},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use std::io::Write;
use tokio::sync::mpsc;

#[derive(PartialEq, Copy, Clone)]
enum View {
    Selection,
    Terminal,
    Inputs,
}

struct RunningTask {
    session: PtySession,
    logs: String,
    scroll: u16,
}

struct App {
    tasks: Vec<cmdhub_core::models::Task>,
    list_state: ListState,
    running_tasks: HashMap<usize, RunningTask>,
    active_task_index: Option<usize>,
    current_view: View,
    use_native_scrollback: bool,
    log_rx: mpsc::Receiver<(usize, Vec<u8>)>,
    log_tx: mpsc::Sender<(usize, Vec<u8>)>,
    input_state: Option<InputState>,
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

impl App {
    fn new(tasks: Vec<cmdhub_core::models::Task>) -> App {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let (tx, rx) = mpsc::channel(1000);
        App {
            tasks,
            list_state,
            running_tasks: HashMap::new(),
            active_task_index: None,
            current_view: View::Selection,
            use_native_scrollback: true,
            log_rx: rx,
            log_tx: tx,
            input_state: None,
        }
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.tasks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tasks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    async fn start_task(&mut self, index: usize) -> Result<()> {
        if !self.running_tasks.contains_key(&index) {
            let task = &self.tasks[index];
            let rendered = render_command(&task.command, &HashMap::new(), task.inputs.as_ref())?;
            let env_clear = task.env_clear.unwrap_or(false);
            let session =
                PtySession::new(&rendered, task.cwd.clone(), task.env.clone(), env_clear)?;

            let tx = self.log_tx.clone();
            let task_index = index;

            let (session_tx, mut session_rx) = mpsc::channel::<Vec<u8>>(100);
            session.run(session_tx).await?;

            tokio::spawn(async move {
                while let Some(data) = session_rx.recv().await {
                    if tx.send((task_index, data)).await.is_err() {
                        break;
                    }
                }
            });

            self.running_tasks.insert(
                index,
                RunningTask {
                    session,
                    logs: String::new(),
                    scroll: 0,
                },
            );
        }

        self.active_task_index = Some(index);
        self.current_view = View::Terminal;
        Ok(())
    }

    async fn start_task_with_inputs(
        &mut self,
        index: usize,
        values: &HashMap<String, String>,
    ) -> Result<()> {
        if !self.running_tasks.contains_key(&index) {
            let task = &self.tasks[index];
            let rendered = render_command(&task.command, values, task.inputs.as_ref())?;
            let env_clear = task.env_clear.unwrap_or(false);
            let session =
                PtySession::new(&rendered, task.cwd.clone(), task.env.clone(), env_clear)?;

            let tx = self.log_tx.clone();
            let task_index = index;

            let (session_tx, mut session_rx) = mpsc::channel::<Vec<u8>>(100);
            session.run(session_tx).await?;

            tokio::spawn(async move {
                while let Some(data) = session_rx.recv().await {
                    if tx.send((task_index, data)).await.is_err() {
                        break;
                    }
                }
            });

            self.running_tasks.insert(
                index,
                RunningTask {
                    session,
                    logs: String::new(),
                    scroll: 0,
                },
            );
        }

        self.active_task_index = Some(index);
        self.current_view = View::Terminal;
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
        if let Some(index) = self.active_task_index {
            if let Some(mut task) = self.running_tasks.remove(&index) {
                task.session.kill()?;
            }
            self.current_view = View::Selection;
        }
        Ok(())
    }

    fn kill_all_tasks(&mut self) -> Result<()> {
        for (_, mut task) in self.running_tasks.drain() {
            let _ = task.session.kill();
        }
        Ok(())
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.kill_all_tasks();
    }
}

fn sanitize_log_chunk(data: &[u8]) -> String {
    let cleaned = strip_ansi_escapes::strip(data);
    String::from_utf8_lossy(&cleaned)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load config
    let config = load_config("config.toml").await?;
    let app = App::new(config.tasks);

    // Setup terminal
    let mut stdout = io::stdout();
    execute!(stdout, TermClear(ClearType::All), MoveTo(0, 0))?;
    enable_raw_mode()?;
    execute!(stdout, DisableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    terminal.show_cursor()?;
    println!("--- CmdHub exited ---");

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend + Write>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        while let Ok((idx, data)) = app.log_rx.try_recv() {
            if let Some(task) = app.running_tasks.get_mut(&idx) {
                let normalized = sanitize_log_chunk(&data);
                task.logs.push_str(&normalized);

                // Performance: Limit logs buffer to last 2000 lines
                let lines: Vec<&str> = task.logs.lines().collect();
                if lines.len() > 2000 {
                    task.logs = lines[lines.len() - 2000..].join("\n");
                }

                // Auto scroll to bottom if we are near the bottom
                let line_count = task.logs.lines().count() as u16;
                let height = terminal.size()?.height.saturating_sub(2);
                let max_scroll = line_count.saturating_sub(height);
                if app.use_native_scrollback {
                    task.scroll = max_scroll;
                } else if task.scroll >= max_scroll {
                    task.scroll = max_scroll;
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => match app.current_view {
                    View::Selection => match key.code {
                        KeyCode::Char('q') => {
                            app.kill_all_tasks()?;
                            return Ok(());
                        }
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        KeyCode::Enter => {
                            if let Some(index) = app.list_state.selected() {
                                if app.tasks[index]
                                    .inputs
                                    .as_ref()
                                    .map_or(false, |v| !v.is_empty())
                                {
                                    app.prepare_inputs(index);
                                } else {
                                    app.start_task(index).await?;
                                }
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
                            if let Some(index) = app.active_task_index {
                                if let Some(task) = app.running_tasks.get_mut(&index) {
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
                            if let Some(index) = app.active_task_index {
                                if let Some(task) = app.running_tasks.get_mut(&index) {
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
                },
                Event::Mouse(mouse_event) => {
                    if app.use_native_scrollback {
                        continue;
                    }
                    if app.current_view == View::Terminal {
                        if let Some(index) = app.active_task_index {
                            if let Some(task) = app.running_tasks.get_mut(&index) {
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
                .tasks
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let status = if app.running_tasks.contains_key(&i) {
                        " [Running]"
                    } else {
                        ""
                    };
                    ListItem::new(format!("{}{}", t.name, status))
                })
                .collect();
            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Select Command (Enter to run/view, q to exit)")
                        .borders(Borders::ALL),
                )
                .highlight_style(
                    ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            f.render_stateful_widget(list, area, &mut app.list_state);
        }
        View::Terminal => {
            if let Some(index) = app.active_task_index {
                if let Some(task) = app.running_tasks.get(&index) {
                    let line_count = task.logs.lines().count();
                    let height = area.height.saturating_sub(2) as usize;

                    let title = if line_count > height {
                        let top = task.scroll as usize + 1;
                        let bottom = (task.scroll as usize + height).min(line_count);
                        let percent = if line_count == 0 {
                            0
                        } else {
                            (bottom * 100) / line_count
                        };
                        format!(
                            "Logs: {} [Lines {}-{} / {} ({}%)] (Esc: Kill, q/Backspace: Back)",
                            app.tasks[index].name, top, bottom, line_count, percent
                        )
                    } else {
                        format!(
                            "Logs: {} (Esc: Kill, q/Backspace: Back)",
                            app.tasks[index].name
                        )
                    };

                    let log_block = Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(
                            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
                        );

                    let logs = Paragraph::new(task.logs.as_str())
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
                "Task Inputs (Enter: Run, Esc: Cancel, Tab/Up/Down: Select, Left/Right: Option)",
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
