# Feature: Setup Screen Tabs

Add a tabbed interface to the setup screen. The existing form becomes the **Run** tab.
A new **Settings** tab exposes all persistent configuration options that are currently
hardcoded or hidden in `SavedConfig` / `ChecksConfig`.

---

## Issue T-1 — Add tab state to App and wire Tab/Shift+Tab

### Summary

Track which setup tab is active (`Run` or `Settings`) in `App` state and switch between
them with keyboard input.

### Files likely involved

- `src/app.rs`
- `src/main.rs`

### Requirements

- Add `pub setup_tab: SetupTab` to `App`.
- Add `pub enum SetupTab { Run, Settings }` (derives `Clone`, `PartialEq`).
- `SetupTab` starts as `Run`.
- Add `App::setup_tab_next()` and `App::setup_tab_prev()` that cycle through the two tabs.
- In `handle_key` for `AppMode::Setup`, map:
  - `F1` → `SetupTab::Run`
  - `F2` → `SetupTab::Settings`
  - When `config_dropdown_open` is false, `Tab` key no longer calls `setup_focus_next()` —
    instead if focus is on the *last* field of the current tab, advance to the next tab
    (wrap-around). If not on the last field, advance focus within the tab as before.
    `Shift+Tab` (`BackTab`) moves backward similarly.
- `Esc` always closes any dropdown and does **not** change tab.

### Acceptance Criteria

- `App::new()` starts with `setup_tab == SetupTab::Run`.
- Pressing `F1` / `F2` switches tabs.
- `cargo check` passes.

---

## Issue T-2 — Render the tab bar at the top of the setup screen

### Summary

Replace the plain title `Paragraph` on the setup screen with a Ratatui `Tabs` widget
that shows `Run` and `Settings` as tab titles.

### Files likely involved

- `src/draw.rs`

### Requirements

- Import `ratatui::widgets::Tabs`.
- In `draw_setup`, replace the title `Paragraph` / `Borders::BOTTOM` block with a `Tabs`
  widget in the same 3-line slot (`chunks[0]`).
- Tab titles: `" Run "` and `" Settings "`.
- The selected tab is highlighted (`Color::Cyan` fg, `Modifier::BOLD`); inactive tabs use
  `Color::DarkGray`.
- The `Tabs` block uses `Borders::BOTTOM` and the same dim border style already in use
  (`Color::Rgb(60, 60, 100)`).
- Pass `app.setup_tab` (as a `usize` index) to `Tabs::select()`.
- Below the tab bar, render only the content for the active tab (delegate to two private
  helpers: `draw_setup_run_tab` and `draw_setup_settings_tab`).

### Acceptance Criteria

- Both tabs are visible in the tab bar.
- The active tab is visually distinct.
- Switching tabs (via T-1) changes which content is shown.
- `cargo check` passes.

---

## Issue T-3 — Move the Run tab content into its own helper

### Summary

Extract the existing project-path / branch / config-selector / error / checkout-log /
help-bar rendering from `draw_setup` into a dedicated `draw_setup_run_tab` function.
No behaviour change — this is a pure refactor to make T-4 clean.

### Files likely involved

- `src/draw.rs`

### Requirements

- Create `fn draw_setup_run_tab(f: &mut Frame, app: &App, area: Rect)`.
- Move the form rows, config dropdown, checkout log, and help bar into it unchanged.
- `draw_setup` calls this function when `app.setup_tab == SetupTab::Run`.
- No logic changes.

### Acceptance Criteria

- Run tab looks identical to the setup screen before this change.
- `cargo test` passes.

---

## Issue T-4 — Add Settings tab fields to App state

### Summary

Add the editable fields shown on the Settings tab to `App` and to `SavedConfig`, so they
can be persisted between sessions.

### Files likely involved

- `src/app.rs`
- `src/config.rs`

### Requirements

Add to `SavedConfig` (new `key=value` lines):

| Key | Default | Description |
|---|---|---|
| `run_log_path` | `last_run.log` | Path for `last_run.log` written after every run |
| `failed_log_path` | `last_failed.log` | Path for failed-checks log (overrides per-config value) |
| `audit_dir` | `.autocheck/runs` | Directory where timestamped audit logs are archived |
| `mouse_capture_default` | `true` | Whether mouse capture is enabled at startup |

- `SavedConfig::parse` reads these new keys; unknown keys are still silently ignored.
- `SavedConfig::to_string_repr` writes all fields.
- Round-trip tests cover the new keys.

Add to `App`:

```rust
// Settings tab fields (mirrors SavedConfig extra keys)
pub settings_run_log_path: String,
pub settings_failed_log_path: String,
pub settings_audit_dir: String,
pub settings_mouse_default: bool,
pub settings_focus: SettingsField,
```

- Add `pub enum SettingsField { RunLogPath, FailedLogPath, AuditDir, MouseDefault }`.
- Initialise from `SavedConfig` in `App::new()`.
- Add `App::settings_focus_next()` and `App::settings_focus_prev()`.
- Add `App::settings_type_char(c: char)` — appends to focused text field; ignored for
  `MouseDefault`.
- Add `App::settings_backspace()` — removes last char from focused text field.
- Add `App::settings_toggle_mouse()` — flips `settings_mouse_default`.

### Acceptance Criteria

