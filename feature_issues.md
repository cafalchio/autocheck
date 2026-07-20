# AutoCheck — Feature Enhancement Issues

This document lists the next wave of improvements, split into individual implementable issues. Each issue is self-contained with clear acceptance criteria. Issues are grouped by theme and ordered so that dependencies come first.

---

## Issue F-01 — Improve setup screen alignment and layout consistency

### Summary

The setup screen has inconsistent inner padding, the form fields stretch across the full terminal width with no max-width cap, and the help bar text can overflow on narrow terminals.

### Files likely involved

- `src/draw.rs`

### Requirements

- Centre the setup form horizontally with a maximum content width of 80 columns (use horizontal `Constraint::Percentage` margins on wide terminals).
- Add 1-cell inner padding on left and right inside every form field border so typed text does not touch the border glyph.
- Align all three field labels flush-left with consistent capitalisation.
- Truncate the help bar text with `…` instead of wrapping or overflowing when the terminal is narrower than the text.
- Ensure the checkout log area uses the same horizontal bounds as the form fields (no wider, no narrower).

### Acceptance Criteria

- On an 80-column terminal, form fields are full-width and neatly bordered.
- On a 200-column terminal, the form is centred and capped at 80 columns.
- No text visually clips or bleeds into borders.
- Help text never wraps onto a second line; it truncates with `…`.

---

## Issue F-02 — Improve runner screen layout and column alignment

### Summary

The left check-list pane has no scrolling when the list is taller than the visible area, check names are not indented consistently, and the elapsed time column is not right-aligned.

### Files likely involved

- `src/draw.rs`
- `src/app.rs`

### Requirements

- Track a `list_scroll_offset: usize` in `App` so the cursor can scroll the check list when it goes out of view (keep cursor visible, scroll by one when it moves past either edge).
- Update `list_up` / `list_down` in `app.rs` to bump `list_scroll_offset` when needed.
- Render only the visible window of entries in `draw_check_list`, starting at `list_scroll_offset`.
- Right-align the elapsed time column in the check list (pad between name and time).
- Indent check rows by exactly 4 spaces after the selection bracket so check names align regardless of bracket width.

### Acceptance Criteria

- A config with more checks than the terminal height can display is scrollable with `↑`/`↓`.
- Cursor never goes off-screen.
- Elapsed time `0.3s` right-aligns with `12.4s` in the same column.

---

## Issue F-03 — Add richer status and UI icons

### Summary

Status icons use plain Unicode circles/ticks. Replace them with a consistent, visually distinct icon set and add a group-level status icon that reflects the worst status of its checks.

### Files likely involved

- `src/app.rs`
- `src/draw.rs`

### Requirements

Replace `CheckStatus::icon()` return values:

| Status | New icon |
|---|---|
| Pending | `·` |
| Running | `⟳` |
| Passed | `✔` |
| Failed | `✘` |
| Skipped | `⊘` |
| ManualPassed | `✔` |
| ManualFailed | `✘` |

Add a `group_status_icon(group_idx)` helper on `App` that returns:
- `✘` if any check in the group failed
- `✔` if all selected checks in the group passed
- `⟳` if any check is running
- `·` otherwise

Show this icon beside the group label in the check list.

Add a mode icon to the runner header beside the mode string:

| Mode | Icon |
|---|---|
| Selecting | `⊙` |
| Running | `⟳` |
| Done (all pass) | `✔` |
| Done (any fail) | `✘` |
| Awaiting verdict | `?` |

### Acceptance Criteria

- All new icons are visually distinct in a standard terminal font.
- Group icon updates live as checks complete.
- Mode icon in header is correct for every mode.

---

## Issue F-04 — Expand and standardise the colour palette

### Summary

The current colour usage is inconsistent: some borders use `Color::White` implicitly, the cursor highlight uses a hard `DarkGray` background that is invisible on light terminals, and status colours are scattered inline rather than centralised.

### Files likely involved

- `src/draw.rs`

### Requirements

- Extract a `theme` module (or a `const` block) in `draw.rs` that defines named colours for:
  - `BORDER_FOCUSED`, `BORDER_UNFOCUSED`
  - `STATUS_PASS`, `STATUS_FAIL`, `STATUS_RUNNING`, `STATUS_SKIP`, `STATUS_PENDING`
  - `GROUP_HEADER_DEFAULT`
  - `CURSOR_BG`, `CURSOR_FG`
  - `ERROR_FG`, `WARN_FG`, `INFO_FG`
  - `HEADER_TITLE`, `HEADER_PATH`, `HEADER_BRANCH`
