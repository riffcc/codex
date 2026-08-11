//! App-level persistence glue for `RecentOpenRouterModels`.
//!
//! Mirrors the `persist_project_navigation_state` helper: wrap the codex_home
//! save in error logging + a user-visible error so a failed write never
//! silently loses the recents list.

use super::App;

impl App {
    /// Persist the recent OpenRouter models list to codex_home. Failures are
    /// logged and surfaced as a chat error rather than panicked on.
    pub(super) fn persist_recent_openrouter_models(&mut self, action: &str) {
        if let Err(err) = self
            .recent_openrouter_models
            .save(&self.config.codex_home)
        {
            tracing::warn!(error = %err, action, "failed to persist recent OpenRouter models");
            self.chat_widget
                .add_error_message(format!("Failed to save recent OpenRouter models: {err}"));
        }
    }

    /// Push the authoritative recents snapshot to the widget so the `/model`
    /// popup renders the current list. Called at startup and after every
    /// mutation (record / remove).
    pub(super) fn sync_recent_openrouter_models_to_widget(&mut self) {
        let snapshot: Vec<String> = self
            .recent_openrouter_models
            .iter()
            .map(String::from)
            .collect();
        self.chat_widget.set_recent_openrouter_models(snapshot);
    }
}
