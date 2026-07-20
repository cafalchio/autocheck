use std::path::PathBuf;

/// Saved setup config — persisted at `~/.config/autocheck/config`
#[derive(Debug, Clone)]
pub struct SavedConfig {
    pub repo: String,
    pub branch: String,
    pub selected_config: String,
    pub run_log_path: String,
    pub failed_log_path: String,
    pub audit_dir: String,
    pub mouse_capture_default: bool,
}

impl Default for SavedConfig {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            selected_config: String::new(),
            run_log_path: "last_run.log".to_string(),
            failed_log_path: "last_failed.log".to_string(),
            audit_dir: ".autocheck/runs".to_string(),
            mouse_capture_default: true,
        }
    }
}

impl SavedConfig {
    /// Parse key=value lines; unknown keys are silently ignored.
    pub fn parse(content: &str) -> Self {
        let mut cfg = SavedConfig::default();
        for line in content.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("repo=") {
                cfg.repo = v.to_string();
            } else if let Some(v) = line.strip_prefix("branch=") {
                cfg.branch = v.to_string();
            } else if let Some(v) = line.strip_prefix("selected_config=") {
                cfg.selected_config = v.to_string();
            } else if let Some(v) = line.strip_prefix("run_log_path=") {
                cfg.run_log_path = v.to_string();
            } else if let Some(v) = line.strip_prefix("failed_log_path=") {
                cfg.failed_log_path = v.to_string();
            } else if let Some(v) = line.strip_prefix("audit_dir=") {
                cfg.audit_dir = v.to_string();
            } else if let Some(v) = line.strip_prefix("mouse_capture_default=") {
                cfg.mouse_capture_default = v == "true";
            }
            // unknown keys ignored
        }
        cfg
    }

    /// Serialise to key=value format.
    pub fn to_string_repr(&self) -> String {
        format!(
            "repo={}\nbranch={}\nselected_config={}\nrun_log_path={}\nfailed_log_path={}\naudit_dir={}\nmouse_capture_default={}\n",
            self.repo,
            self.branch,
            self.selected_config,
            self.run_log_path,
            self.failed_log_path,
            self.audit_dir,
            self.mouse_capture_default,
        )
    }

    /// Primary config path.
    pub fn config_path() -> Option<PathBuf> {
        dirs_path().map(|d| d.join("config"))
    }

    /// Legacy fallback path (pre_commit).
    pub fn legacy_path() -> Option<PathBuf> {
        home_dir().map(|h| h.join(".config").join("pre_commit").join("config"))
    }

    /// Load from disk; try primary then legacy; return default if neither found.
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return Self::parse(&content);
            }
        }
        if let Some(path) = Self::legacy_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return Self::parse(&content);
            }
        }
        SavedConfig::default()
    }

    /// Save to disk (creates parent directory if needed).
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, self.to_string_repr())?;
        }
        Ok(())
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

fn dirs_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".config").join("autocheck"))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_all_keys() {
        let content = "repo=/my/repo\nbranch=main\nselected_config=/some/config.json\n";
        let cfg = SavedConfig::parse(content);
        assert_eq!(cfg.repo, "/my/repo");
        assert_eq!(cfg.branch, "main");
        assert_eq!(cfg.selected_config, "/some/config.json");
    }

    #[test]
    fn test_missing_keys_default_empty() {
        let cfg = SavedConfig::parse("");
        assert_eq!(cfg.repo, "");
        assert_eq!(cfg.branch, "");
        assert_eq!(cfg.selected_config, "");
    }

    #[test]
    fn test_unknown_keys_ignored() {
        let content = "foo=bar\nrepo=/x\nbaz=qux\n";
        let cfg = SavedConfig::parse(content);
        assert_eq!(cfg.repo, "/x");
        assert_eq!(cfg.branch, "");
    }

    #[test]
    fn test_round_trip() {
        let cfg = SavedConfig {
            repo: "/repo".into(),
            branch: "dev".into(),
            selected_config: "/cfg.json".into(),
            ..Default::default()
        };
        let s = cfg.to_string_repr();
        let cfg2 = SavedConfig::parse(&s);
        assert_eq!(cfg2.repo, cfg.repo);
        assert_eq!(cfg2.branch, cfg.branch);
        assert_eq!(cfg2.selected_config, cfg.selected_config);
    }

    #[test]
    fn test_new_fields_round_trip() {
        let cfg = SavedConfig {
            run_log_path: "my_run.log".into(),
            failed_log_path: "my_failed.log".into(),
            audit_dir: ".custom/runs".into(),
            mouse_capture_default: false,
            ..Default::default()
        };
        let s = cfg.to_string_repr();
        let cfg2 = SavedConfig::parse(&s);
        assert_eq!(cfg2.run_log_path, "my_run.log");
        assert_eq!(cfg2.failed_log_path, "my_failed.log");
        assert_eq!(cfg2.audit_dir, ".custom/runs");
        assert_eq!(cfg2.mouse_capture_default, false);
    }

    #[test]
    fn test_new_fields_defaults() {
        let cfg = SavedConfig::default();
        assert_eq!(cfg.run_log_path, "last_run.log");
        assert_eq!(cfg.failed_log_path, "last_failed.log");
        assert_eq!(cfg.audit_dir, ".autocheck/runs");
        assert_eq!(cfg.mouse_capture_default, true);
    }

    #[test]
    fn test_mouse_capture_default_parse_bool() {
        let cfg_true = SavedConfig::parse("mouse_capture_default=true\n");
        assert_eq!(cfg_true.mouse_capture_default, true);
        let cfg_false = SavedConfig::parse("mouse_capture_default=false\n");
        assert_eq!(cfg_false.mouse_capture_default, false);
        let cfg_other = SavedConfig::parse("mouse_capture_default=yes\n");
        assert_eq!(cfg_other.mouse_capture_default, false);
    }
}
