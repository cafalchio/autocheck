# AutoCheck

A Rust terminal UI (TUI) application for running pre-configured checks (linting, formatting, tests, etc.) on your codebase with an interactive interface.

## Features

- Interactive setup screen to pick project folder, branch/PR, and checks config
- Runner window for selecting and executing checks
- Live streaming output per check
- Manual pass/fail checks (human-in-the-loop verification)
- Per-check log retention and post-run log review
- `last_run.log` and `last_failed.log` written after every run
- System CPU and memory monitoring in the header
- Mouse support for toggling checks

---

## Build

```bash
cargo build --release
```

Run in place during development:

```bash
cargo run
```

---

## JSON Config Format

Place a `.json` config file in your project directory. AutoCheck discovers all valid JSON config files automatically.

```json
{
  "project_root": ".",
  "path_log_file": "last_failed.log",
  "groups": [
    {
      "label": "Code Quality",
      "color": "Green",
      "checks": [
        {
          "name": "fmt",
          "description": "Check Rust formatting",
          "cmd": ["cargo", "fmt", "--check"],
          "manual": false
        },
        {
          "name": "clippy",
          "description": "Run clippy linter",
          "cmd": ["cargo", "clippy", "--", "-D", "warnings"],
          "manual": false
        }
      ]
    },
    {
      "label": "Tests",
      "color": "Cyan",
      "checks": [
        {
          "name": "unit-tests",
          "description": "Run all unit tests",
          "cmd": "cargo test",
          "manual": false
        }
      ]
    },
    {
      "label": "Manual Review",
      "color": "Yellow",
      "checks": [
        {
          "name": "code-review",
          "description": "Review the changes manually before merging",
          "cmd": ["echo", "Please review the diff"],
          "manual": true
        }
      ]
    }
  ]
}
```

### Config Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `project_root` | string | no | Root path override |
| `path_log_file` | string | no | Path for failed log (default: `last_failed.log`) |
| `groups` | array | **yes** | One or more check groups |
| `groups[].label` | string | yes | Group display name |
| `groups[].color` | string | no | Group label color (`Green`, `Red`, `Blue`, `Yellow`, `Cyan`, `Magenta`) |
| `groups[].checks` | array | yes | One or more checks in the group |
| `checks[].name` | string | yes | Short check identifier |
| `checks[].description` | string | yes | Shown in details pane |
| `checks[].cmd` | string or array | yes | Command to run |
| `checks[].manual` | bool | no | If `true`, pauses for user verdict after command |

---

## Setup Screen

On launch you see the **Setup Screen**.

### Fields

- **Project path (required)** — absolute or relative path to your repo. The detected current Git branch is shown in the title.
- **Branch or PR number (optional)** — enter a branch name to `git switch` to it, or a PR number (digits only) to fetch and check out `origin/pull/<N>/head`.
- **Checks config** — cycle through discovered `.json` config files with `←` / `→`.

### Setup Keys

| Key | Action |
|---|---|
| `Tab` / `↓` | Move focus to next field |
| `↑` | Move focus to previous field |
| `←` / `→` | Cycle configs (when Config field is focused) |
| `Space` | Open/close config dropdown |
| `Esc` | Close config dropdown |
| `Enter` | Confirm setup and open runner |
| `Backspace` | Delete last character |
| `q` | Quit |

---

## Runner Screen

After confirming setup the **Runner Screen** opens.

### Layout

```
┌─ Header: project │ branch │ mode │ CPU % │ MEM % ─────────────────┐
│                                                                     │
│  Left pane (35%)         │  Right pane (65%)                       │
│  ▶ Group Name (N checks) │  Selected check description or log      │
│    [✓] ✓ check-name      │  (live output while running)            │
│    [ ] ○ check-name      │                                         │
│                                                                     │
├─ Footer: contextual help + staged file count + pass/fail/skip ──────┤
```

### Runner Keys

| Key | Mode | Action |
|---|---|---|
| `↑` / `↓` | Selecting / Done | Move cursor |
| `Space` | Selecting | Toggle check or group |
| `a` | Selecting | Select all checks |
| `n` | Selecting | Deselect all checks |
| `Enter` | Selecting | Start running selected checks |
| `Enter` | Done | Rerun same checks |
| `r` | Running / Done | Cancel / reset to selecting |
| `q` | Any | Quit (cancels active run) |
| `PageUp` / `PageDown` | Any | Scroll output |
| `m` | Any | Toggle mouse capture |
| `p` / `f` | Awaiting manual | Mark manual check pass / fail |

### Mouse

- **Left-click** on a check or group to toggle it.
- **Scroll wheel** to scroll output.
- Press `m` to disable mouse capture if needed (e.g., to copy text).

---

## Manual Checks

If a check has `"manual": true`, after its command finishes the app enters **Awaiting verdict** mode and displays:

```
── Manual check: press p (pass) or f (fail) ──
```

Review the output in the right pane, then press `p` to mark it passed or `f` to mark it failed. The runner continues to the next check.

---

## Log Files

After every run AutoCheck writes two files to the repo root:

| File | Contents |
|---|---|
| `last_run.log` | All checks, statuses, and output from the latest run |
| `last_failed.log` | Only checks that failed (or received manual fail verdict) |

Both files begin with a header showing the branch and staged file list.

The failed log path can be customised via `path_log_file` in the config.

---

## Saved Config

AutoCheck remembers your last project path, branch, and selected config at:

```
~/.config/autocheck/config
```

The file uses simple `key=value` format:

```
repo=/path/to/project
branch=main
selected_config=/path/to/checks.json
```

---

## Troubleshooting

**No config files found** — place a `.json` checks config in the current working directory, the directory containing the `autocheck` binary, or `src/checks/` during development.

**Branch detection shows `?`** — the project path is not a Git repository, or `git` is not on `PATH`.

**Checkout failed** — check that the branch name is correct and that your working tree is clean. The checkout output is shown in the setup log area.

**Checks not running** — confirm at least one check is selected (`a` to select all), and that `Enter` is pressed in **Selecting** mode.

**Mouse is stuck** — press `m` to toggle mouse capture off, allowing normal terminal selection.
