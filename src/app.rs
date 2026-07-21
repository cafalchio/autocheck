use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::Instant;

use sysinfo::System;

use crate::checks::{discover_config_files, Check, ChecksConfig, Group};
use crate::config::SavedConfig;

// ── Modes ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Setup,
    Selecting,
    Running { idx: usize },
    AwaitingManualResult { idx: usize },
    Done,
}

// ── Setup tab ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SetupTab {
    Run,
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SetupField {
    ProjectPath,
    Branch,
    Config,
    LogFolder,
}

// ── Check status ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
    ManualPassed,
    ManualFailed,
}

impl CheckStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            CheckStatus::Pending => "·",
            CheckStatus::Running => "⟳",
            CheckStatus::Passed | CheckStatus::ManualPassed => "✔",
            CheckStatus::Failed | CheckStatus::ManualFailed => "✘",
            CheckStatus::Skipped => "⊘",
        }
    }
}

// ── List entry ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ListEntry {
    Group { group_idx: usize },
    Check { group_idx: usize, check_idx: usize },
}

// ── App ────────────────────────────────────────────────────────────────────

pub struct App {
    pub mode: AppMode,

    // Config data
    pub groups: Vec<Group>,
    pub checks: Vec<Check>,
    pub check_group_map: Vec<usize>,

    // Selection / status
    pub selected: Vec<bool>,
    pub statuses: Vec<CheckStatus>,
    pub elapsed: Vec<Option<std::time::Duration>>,

    // Runner list state
    pub list_entries: Vec<ListEntry>,
    pub list_cursor: usize,
    pub list_scroll_offset: usize,
    pub list_visible_height: u16,
    pub list_area: Option<ratatui::layout::Rect>,

    // Output
    pub output_lines: Vec<String>,
    pub check_logs: HashMap<usize, Vec<String>>,
    pub audited_passed: HashSet<usize>,
    pub output_scroll: usize,
    pub output_pane_height: u16,

    // Execution channel / cancel
    pub log_rx: Option<mpsc::Receiver<String>>,
    pub cancel_flag: Option<Arc<AtomicBool>>,

    // Repo
    pub repo_root: PathBuf,
    pub current_branch: String,
    pub staged_files: Vec<String>,

    // Setup form
    pub setup_tab: SetupTab,
    pub setup_project_path: String,
    pub setup_branch: String,
    pub setup_log_folder: String,
    pub setup_focus: SetupField,
    pub setup_error: Option<String>,
    pub setup_log: Vec<String>,

    // Config selector
    pub config_paths: Vec<PathBuf>,
    pub config_idx: usize,
    pub config_dropdown_open: bool,
    pub selected_config_path: Option<PathBuf>,

    // Mouse
    pub mouse_capture: bool,

    // System monitoring
    pub sys: System,
    pub cpu_usage: f32,
    pub mem_usage: f32,
}

impl App {
    pub fn new() -> Self {
        let saved = SavedConfig::load();
        let config_paths = discover_config_files();

        // Pre-select the saved config if it still exists
        let config_idx = if !saved.selected_config.is_empty() {
            config_paths
                .iter()
                .position(|p| p.to_string_lossy() == saved.selected_config)
                .unwrap_or(0)
        } else {
            0
        };

        let mut app = App {
            mode: AppMode::Setup,
            groups: Vec::new(),
            checks: Vec::new(),
            check_group_map: Vec::new(),
            selected: Vec::new(),
            statuses: Vec::new(),
            elapsed: Vec::new(),
            list_entries: Vec::new(),
            list_cursor: 0,
            list_scroll_offset: 0,
            list_visible_height: 20,
            list_area: None,
            output_lines: Vec::new(),
            check_logs: HashMap::new(),
            audited_passed: HashSet::new(),
            output_scroll: 0,
            output_pane_height: 20,
            log_rx: None,
            cancel_flag: None,
            repo_root: PathBuf::from("."),
            current_branch: String::new(),
            staged_files: Vec::new(),
            setup_tab: SetupTab::Run,
            setup_project_path: saved.repo.clone(),
            setup_branch: saved.branch.clone(),
            setup_log_folder: saved.log_folder.clone(),
            setup_focus: SetupField::ProjectPath,
            setup_error: None,
            setup_log: Vec::new(),
            config_paths,
            config_idx,
            config_dropdown_open: false,
            selected_config_path: None,
            mouse_capture: true,
            sys: System::new_all(),
            cpu_usage: 0.0,
            mem_usage: 0.0,
        };

        // Detect branch from saved repo if it exists
        if !saved.repo.is_empty() {
            let p = PathBuf::from(&saved.repo);
            app.current_branch = detect_current_branch(&p);
        }

        // Load the saved config if available
        if !app.config_paths.is_empty() {
            app.load_config_at(app.config_idx);
        }

        app
    }

    // ── System monitoring ─────────────────────────────────────────────────

