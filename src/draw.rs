use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, CheckStatus, ListEntry, SetupField};

// ── Theme ─────────────────────────────────────────────────────────────────
mod theme {
    use ratatui::style::Color;
    // Calm dark palette with a few purposeful accents.
    pub const APP_BG: Color = Color::Rgb(8, 10, 18);
    pub const PANEL_BG: Color = Color::Rgb(14, 17, 28);
    pub const PANEL_BG_SOFT: Color = Color::Rgb(18, 22, 34);
    pub const BORDER_FOCUSED: Color = Color::Rgb(255, 215, 0); // Gold
    pub const BORDER_UNFOCUSED: Color = Color::Rgb(54, 61, 82);
    pub const TEXT_PRIMARY: Color = Color::Rgb(224, 228, 238);
    pub const TEXT_SECONDARY: Color = Color::Rgb(148, 156, 178);
    pub const TEXT_MUTED: Color = Color::Rgb(96, 104, 126);
    pub const ACCENT: Color = Color::Rgb(100, 200, 255);
    pub const STATUS_PASS: Color = Color::Rgb(50, 205, 50); // Lime green
    pub const STATUS_FAIL: Color = Color::Rgb(255, 100, 100); // Light red
    pub const STATUS_RUNNING: Color = Color::Rgb(255, 165, 0); // Orange
    pub const STATUS_SKIP: Color = Color::Rgb(128, 128, 128);
    pub const STATUS_PENDING: Color = Color::Rgb(160, 160, 180);
    pub const CURSOR_BG: Color = Color::Rgb(50, 60, 100);
    pub const CURSOR_FG: Color = Color::Rgb(255, 255, 255);
    pub const ERROR_FG: Color = Color::Rgb(255, 100, 100);
    pub const HEADER_TITLE: Color = Color::Rgb(100, 200, 255);
    pub const HEADER_BRANCH: Color = Color::Rgb(144, 238, 144);
    pub const HELP_TEXT: Color = Color::Rgb(120, 120, 140);
    // Enhanced row backgrounds with subtle gradients
    pub const ROW_PASS_BG: Color = Color::Rgb(0, 50, 20);
    pub const ROW_FAIL_BG: Color = Color::Rgb(10, 35, 70);
    pub const ROW_RUNNING_BG: Color = Color::Rgb(50, 45, 0);
    pub const ROW_SKIP_BG: Color = Color::Rgb(30, 30, 35);
    // Progress bar colors
    pub const PROGRESS_BG: Color = Color::Rgb(40, 40, 50);
    pub const PROGRESS_FG: Color = Color::Rgb(100, 200, 255);
    // Summary panel colors
    pub const SUMMARY_BG: Color = Color::Rgb(20, 25, 35);
}

// ── Status row style ──────────────────────────────────────────────────────

fn status_row_style(status: &CheckStatus, is_cursor: bool) -> Style {
    if is_cursor {
        return Style::default().bg(theme::CURSOR_BG).fg(theme::CURSOR_FG);
    }
    match status {
        CheckStatus::Passed | CheckStatus::ManualPassed => Style::default()
            .bg(theme::ROW_PASS_BG)
            .fg(theme::STATUS_PASS),
        CheckStatus::Failed | CheckStatus::ManualFailed => Style::default()
            .bg(theme::ROW_FAIL_BG)
            .fg(theme::STATUS_FAIL),
        CheckStatus::Running => Style::default()
            .bg(theme::ROW_RUNNING_BG)
            .fg(theme::STATUS_RUNNING),
        CheckStatus::Skipped => Style::default()
            .bg(theme::ROW_SKIP_BG)
            .fg(theme::STATUS_SKIP),
        CheckStatus::Pending => Style::default().fg(theme::STATUS_PENDING),
    }
}

// ── Spinner animation ─────────────────────────────────────────────────────
fn spinner_frame() -> &'static str {
    use std::time::{SystemTime, UNIX_EPOCH};
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let idx = (millis / 80) % frames.len() as u128;
    frames[idx as usize]
}

