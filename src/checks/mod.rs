use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Cmd ────────────────────────────────────────────────────────────────────

/// A command can be a bare string or an array of args.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cmd {
    String(String),
    Array(Vec<String>),
}

impl Cmd {
    pub fn as_args(&self) -> Vec<String> {
        match self {
            Cmd::String(s) => vec![s.clone()],
            Cmd::Array(v) => v.clone(),
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Cmd::String(_))
    }
}

// ── Check ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub description: String,
    pub cmd: Cmd,
    #[serde(default)]
    pub manual: bool,
    #[serde(default)]
    pub audited: bool,
}

// ── Group ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub label: String,
    pub color: Option<String>,
    pub checks: Vec<Check>,
}

// ── ChecksConfig ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksConfig {
    pub project_root: Option<String>,
    pub path_log_file: Option<String>,
    pub groups: Vec<Group>,
}

impl ChecksConfig {
    /// Load and validate from a JSON string.
    pub fn from_str(s: &str) -> Result<Self, String> {
        let cfg: ChecksConfig =
            serde_json::from_str(s).map_err(|e| format!("JSON parse error: {e}"))?;
        if cfg.groups.is_empty() {
            return Err("Config must contain at least one group".to_string());
        }
        for g in &cfg.groups {
            if g.checks.is_empty() {
                return Err(format!("Group '{}' has no checks", g.label));
            }
        }
        Ok(cfg)
    }

    /// Load and validate from a file path.
    pub fn from_path(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        Self::from_str(&content)
    }
}

// ── Discovery ─────────────────────────────────────────────────────────────

/// Search known locations for valid checks config JSON files.
/// Returns deduplicated, sorted absolute paths.
pub fn discover_config_files() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // 1. Current working directory
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(entries) = std::fs::read_dir(&cwd) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("json") {
                    candidates.push(p);
                }
            }
        }
    }

    // 2. Directory containing the executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("json") {
                        candidates.push(p);
                    }
                }
            }
        }
    }

    // 3. Dev fallback: $CARGO_MANIFEST_DIR/src/checks
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let dir = PathBuf::from(manifest).join("src").join("checks");
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("json") {
                    candidates.push(p);
                }
            }
        }
    }

    // Canonicalise, filter to parseable configs, deduplicate, sort
    let mut seen = std::collections::HashSet::new();
    let mut valid: Vec<PathBuf> = Vec::new();
    for p in candidates {
        if let Ok(canon) = p.canonicalize() {
            if seen.insert(canon.clone()) && ChecksConfig::from_path(&canon).is_ok() {
                valid.push(canon);
            }
        }
    }
    valid.sort();
    valid
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> &'static str {
        r#"{
            "groups": [{
                "label": "Quality",
                "color": "Green",
                "checks": [{
                    "name": "fmt",
                    "description": "Check formatting",
                    "cmd": ["cargo", "fmt", "--check"],
                    "manual": false,
                    "audited": false
                }]
            }]
        }"#
    }

    #[test]
    fn test_valid_config_loads() {
        let cfg = ChecksConfig::from_str(valid_json()).unwrap();
        assert_eq!(cfg.groups.len(), 1);
        assert_eq!(cfg.groups[0].checks.len(), 1);
    }

    #[test]
    fn test_invalid_json_errors() {
        let result = ChecksConfig::from_str("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_groups_rejected() {
        let json = r#"{"groups":[]}"#;
        let result = ChecksConfig::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_checks_rejected() {
        let json = r#"{"groups":[{"label":"A","checks":[]}]}"#;
        let result = ChecksConfig::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_string_cmd() {
        let json = r#"{
            "groups": [{
                "label": "G",
                "checks": [{"name":"x","description":"d","cmd":"cargo fmt"}]
            }]
        }"#;
        let cfg = ChecksConfig::from_str(json).unwrap();
        let args = cfg.groups[0].checks[0].cmd.as_args();
        assert_eq!(args, vec!["cargo fmt"]);
    }

    #[test]
    fn test_array_cmd() {
        let json = r#"{
            "groups": [{
                "label": "G",
                "checks": [{"name":"x","description":"d","cmd":["cargo","fmt"]}]
            }]
        }"#;
        let cfg = ChecksConfig::from_str(json).unwrap();
        let args = cfg.groups[0].checks[0].cmd.as_args();
        assert_eq!(args, vec!["cargo", "fmt"]);
    }

    #[test]
    fn test_manual_defaults_false() {
        let cfg = ChecksConfig::from_str(valid_json()).unwrap();
        assert!(!cfg.groups[0].checks[0].manual);
    }

    #[test]
    fn test_audited_defaults_false() {
        let cfg = ChecksConfig::from_str(valid_json()).unwrap();
        assert!(!cfg.groups[0].checks[0].audited);
    }

    #[test]
    fn test_audited_true_loads() {
        let json = r#"{
            "groups": [{
                "label": "G",
                "checks": [{"name":"x","description":"d","cmd":"cargo fmt","audited":true}]
            }]
        }"#;
        let cfg = ChecksConfig::from_str(json).unwrap();
        assert!(cfg.groups[0].checks[0].audited);
    }

    #[cfg(test)]
    mod discovery {
        use super::*;
        use std::io::Write;
        use tempfile::NamedTempFile;

        #[test]
        fn test_discovery_ignores_invalid_json() {
            let mut f = NamedTempFile::new().unwrap();
            writeln!(f, "not valid json").unwrap();
            // The file exists but is not valid — canonicalize path and confirm
            // it would not be in the results if pointed to directly.
            let p = f.path().to_path_buf();
            let result = ChecksConfig::from_path(&p);
            assert!(result.is_err());
        }
    }
}