    pub fn refresh_system_stats(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        let cpus = self.sys.cpus();
        if !cpus.is_empty() {
            let total: f32 = cpus.iter().map(|c| c.cpu_usage()).sum();
            self.cpu_usage = total / cpus.len() as f32;
        }
        let total_mem = self.sys.total_memory();
        let used_mem = self.sys.used_memory();
        if total_mem > 0 {
            self.mem_usage = used_mem as f32 / total_mem as f32 * 100.0;
        }
    }

    // ── Config selector ───────────────────────────────────────────────────

    pub fn next_config(&mut self) {
        if self.config_paths.is_empty() {
            return;
        }
        self.config_idx = (self.config_idx + 1) % self.config_paths.len();
        self.load_config_at(self.config_idx);
    }

    pub fn prev_config(&mut self) {
        if self.config_paths.is_empty() {
            return;
        }
        self.config_idx = self
            .config_idx
            .checked_sub(1)
            .unwrap_or(self.config_paths.len() - 1);
        self.load_config_at(self.config_idx);
    }

    pub fn load_config_at(&mut self, idx: usize) {
        if let Some(path) = self.config_paths.get(idx) {
            match ChecksConfig::from_path(path) {
                Ok(cfg) => {
                    self.selected_config_path = Some(path.clone());
                    self.load_checks_from_groups(cfg.groups);
                }
                Err(e) => {
                    self.setup_error = Some(e);
                }
            }
        }
    }

    pub fn toggle_dropdown(&mut self) {
        self.config_dropdown_open = !self.config_dropdown_open;
    }

    pub fn close_dropdown(&mut self) {
        self.config_dropdown_open = false;
    }

    // ── Setup field navigation ────────────────────────────────────────────

    pub fn setup_focus_next(&mut self) {
        self.setup_focus = match self.setup_focus {
            SetupField::ProjectPath => SetupField::Branch,
            SetupField::Branch => SetupField::Config,
            SetupField::Config => SetupField::ProjectPath,
            SetupField::LogFolder => SetupField::LogFolder,
        };
    }

    pub fn setup_focus_prev(&mut self) {
        self.setup_focus = match self.setup_focus {
            SetupField::ProjectPath => SetupField::Config,
            SetupField::Branch => SetupField::ProjectPath,
            SetupField::Config => SetupField::Branch,
            SetupField::LogFolder => SetupField::LogFolder,
        };
    }

    pub fn setup_tab_next(&mut self) {
        self.setup_tab = match self.setup_tab {
            SetupTab::Run => SetupTab::Settings,
            SetupTab::Settings => SetupTab::Run,
        };
        self.setup_focus = match self.setup_tab {
            SetupTab::Run => SetupField::ProjectPath,
            SetupTab::Settings => SetupField::LogFolder,
        };
    }

    pub fn setup_tab_prev(&mut self) {
        self.setup_tab = match self.setup_tab {
            SetupTab::Run => SetupTab::Settings,
            SetupTab::Settings => SetupTab::Run,
        };
        self.setup_focus = match self.setup_tab {
            SetupTab::Run => SetupField::ProjectPath,
            SetupTab::Settings => SetupField::LogFolder,
        };
    }

    pub fn setup_type_char(&mut self, c: char) {
        match self.setup_focus {
            SetupField::ProjectPath => {
                self.setup_project_path.push(c);
                let p = PathBuf::from(&self.setup_project_path);
                self.current_branch = detect_current_branch(&p);
                self.setup_error = None;
                self.setup_log.clear();
            }
            SetupField::Branch => {
                self.setup_branch.push(c);
                self.setup_error = None;
                self.setup_log.clear();
            }
            SetupField::Config => {}
            SetupField::LogFolder => {
                self.setup_log_folder.push(c);
                self.setup_error = None;
                self.setup_log.clear();
            }
        }
    }

    pub fn setup_backspace(&mut self) {
        match self.setup_focus {
            SetupField::ProjectPath => {
                self.setup_project_path.pop();
                let p = PathBuf::from(&self.setup_project_path);
                self.current_branch = detect_current_branch(&p);
                self.setup_error = None;
            }
            SetupField::Branch => {
                self.setup_branch.pop();
                self.setup_error = None;
            }
            SetupField::Config => {}
            SetupField::LogFolder => {
                self.setup_log_folder.pop();
                self.setup_error = None;
            }
        }
    }

    // ── Setup confirmation (T-11) ─────────────────────────────────────────

    pub fn confirm_setup(&mut self) {
        // Validate project path
        let path = PathBuf::from(&self.setup_project_path);
        if !path.exists() {
            self.setup_error = Some(format!("Path does not exist: {}", path.display()));
            return;
        }
        if !path.is_dir() {
            self.setup_error = Some(format!("Path is not a directory: {}", path.display()));
            return;
        }

        // Check a config is loaded
        if self.checks.is_empty() {
            self.setup_error = Some("No checks config loaded. Select a valid config file.".into());
            return;
        }

        // Optional branch checkout
        if !self.setup_branch.is_empty() {
            if let Err(e) = self.checkout_branch(&path, &self.setup_branch.clone()) {
                self.setup_error = Some(e);
                return;
            }
        }

        // All good — commit
        self.repo_root = path.clone();
        self.current_branch = detect_current_branch(&path);
        self.staged_files = get_staged_files(&path);

        // Save config
        let saved = SavedConfig {
            repo: self.setup_project_path.clone(),
            branch: self.setup_branch.clone(),
            selected_config: self
                .selected_config_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            log_folder: self.setup_log_folder.clone(),
        };
        let _ = saved.save();

        // Reset run state and transition
        self.reset_run_state();
        self.setup_error = None;
        self.mode = AppMode::Selecting;
    }