// ── Progress bar ──────────────────────────────────────────────────────────
fn render_progress_bar(width: usize, progress: f32) -> String {
    let filled = ((width as f32 * progress).round() as usize).min(width);
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
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

    // Fill background
    f.render_widget(
        Block::default().style(Style::default().bg(theme::APP_BG)),
        area,
    );

    // Centre horizontally inside the border: max 80 columns
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Max(80),
            Constraint::Fill(1),
        ])
        .split(area);
    let area = horiz[1];

    let setup_card = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_UNFOCUSED))
        .style(Style::default().bg(theme::PANEL_BG));
    let card_area = setup_card.inner(area);
    f.render_widget(setup_card, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(0),
        ])
        .split(card_area);

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "✦ AutoCheck",
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  setup", Style::default().fg(theme::TEXT_SECONDARY)),
        ]),
        Line::from(Span::styled(
            "Choose the repository and checks configuration for this run.",
            Style::default().fg(theme::TEXT_MUTED),
        )),
    ])
    .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    draw_setup_run_tab(f, app, chunks[1]);
}

fn setup_input_block(title: &str, focused: bool) -> Block<'_> {
    let border_color = if focused {
        theme::BORDER_FOCUSED
    } else {
        theme::BORDER_UNFOCUSED
    };
    let title_color = if focused {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme::PANEL_BG_SOFT))
        .title(Span::styled(
            title.to_string(),
            Style::default().fg(title_color),
        ))
}

fn draw_setup_run_tab(f: &mut Frame, app: &App, area: Rect) {
    // Outer layout: form / log / help
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(15), // form
            Constraint::Min(3),     // checkout log
            Constraint::Length(2),  // help
        ])
        .split(area);

    // ── Form ─────────────────────────────────────────────────────────────
    let form_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // intro
            Constraint::Length(1), // spacing
            Constraint::Length(3), // project path
            Constraint::Length(1), // spacing
            Constraint::Length(3), // branch
            Constraint::Length(1), // spacing
            Constraint::Length(3), // config
            Constraint::Length(1), // spacing
            Constraint::Min(1),    // error
        ])
        .split(chunks[0]);

    let intro = Paragraph::new(Line::from(vec![
        Span::styled(
            "1 ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Configure run target",
            Style::default()
                .fg(theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ·  Enter confirms when ready",
            Style::default().fg(theme::TEXT_MUTED),
        ),
    ]));
    f.render_widget(intro, form_rows[0]);

    // Project path
    let path_focused = app.setup_focus == SetupField::ProjectPath;
    let path_block = setup_input_block(" Project path (required) ", path_focused);
    let path_value = if app.setup_project_path.is_empty() {
        "./path/to/project"
    } else {
        &app.setup_project_path
    };
    let path_text = format!("  {}", path_value);
    let cursor_suffix = if path_focused { "█" } else { "" };
    let path_color = if app.setup_project_path.is_empty() {
        theme::TEXT_MUTED
    } else {
        theme::TEXT_PRIMARY
    };
    let path_para = Paragraph::new(format!("{path_text}{cursor_suffix}"))
        .style(Style::default().fg(path_color))
        .block(path_block);
    f.render_widget(path_para, form_rows[2]);

    // Branch
    let branch_focused = app.setup_focus == SetupField::Branch;
    let detected = if app.current_branch.is_empty() {
        String::new()
    } else {
        format!("  [detected: {}]", app.current_branch)
    };
    let branch_title = format!(" Branch or PR number (optional){detected} ");
    let branch_block = setup_input_block(&branch_title, branch_focused);
    let cursor_suffix = if branch_focused { "█" } else { "" };
    let branch_value = if app.setup_branch.is_empty() {
        "leave empty to keep current branch"
    } else {
        &app.setup_branch
    };
    let branch_color = if app.setup_branch.is_empty() {
        theme::TEXT_MUTED
    } else {
        theme::TEXT_PRIMARY
    };
    let branch_para = Paragraph::new(format!("  {branch_value}{cursor_suffix}"))
        .style(Style::default().fg(branch_color))
        .block(branch_block);
    f.render_widget(branch_para, form_rows[4]);

    // Config selector
    let config_focused = app.setup_focus == SetupField::Config;
    let config_display = if app.config_paths.is_empty() {
        "  ⚠  No config files found — place a .json config in the current directory".to_string()
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
    let config_block =
        setup_input_block(" Checks config  [← → cycle | Space list] ", config_focused);
    let config_para = Paragraph::new(config_display)
        .style(Style::default().fg(theme::TEXT_PRIMARY))
        .block(config_block);
    f.render_widget(config_para, form_rows[6]);

    // Error area
    if let Some(err) = &app.setup_error {
        let err_para =
            Paragraph::new(format!(" ⚠  {err}")).style(Style::default().fg(theme::ERROR_FG));
        f.render_widget(err_para, form_rows[8]);
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
                    Style::default().fg(Color::White)
                };
                ListItem::new(format!(" {name}")).style(style)
            })
            .collect();
        let dropdown = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Select config "),
        );
        // Overlay on top of the config row
        let dropdown_area = Rect {
            x: form_rows[6].x + 2,
            y: form_rows[6].y + 3,
            width: form_rows[6].width.saturating_sub(4),
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
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Rgb(80, 80, 100)))
                    .title(" Checkout output "),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(log_para, chunks[1]);
    }

    // ── Help bar ─────────────────────────────────────────────────────────
    let help_full =
        " Tab/↓ next field  ↑ prev  ←/→ cycle config  Space list  Enter confirm  q quit";
    let help_trimmed = &help_full[..help_full.len().min((area.width as usize).saturating_sub(1))];
    let help = Paragraph::new(help_trimmed)
        .block(Block::default())
        .style(Style::default().fg(theme::HELP_TEXT));
    f.render_widget(help, chunks[2]);
}


