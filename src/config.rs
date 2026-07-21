use std::path::PathBuf;

/// Saved setup config — persisted at `~/.config/autocheck/config`
#[derive(Debug, Clone)]
pub struct SavedConfig {
    pub repo: String,
    pub branch: String,
    pub selected_config: String,
    pub log_folder: String,
}

impl Default for SavedConfig {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: String::new(),
            selected_config: String::new(),
            log_folder: String::new(),
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
            } else if let Some(v) = line.strip_prefix("log_folder=") {
                cfg.log_folder = v.to_string();
            }
            // unknown keys ignored
        }
        cfg
    }

    /// Serialise to key=value format.
    pub fn to_string_repr(&self) -> String {
        format!(
            "repo={}\nbranch={}\nselected_config={}\nlog_folder={}\n",
            self.repo, self.branch, self.selected_config, self.log_folder,
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
        let content = "repo=/my/repo\nbranch=main\nselected_config=/some/config.json\nlog_folder=logs\n";
        let cfg = SavedConfig::parse(content);
        assert_eq!(cfg.repo, "/my/repo");
        assert_eq!(cfg.branch, "main");
        assert_eq!(cfg.selected_config, "/some/config.json");
        assert_eq!(cfg.log_folder, "logs");
    }

    #[test]
    fn test_missing_keys_default_empty() {
        let cfg = SavedConfig::parse("");
        assert_eq!(cfg.repo, "");
        assert_eq!(cfg.branch, "");
        assert_eq!(cfg.selected_config, "");
        assert_eq!(cfg.log_folder, "");
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
            log_folder: "logs".into(),
        };
        let s = cfg.to_string_repr();
        let cfg2 = SavedConfig::parse(&s);
        assert_eq!(cfg2.repo, cfg.repo);
        assert_eq!(cfg2.branch, cfg.branch);
        assert_eq!(cfg2.selected_config, cfg.selected_config);
        assert_eq!(cfg2.log_folder, cfg.log_folder);
    }

}
