use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, CheckStatus, ListEntry, SetupField};

// ── Colour helpers ────────────────────────────────────────────────────────

fn group_color(label: &str) -> Color {
    // Simple deterministic colour by hash
    match label.len() % 6 {
        0 => Color::Green,
        1 => Color::Cyan,
        2 => Color::Yellow,
        3 => Color::Blue,
        4 => Color::Magenta,
        _ => Color::White,
    }
}

fn color_from_str(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "green" => Color::Green,
        "red" => Color::Red,
        "blue" => Color::Blue,
        "yellow" => Color::Yellow,
        "cyan" => Color::Cyan,
        "magenta" => Color::Magenta,
        "white" => Color::White,
        _ => Color::White,
    }
}

// ── Main draw entry point ─────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
    match &app.mode.clone() {
        AppMode::Setup => draw_setup(f, app),
        AppMode::Selecting
        | AppMode::Running { .. }
        | AppMode::AwaitingManualResult { .. }
        | AppMode::Done => draw_runner(f, app),
    }
}

// ── Setup screen ──────────────────────────────────────────────────────────

pub fn draw_setup(f: &mut Frame, app: &App) {
    let area = f.area();

    // Outer layout: title / form / log / help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // title
            Constraint::Length(10), // form
            Constraint::Min(3),     // checkout log
            Constraint::Length(2),  // help
        ])
        .split(area);

    // ── Title ────────────────────────────────────────────────────────────
    let title = Paragraph::new("  AutoCheck — Setup")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    // ── Form ─────────────────────────────────────────────────────────────
    let form_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // project path
            Constraint::Length(3), // branch
            Constraint::Length(3), // config
            Constraint::Min(1),    // error
        ])
        .split(chunks[1]);

    // Project path
    let path_focused = app.setup_focus == SetupField::ProjectPath;
    let path_style = if path_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let path_block = Block::default()
        .borders(Borders::ALL)
        .border_style(path_style)
        .title(Span::styled(
            " Project path (required) ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
    let path_text = format!("{}", app.setup_project_path);
    let cursor_suffix = if path_focused { "█" } else { "" };
    let path_para = Paragraph::new(format!("{path_text}{cursor_suffix}")).block(path_block);
    f.render_widget(path_para, form_rows[0]);

    // Branch
    let branch_focused = app.setup_focus == SetupField::Branch;
    let branch_style = if branch_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let detected = if app.current_branch.is_empty() {
        String::new()
    } else {
        format!("  [detected: {}]", app.current_branch)
    };
    let branch_block = Block::default()
        .borders(Borders::ALL)
        .border_style(branch_style)
        .title(Span::styled(
            format!(" Branch or PR number (optional){detected} "),
            Style::default().add_modifier(Modifier::BOLD),
        ));
    let cursor_suffix = if branch_focused { "█" } else { "" };
    let branch_para =
        Paragraph::new(format!("{}{cursor_suffix}", app.setup_branch)).block(branch_block);
    f.render_widget(branch_para, form_rows[1]);

    // Config selector
    let config_focused = app.setup_focus == SetupField::Config;
    let config_style = if config_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let config_display = if app.config_paths.is_empty() {
        "  No config files found — place a .json config in the current directory".to_string()
    } else {
        let path = &app.config_paths[app.config_idx];
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        format!(
            "  {} ({}/{}) ",
            name,
            app.config_idx + 1,
            app.config_paths.len()
        )
    };
    let config_block = Block::default()
        .borders(Borders::ALL)
        .border_style(config_style)
        .title(Span::styled(
            " Checks config  [← → to cycle | Space to list] ",
            Style::default().add_modifier(Modifier::BOLD),
        ));
    let config_para = Paragraph::new(config_display).block(config_block);
    f.render_widget(config_para, form_rows[2]);

    // Error area
    if let Some(err) = &app.setup_error {
        let err_para = Paragraph::new(format!(" ⚠  {err}"))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        f.render_widget(err_para, form_rows[3]);
    }

    // ── Config dropdown ──────────────────────────────────────────────────
    if app.config_dropdown_open && !app.config_paths.is_empty() {
        let items: Vec<ListItem> = app
            .config_paths
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let style = if i == app.config_idx {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(name).style(style)
            })
            .collect();
        let dropdown = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select config "),
        );
        // Overlay on top of the config row
        let dropdown_area = Rect {
            x: form_rows[2].x + 2,
            y: form_rows[2].y + 3,
            width: form_rows[2].width.saturating_sub(4),
            height: (app.config_paths.len() as u16 + 2).min(area.height / 2),
        };
        f.render_widget(ratatui::widgets::Clear, dropdown_area);
        f.render_widget(dropdown, dropdown_area);
    }

    // ── Checkout log ─────────────────────────────────────────────────────
    if !app.setup_log.is_empty() {
        let log_text: Vec<Line> = app
            .setup_log
            .iter()
            .map(|l| Line::from(l.clone()))
            .collect();
        let log_para = Paragraph::new(log_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Checkout output "),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(log_para, chunks[2]);
    }

    // ── Help bar ─────────────────────────────────────────────────────────
    let help = Paragraph::new(
        " Tab/↓ next field  ↑ prev  ←/→ cycle config  Space list  Enter confirm  q quit",
    )
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}