- `SavedConfig` round-trips the four new keys.
- `App::new()` initialises settings fields from saved config.
- `cargo test` passes (existing tests still pass; new round-trip tests added).

---

## Issue T-5 — Render the Settings tab content

### Summary

Draw the Settings tab with editable fields for the four configurable paths/flags.

### Files likely involved

- `src/draw.rs`

### Requirements

Create `fn draw_setup_settings_tab(f: &mut Frame, app: &App, area: Rect)`.

Layout (all inside the tab content area, vertical stack):

```
┌─ Run log path ─────────────────────────────────────────┐
│  last_run.log                                           │
└─────────────────────────────────────────────────────────┘
┌─ Failed log path ───────────────────────────────────────┐
│  last_failed.log                                        │
└─────────────────────────────────────────────────────────┘
┌─ Audit archive dir ─────────────────────────────────────┐
│  .autocheck/runs                                        │
└─────────────────────────────────────────────────────────┘
┌─ Mouse capture on startup  [✓ enabled / ○ disabled] ───┐
└─────────────────────────────────────────────────────────┘
  (info line)  Settings saved to ~/.config/autocheck/config
  (help bar)   Tab next  ↑ prev  Backspace edit  Space toggle  q quit
```

- Each text field uses the same focused/unfocused border style as the Run tab fields
  (`BorderType::Thick` when focused, `BorderType::Rounded` otherwise; yellow when
  focused, `Rgb(80,80,100)` otherwise).
- `MouseDefault` field renders a toggle line: `[✓] enabled` in green or `[ ] disabled`
  in gray, toggled with `Space`.
- An info line at the bottom shows the save path in `DarkGray`.
- The help bar shows Settings-specific key hints.

### Acceptance Criteria

- All four fields are visible on the Settings tab.
- Focused field has a yellow thick border.
- `cargo check` passes.

---

## Issue T-6 — Wire Settings tab keyboard input

### Summary

Hook up typing, backspace, and Space to the Settings tab fields in the key handler.

### Files likely involved

- `src/main.rs`
- `src/app.rs`

### Requirements

In `handle_key` for `AppMode::Setup`, when `app.setup_tab == SetupTab::Settings`:

- `Tab` / `Down` → `app.settings_focus_next()`
- `Shift+Tab` / `Up` → `app.settings_focus_prev()`
- `Backspace` → `app.settings_backspace()`
- `Char(c)` → `app.settings_type_char(c)` (only for text fields; `MouseDefault` ignores
  char input)
- `Space` → `app.settings_toggle_mouse()` when focus is on `SettingsField::MouseDefault`
- `Enter` → save settings (call `app.save_settings()`, defined below) and switch tab back
  to `SetupTab::Run`
- `q` → quit (unchanged)

Add `App::save_settings(&self)`:
- Construct `SavedConfig` from current `App` state (repo, branch, selected config, plus
  the four new settings fields).
- Call `SavedConfig::save()`.
- Any IO error is silently ignored (same pattern as the rest of the app).

### Acceptance Criteria

- Typing updates the field text.
- Backspace removes characters.
- Space toggles the mouse flag.
- Enter saves and switches back to Run tab.
- `cargo test` passes.

---

## Issue T-7 — Use Settings values at runtime

### Summary

Replace the hardcoded `"last_run.log"`, `"last_failed.log"`, `.autocheck/runs` paths and
the startup mouse-capture value with the values from `App`'s settings fields.

### Files likely involved

- `src/app.rs`

### Requirements

- In `App::new()`, set `app.mouse_capture = app.settings_mouse_default` instead of
  `true`.
- In `write_logs()`, replace the hardcoded `"last_run.log"` with
  `self.settings_run_log_path` and the audit dir `.autocheck/runs` with
  `self.settings_audit_dir`.
- The `last_failed.log` path already respects `ChecksConfig::path_log_file`; add a
  fallback to `self.settings_failed_log_path` instead of the literal
  `"last_failed.log"`.

### Acceptance Criteria

- Changing the run log path in Settings and running checks writes to the new path.
- Mouse capture respects the saved default.
- Existing tests still pass.

---

## Issue T-8 — Add tests for Settings tab state

### Summary

Cover the new Settings tab state machine with unit tests.

### Files likely involved

- `src/app.rs`
- `src/config.rs`

### Requirements

Test in `src/app.rs`:

- `App::new()` starts on `SetupTab::Run`.
- `settings_focus_next` cycles through all `SettingsField` variants.
- `settings_type_char` appends to the correct field.
- `settings_backspace` removes the last character.
- `settings_toggle_mouse` flips the flag.

Test in `src/config.rs`:

- `SavedConfig::parse` reads the four new keys.
- `SavedConfig::to_string_repr` writes the four new keys.
- Round-trip preserves all values.

### Acceptance Criteria

- `cargo test` passes with all new tests green.
- Tests are deterministic and do not require a real terminal or filesystem.

---

## Implementation Order

1. **T-1** — App state + key wiring (no UI change yet; compiles and tests pass).
2. **T-2** — Tab bar widget in draw.
3. **T-3** — Refactor Run tab into helper (no behaviour change).
4. **T-4** — Settings fields in App + SavedConfig (logic only, no draw).
5. **T-5** — Draw Settings tab (read-only display wired to new fields).
6. **T-6** — Key handler for Settings tab (makes fields editable).
7. **T-7** — Wire saved values into runtime behaviour.
8. **T-8** — Tests.
