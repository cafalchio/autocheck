mod app;
mod checks;
mod config;
mod draw;

use std::io;
use std::time::Duration;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, AppMode};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app);

    // Always restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }
    Ok(())
}

fn run_app<B: ratatui::backend::Backend + std::io::Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        // Poll log channel if running or awaiting
        match app.mode {
            AppMode::Running { .. } | AppMode::AwaitingManualResult { .. } => {
                app.poll_log();
            }
            _ => {}
        }

        // Refresh system stats
        app.refresh_system_stats();

        // Draw
        terminal.draw(|f| draw::draw(f, app))?;

        // Mouse capture toggle needs to be applied to the terminal
        // (handled below when 'm' is pressed)

        // Poll for events with a short timeout so the UI stays live
        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    let quit = handle_key(app, key.code, key.modifiers);
                    if quit {
                        // Cancel any running check before exit
                        app.cancel_run();
                        return Ok(());
                    }
                    // Handle mouse capture toggle
                    if key.code == KeyCode::Char('m') {
                        if app.mouse_capture {
                            execute!(terminal.backend_mut(), EnableMouseCapture)?;
                        } else {
                            execute!(terminal.backend_mut(), DisableMouseCapture)?;
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    if app.mouse_capture {
                        match mouse.kind {
                            MouseEventKind::Down(_) => {
                                app.handle_click(mouse.column, mouse.row);
                            }
                            MouseEventKind::ScrollUp => {
                                app.scroll_up();
                            }
                            MouseEventKind::ScrollDown => {
                                app.scroll_down();
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Handle a key event. Returns true if the app should quit.
fn handle_key(app: &mut App, code: KeyCode, _mods: KeyModifiers) -> bool {
    match &app.mode.clone() {
        // ── Setup ──────────────────────────────────────────────────────────
        AppMode::Setup => {
            if app.config_dropdown_open {
                match code {
                    KeyCode::Up => app.prev_config(),
                    KeyCode::Down => app.next_config(),
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => app.close_dropdown(),
                    _ => {}
                }
                return false;
            }

            match code {
                KeyCode::Char('q') => return true,
                KeyCode::Left => {
                    if app.setup_focus == app::SetupField::Config {
                        app.prev_config();
                    }
                }
                KeyCode::Right => {
                    if app.setup_focus == app::SetupField::Config {
                        app.next_config();
                    }
                }
                KeyCode::Tab | KeyCode::Down => app.setup_focus_next(),
                KeyCode::BackTab | KeyCode::Up => app.setup_focus_prev(),
                KeyCode::Char(' ') => {
                    if app.setup_focus == app::SetupField::Config {
                        app.toggle_dropdown();
                    }
                }
                KeyCode::Esc => app.close_dropdown(),
                KeyCode::Enter => app.confirm_setup(),
                KeyCode::Backspace => app.setup_backspace(),
                KeyCode::Char(c) => app.setup_type_char(c),
                _ => {}
            }
        }

        // ── Selecting ──────────────────────────────────────────────────────
        AppMode::Selecting => match code {
            KeyCode::Char('q') => return true,
            KeyCode::Up => app.list_up(),
            KeyCode::Down => app.list_down(),
            KeyCode::Char(' ') => app.toggle_current(),
            KeyCode::Char('a') => app.select_all(),
            KeyCode::Char('n') => app.select_none(),
            KeyCode::Enter => app.start_running(),
            KeyCode::Char('m') => app.toggle_mouse_capture(),
            KeyCode::Home => app.scroll_to_top(),
            KeyCode::End => app.scroll_to_bottom(),
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),
            _ => {}
        },

        // ── Running ────────────────────────────────────────────────────────
        AppMode::Running { .. } => match code {
            KeyCode::Char('q') => {
                app.cancel_run();
                return true;
            }
            KeyCode::Char('r') => app.reset_to_selecting(),
            KeyCode::Home => app.scroll_to_top(),
            KeyCode::End => app.scroll_to_bottom(),
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),
            _ => {}
        },

        // ── Awaiting manual result ─────────────────────────────────────────
        AppMode::AwaitingManualResult { .. } => match code {
            KeyCode::Char('p') => app.manual_pass(),
            KeyCode::Char('f') => app.manual_fail(),
            KeyCode::Char('q') => {
                app.cancel_run();
                return true;
            }
            _ => {}
        },

        // ── Done ───────────────────────────────────────────────────────────
        AppMode::Done => match code {
            KeyCode::Char('q') => return true,
            KeyCode::Enter => app.rerun(),
            KeyCode::Char('r') => app.reset_to_selecting(),
            KeyCode::Char('R') => app.reset_audited(),
            KeyCode::Char('x') => app.reset_current_audited(),
            KeyCode::Up => app.list_up(),
            KeyCode::Down => app.list_down(),
            KeyCode::Home => app.scroll_to_top(),
            KeyCode::End => app.scroll_to_bottom(),
            KeyCode::PageUp => app.page_up(),
            KeyCode::PageDown => app.page_down(),
            KeyCode::Char('m') => app.toggle_mouse_capture(),
            _ => {}
        },
    }

    false
}
