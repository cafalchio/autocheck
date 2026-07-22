# AutoCheck - Interactive Terminal UI for Automated Task Execution

**Repository:** https://github.com/cafalchio/autocheck

**AutoCheck** is a Rust-based Terminal User Interface (TUI) application that streamlines the execution of release workflows, CI pipelines, and any repeated development tasks through an interactive, visual interface.

## What It Does

AutoCheck transforms complex, multi-step workflows (like releases, deployments, or quality gates) into organized, trackable checklists that can be executed interactively. It provides:

### Screenshots

#### Setup Screen - Run Checks Tab
![Setup Screen](screenshots/screen1.png)
*Initial configuration screen with project path input, optional branch/PR selection, and config file chooser. Navigate between "Run checks" and "Settings" tabs.*

#### Setup Screen - Settings Tab
![Settings Screen](screenshots/settings.png)
*Configure log output folder and stop-on-failure behavior. Logs are written per check with format: `group_check.log`*

#### Runner Screen - Check Selection & Execution
![Runner Screen](screenshots/runner.png)
*Main interface showing 85 checks organized into 5 groups (Version Update, Python Dependencies, Rust & JavaScript, Quality Gates, Test Gates). Real-time system monitoring (CPU: 5%, MEM: 71%) in header. Left panel displays hierarchical check list with selection counts per group. Right panel shows output from selected check.*

---

- **Visual Task Management**: Organize checks into logical groups with color-coded labels
- **Interactive Execution**: Select which checks to run, execute them sequentially, and monitor real-time output
- **Manual Verification Support**: Pause execution for human review and approval (manual checks)
- **Comprehensive Logging**: Automatic generation of `last_run.log` and `last_failed.log` for audit trails
- **Git Integration**: Automatic branch detection and checkout support for PR testing
- **System Monitoring**: Real-time CPU and memory usage displayed in the header
- **Flexible Configuration**: JSON-based check definitions that can be version-controlled

## Primary Use Cases

### 1. **Release Automation**
Execute complex release checklists with 50+ steps including:
- Version bumping and dependency updates
- Security scanning (Dependabot, code scanning, pip-audit, cargo-vet)
- Multi-language testing (Python, Rust, JavaScript)
- Container builds and smoke tests
- SSO and observability verification
- Documentation deployment
- Manual review gates

### 2. **CI/CD Workflows**
Run pre-commit, pre-push, or CI pipeline checks locally:
- Code formatting and linting
- Unit and integration tests
- Security audits
- Build verification

### 3. **Quality Gates**
Enforce quality standards before merging:
- Code coverage thresholds
- Static analysis (SonarQube, Semgrep)
- Dependency vulnerability scanning
- SBOM generation

### 4. **Deployment Verification**
Post-deployment validation:
- Health checks
- Load testing
- Monitoring stack verification
- Upgrade path testing

## Key Features

### Setup & Configuration
- **Dual-Tab Interface**: Separate "Run checks" and "Settings" tabs for clean organization
- **Project Path Selection**: Specify any repository path with auto-completion
- **Branch/PR Support**: Optional branch or PR number input with automatic detection of current branch
- **Config File Discovery**: Automatic detection and cycling through available `.json` config files
- **Persistent Settings**: Remembers last used project path, branch, and configuration
- **Log Configuration**: Customize log output folder location (relative to repository)
- **Stop-on-Failure**: Optional setting to halt execution on first failed check

### Check Organization & Selection
- **Hierarchical Groups**: Organize checks into logical categories (Version Update, Dependencies, Quality Gates, etc.)
- **Color-Coded Labels**: Visual distinction between check groups for quick identification
- **Bulk Selection**: Toggle entire groups on/off with a single action
- **Individual Control**: Enable/disable specific checks within groups
- **Selection Counter**: Real-time display of selected checks per group (e.g., "6/6 selected")
- **Total Check Count**: Summary showing total checks available (e.g., "Checks (85)")
- **Collapsible Groups**: Expand/collapse groups to focus on relevant sections

### Execution & Monitoring
- **Real-Time System Monitoring**: Live CPU and memory usage displayed in header
- **Progress Tracking**: Visual progress bar showing completion percentage (0/72 checks)
- **Status Indicators**: Color-coded symbols for check states:
  - ✓ (green) - Passed
  - ✗ (red) - Failed  
  - ○ (gray) - Pending
  - ◐ (yellow) - Running