- Replace all scattered `Color::*` literals in rendering functions with these named constants.
- Change the cursor highlight from a plain `DarkGray` background to a `Color::Rgb(40, 40, 80)` background with `Color::White` foreground so it is visible on both dark and light themes.
- Add `Color::Rgb` support to `color_from_str` for hex-like strings e.g. `"#3a7bd5"` (parse `#RRGGBB`).

### Acceptance Criteria

- All status colours reference the theme constants, not inline literals.
- Cursor is clearly visible on a dark terminal.
- `color_from_str("#1a2b3c")` returns `Color::Rgb(0x1a, 0x2b, 0x3c)`.
- `cargo test` passes.

---

## Issue F-05 — Interactive JSON config file picker (filesystem browser)

### Summary

Config discovery is silent and automatic. When no config is found, or when the user wants to pick a file not in the standard search paths, there is no way to browse the filesystem. Add a dedicated file-picker overlay on the setup screen.

### Files likely involved

- `src/app.rs`
- `src/draw.rs`
- `src/main.rs`

### Requirements

- Add a new `AppMode::FilePicker { origin: FilePickerOrigin }` where `FilePickerOrigin` is `Config`.
- The picker is opened from the Config field with a new key (e.g. `o` for "open").
- The picker shows a scrollable directory listing of the current browse directory.
- Entries are displayed as `📁 dirname/` or `📄 filename.json` (filter to `.json` files and directories only).
- `↑`/`↓` moves the cursor; `Enter` enters a directory or selects a file; `Backspace` goes up one level; `Esc` cancels without changing the selection.
- On file selection, validate it as a `ChecksConfig`; show an inline error if invalid; close the picker on success.
- Persist the selected path in `App.config_paths` if not already present.

### Acceptance Criteria

- Pressing `o` on the Config field opens the picker.
- User can navigate directories and select a `.json` file.
- Invalid JSON files show an error instead of crashing.
- `Esc` returns to setup with the previous config unchanged.
- Selecting a valid file loads its checks immediately.

---

## Issue F-06 — Interactive project path selector (filesystem browser)

### Summary

The project path field is a free-text input. Add tab-completion / directory picker so the user does not need to type an absolute path from memory.

### Files likely involved

- `src/app.rs`
- `src/draw.rs`
- `src/main.rs`

### Requirements

- Reuse the file-picker mechanism from F-05 with `FilePickerOrigin::ProjectPath`.
- Open with `o` key when the Project path field is focused.
- Only directories are shown and selectable (no file extension filter).
- On selection, close the picker, populate `setup_project_path`, and immediately run `detect_current_branch`.
- Directory entries show a trailing `/`.

### Acceptance Criteria

- `o` on the Project path field opens a directory browser.
- Only directories are shown.
- Selecting a directory closes the picker, fills the field, and shows the detected branch.
- Resolves to an absolute path.

> **Dependency:** Implement after F-05 to reuse the picker infrastructure.

---

## Issue F-07 — Interactive branch selector (git branch list)

### Summary

The branch field is free-text. Add a picker that reads local Git branches for the selected project and lets the user choose with `↑`/`↓`.

### Files likely involved

- `src/app.rs`
- `src/draw.rs`
- `src/main.rs`

### Requirements

- Add `get_local_branches(repo: &PathBuf) -> Vec<String>` using `git branch --list --format=%(refname:short)`.
- Add a `BranchPicker` overlay mode (can reuse the generic List overlay pattern from F-05).
- Open with `o` when the Branch field is focused (only if a valid project path is already set).
- Show branch list, current branch highlighted.
- `↑`/`↓` navigates; `Enter` selects; `Esc` cancels.
- Typed characters filter the list in real time.
- Remote branches (`git branch -r`) are shown in a separate section below local branches.

### Acceptance Criteria

- `o` on Branch field opens the picker (only when project path is a valid repo).
- List shows all local branches.
- Filtering by typing narrows the list.
- Selecting sets `setup_branch`.
- Non-Git repos show a "Not a git repository" message instead of an empty list.

> **Dependency:** Implement after F-06 (project path must be resolved first).

---

