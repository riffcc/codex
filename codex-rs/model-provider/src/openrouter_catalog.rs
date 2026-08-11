use std::collections::HashMap;

use codex_models_manager::model_info::BASE_INSTRUCTIONS;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::openai_models::ModelVisibility;
use codex_protocol::openai_models::TruncationPolicyConfig;
use codex_protocol::openai_models::WebSearchToolType;
use serde::Deserialize;

const MODELS_PATH: &str = "/models";

/// Parsed OpenRouter `/models` catalog, keyed by model id.
///
/// Fetched once per session (see
/// `OpenAiModelsEndpoint::resolve_native_model_info`) so free-typed slugs
/// resolve with accurate metadata without shipping a curated model list. The
/// catalog is resolution-only — it never feeds the model picker.
#[derive(Debug, Default)]
pub(crate) struct OpenRouterCatalog {
    models: HashMap<String, ModelInfo>,
}

impl OpenRouterCatalog {
    /// Fetch and parse the full OpenRouter model catalog from `{base_url}/models`.
    ///
    /// The endpoint is public (no auth), so this works before any API key is
    /// configured. Any transport, status, or parse error propagates so the
    /// caller can log and leave the session cache unset, retrying on the next
    /// resolution rather than caching a failure.
    pub(crate) async fn fetch(
        client: &reqwest::Client,
        base_url: &str,
    ) -> Result<Self, reqwest::Error> {
        let url = format!("{base_url}{MODELS_PATH}");
        tracing::info!(catalog_url = %url, "openrouter: fetching model catalog");
        let response = client.get(&url).send().await?.error_for_status()?;
        let parsed: OpenRouterModelsResponse = response.json().await?;
        Ok(Self::from_entries(parsed.data))
    }

    fn from_entries(entries: Vec<OpenRouterModelEntry>) -> Self {
        let mut models = HashMap::with_capacity(entries.len());
        for entry in entries {
            if entry.id.is_empty() {
                continue;
            }
            let id = entry.id.clone();
            models.insert(id, entry_to_model_info(&entry));
        }
        Self { models }
    }

    pub(crate) fn get(&self, model: &str) -> Option<&ModelInfo> {
        self.models.get(model)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.models.len()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    #[serde(default)]
    data: Vec<OpenRouterModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelEntry {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    context_length: Option<i64>,
    #[serde(default)]
    architecture: OpenRouterArchitecture,
}

#[derive(Debug, Default, Deserialize)]
struct OpenRouterArchitecture {
    #[serde(default)]
    input_modalities: Vec<String>,
}

/// Map a raw OpenRouter catalog entry onto full `ModelInfo` metadata.
///
/// Accurate fields (`display_name`, `description`, `context_window`,
/// `max_context_window`, `input_modalities`) come from the catalog; the rest
/// use conservative defaults so a tool-capable routed model behaves like a
/// standard Codex-compatible chat model over the Chat wire.
fn entry_to_model_info(entry: &OpenRouterModelEntry) -> ModelInfo {
    let display_name = entry
        .name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| entry.id.clone());
    ModelInfo {
        slug: entry.id.clone(),
        display_name,
        description: entry
            .description
            .clone()
            .filter(|description| !description.trim().is_empty()),
        default_reasoning_level: None,
        supported_reasoning_levels: Vec::new(),
        shell_type: ConfigShellToolType::Default,
        visibility: ModelVisibility::None,
        supported_in_api: true,
        priority: 0,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        availability_nux: None,
        upgrade: None,
        base_instructions: BASE_INSTRUCTIONS.to_string(),
        model_messages: None,
        supports_reasoning_summaries: false,
        default_reasoning_summary: ReasoningSummary::Auto,
        support_verbosity: false,
        default_verbosity: None,
        apply_patch_tool_type: None,
        web_search_tool_type: WebSearchToolType::Text,
        truncation_policy: TruncationPolicyConfig::bytes(/*limit*/ 10_000),
        supports_parallel_tool_calls: false,
        supports_image_detail_original: false,
        context_window: entry.context_length,
        max_context_window: entry.context_length,
        auto_compact_token_limit: None,
        effective_context_window_percent: 95,
        experimental_supported_tools: Vec::new(),
        input_modalities: parse_input_modalities(&entry.architecture.input_modalities),
        used_fallback_model_metadata: false,
        supports_search_tool: false,
        use_responses_lite: false,
        auto_review_model_override: None,
        tool_mode: None,
        multi_agent_version: None,
    }
}

fn parse_input_modalities(raw: &[String]) -> Vec<InputModality> {
    let mut out = Vec::new();
    for value in raw {
        match value.to_ascii_lowercase().as_str() {
            "text" if !out.contains(&InputModality::Text) => out.push(InputModality::Text),
            "image" if !out.contains(&InputModality::Image) => out.push(InputModality::Image),
            _ => {}
        }
    }
    if out.is_empty() {
        out.push(InputModality::Text);
    }
    out
}

#[cfg(test)]
#[path = "openrouter_catalog_tests.rs"]
mod tests;
