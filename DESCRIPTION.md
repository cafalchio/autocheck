# AutoCheck - Interactive Terminal UI for Automated Task Execution

**Repository:** https://github.com/cafalchio/autocheck

A Rust-based Terminal User Interface (TUI) application that streamlines the execution of release workflows, CI pipelines, and repeated development tasks through an interactive, visual interface.

## Screenshots

<p align="center">
  <img src="screenshots/screen1.png" width="400" alt="Setup Screen - Run Checks Tab"/>
  <img src="screenshots/settings.png" width="400" alt="Setup Screen - Settings Tab"/>
</p>

*Left: Initial setup with project path, branch/PR selection, and config chooser. Right: Settings for log folder and stop-on-failure behavior.*

<p align="center">
  <img src="screenshots/runner.png" width="800" alt="Runner Screen"/>
</p>

*Main interface showing 85 checks in 5 groups with real-time system monitoring (CPU: 5%, MEM: 71%). Left panel: hierarchical check list with selection counts. Right panel: live output from selected check.*

---

## What It Does

AutoCheck transforms complex, multi-step workflows into organized, trackable checklists that can be executed interactively:

- **Visual Task Management**: Organize checks into color-coded groups
- **Interactive Execution**: Select checks, execute sequentially, monitor real-time output
- **Manual Verification**: Pause for human review and approval
- **Comprehensive Logging**: Auto-generate `last_run.log` and `last_failed.log` with per-check logs
- **Git Integration**: Branch detection and checkout support for PR testing
- **System Monitoring**: Real-time CPU and memory usage in header
- **Flexible Configuration**: JSON-based, version-controlled check definitions

## Primary Use Cases

**Release Automation** - Execute 50+ step checklists: version bumping, security scanning (Dependabot, pip-audit, cargo-vet), multi-language testing, container builds, SSO verification, documentation deployment, manual review gates.

**CI/CD Workflows** - Run pre-commit/pre-push checks locally: formatting, linting, unit/integration tests, security audits, build verification.

**Quality Gates** - Enforce standards: code coverage thresholds, static analysis (SonarQube, Semgrep), dependency scanning, SBOM generation.

**Deployment Verification** - Post-deployment validation: health checks, load testing, monitoring verification, upgrade path testing.

## Key Features

### Setup & Configuration
- Dual-tab interface (Run checks / Settings)
- Project path selection with auto-completion
- Branch/PR support with automatic detection
- Config file discovery and cycling
- Persistent settings (remembers last project, branch, config)
- Customizable log folder location
- Stop-on-failure option

### Check Organization & Execution
- Hierarchical groups with color-coded labels
- Bulk selection (toggle entire groups)
- Individual check control
- Real-time system monitoring (CPU/MEM)
- Progress tracking with completion percentage
- Status indicators: ✓ passed, ✗ failed, ○ pending, ◐ running
- Live output streaming with pagination
- Per-check logs (`group_check.log` format)
- Audit folder (`.autocheck_audited/`)

### User Interface
- Full keyboard navigation (`Tab/↓/↑`, `←/→`, `Space`, `Enter`, `q`)
- Mouse support (click to toggle, scroll output)
- Split-pane layout (checks list | output panel)
- Responsive design with proper scrolling
- Manual verification checks (pause for human approval)
- Command cancellation with proper cleanup

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

### The Problem
Manual workflows are error-prone and time-consuming:
- Forgotten steps in 50+ step processes
- Lost context when output scrolls away
- No audit trail of what was run
- Repetitive command typing
- Unclear progress tracking
- Difficult recovery from failures

### The Solution
**Single Interactive Interface** - All checks organized with visual hierarchy, no command memorization, clear progress indication.

**Visual Progress Tracking** - Color-coded status indicators, real-time system monitoring, group-level selection counts.

**Comprehensive Logging** - Per-check log files, dedicated audit folder, failed check tracking, full output with timestamps.

**Flexible Execution Control** - Select/deselect checks or groups, pause for manual verification, stop on failure or continue, cancel with cleanup.

**Reusable Configurations** - JSON-based definitions, version-controlled workflows, team sharing, multiple configs per project.

**Developer-Friendly** - Full keyboard navigation, optional mouse support, split-pane layout, responsive design.

## Perfect For

- **Release Engineers**: Multi-phase release checklists with 50+ steps
- **DevOps Teams**: Deployment verification and smoke tests
- **Development Teams**: Pre-commit quality gates
- **QA Engineers**: Manual and automated testing workflows
- **Security Teams**: Comprehensive security audit pipelines

AutoCheck turns tedious, error-prone manual workflows into reliable, repeatable, and auditable processes.