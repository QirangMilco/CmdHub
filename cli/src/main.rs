use anyhow::Result;
use cmdhub_core::config::load_config;
use cmdhub_core::pty::PtySession;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio::sync::mpsc;

struct App {
    tasks: Vec<cmdhub_core::models::Task>,
    list_state: ListState,
    logs: String,
    rx: Option<mpsc::Receiver<Vec<u8>>>,
    session: Option<PtySession>,
    scroll: u16,
}

impl App {
    fn new(tasks: Vec<cmdhub_core::models::Task>) -> App {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        App {
            tasks,
            list_state,
            logs: String::new(),
            rx: None,
            session: None,
            scroll: 0,
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load config
    let config = load_config("config.toml").await?;
    let app = App::new(config.tasks);

    // Run app
    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Some(ref mut rx) = app.rx {
            while let Ok(data) = rx.try_recv() {
                app.logs.push_str(&String::from_utf8_lossy(&data).replace("\r\n", "\n").replace('\r', "\n"));
                
                // Performance: Limit logs buffer to last 1000 lines
                let lines: Vec<&str> = app.logs.lines().collect();
                if lines.len() > 1000 {
                    app.logs = lines[lines.len() - 1000..].join("\n");
                }

                // Auto scroll to bottom
                let line_count = app.logs.lines().count() as u16;
                let height = terminal.size()?.height.saturating_sub(2); // subtract borders
                if app.scroll >= line_count.saturating_sub(height + 1) {
                    if line_count > height {
                        app.scroll = line_count - height;
                    }
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down => app.next(),
                        KeyCode::Up => app.previous(),
                        KeyCode::PageDown => {
                            let line_count = app.logs.lines().count() as u16;
                            // Allow scrolling beyond logical lines because of Wrap
                            app.scroll = app.scroll.saturating_add(5).min(line_count * 3);
                        }
                        KeyCode::PageUp => {
                            app.scroll = app.scroll.saturating_sub(5);
                        }
                        KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                            if let Some(ref mut session) = app.session {
                                session.kill()?;
                                app.logs.push_str("\n[Command Interrupted]\n");
                            }
                        }
                        KeyCode::Esc => {
                            if let Some(ref mut session) = app.session {
                                session.kill()?;
                                app.logs.push_str("\n[Command Terminated]\n");
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(index) = app.list_state.selected() {
                                let task = &app.tasks[index];
                                let (tx, rx) = mpsc::channel(100);
                                let session = PtySession::new(&task.command, task.cwd.clone())?;
                                session.run(tx).await?;
                                app.session = Some(session);
                                app.rx = Some(rx);
                                app.logs.clear();
                                app.scroll = 0;
                            }
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        MouseEventKind::ScrollUp => {
                            app.scroll = app.scroll.saturating_sub(2);
                        }
                        MouseEventKind::ScrollDown => {
                            let line_count = app.logs.lines().count() as u16;
                            app.scroll = app.scroll.saturating_add(2).min(line_count * 3);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.size());

    // Left: Tasks
    let items: Vec<ListItem> = app
        .tasks
        .iter()
        .map(|t| ListItem::new(t.name.clone()))
        .collect();
    let list = List::new(items)
        .block(Block::default().title("Tasks").borders(Borders::ALL))
        .highlight_style(ratatui::style::Style::default().add_modifier(ratatui::style::Modifier::BOLD))
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, chunks[0], &mut app.list_state);

    // Right: Logs
    f.render_widget(Clear, chunks[1]); // Clear the area to prevent artifacts
    let line_count = app.logs.lines().count();
    let height = chunks[1].height.saturating_sub(2) as usize;
    
    // tmux style indicator: [Scroll: 10/100]
    let title = if line_count > height {
        format!(
            "Logs [Line {}/{}] (Esc to kill)",
            app.scroll + 1,
            line_count
        )
    } else {
        "Logs (Esc to kill)".to_string()
    };

    let log_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
    
    let logs = Paragraph::new(app.logs.as_str())
        .block(log_block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    
    f.render_widget(logs, chunks[1]);
}