- **Live Output Streaming**: Real-time command output in dedicated panel
- **Output Pagination**: Navigate through output with "Output - 1/1" indicator
- **Branch Display**: Current branch shown in header (e.g., "branch: main")
- **Mode Indicator**: Shows current mode (Selecting, Running, etc.)

### Logging & Audit Trail
- **Per-Check Logs**: Individual log files for each check (format: `group_check.log`)
- **Structured Output**: Logs include timestamps, exit codes, and full command output
- **Audit Folder**: Dedicated `.autocheck_audited/` directory for log storage
- **Failed Check Tracking**: Automatic generation of `last_failed.log` for debugging
- **Run History**: Complete audit trail of all executions

### User Interface
- **Keyboard Navigation**: Full keyboard control with intuitive shortcuts
  - `Tab/↓` - Next field
  - `↑` - Previous field  
  - `←/→` - Cycle config files
  - `Space` - Toggle selection
  - `Enter` - Confirm/Save
  - `F1` - Switch to run tab
  - `Backspace` - Edit mode
  - `q` - Quit
- **Mouse Support**: Click to toggle checks, scroll output, or select items
- **Split-Pane Layout**: Checks list on left, output panel on right for efficient workflow
- **Responsive Design**: Adapts to terminal size with proper scrolling
- **Visual Feedback**: Clear indicators for focused fields and selected items

### Advanced Capabilities
- **Manual Verification Checks**: Pause execution for human review and approval
- **Command Cancellation**: Stop running checks with proper process cleanup
- **Git Integration**: Automatic branch detection and checkout support
- **Multi-Language Support**: Works with any language or tool (Python, Rust, JavaScript, etc.)
- **Flexible Commands**: Execute any shell command or script
- **Error Recovery**: Continue execution after failures (unless stop-on-failure enabled)

## Configuration Example

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
          "description": "Check formatting",
          "cmd": ["cargo", "fmt", "--check"],
          "manual": false
        },
        {
          "name": "code-review",
          "description": "Manual code review",
          "cmd": ["echo", "Review changes"],
          "manual": true
        }
      ]
    }
  ]
}
```

## Why Use AutoCheck?

### The Problem with Manual Workflows

Running complex workflows manually is error-prone and time-consuming:
- **Forgotten Steps**: Easy to skip critical checks in 50+ step release processes
- **Lost Context**: Command output scrolls away, making debugging difficult
- **No Audit Trail**: Hard to prove which checks were run and what failed
- **Repetitive Typing**: Same commands executed repeatedly across projects
- **Context Switching**: Jumping between terminal, documentation, and checklists
- **No Progress Tracking**: Unclear how far through the workflow you are
- **Difficult Recovery**: When a check fails, hard to resume from that point

### The AutoCheck Solution

**Single Interactive Interface**
- All checks organized in one place with visual hierarchy
- No need to remember or look up commands
- Clear progress indication with completion percentages

**Visual Progress Tracking**
- Color-coded status indicators (✓ passed, ✗ failed, ○ pending, ◐ running)
- Real-time system monitoring (CPU/memory usage)
- Group-level selection counts (e.g., "6/6 selected")

**Comprehensive Logging**
- Automatic per-check log files (`group_check.log` format)
- Dedicated audit folder (`.autocheck_audited/`)
- Failed check tracking in `last_failed.log`
- Full command output with timestamps and exit codes

**Flexible Execution Control**
- Select/deselect individual checks or entire groups
- Pause for manual verification steps
- Stop on first failure or continue through errors
- Cancel running checks with proper cleanup

**Reusable Configurations**
- JSON-based check definitions
- Version-controlled workflow files
- Share configurations across team
- Multiple configs per project (release.json, test.json, etc.)

**Developer-Friendly**
- Full keyboard navigation with intuitive shortcuts
- Optional mouse support for quick toggling
- Split-pane layout for simultaneous check list and output viewing
- Responsive design that adapts to terminal size

## Perfect For

- **Release Engineers**: Executing multi-phase release checklists with 50+ steps
- **DevOps Teams**: Running deployment verification and smoke tests
- **Development Teams**: Enforcing pre-commit quality gates
- **QA Engineers**: Coordinating manual and automated testing workflows
- **Security Teams**: Running comprehensive security audit pipelines

AutoCheck turns tedious, error-prone manual workflows into reliable, repeatable, and auditable processes.