// ── Runner screen ─────────────────────────────────────────────────────────

pub fn draw_runner(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Outer: header / body / footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(1),    // body
            Constraint::Length(2), // footer
        ])
        .split(area);

    // ── Header ───────────────────────────────────────────────────────────
    draw_header(f, app, outer[0]);

    // ── Body: left (checks) / right (output) ─────────────────────────────
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[1]);

    // Store list area for mouse hit-testing
    app.list_area = Some(body[0]);

    draw_check_list(f, app, body[0]);
    draw_output_pane(f, app, body[1]);

    // ── Footer ───────────────────────────────────────────────────────────
    draw_footer(f, app, outer[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match &app.mode {
        AppMode::Setup => "Setup",
        AppMode::Selecting => "Selecting",
        AppMode::Running { .. } => "Running",
        AppMode::AwaitingManualResult { .. } => "Awaiting verdict",
        AppMode::Done => "Done",
    };

    let cpu_color = if app.cpu_usage > 80.0 {
        Color::Red
    } else if app.cpu_usage > 50.0 {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let mem_color = if app.mem_usage > 80.0 {
        Color::Red
    } else if app.mem_usage > 60.0 {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let mouse_str = if app.mouse_capture {
        "mouse:on"
    } else {
        "mouse:off"
    };

    let header_line = Line::from(vec![
        Span::styled(
            " AutoCheck ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled(
            app.repo_root.to_string_lossy().to_string(),
            Style::default().fg(Color::White),
        ),
        Span::raw("  branch: "),
        Span::styled(
            app.current_branch.clone(),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  mode: "),
        Span::styled(mode_str, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(
            format!("CPU:{:.0}%", app.cpu_usage),
            Style::default().fg(cpu_color),
        ),
        Span::raw("  "),
        Span::styled(
            format!("MEM:{:.0}%", app.mem_usage),
            Style::default().fg(mem_color),
        ),
        Span::raw("  "),
        Span::styled(mouse_str, Style::default().fg(Color::DarkGray)),
    ]);

    let header = Paragraph::new(header_line).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(header, area);
}

fn draw_check_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .list_entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let selected_style = if i == app.list_cursor {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            match entry {
                ListEntry::Group { group_idx } => {
                    let group = &app.groups[*group_idx];
                    // Count selected checks in this group
                    let selected_in_group = app
                        .check_group_map
                        .iter()
                        .enumerate()
                        .filter(|(_, &gi)| gi == *group_idx)
                        .filter(|(ci, _)| app.selected[*ci])
                        .count();
                    let total_in_group = group.checks.len();
                    let col = group
                        .color
                        .as_deref()
                        .map(color_from_str)
                        .unwrap_or_else(|| group_color(&group.label));
                    let text = format!(
                        " ▶ {}  ({}/{} selected)",
                        group.label, selected_in_group, total_in_group
                    );
                    ListItem::new(text).style(Style::default().fg(col).add_modifier(Modifier::BOLD))
                }
                ListEntry::Check { check_idx, .. } => {
                    let check = &app.checks[*check_idx];
                    let sel = if app.selected[*check_idx] {
                        "[✓]"
                    } else {
                        "[ ]"
                    };
                    let status = &app.statuses[*check_idx];
                    let icon = status.icon();
                    let elapsed_str = app.elapsed[*check_idx]
                        .map(|d| format!(" {:.1}s", d.as_secs_f32()))
                        .unwrap_or_default();
                    let status_color = match status {
                        CheckStatus::Passed | CheckStatus::ManualPassed => Color::Green,
                        CheckStatus::Failed | CheckStatus::ManualFailed => Color::Red,
                        CheckStatus::Running => Color::Yellow,
                        CheckStatus::Skipped => Color::DarkGray,
                        CheckStatus::Pending => Color::White,
                    };
                    let text = format!("  {sel} {icon} {}{elapsed_str}", check.name);
                    ListItem::new(text).style(Style::default().fg(status_color).add_modifier(
                        if i == app.list_cursor {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        },
                    ))
                }
            }
            .style(selected_style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Checks "));
    f.render_widget(list, area);
}

fn draw_output_pane(f: &mut Frame, app: &App, area: Rect) {
    let title;
    let lines: Vec<Line> = match &app.mode {
        AppMode::Running { .. } => {
            title = " Output (live) ";
            let total = app.output_lines.len();
            let skip = app.output_scroll.min(total.saturating_sub(1));
            app.output_lines[skip..]
                .iter()
                .map(|l| Line::from(l.clone()))
                .collect()
        }
        _ => {
            // Show selected check's log or description
            if let Some(ListEntry::Check { check_idx, .. }) = app.list_entries.get(app.list_cursor)
            {
                if let Some(log) = app.check_logs.get(check_idx) {
                    title = " Check log ";
                    let total = log.len();
                    let skip = app.output_scroll.min(total.saturating_sub(1));
                    log[skip..].iter().map(|l| Line::from(l.clone())).collect()
                } else {
                    title = " Check description ";
                    vec![Line::from(app.checks[*check_idx].description.clone())]
                }
            } else {
                title = " Output ";
                app.output_lines
                    .iter()
                    .map(|l| Line::from(l.clone()))
                    .collect()
            }
        }
    };

    // AwaitingManualResult: overlay prompt
    let pane_block = Block::default().borders(Borders::ALL).title(title);

    if let AppMode::AwaitingManualResult { .. } = &app.mode {
        let mut augmented = lines;
        augmented.push(Line::from(""));
        augmented.push(Line::from(Span::styled(
            " ── Manual check: press p (pass) or f (fail) ──",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        let para = Paragraph::new(augmented)
            .block(pane_block)
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    } else {
        let para = Paragraph::new(lines)
            .block(pane_block)
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        AppMode::Setup => " Tab/↓ next  ↑ prev  Enter confirm  q quit".to_string(),
        AppMode::Selecting => {
            let (p, fail, s) = app.summary_counts();
            format!(
                " ↑↓ navigate  Space toggle  a all  n none  Enter run  q quit | staged:{}  pass:{p} fail:{fail} skip:{s}",
                app.staged_files.len()
            )
        }
        AppMode::Running { .. } => " r reset  q quit  (running…)".to_string(),
        AppMode::AwaitingManualResult { .. } => " p pass  f fail".to_string(),
        AppMode::Done => {
            let (p, fail, s) = app.summary_counts();
            format!(" Enter rerun  r reset  q quit  ↑↓ logs  PageUp/Dn scroll | pass:{p} fail:{fail} skip:{s}")
        }
    };

    let para = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    f.render_widget(para, area);
}
