use super::*;
use codex_login::default_client::build_reqwest_client;
use codex_protocol::openai_models::InputModality;
use serde_json::json;
use wiremock::matchers::method;
use wiremock::matchers::path;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;

fn sample_payload() -> serde_json::Value {
    json!({
        "data": [
            {
                "id": "vendor/text-and-image",
                "name": "Vendor: Text & Image",
                "description": "A multimodal routed model.",
                "context_length": 200_000,
                "architecture": { "input_modalities": ["text", "image"] }
            },
            {
                "id": "vendor/text-only",
                "name": "Vendor: Text Only",
                "description": "   ",
                "context_length": 128_000,
                "architecture": { "input_modalities": ["text"] }
            },
            {
                "id": "vendor/no-metadata",
                "architecture": { "input_modalities": [] }
            },
            {
                "id": "",
                "name": "should be skipped"
            }
        ]
    })
}

#[tokio::test]
async fn fetch_parses_catalog_and_maps_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_payload()))
        .mount(&server)
        .await;

    let catalog = OpenRouterCatalog::fetch(&build_reqwest_client(), &server.uri())
        .await
        .expect("catalog should fetch");

    // Empty ids are dropped.
    assert_eq!(catalog.len(), 3);

    let multimodal = catalog
        .get("vendor/text-and-image")
        .expect("multimodal entry should resolve");
    assert_eq!(multimodal.slug, "vendor/text-and-image");
    assert_eq!(multimodal.display_name, "Vendor: Text & Image");
    assert_eq!(multimodal.description.as_deref(), Some("A multimodal routed model."));
    assert_eq!(multimodal.context_window, Some(200_000));
    assert_eq!(multimodal.max_context_window, Some(200_000));
    assert_eq!(
        multimodal.input_modalities,
        vec![InputModality::Text, InputModality::Image]
    );
    assert!(!multimodal.used_fallback_model_metadata);

    // Blank descriptions collapse to None rather than whitespace.
    let text_only = catalog
        .get("vendor/text-only")
        .expect("text-only entry should resolve");
    assert_eq!(text_only.description, None);
    assert_eq!(text_only.context_window, Some(128_000));
    assert_eq!(text_only.input_modalities, vec![InputModality::Text]);

    // Missing metadata degrades gracefully: display name falls back to id,
    // context window is unknown, and modality defaults to text.
    let sparse = catalog
        .get("vendor/no-metadata")
        .expect("sparse entry should resolve");
    assert_eq!(sparse.display_name, "vendor/no-metadata");
    assert_eq!(sparse.description, None);
    assert_eq!(sparse.context_window, None);
    assert_eq!(sparse.input_modalities, vec![InputModality::Text]);
}

#[tokio::test]
async fn fetch_returns_err_on_non_2xx() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let result = OpenRouterCatalog::fetch(&build_reqwest_client(), &server.uri()).await;
    assert!(result.is_err(), "non-2xx should surface as an error");
}

#[tokio::test]
async fn fetch_returns_err_on_malformed_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/models"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&server)
        .await;

    let result = OpenRouterCatalog::fetch(&build_reqwest_client(), &server.uri()).await;
    assert!(result.is_err(), "malformed body should surface as an error");
}

#[test]
fn empty_catalog_has_no_entries() {
    let catalog = OpenRouterCatalog::default();
    assert!(catalog.is_empty());
    assert!(catalog.get("anything").is_none());
}