    // ── Branch checkout (T-10) ────────────────────────────────────────────

    fn checkout_branch(&mut self, repo: &PathBuf, branch: &str) -> Result<(), String> {
        self.setup_log.clear();

        // Numeric input → treat as PR number
        let is_pr = branch.chars().all(|c| c.is_ascii_digit());
        let pr_ref;
        let git_args: Vec<&str> = if is_pr {
            pr_ref = format!("pull/{branch}/head:pr-{branch}");
            vec!["fetch", "origin", pr_ref.as_str()]
        } else {
            pr_ref = String::new();
            let _ = pr_ref; // suppress unused warning
            vec!["switch", branch]
        };

        let out = Command::new("git")
            .args(&git_args)
            .current_dir(repo)
            .output();

        match out {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stdout.lines().chain(stderr.lines()) {
                    self.setup_log.push(line.to_string());
                }
                if !output.status.success() {
                    return Err(format!(
                        "git failed: {}",
                        stderr.lines().next().unwrap_or("unknown error")
                    ));
                }
                // After PR fetch, check out the local branch
                if is_pr {
                    let checkout_out = Command::new("git")
                        .args(["switch", &format!("pr-{branch}")])
                        .current_dir(repo)
                        .output();
                    if let Ok(o) = checkout_out {
                        if !o.status.success() {
                            let se = String::from_utf8_lossy(&o.stderr);
                            return Err(format!("checkout pr branch failed: {se}"));
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(format!("Failed to run git: {e}")),
        }
    }

    // ── List navigation / selection (T-13) ───────────────────────────────

    pub fn list_up(&mut self) {
        if self.list_cursor > 0 {
            self.list_cursor -= 1;
        }
        self.output_scroll = 0;
        // Scroll the list viewport up if cursor goes above visible area
        if self.list_cursor < self.list_scroll_offset {
            self.list_scroll_offset = self.list_cursor;
        }
    }

    pub fn list_down(&mut self) {
        if self.list_cursor + 1 < self.list_entries.len() {
            self.list_cursor += 1;
        }
        self.output_scroll = 0;
        // list_visible_height is updated by draw each frame; use a default of 20 if not set
        let visible = self.list_visible_height.max(1) as usize;
        if self.list_cursor >= self.list_scroll_offset + visible {
            self.list_scroll_offset = self.list_cursor + 1 - visible;
        }
    }

    pub fn toggle_current(&mut self) {
        match self.list_entries.get(self.list_cursor).cloned() {
            Some(ListEntry::Check { check_idx, .. }) => {
                self.selected[check_idx] = !self.selected[check_idx];
            }
            Some(ListEntry::Group { group_idx }) => {
                self.toggle_group(group_idx);
            }
            None => {}
        }
    }

    pub fn toggle_group(&mut self, group_idx: usize) {
        // Collect check indices in this group
        let check_indices: Vec<usize> = self
            .check_group_map
            .iter()
            .enumerate()
            .filter(|(_, &gi)| gi == group_idx)
            .map(|(ci, _)| ci)
            .collect();

        let all_selected = check_indices.iter().all(|&ci| self.selected[ci]);
        let new_val = !all_selected; // if all selected, deselect; otherwise select all
        for ci in check_indices {
            self.selected[ci] = new_val;
        }
    }

    pub fn select_all(&mut self) {
        for s in self.selected.iter_mut() {
            *s = true;
        }
    }

    pub fn select_none(&mut self) {
        for s in self.selected.iter_mut() {
            *s = false;
        }
    }

    pub fn scroll_up(&mut self) {
        self.output_scroll = self.output_scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.output_scroll += 1;
    }

    pub fn page_up(&mut self) {
        let page = self.output_pane_height.max(1) as usize;
        self.output_scroll = self.output_scroll.saturating_sub(page);
    }

    pub fn page_down(&mut self) {
        let page = self.output_pane_height.max(1) as usize;
        self.output_scroll += page;
    }

    pub fn scroll_to_top(&mut self) {
        self.output_scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.output_scroll = self.output_lines.len().saturating_sub(1);
    }

    // ── Mouse support (T-14) ──────────────────────────────────────────────

    pub fn toggle_mouse_capture(&mut self) {
        self.mouse_capture = !self.mouse_capture;
    }

    pub fn handle_click(&mut self, col: u16, row: u16) {
        if let Some(area) = self.list_area {
            if col >= area.x
                && col < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
            {
                // Row inside list area (account for border)
                let inner_row = row.saturating_sub(area.y + 1) as usize;
                if inner_row < self.list_entries.len() {
                    self.list_cursor = inner_row;
                    self.toggle_current();
                }
            }
        }
    }

    // ── Command execution (T-15) ──────────────────────────────────────────

    pub fn start_running(&mut self) {
        let first = self
            .selected
            .iter()
            .enumerate()
            .find(|(idx, sel)| **sel && !self.is_audited_locked(*idx))
            .map(|(idx, _)| idx);
        if first.is_none() {
            return;
        }

        for (i, &sel) in self.selected.iter().enumerate() {
            if !sel || self.is_audited_locked(i) {
                self.statuses[i] = CheckStatus::Skipped;
            }
        }

        self.output_lines.clear();
        self.check_logs.clear();
        self.output_scroll = 0;

        // Kick off execution in background thread
        let (tx, rx) = mpsc::channel::<String>();
        self.log_rx = Some(rx);

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_flag = Some(cancel.clone());

        let checks = self.checks.clone();
        let selected = self.selected.clone();
        let repo_root = self.repo_root.clone();
        let audited_passed = self.audited_passed.clone();
        let tx_clone = tx.clone();

        std::thread::spawn(move || {
            for (idx, check) in checks.iter().enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if !selected[idx] || (check.audited && audited_passed.contains(&idx)) {
                    continue;
                }

                let _ = tx_clone.send(format!("STATUS:{idx}:Running"));
                let _ = tx_clone.send(format!("LOG:{idx}:▶ {}", check.name));
                let start = Instant::now();

                let output = run_check(check, &repo_root, &cancel);

                let elapsed_ms = start.elapsed().as_millis();
                let _ = tx_clone.send(format!("ELAPSED:{idx}:{elapsed_ms}"));

                let mut lines: Vec<String> = Vec::new();
                let (status, exit_ok) = match output {
                    Ok((stdout, exit_ok)) => {
                        for line in stdout.lines() {
                            lines.push(line.to_string());
                            let _ = tx_clone.send(format!("LOG:{idx}:{line}"));
                            let _ = tx_clone.send(format!("OUT:{line}"));
                        }
                        if exit_ok {
                            ("Passed", true)
                        } else {
                            ("Failed", false)
                        }
                    }
                    Err(e) => {
                        let msg = format!("Error: {e}");
                        let _ = tx_clone.send(format!("LOG:{idx}:{msg}"));
                        let _ = tx_clone.send(format!("OUT:{msg}"));
                        ("Failed", false)
                    }
                };

                if check.manual && exit_ok {
                    let _ = tx_clone.send(format!("STATUS:{idx}:AwaitManual"));
                    let _ = tx_clone.send(format!("MANUAL:{idx}:prompt"));
                    return;
                } else {
                    let _ = tx_clone.send(format!("STATUS:{idx}:{status}"));
                    if check.audited && exit_ok {
                        let _ = tx_clone.send(format!("AUDITED_PASS:{idx}"));
                    }
                }
            }
            let _ = tx_clone.send("DONE:".to_string());
        });

        self.mode = AppMode::Running { idx: 0 };
    }

    /// Called each tick to drain the log channel.
    pub fn poll_log(&mut self) {
        let mut done = false;
        let mut manual_prompt: Option<usize> = None;

        // Drain all pending messages
        loop {
            let msg = if let Some(rx) = &self.log_rx {
                rx.try_recv().ok()
            } else {
                break;
            };
            match msg {
                None => break,
                Some(m) => {
                    if let Some(rest) = m.strip_prefix("OUT:") {
                        self.output_lines.push(rest.to_string());
                        // Auto-tail: keep scroll at end if near end
                        let tail = self.output_lines.len().saturating_sub(1);
                        if self.output_scroll + (self.output_pane_height as usize) + 2 >= tail {
                            self.output_scroll = tail;
                        }
                    } else if let Some(rest) = m.strip_prefix("LOG:") {
                        let mut parts = rest.splitn(2, ':');
                        if let (Some(idx_s), Some(line)) = (parts.next(), parts.next()) {
                            if let Ok(idx) = idx_s.parse::<usize>() {
                                self.check_logs
                                    .entry(idx)
                                    .or_default()
                                    .push(line.to_string());
                            }
                        }
                    } else if let Some(rest) = m.strip_prefix("STATUS:") {
                        let mut parts = rest.splitn(2, ':');
                        if let (Some(idx_s), Some(val)) = (parts.next(), parts.next()) {
                            if let Ok(idx) = idx_s.parse::<usize>() {
                                self.statuses[idx] = match val {
                                    "Running" => CheckStatus::Running,
                                    "Passed" => CheckStatus::Passed,
                                    "Failed" => CheckStatus::Failed,
                                    "Skipped" => CheckStatus::Skipped,
                                    _ => CheckStatus::Pending,
                                };
                                if matches!(self.mode, AppMode::Running { .. }) {
                                    self.mode = AppMode::Running { idx };
                                }
                            }
                        }
                    } else if let Some(rest) = m.strip_prefix("ELAPSED:") {
                        let mut parts = rest.splitn(2, ':');
                        if let (Some(idx_s), Some(ms_s)) = (parts.next(), parts.next()) {
                            if let (Ok(idx), Ok(ms)) = (idx_s.parse::<usize>(), ms_s.parse::<u64>())
                            {
                                self.elapsed[idx] = Some(std::time::Duration::from_millis(ms));
                            }
                        }
                    } else if let Some(rest) = m.strip_prefix("MANUAL:") {
                        let mut parts = rest.splitn(2, ':');
                        if let Some(idx_s) = parts.next() {
                            if let Ok(idx) = idx_s.parse::<usize>() {
                                manual_prompt = Some(idx);
                            }
                        }
                    } else if let Some(rest) = m.strip_prefix("AUDITED_PASS:") {
                        if let Ok(idx) = rest.parse::<usize>() {
                            self.record_audited_pass(idx);
                        }
                    } else if m.starts_with("DONE:") {
                        done = true;
                    }
                }
            }
        }

        if let Some(idx) = manual_prompt {
            self.mode = AppMode::AwaitingManualResult { idx };
        } else if done {
            self.finish_run();
        }
    }

    // ── Manual check verdict (T-17) ───────────────────────────────────────

    pub fn manual_pass(&mut self) {
        if let AppMode::AwaitingManualResult { idx } = self.mode.clone() {
            self.statuses[idx] = CheckStatus::ManualPassed;
            let line = format!("[manual] {} — PASSED", self.checks[idx].name);
            self.check_logs.entry(idx).or_default().push(line.clone());
            self.output_lines.push(line);
            self.record_audited_pass(idx);
            self.continue_after_manual(idx);
        }
    }

    pub fn manual_fail(&mut self) {
        if let AppMode::AwaitingManualResult { idx } = self.mode.clone() {
            self.statuses[idx] = CheckStatus::ManualFailed;
            let line = format!("[manual] {} — FAILED", self.checks[idx].name);
            self.check_logs.entry(idx).or_default().push(line.clone());
            self.output_lines.push(line);
            self.continue_after_manual(idx);
        }
    }

    fn continue_after_manual(&mut self, done_idx: usize) {
        // Find next selected check after done_idx
        let next = self.selected[done_idx + 1..]
            .iter()
            .enumerate()
            .find(|(rel, s)| **s && !self.is_audited_locked(done_idx + 1 + *rel))
            .map(|(rel, _)| rel + done_idx + 1);

        if let Some(next_idx) = next {
            // Resume execution by spawning a new thread for remaining checks
            self.spawn_run_from(next_idx);
        } else {
            self.finish_run();
        }
    }

    fn spawn_run_from(&mut self, start_idx: usize) {
        let (tx, rx) = mpsc::channel::<String>();
        self.log_rx = Some(rx);

        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel_flag = Some(cancel.clone());

        let checks = self.checks.clone();
        let selected = self.selected.clone();
        let repo_root = self.repo_root.clone();
        let audited_passed = self.audited_passed.clone();

        std::thread::spawn(move || {
            for idx in start_idx..checks.len() {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let check = &checks[idx];
                if !selected[idx] || (check.audited && audited_passed.contains(&idx)) {
                    continue;
                }
                let _ = tx.send(format!("STATUS:{idx}:Running"));
                let _ = tx.send(format!("LOG:{idx}:▶ {}", check.name));
                let start = Instant::now();

                let output = run_check(check, &repo_root, &cancel);

                let elapsed_ms = start.elapsed().as_millis();
                let _ = tx.send(format!("ELAPSED:{idx}:{elapsed_ms}"));

                let (status, exit_ok) = match output {
                    Ok((stdout, ok)) => {
                        for line in stdout.lines() {
                            let _ = tx.send(format!("LOG:{idx}:{line}"));
                            let _ = tx.send(format!("OUT:{line}"));
                        }
                        if ok {
                            ("Passed", true)
                        } else {
                            ("Failed", false)
                        }
                    }
                    Err(e) => {
                        let msg = format!("Error: {e}");
                        let _ = tx.send(format!("LOG:{idx}:{msg}"));
                        let _ = tx.send(format!("OUT:{msg}"));
                        ("Failed", false)
                    }
                };

                if check.manual && exit_ok {
                    let _ = tx.send(format!("MANUAL:{idx}:prompt"));
                    return;
                } else {
                    let _ = tx.send(format!("STATUS:{idx}:{status}"));
                    if check.audited && exit_ok {
                        let _ = tx.send(format!("AUDITED_PASS:{idx}"));
                    }
                }
            }
            let _ = tx.send("DONE:".to_string());
        });

        self.mode = AppMode::Running { idx: start_idx };
    }

    // ── Finish run (T-22 done screen + T-19 log writing) ─────────────────

    fn finish_run(&mut self) {
        self.mode = AppMode::Done;
        self.write_logs();
        self.save_audited_state();
    }

    // ── Cancellation (T-16) ───────────────────────────────────────────────

    pub fn cancel_run(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn reset_to_selecting(&mut self) {
        self.cancel_run();
        self.reset_run_state();
        self.mode = AppMode::Selecting;
    }

    pub fn reset_audited(&mut self) {
        self.audited_passed.clear();
        self.save_audited_state();
        self.reset_to_selecting();
    }

    pub fn reset_current_audited(&mut self) {
        if let Some(ListEntry::Check { check_idx, .. }) = self.list_entries.get(self.list_cursor) {
            if self.checks[*check_idx].audited {
                self.audited_passed.remove(check_idx);
                self.save_audited_state();
                self.statuses[*check_idx] = CheckStatus::Pending;
            }
        }
    }

    // ── Done screen actions (T-22) ────────────────────────────────────────

    pub fn rerun(&mut self) {
        self.reset_run_state();
        self.start_running();
    }

    // ── Log file writing (T-19) ───────────────────────────────────────────

    fn write_logs(&self) {
        let header = format!(
            "Branch: {}\nStaged files: {}\n---\n",
            self.current_branch,
            if self.staged_files.is_empty() {
                "(none)".to_string()
            } else {
                self.staged_files.join(", ")
            }
        );

        // last_run.log
        let mut run_content = header.clone();
        for (idx, check) in self.checks.iter().enumerate() {
            run_content.push_str(&format!("\n## {} — {:?}\n", check.name, self.statuses[idx]));
            if let Some(log) = self.check_logs.get(&idx) {
                for line in log {
                    run_content.push_str(line);
                    run_content.push('\n');
                }
            }
        }
        let _ = std::fs::write(self.repo_root.join("last_run.log"), &run_content);

        // last_failed.log
        let log_path = self
            .selected_config_path
            .as_ref()
            .and_then(|p| {
                ChecksConfig::from_path(p)
                    .ok()
                    .and_then(|c| c.path_log_file)
            })
            .unwrap_or_else(|| "last_failed.log".to_string());

        let mut fail_content = header;
        let mut has_failures = false;
        for (idx, check) in self.checks.iter().enumerate() {
            if matches!(
                self.statuses[idx],
                CheckStatus::Failed | CheckStatus::ManualFailed
            ) {
                has_failures = true;
                fail_content.push_str(&format!("\n## {} — FAILED\n", check.name));
                if let Some(log) = self.check_logs.get(&idx) {
                    for line in log {
                        fail_content.push_str(line);
                        fail_content.push('\n');
                    }
                }
            }
        }
        if has_failures {
            let _ = std::fs::write(self.repo_root.join(&log_path), &fail_content);
        } else {
            // Truncate/clear the failed log on success
            let _ = std::fs::write(self.repo_root.join(&log_path), "");
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn is_audited_locked(&self, idx: usize) -> bool {
        self.checks[idx].audited && self.audited_passed.contains(&idx)
    }

    fn audited_state_path(&self) -> PathBuf {
        let folder = if self.setup_log_folder.trim().is_empty() {
            "logs"
        } else {
            self.setup_log_folder.trim()
        };
        self.repo_root.join(folder)
    }

    fn audited_log_path(&self, idx: usize) -> PathBuf {
        let group_label = self.groups[self.check_group_map[idx]]
            .label
            .replace([' ', '/'], "_")
            .to_lowercase();
        let check_name = self.checks[idx].name.replace([' ', '/'], "_").to_lowercase();
        self.audited_state_path()
            .join(format!("{group_label}_{check_name}.log"))
    }

    fn load_audited_state(&mut self) {
        self.audited_passed.clear();
        for idx in 0..self.checks.len() {
            if self.checks[idx].audited && self.audited_log_path(idx).exists() {
                self.audited_passed.insert(idx);
            }
        }
    }

    fn save_audited_state(&self) {
        let dir = self.audited_state_path();
        let _ = std::fs::create_dir_all(&dir);
    }

    fn record_audited_pass(&mut self, idx: usize) {
        if !self.checks[idx].audited {
            return;
        }
        let dir = self.audited_state_path();
        let _ = std::fs::create_dir_all(&dir);
        let mut content = format!("check={}\nstatus={:?}\n", self.checks[idx].name, self.statuses[idx]);
        if let Some(log) = self.check_logs.get(&idx) {
            for line in log {
                content.push_str(line);
                content.push('\n');
            }
        }
        let _ = std::fs::write(self.audited_log_path(idx), content);
        self.audited_passed.insert(idx);
    }

    pub fn rebuild_list_entries(&mut self) {
        self.list_entries.clear();
        for (g_idx, _group) in self.groups.iter().enumerate() {
            self.list_entries
                .push(ListEntry::Group { group_idx: g_idx });
            let check_indices: Vec<usize> = self
                .check_group_map
                .iter()
                .enumerate()
                .filter(|(_, &gi)| gi == g_idx)
                .map(|(ci, _)| ci)
                .collect();
            for ci in check_indices {
                self.list_entries.push(ListEntry::Check {
                    group_idx: g_idx,
                    check_idx: ci,
                });
            }
        }
    }

    pub fn reset_run_state(&mut self) {
        let audited_passed = self.audited_passed.clone();
        for (idx, status) in self.statuses.iter_mut().enumerate() {
            *status = if self.checks[idx].audited && audited_passed.contains(&idx) {
                CheckStatus::Passed
            } else {
                CheckStatus::Pending
            };
        }
        self.output_lines.clear();
        self.check_logs.clear();
        self.output_scroll = 0;
        self.log_rx = None;
        self.cancel_flag = None;
        self.elapsed = vec![None; self.checks.len()];
    }

    pub fn load_checks_from_groups(&mut self, groups: Vec<Group>) {
        self.checks.clear();
        self.check_group_map.clear();
        for (g_idx, group) in groups.iter().enumerate() {
            for check in &group.checks {
                self.checks.push(check.clone());
                self.check_group_map.push(g_idx);
            }
        }
        self.groups = groups;
        self.selected = vec![true; self.checks.len()];
        self.statuses = vec![CheckStatus::Pending; self.checks.len()];
        self.elapsed = vec![None; self.checks.len()];
        self.load_audited_state();
        self.reset_run_state();
        self.rebuild_list_entries();
    }

    pub fn summary_counts(&self) -> (usize, usize, usize) {
        let passed = self
            .statuses
            .iter()
            .filter(|s| matches!(s, CheckStatus::Passed | CheckStatus::ManualPassed))
            .count();
        let failed = self
            .statuses
            .iter()
            .filter(|s| matches!(s, CheckStatus::Failed | CheckStatus::ManualFailed))
            .count();
        let skipped = self
            .statuses
            .iter()
            .filter(|s| matches!(s, CheckStatus::Skipped))
            .count();
        (passed, failed, skipped)
    }

    pub fn group_status_icon(&self, group_idx: usize) -> &'static str {
        let indices: Vec<usize> = self
            .check_group_map
            .iter()
            .enumerate()
            .filter(|(_, &gi)| gi == group_idx)
            .map(|(ci, _)| ci)
            .collect();
        if indices
            .iter()
            .any(|&ci| matches!(self.statuses[ci], CheckStatus::Running))
        {
            return "⟳";
        }
        if indices.iter().any(|&ci| {
            matches!(
                self.statuses[ci],
                CheckStatus::Failed | CheckStatus::ManualFailed
            )
        }) {
            return "✘";
        }
        if indices.iter().all(|&ci| {
            !self.selected[ci]
                || matches!(
                    self.statuses[ci],
                    CheckStatus::Passed | CheckStatus::ManualPassed | CheckStatus::Skipped
                )
        }) {
            if indices.iter().any(|&ci| {
                self.selected[ci]
                    && matches!(
                        self.statuses[ci],
                        CheckStatus::Passed | CheckStatus::ManualPassed
                    )
            }) {
                return "✔";
            }
        }
        "·"
    }
}

// ── Free functions ─────────────────────────────────────────────────────────

/// Detect the current git branch in the given repo directory.
pub fn detect_current_branch(repo: &PathBuf) -> String {
    if !repo.exists() || !repo.is_dir() {
        return "?".to_string();
    }
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "?".to_string(),
    }
}

/// Get staged files in the repo.
pub fn get_staged_files(repo: &PathBuf) -> Vec<String> {
    let out = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMR"])
        .current_dir(repo)
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Run a single check command, streaming combined stdout/stderr to a String.
fn run_check(
    check: &Check,
    repo_root: &PathBuf,
    cancel: &Arc<AtomicBool>,
) -> Result<(String, bool), String> {
    let args = check.cmd.as_args();
    let (program, rest) = match args.as_slice() {
        [p, rest @ ..] => (p.as_str(), rest.to_vec()),
        [] => return Err("Empty command".to_string()),
    };

    // For single-string shell commands, pass through sh -c
    let (prog, prog_args): (&str, Vec<String>) = if check.cmd.is_string() {
        ("sh", vec!["-c".to_string(), args[0].clone()])
    } else {
        (program, rest)
    };

    let mut child = Command::new(prog)
        .args(&prog_args)
        .current_dir(repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn: {e}"))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let mut combined = String::new();

    // Read stdout and stderr on separate threads
    let (tx_out, rx_out) = mpsc::channel::<String>();
    let tx_err = tx_out.clone();

    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx_out.send(line);
        }
    });
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx_err.send(line);
        }
    });

    // Collect output while checking cancel
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            return Err("Cancelled".to_string());
        }
        match rx_out.try_recv() {
            Ok(line) => {
                combined.push_str(&line);
                combined.push('\n');
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Check if process has exited
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Drain remaining
                        while let Ok(line) = rx_out.try_recv() {
                            combined.push_str(&line);
                            combined.push('\n');
                        }
                        break;
                    }
                    Ok(None) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => return Err(format!("Wait error: {e}")),
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
        }
    }

    let status = child.wait().map_err(|e| format!("Wait error: {e}"))?;
    Ok((combined, status.success()))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::{Check, Cmd, Group};

    fn make_app_with_checks(checks: Vec<Check>) -> App {
        let mut app = App {
            mode: AppMode::Selecting,
            groups: vec![Group {
                label: "Test".into(),
                color: None,
                checks: checks.clone(),
            }],
            checks: checks.clone(),
            check_group_map: vec![0; checks.len()],
            selected: vec![true; checks.len()],
            statuses: vec![CheckStatus::Pending; checks.len()],
            elapsed: vec![None; checks.len()],
            list_entries: Vec::new(),
            list_cursor: 0,
            list_scroll_offset: 0,
            list_visible_height: 20,
            list_area: None,
            output_lines: Vec::new(),
            check_logs: HashMap::new(),
            audited_passed: HashSet::new(),
            output_scroll: 0,
            output_pane_height: 20,
            log_rx: None,
            cancel_flag: None,
            repo_root: PathBuf::from("."),
            current_branch: String::new(),
            staged_files: Vec::new(),
            setup_tab: SetupTab::Run,
            setup_project_path: String::new(),
            setup_branch: String::new(),
            setup_log_folder: String::new(),
            setup_focus: SetupField::ProjectPath,
            setup_error: None,
            setup_log: Vec::new(),
            config_paths: Vec::new(),
            config_idx: 0,
            config_dropdown_open: false,
            selected_config_path: None,
            mouse_capture: true,
            sys: System::new(),
            cpu_usage: 0.0,
            mem_usage: 0.0,
        };
        app.rebuild_list_entries();
        app
    }

    fn dummy_check(name: &str) -> Check {
        Check {
            name: name.into(),
            description: "desc".into(),
            cmd: Cmd::Array(vec!["true".into()]),
            manual: false,
            audited: false,
        }
    }

    fn audited_check(name: &str) -> Check {
        Check {
            name: name.into(),
            description: "desc".into(),
            cmd: Cmd::Array(vec!["true".into()]),
            manual: false,
            audited: true,
        }
    }

    #[test]
    fn test_list_entries_generated() {
        let app = make_app_with_checks(vec![dummy_check("a"), dummy_check("b")]);
        // 1 group header + 2 check entries
        assert_eq!(app.list_entries.len(), 3);
    }

    #[test]
    fn test_toggle_check() {
        let mut app = make_app_with_checks(vec![dummy_check("a")]);
        // cursor 0 = group header, cursor 1 = check
        app.list_cursor = 1;
        assert!(app.selected[0]);
        app.toggle_current();
        assert!(!app.selected[0]);
        app.toggle_current();
        assert!(app.selected[0]);
    }

    #[test]
    fn test_toggle_group_all_selected_deselects() {
        let mut app = make_app_with_checks(vec![dummy_check("a"), dummy_check("b")]);
        app.selected = vec![true, true];
        app.list_cursor = 0; // group header
        app.toggle_current();
        assert!(!app.selected[0]);
        assert!(!app.selected[1]);
    }

    #[test]
    fn test_toggle_group_partial_selects_all() {
        let mut app = make_app_with_checks(vec![dummy_check("a"), dummy_check("b")]);
        app.selected = vec![true, false];
        app.list_cursor = 0;
        app.toggle_current();
        assert!(app.selected[0]);
        assert!(app.selected[1]);
    }

    #[test]
    fn test_select_all() {
        let mut app = make_app_with_checks(vec![dummy_check("a"), dummy_check("b")]);
        app.selected = vec![false, false];
        app.select_all();
        assert!(app.selected.iter().all(|&s| s));
    }

    #[test]
    fn test_select_none() {
        let mut app = make_app_with_checks(vec![dummy_check("a"), dummy_check("b")]);
        app.select_none();
        assert!(app.selected.iter().all(|&s| !s));
    }

    #[test]
    fn test_summary_counts() {
        let mut app =
            make_app_with_checks(vec![dummy_check("a"), dummy_check("b"), dummy_check("c")]);
        app.statuses[0] = CheckStatus::Passed;
        app.statuses[1] = CheckStatus::Failed;
        app.statuses[2] = CheckStatus::Skipped;
        let (p, f, s) = app.summary_counts();
        assert_eq!(p, 1);
        assert_eq!(f, 1);
        assert_eq!(s, 1);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut app = make_app_with_checks(vec![dummy_check("a")]);
        app.statuses[0] = CheckStatus::Passed;
        app.output_lines.push("line".into());
        app.reset_run_state();
        assert!(matches!(app.statuses[0], CheckStatus::Pending));
        assert!(app.output_lines.is_empty());
    }

    #[test]
    fn test_app_starts_in_setup_mode() {
        let app = App::new();
        assert_eq!(app.mode, AppMode::Setup);
    }

    #[test]
    fn test_reset_keeps_audited_checks_passed() {
        let mut app = make_app_with_checks(vec![audited_check("audit")]);
        app.audited_passed.insert(0);
        app.reset_run_state();
        assert_eq!(app.statuses[0], CheckStatus::Passed);
    }

    #[test]
    fn test_reset_current_audited_unlocks_selected_check() {
        let mut app = make_app_with_checks(vec![audited_check("audit")]);
        app.audited_passed.insert(0);
        app.list_cursor = 1;
        app.statuses[0] = CheckStatus::Passed;
        app.reset_current_audited();
        assert!(!app.audited_passed.contains(&0));
        assert_eq!(app.statuses[0], CheckStatus::Pending);
    }
}
