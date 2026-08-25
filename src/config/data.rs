use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DataConfig {
    pub dir: String,
    pub serve: Vec<String>,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            dir: "data".into(),
            serve: vec![],
        }
    }
}

impl DataConfig {
    pub fn may_serve(&self, path: &Path) -> bool {
        if self.serve.iter().any(|entry| entry == "*") {
            return true;
        }

        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            return false;
        };

        self.serve.iter().any(|entry| {
            entry
                .trim_start_matches('.')
                .eq_ignore_ascii_case(extension)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(serve: &[&str]) -> DataConfig {
        DataConfig {
            dir: "data".into(),
            serve: serve.iter().map(|entry| entry.to_string()).collect(),
        }
    }

    #[test]
    fn empty_allowlist_serves_nothing() {
        assert!(!config(&[]).may_serve(Path::new("a.zip")));
        assert!(!config(&[]).may_serve(Path::new("README")));
    }

    #[test]
    fn star_serves_everything_including_extensionless_files() {
        assert!(config(&["*"]).may_serve(Path::new("a.zip")));
        assert!(config(&["*"]).may_serve(Path::new("README")));
    }

    #[test]
    fn allowlist_matches_extension_case_insensitively_with_optional_dot() {
        assert!(config(&["zip"]).may_serve(Path::new("a.ZIP")));
        assert!(config(&[".ZIP"]).may_serve(Path::new("a.zip")));
        assert!(!config(&["zip"]).may_serve(Path::new("a.txt")));
        assert!(!config(&["zip"]).may_serve(Path::new("README")));
    }
}
