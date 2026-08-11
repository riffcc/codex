//! Recent OpenRouter model slugs surfaced from the `/model` popup.
//!
//! The `/model` popup gains a "Recent OpenRouter models" drill-down row (only
//! when recents are non-empty). It opens a list of slugs; each slug opens a
//! small sub-menu to either switch to it (mid-session, re-using the existing
//! [`AppEvent::SwitchToOpenRouterModel`] path) or remove it from recents.
//!
//! The list is a display snapshot synced from the App's authoritative
//! [`RecentOpenRouterModels`] store. Removal refreshes the list in place via
//! `replace_active_views_with_selection_view`, which handles both the
//! "sub-menu still on top" and "list on top" cases without stacking views.

use super::*;

const RECENT_OPENROUTER_VIEW_ID: &str = "recent-openrouter-models";
const OPENROUTER_MODEL_MENU_VIEW_ID: &str = "openrouter-model-menu";

impl ChatWidget {
    /// Replace the display snapshot of recent OpenRouter model slugs. Called by
    /// the App after loading or mutating the authoritative recents store.
    pub(crate) fn set_recent_openrouter_models(&mut self, recents: Vec<String>) {
        self.recent_openrouter_models = recents;
    }

    /// The drill-down row shown in the `/model` popup, or `None` when there are
    /// no recents to surface.
    pub(crate) fn recent_openrouter_models_entry_item(&self) -> Option<SelectionItem> {
        if self.recent_openrouter_models.is_empty() {
            return None;
        }
        Some(SelectionItem {
            name: "Recent OpenRouter models".to_string(),
            description: Some(format!(
                "Reuse or remove a previously typed slug ({} saved).",
                self.recent_openrouter_models.len()
            )),
            actions: vec![Box::new(|tx| {
                tx.send(AppEvent::OpenRecentOpenRouterModelsPopup);
            })],
            dismiss_on_select: false,
            ..Default::default()
        })
    }

    /// Open the recents list. Each slug row opens the per-slug sub-menu.
    pub(crate) fn open_recent_openrouter_models_popup(&mut self) {
        let params = self.build_recent_openrouter_models_popup_params();
        self.bottom_pane.show_selection_view(params);
    }

    /// Rebuild the recents list and swap it into the active view stack in place.
    /// Called after a removal so the list reflects the change immediately
    /// without stacking a second popup.
    pub(crate) fn refresh_recent_openrouter_models_popup(&mut self) {
        let params = self.build_recent_openrouter_models_popup_params();
        // Covers both race states: the per-slug sub-menu may still be the top
        // view (its dismiss is async) or the list may already be on top.
        if !self.bottom_pane.replace_active_views_with_selection_view(
            &[RECENT_OPENROUTER_VIEW_ID, OPENROUTER_MODEL_MENU_VIEW_ID],
            params,
        ) {
            // Neither view is active (user navigated away) -- nothing to refresh.
        }
    }

    fn build_recent_openrouter_models_popup_params(&self) -> SelectionViewParams {
        let mut items: Vec<SelectionItem> = self
            .recent_openrouter_models
            .iter()
            .map(|slug| {
                let model = slug.clone();
                SelectionItem {
                    name: slug.clone(),
                    description: Some("Select to switch to or remove this slug.".to_string()),
                    actions: vec![Box::new(move |tx| {
                        tx.send(AppEvent::OpenRecentOpenRouterModelMenu {
                            model: model.clone(),
                        });
                    })],
                    dismiss_on_select: false,
                    ..Default::default()
                }
            })
            .collect();

        if items.is_empty() {
            items.push(SelectionItem {
                name: "No recent OpenRouter models yet.".to_string(),
                description: Some(
                    "Type a routed model id from the model menu to save it here.".to_string(),
                ),
                is_disabled: true,
                ..Default::default()
            });
        }

        let mut header = ColumnRenderable::new();
        header.push(Line::from("Recent OpenRouter models".bold()));
        header.push(Line::from(
            "Reuse a slug or remove one you no longer want.".dim(),
        ));

        SelectionViewParams {
            view_id: Some(RECENT_OPENROUTER_VIEW_ID),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            header: Box::new(header),
            ..Default::default()
        }
    }

    /// Open the per-slug sub-menu with a "Switch" and a "Remove" action.
    pub(crate) fn open_recent_openrouter_model_menu(&mut self, model: String) {
        let switch_model = model.clone();
        let remove_model = model.clone();
        let switch_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::SwitchToOpenRouterModel {
                model: switch_model.clone(),
            });
        })];
        let remove_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::RemoveRecentOpenRouterModel {
                model: remove_model.clone(),
            });
        })];

        let mut header = ColumnRenderable::new();
        header.push(Line::from(format!("OpenRouter · {model}").bold()));
        header.push(Line::from(
            "Switch the session to this model, or remove it from recents.".dim(),
        ));

        self.bottom_pane.show_selection_view(SelectionViewParams {
            view_id: Some(OPENROUTER_MODEL_MENU_VIEW_ID),
            footer_hint: Some(standard_popup_hint_line()),
            header: Box::new(header),
            items: vec![
                SelectionItem {
                    name: format!("Switch to {model}"),
                    description: Some(
                        "Re-use this slug: switch the current session to OpenRouter.".to_string(),
                    ),
                    actions: switch_actions,
                    dismiss_on_select: true,
                    ..Default::default()
                },
                SelectionItem {
                    name: "Remove from recents".to_string(),
                    description: Some(
                        "Drop this slug from the recent OpenRouter models list.".to_string(),
                    ),
                    actions: remove_actions,
                    dismiss_on_select: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
    }
}
