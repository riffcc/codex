//! Persistence for recently-used OpenRouter model slugs.
//!
//! Mirrors the codex_home JSON persistence pattern from `project_navigation`:
//! a missing file yields an empty list, corrupt JSON is swallowed to an empty
//! list at the public `load` boundary, and writes are pretty-printed after
//! ensuring the parent directory exists. The recents list powers the
//! "recent OpenRouter models" rows in the `/model` popup.

use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

const RECENT_OPENROUTER_MODELS_FILE: &str = "recent-openrouter-models.json";
const MAX_RECENT_OPENROUTER_MODELS: usize = 8;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecentOpenRouterModels {
    #[serde(default)]
    pub(crate) recents: Vec<String>,
}

impl RecentOpenRouterModels {
    /// Load recents from codex_home. A missing or unparseable file resolves to
    /// an empty list so startup never fails on a bad state file.
    pub(crate) fn load(codex_home: &Path) -> Self {
        Self::load_persisted(codex_home).unwrap_or_default()
    }

    pub(crate) fn save(&self, codex_home: &Path) -> io::Result<()> {
        let path = Self::state_file_path(codex_home);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|err| io::Error::other(format!("failed to serialize recent OpenRouter models: {err}")))?;
        fs::write(path, json)
    }

    pub(crate) fn state_file_path(codex_home: &Path) -> PathBuf {
        codex_home.join(RECENT_OPENROUTER_MODELS_FILE)
    }

    fn load_persisted(codex_home: &Path) -> io::Result<Self> {
        let path = Self::state_file_path(codex_home);
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(err),
        };
        serde_json::from_str(&raw).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("failed to parse recent OpenRouter models: {err}"),
            )
        })
    }

    /// Record a slug as most-recently-used: dedupe, promote to front, cap at
    /// `MAX_RECENT_OPENROUTER_MODELS`.
    pub(crate) fn record(&mut self, slug: String) {
        self.recents.retain(|existing| existing != &slug);
        self.recents.insert(0, slug);
        self.recents.truncate(MAX_RECENT_OPENROUTER_MODELS);
    }

    pub(crate) fn remove(&mut self, slug: &str) {
        self.recents.retain(|existing| existing != slug);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.recents.is_empty()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &str> {
        self.recents.iter().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_missing_file_returns_empty() {
        let dir = tempdir().unwrap();
        let loaded = RecentOpenRouterModels::load(dir.path());
        assert!(loaded.is_empty());
        assert_eq!(loaded.recents, Vec::<String>::new());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempdir().unwrap();
        let mut models = RecentOpenRouterModels::default();
        models.record("anthropic/claude-sonnet-4".to_string());
        models.record("google/gemma-2-9b".to_string());
        models.save(dir.path()).unwrap();

        let loaded = RecentOpenRouterModels::load(dir.path());
        assert_eq!(loaded, models);
    }

    #[test]
    fn record_promotes_existing_slug_to_front_without_duplicate() {
        let mut models = RecentOpenRouterModels::default();
        models.record("a".to_string());
        models.record("b".to_string());
        models.record("c".to_string());
        models.record("a".to_string());
        assert_eq!(models.recents, vec!["a".to_string(), "c".to_string(), "b".to_string()]);
    }

    #[test]
    fn record_caps_at_max_and_keeps_most_recent() {
        let mut models = RecentOpenRouterModels::default();
        for i in 0..(MAX_RECENT_OPENROUTER_MODELS + 3) {
            models.record(format!("m{i}"));
        }
        assert_eq!(models.recents.len(), MAX_RECENT_OPENROUTER_MODELS);
        // Most-recently-recorded is first; the oldest three (m0, m1, m2) drop.
        assert_eq!(models.recents[0], format!("m{}", MAX_RECENT_OPENROUTER_MODELS + 2));
        assert!(!models.recents.contains(&"m0".to_string()));
    }

    #[test]
    fn remove_drops_only_the_matching_slug() {
        let mut models = RecentOpenRouterModels::default();
        models.record("a".to_string());
        models.record("b".to_string());
        models.remove("a");
        assert_eq!(models.recents, vec!["b".to_string()]);
    }

    #[test]
    fn load_corrupt_json_falls_back_to_empty() {
        let dir = tempdir().unwrap();
        let path = RecentOpenRouterModels::state_file_path(dir.path());
        std::fs::write(path, "{ not valid json").unwrap();
        let loaded = RecentOpenRouterModels::load(dir.path());
        assert!(loaded.is_empty());
    }
}