## Issue F-08 — Output pane scroll indicators and keyboard scroll improvements

### Summary

The output pane has no visual indication of how far through the log the user is, and the scroll position does not reset when switching between checks. `PageUp`/`PageDown` jump by a fixed 20 lines regardless of terminal height.

### Files likely involved

- `src/draw.rs`
- `src/app.rs`

### Requirements

- Add a scrollbar to the right edge of the output pane (use `ratatui::widgets::Scrollbar` introduced in ratatui 0.24+).
- Show current scroll position as `line N / total` in the output pane title bar.
- Reset `output_scroll` to the bottom when switching to a new check's log in `list_up`/`list_down`.
- Change `page_up`/`page_down` to use the actual visible pane height (store `output_pane_height: u16` in `App`, set each draw tick) rather than a hardcoded 20.
- Add `Home` key to scroll to top and `End` key to scroll to bottom.
- Auto-tail (scroll-to-bottom) should only engage when `output_scroll` is already within 2 lines of the tail; otherwise a user who scrolled up should not have their position yanked back.

### Acceptance Criteria

- Scrollbar is visible and positioned correctly.
- Title shows `Output (live) — line 42/120`.
- `Home` / `End` jump to top/bottom.
- Manual scroll-up stops auto-tail; reaching bottom re-enables it.
- Page size matches the visible pane height.

---

## Issue F-09 — Persist run audit log with timestamp index

### Summary

`last_run.log` is overwritten on every run, so previous runs are lost. Add a timestamped audit directory so every run's output is permanently archived.

### Files likely involved

- `src/app.rs`

### Requirements

- After every completed run, write a dated log file:
  `<repo_root>/.autocheck/runs/YYYY-MM-DDTHH-MM-SS.log`
- The file format is identical to the current `last_run.log`.
- Create `.autocheck/runs/` if it does not exist.
- Keep `last_run.log` and `last_failed.log` in place for backward compatibility.
- Add a `.autocheck/runs/index.txt` that appends one line per run:
  `YYYY-MM-DDTHH-MM-SS  pass:N fail:N skip:N  <branch>`
- Trim the index to the most recent 100 entries.

### Acceptance Criteria

- After each run a new dated file appears in `.autocheck/runs/`.
- `index.txt` has one entry per run.
- `last_run.log` still exists and is still current.
- Re-running without changes does not corrupt existing entries.
- `cargo test` passes (no file-system side effects in unit tests).

---

## Issue F-10 — Audit log viewer (browse previous runs inside the TUI)

### Summary

The audit logs written by F-09 are inaccessible from inside the app. Add a log audit browser so users can review previous runs without leaving the terminal.

### Files likely involved

- `src/app.rs`
- `src/draw.rs`
- `src/main.rs`

### Requirements

- Add `AppMode::AuditViewer` entered from the Done screen with key `L` (capital L).
- The viewer shows a two-panel layout:
  - Left panel: scrollable list of past runs from `index.txt` (most recent first), showing timestamp, branch, pass/fail/skip counts.
  - Right panel: full content of the selected run's log file, scrollable with `↑`/`↓` / `PageUp`/`PageDown`.
- `Esc` or `q` returns to the Done screen.
- If no audit directory exists, show a message: `No previous runs found. Complete a run first.`
- Runs with any failures are highlighted in red in the left panel; all-pass runs in green.

### Acceptance Criteria

- `L` on the Done screen opens the audit viewer.
- Left panel lists runs in reverse-chronological order.
- Right panel shows the full log of the highlighted run.
- Navigation is keyboard-only.
- `Esc` / `q` exits back to Done.

> **Dependency:** Implement after F-09.

---

## Recommended Implementation Order

```
F-01  (alignment)        — no dependencies
F-02  (list scrolling)   — no dependencies
F-03  (icons)            — no dependencies
F-04  (colours)          — no dependencies
F-05  (config picker)    — no dependencies
F-06  (path picker)      — depends on F-05 (reuse picker)
F-07  (branch picker)    — depends on F-06
F-08  (scroll UX)        — no dependencies
F-09  (audit log write)  — no dependencies
F-10  (audit viewer)     — depends on F-09
```

F-01 through F-04 and F-08 can all be done in **parallel** (UI only, no shared state changes).  
F-05, F-06, F-07 are **sequential** (each reuses the previous picker).  
F-09 and F-10 are **sequential**.