// ── Runner screen ─────────────────────────────────────────────────────────

pub fn draw_runner(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Outer: header / summary / body / footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(4), // summary panel
            Constraint::Min(1),    // body
            Constraint::Length(2), // footer
        ])
        .split(area);

    // ── Header ───────────────────────────────────────────────────────────
    draw_header(f, app, outer[0]);

    // ── Summary Panel ────────────────────────────────────────────────────
    draw_summary_panel(f, app, outer[1]);

    // ── Body: left (checks) / right (output) ─────────────────────────────
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[2]);

    // Store list area for mouse hit-testing
    app.list_area = Some(body[0]);

    draw_check_list(f, app, body[0]);
    draw_output_pane(f, app, body[1]);

    // ── Footer ───────────────────────────────────────────────────────────
    draw_footer(f, app, outer[3]);
}

fn draw_summary_panel(f: &mut Frame, app: &App, area: Rect) {
    let (passed, failed, skipped) = app.summary_counts();
    let total = app.checks.len();
    let selected = app.selected.iter().filter(|&&s| s).count();

    // Calculate progress
    let completed = passed + failed + skipped;
    let progress = if total > 0 {
        completed as f32 / total as f32
    } else {
        0.0
    };

    // Progress bar (use 30 chars width)
    let bar_width = 30;
    let progress_bar = render_progress_bar(bar_width, progress);

    // Running checks count
    let running = app
        .statuses
        .iter()
        .filter(|s| matches!(s, CheckStatus::Running))
        .count();

    let summary_line = Line::from(vec![
        Span::raw("  "),
        Span::styled("✔ ", Style::default().fg(theme::STATUS_PASS)),
        Span::styled(
            format!("{:>3}", passed),
            Style::default().fg(theme::STATUS_PASS),
        ),
        Span::raw("  "),
        Span::styled("✘ ", Style::default().fg(theme::STATUS_FAIL)),
        Span::styled(
            format!("{:>3}", failed),
            Style::default().fg(theme::STATUS_FAIL),
        ),
        Span::raw("  "),
        Span::styled("⊘ ", Style::default().fg(theme::STATUS_SKIP)),
        Span::styled(
            format!("{:>3}", skipped),
            Style::default().fg(theme::STATUS_SKIP),
        ),
        Span::raw("  "),
        Span::styled("◉ ", Style::default().fg(Color::Rgb(120, 180, 200))),
        Span::styled(
            format!("{:>3}", selected),
            Style::default().fg(Color::Rgb(120, 180, 200)),
        ),
        Span::raw("  │  "),
        Span::styled(
            &progress_bar,
            Style::default()
                .fg(theme::PROGRESS_FG)
                .bg(theme::PROGRESS_BG),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{}/{} ({:.0}%)", completed, total, progress * 100.0),
            Style::default().fg(Color::Rgb(180, 180, 200)),
        ),
    ]);

    let running_line = if running > 0 {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(spinner_frame(), Style::default().fg(theme::STATUS_RUNNING)),
            Span::raw(" "),
            Span::styled(
                format!(
                    "{} check{} running",
                    running,
                    if running == 1 { "" } else { "s" }
                ),
                Style::default().fg(theme::STATUS_RUNNING),
            ),
        ])
    } else {
        Line::from("")
    };

    let summary_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 70, 90)))
        .style(Style::default().bg(theme::SUMMARY_BG))
        .title(Span::styled(
            " Summary ",
            Style::default().fg(Color::Rgb(140, 180, 200)),
        ));

    let summary_para = Paragraph::new(vec![summary_line, running_line]).block(summary_block);

    f.render_widget(summary_para, area);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let mode_str = match &app.mode {
        AppMode::Setup => "Setup",
        AppMode::Selecting => "Selecting",
        AppMode::Running { .. } => "Running",
        AppMode::AwaitingManualResult { .. } => "Awaiting verdict",
        AppMode::Done => "Done",
    };
    let (mode_icon, mode_color) = match &app.mode {
        AppMode::Selecting => ("⊙", Color::Cyan),
        AppMode::Running { .. } => ("⟳", Color::Yellow),
        AppMode::Done => {
            let (_, fail, _) = app.summary_counts();
            if fail > 0 {
                ("✘", theme::STATUS_FAIL)
            } else {
                ("✔", Color::Green)
            }
        }
        AppMode::AwaitingManualResult { .. } => ("?", Color::Magenta),
        AppMode::Setup => (" ", Color::White),
    };

    let cpu_color = if app.cpu_usage > 80.0 {
        Color::Blue
    } else if app.cpu_usage > 50.0 {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let mem_color = if app.mem_usage > 80.0 {
        Color::Blue
    } else if app.mem_usage > 60.0 {
        Color::Yellow
    } else {
        Color::Cyan
    };

    let mouse_str = if app.mouse_capture {
        "🖱 on"
    } else {
        "🖱 off"
    };

    let header_line = Line::from(vec![
        Span::styled(
            " ✦ AutoCheck ",
            Style::default()
                .fg(theme::HEADER_TITLE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("│", Style::default().fg(Color::Rgb(60, 60, 100))),
        Span::raw(" "),
        Span::styled(
            app.repo_root.to_string_lossy().to_string(),
            Style::default()
                .fg(Color::Rgb(200, 200, 200))
                .add_modifier(Modifier::ITALIC),
        ),
        Span::raw("  "),
        Span::styled("branch:", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(
            app.current_branch.clone(),
            Style::default()
                .fg(theme::HEADER_BRANCH)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("mode:", Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(
            format!("{mode_icon} {mode_str}"),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
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

    let header = Paragraph::new(header_line).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 100))),
    );
    f.render_widget(header, area);
}

fn draw_check_list(f: &mut Frame, app: &mut App, area: Rect) {
    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

    app.list_visible_height = area.height.saturating_sub(2);

    let visible_height = area.height.saturating_sub(2) as usize;
    let offset = app.list_scroll_offset;
    let total_entries = app.list_entries.len();
    let entries_slice: &[ListEntry] = if offset < total_entries {
        &app.list_entries[offset..total_entries.min(offset + visible_height)]
    } else {
        &[]
    };

    let name_width = (area.width as usize).saturating_sub(12);

    let items: Vec<ListItem> = entries_slice
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let real_idx = offset + i;
            let is_cursor = real_idx == app.list_cursor;
            match entry {
                ListEntry::Group { group_idx } => {
                    let group = &app.groups[*group_idx];
                    let selected_in_group = app
                        .check_group_map
                        .iter()
                        .enumerate()
                        .filter(|(_, &gi)| gi == *group_idx)
                        .filter(|(ci, _)| app.selected[*ci])
                        .count();
                    let total_in_group = group.checks.len();
                    let icon = app.group_status_icon(*group_idx);
                    let count = format!("{selected_in_group}/{total_in_group}");
                    let row_style = if is_cursor {
                        Style::default().bg(theme::CURSOR_BG).fg(theme::CURSOR_FG)
                    } else {
                        Style::default()
                            .bg(theme::PANEL_BG)
                            .fg(theme::TEXT_SECONDARY)
                    };
                    let accent_style = if is_cursor {
                        Style::default()
                            .fg(theme::CURSOR_FG)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme::ACCENT)
                    };
                    let label_style = if is_cursor {
                        Style::default()
                            .fg(theme::CURSOR_FG)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme::TEXT_SECONDARY)
                    };
                    let meta_style = if is_cursor {
                        Style::default().fg(theme::CURSOR_FG)
                    } else {
                        Style::default().fg(theme::TEXT_MUTED)
                    };

                    ListItem::new(Line::from(vec![
                        Span::raw(" "),
                        Span::styled("─", accent_style),
                        Span::raw(" "),
                        Span::styled(icon, accent_style),
                        Span::raw(" "),
                        Span::styled(group.label.clone(), label_style),
                        Span::styled("  ", meta_style),
                        Span::styled(count, meta_style),
                        Span::styled(" selected", meta_style),
                    ]))
                    .style(row_style)
                }
                ListEntry::Check { check_idx, .. } => {
                    let check = &app.checks[*check_idx];
                    let sel_icon = if app.selected[*check_idx] {
                        Span::styled(
                            "◉",
                            Style::default()
                                .fg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::styled("○", Style::default().fg(Color::DarkGray))
                    };
                    let status = &app.statuses[*check_idx];

                    // Use animated spinner for running checks
                    let icon = if matches!(status, CheckStatus::Running) {
                        spinner_frame()
                    } else {
                        status.icon()
                    };

                    let elapsed_str = app.elapsed[*check_idx]
                        .map(|d| format!(" {:.1}s", d.as_secs_f32()))
                        .unwrap_or_default();

                    let row_style = status_row_style(status, is_cursor);

                    // Build rich line with styled spans and better icons
                    let (status_icon_span, name_span) = match status {
                        CheckStatus::Passed | CheckStatus::ManualPassed => (
                            Span::styled(
                                "✔",
                                Style::default()
                                    .fg(theme::STATUS_PASS)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(
                                    " {:<width$}{elapsed_str}",
                                    check.name,
                                    width = name_width.max(1)
                                ),
                                Style::default()
                                    .fg(theme::STATUS_PASS)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ),
                        CheckStatus::Failed | CheckStatus::ManualFailed => (
                            Span::styled(
                                "✘",
                                Style::default()
                                    .fg(theme::STATUS_FAIL)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(
                                    " {:<width$}{elapsed_str}",
                                    check.name,
                                    width = name_width.max(1)
                                ),
                                Style::default()
                                    .fg(theme::STATUS_FAIL)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ),
                        CheckStatus::Running => (
                            Span::styled(
                                icon,
                                Style::default()
                                    .fg(theme::STATUS_RUNNING)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(
                                    " {:<width$}{elapsed_str}",
                                    check.name,
                                    width = name_width.max(1)
                                ),
                                Style::default()
                                    .fg(theme::STATUS_RUNNING)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ),
                        CheckStatus::Skipped => (
                            Span::styled("⊘", Style::default().fg(theme::STATUS_SKIP)),
                            Span::styled(
                                format!(
                                    " {:<width$}{elapsed_str}",
                                    check.name,
                                    width = name_width.max(1)
                                ),
                                Style::default().fg(theme::STATUS_SKIP),
                            ),
                        ),
                        _ => (
                            Span::styled("◦", Style::default().fg(theme::STATUS_PENDING)),
                            Span::styled(
                                format!(
                                    " {:<width$}{elapsed_str}",
                                    check.name,
                                    width = name_width.max(1)
                                ),
                                Style::default().fg(theme::STATUS_PENDING),
                            ),
                        ),
                    };

                    // Override fg when cursor is active (keep the bg but use cursor fg)
                    let (sel_span, status_span, name_span) = if is_cursor {
                        (
                            Span::styled(
                                "◉",
                                Style::default()
                                    .fg(theme::CURSOR_FG)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                icon,
                                Style::default()
                                    .fg(theme::CURSOR_FG)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(
                                    " {:<width$}{elapsed_str}",
                                    check.name,
                                    width = name_width.max(1)
                                ),
                                Style::default()
                                    .fg(theme::CURSOR_FG)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        )
                    } else {
                        (sel_icon, status_icon_span, name_span)
                    };

                    ListItem::new(Line::from(vec![
                        Span::raw("  "),
                        sel_span,
                        Span::raw(" "),
                        status_span,
                        name_span,
                    ]))
                    .style(row_style)
                }
            }
        })
        .collect();

    // Scrollbar state
    let scrollable = total_entries.saturating_sub(visible_height);
    let mut scrollbar_state = ScrollbarState::new(scrollable).position(offset);

    let list_title = format!(" Checks ({total_entries}) ");
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 100)))
            .title(Span::styled(
                list_title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, area);

    // Render scrollbar on top of the list (right edge)
    if scrollable > 0 {
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}

fn draw_output_pane(f: &mut Frame, app: &mut App, area: Rect) {
    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

    let pane_height = area.height.saturating_sub(2) as usize;
    app.output_pane_height = pane_height as u16;

    // Determine which lines to show and the title
    let title_buf: String;
    let title: &str;
    let all_lines: Vec<Line> = match &app.mode {
        AppMode::Running { .. } => {
            let total = app.output_lines.len();
            title_buf = format!(
                " Output (live) — {}/{} ",
                app.output_scroll.min(total.saturating_sub(1)) + 1,
                total.max(1)
            );
            title = &title_buf;
            app.output_lines
                .iter()
                .map(|l| Line::from(l.clone()))
                .collect()
        }
        _ => {
            if let Some(ListEntry::Check { check_idx, .. }) = app.list_entries.get(app.list_cursor)
            {
                if let Some(log) = app.check_logs.get(check_idx) {
                    let total = log.len();
                    title_buf = format!(
                        " Check log — {}/{} ",
                        app.output_scroll.min(total.saturating_sub(1)) + 1,
                        total.max(1)
                    );
                    title = &title_buf;
                    log.iter().map(|l| Line::from(l.clone())).collect()
                } else {
                    title_buf = " Check description ".to_string();
                    title = &title_buf;
                    vec![Line::from(app.checks[*check_idx].description.clone())]
                }
            } else {
                let total = app.output_lines.len();
                title_buf = format!(
                    " Output — {}/{} ",
                    app.output_scroll.min(total.saturating_sub(1)) + 1,
                    total.max(1)
                );
                title = &title_buf;
                app.output_lines
                    .iter()
                    .map(|l| Line::from(l.clone()))
                    .collect()
            }
        }
    };

    let total_lines = all_lines.len();

    // Clamp scroll so it never goes past last line
    let max_scroll = total_lines.saturating_sub(1);
    if app.output_scroll > max_scroll {
        app.output_scroll = max_scroll;
    }
    let scroll_pos = app.output_scroll;

    // scrollbar
    let scrollable = total_lines.saturating_sub(pane_height);
    let mut scrollbar_state = ScrollbarState::new(scrollable).position(scroll_pos);

    // Copy-mode hint: always visible when mouse capture is off
    let copy_hint = if !app.mouse_capture {
        Span::styled(
            " [mouse off — select text to copy]",
            Style::default().fg(Color::DarkGray),
        )
    } else {
        Span::styled(
            " [m: toggle mouse for copy]",
            Style::default().fg(Color::Rgb(60, 60, 60)),
        )
    };

    let pane_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 100)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(copy_hint);

    if let AppMode::AwaitingManualResult { .. } = &app.mode {
        let mut augmented = all_lines;
        augmented.push(Line::from(""));
        augmented.push(Line::from(Span::styled(
            " ── Manual check: press p (pass) or f (fail) ──",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        let para = Paragraph::new(augmented)
            .block(pane_block)
            .wrap(Wrap { trim: false })
            .scroll((scroll_pos as u16, 0));
        f.render_widget(para, area);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    } else {
        let para = Paragraph::new(all_lines)
            .block(pane_block)
            .wrap(Wrap { trim: false })
            .scroll((scroll_pos as u16, 0));
        f.render_widget(para, area);
        if scrollable > 0 {
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut scrollbar_state,
            );
        }
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let (text, accent_color) = match &app.mode {
        AppMode::Setup => (
            " Tab/↓ next  ↑ prev  Enter confirm  q quit".to_string(),
            Color::DarkGray,
        ),
        AppMode::Selecting => (
            " ↑↓ navigate  Space toggle  a all  n none  Enter run  m mouse  q quit".to_string(),
            Color::DarkGray,
        ),
        AppMode::Running { .. } => (" r reset  q quit".to_string(), Color::Rgb(180, 180, 140)),
        AppMode::AwaitingManualResult { .. } => (
            " p pass  f fail  q quit".to_string(),
            Color::Rgb(200, 150, 200),
        ),
        AppMode::Done => {
            let (_p, fail, _s) = app.summary_counts();
            let color = if fail > 0 {
                theme::STATUS_FAIL
            } else {
                Color::Rgb(100, 200, 120) // Softer green
            };
            (
                " Enter rerun  r reset  R reset audited  x reset one  q quit".to_string(),
                color,
            )
        }
    };

    let para = Paragraph::new(text)
        .style(Style::default().fg(accent_color))
        .alignment(Alignment::Left);
    f.render_widget(para, area);
}
