use super::*;
use crate::quota::model_pricing::{
    estimate_cost_with_rules_and_reasoning_for_service_tier, load_enabled_price_rules,
};

const FIXTURE: &str = r#"# Pricing
### Standard pricing data
| Model | Short context input | Short context output | Short context cached input | Long context input | Long context output | Long context cached input |
| --- | --- | --- | --- | --- | --- | --- |
| gpt-6.1-terra | $1.00 | $5.00 | $0.05 | $2.00 | $7.50 | $0.10 |
| gpt-5.5 (<272K context length) | $5.00 | $30.00 | $0.50 | $10.00 | $45.00 | $1.00 |
| Model | Input | Output | Cached input |
| --- | --- | --- | --- |
| gpt-5-mini | $0.25 | $2.00 | $0.025 |
### Batch pricing data
#### Batch
| Model | Input | Output | Cached input |
| --- | --- | --- | --- |
| batch-should-not-import | $0.50 | $2.50 | $0.025 |
#### Flex
| Model | Input | Output | Cached input |
| --- | --- | --- | --- |
| gpt-6.1-terra | $0.50 | $2.50 | $0.025 |
### Fast pricing data
| Model | Input | Output | Cached input |
| --- | --- | --- | --- |
| gpt-6.1-terra | $2.00 | $10.00 | $0.10 |
| gpt-5.5 | $12.50 | $75.00 | $1.25 |
### Ultrafast pricing data
| Model | Input | Output | Cached input |
| --- | --- | --- | --- |
| gpt-6.1-terra | $6.00 | $30.00 | $0.30 |
Short context: ≤272K input tokens. Long context: >272K input tokens.
### Pricing Table data
| Model | Input | Output | Cached input |
| --- | --- | --- | --- |
| image-should-not-import | $100.00 | $100.00 | - |
"#;

fn storage() -> Storage {
    let storage = Storage::open_in_memory().unwrap();
    storage.init().unwrap();
    storage
}

#[test]
fn parses_standard_and_absolute_tier_prices_without_tool_or_batch_rows() {
    let rules = parse_official_prices(FIXTURE, 123).unwrap();
    assert_eq!(rules.len(), 7);
    assert!(!rules
        .iter()
        .any(|rule| rule.model_pattern.contains("should-not-import")));
    let standard = rules
        .iter()
        .find(|rule| rule.model_pattern == "gpt-6.1-terra" && rule.billing_mode == "standard")
        .unwrap();
    assert_eq!(standard.long_context_threshold_tokens, Some(272_000));
    assert_eq!(standard.long_context_output_price_per_1m, Some(7.5));
    for (model, tier, expected) in [
        ("gpt-6.1-terra", None, 0.006),
        ("openai/gpt-6.1-terra", Some("fast"), 0.012),
        ("gpt-6.1-terra", Some("priority"), 0.012),
        ("gpt-6.1-terra", Some("flex"), 0.003),
        ("gpt-6.1-terra", Some("ultrafast"), 0.036),
        ("gpt-5.5", Some("fast"), 0.0875),
    ] {
        let cost = estimate_cost_with_rules_and_reasoning_for_service_tier(
            &rules,
            Some(model),
            1000,
            0,
            1000,
            100,
            tier,
        );
        assert!((cost.cost_usd.unwrap() - expected).abs() < 1e-10);
    }
}

#[test]
fn downloaded_base_models_do_not_override_codex_variants_or_newer_versions() {
    let storage = storage();
    let mut rules = parse_official_prices(FIXTURE, 123).unwrap();
    let mut base = rules
        .iter()
        .find(|rule| rule.billing_mode == "standard")
        .unwrap()
        .clone();
    base.model_pattern = "gpt-5".into();
    base.id = "openai-sync-standard-gpt-5".into();
    base.input_price_per_1m = Some(1.25);
    base.output_price_per_1m = Some(10.0);
    rules.push(base);
    apply_prices(&storage, &rules).unwrap();
    let loaded = load_enabled_price_rules(&storage).unwrap();
    let cost = estimate_cost_with_rules_and_reasoning_for_service_tier(
        &loaded,
        Some("gpt-5.3-codex"),
        1000,
        0,
        1000,
        0,
        None,
    );
    assert!((cost.cost_usd.unwrap() - 0.0135).abs() < 1e-10);
    let cost = estimate_cost_with_rules_and_reasoning_for_service_tier(
        &loaded,
        Some("gpt-6.1-terra-2026-10-27"),
        1000,
        0,
        1000,
        0,
        None,
    );
    assert!((cost.cost_usd.unwrap() - 0.006).abs() < 1e-10);
    // A nonexistent tier is not silently charged at the wrong downloaded price.
    let cost = estimate_cost_with_rules_and_reasoning_for_service_tier(
        &loaded,
        Some("gpt-5-mini"),
        1000,
        0,
        1000,
        0,
        Some("ultrafast"),
    );
    assert_eq!(cost.price_status, "missing");
}

#[test]
fn malformed_documents_are_rejected_before_touching_existing_prices() {
    let storage = storage();
    let original = parse_official_prices(FIXTURE, 123).unwrap();
    apply_prices(&storage, &original).unwrap();
    for document in [
        "<html>challenge</html>".to_string(),
        FIXTURE.replace("$1.00", "$NaN"),
        FIXTURE.replace("$1.00", "$-1.00"),
        FIXTURE.replace("≤272K", "≤300K"),
        FIXTURE.replace("### Standard pricing data", "### Changed"),
        FIXTURE.replace("$7.50", "-"),
        FIXTURE.replace("Short context output", "Changed output"),
    ] {
        assert!(parse_official_prices(&document, 456).is_err());
    }
    assert_eq!(
        storage.list_model_price_rules().unwrap().len(),
        original.len()
    );
    assert!(storage
        .list_model_price_rules()
        .unwrap()
        .iter()
        .all(|rule| rule.updated_at == 123));
}

#[test]
fn atomic_snapshot_retains_custom_prices_and_survives_seed_refresh() {
    let storage = storage();
    let rules = parse_official_prices(FIXTURE, 123).unwrap();
    let mut custom = rules[0].clone();
    custom.id = "my-price".into();
    custom.source = "custom".into();
    custom.model_pattern = "gpt-6.1-terra".into();
    custom.billing_mode = "standard".into();
    custom.priority = 20_000;
    custom.input_price_per_1m = Some(9.0);
    custom.output_price_per_1m = Some(9.0);
    storage.upsert_model_price_rule(&custom).unwrap();
    let result = apply_prices(&storage, &rules).unwrap();
    assert_eq!(result.model_count, 3);
    let loaded = load_enabled_price_rules(&storage).unwrap();
    assert!(loaded.iter().any(|rule| rule.id == custom.id));
    assert_eq!(
        loaded
            .iter()
            .filter(|rule| rule.source == "official_sync")
            .count(),
        rules.len()
    );
    let cost = estimate_cost_with_rules_and_reasoning_for_service_tier(
        &loaded,
        Some("gpt-6.1-terra"),
        1000,
        0,
        1000,
        0,
        None,
    );
    assert!((cost.cost_usd.unwrap() - 0.018).abs() < 1e-10);
    let mut collision = rules.clone();
    collision[0].id = custom.id.clone();
    assert!(apply_prices(&storage, &collision).is_err());
    assert!(apply_prices(&storage, &[]).is_err());
    let after = storage.list_model_price_rules().unwrap();
    assert_eq!(
        after
            .iter()
            .filter(|rule| rule.source == "official_sync")
            .count(),
        rules.len()
    );
    assert_eq!(
        after
            .iter()
            .find(|rule| rule.id == custom.id)
            .unwrap()
            .source,
        "custom"
    );
    apply_prices(&storage, &parse_official_prices(FIXTURE, 456).unwrap()).unwrap();
    assert!(storage
        .list_model_price_rules()
        .unwrap()
        .iter()
        .filter(|rule| rule.source == "official_sync")
        .all(|rule| rule.updated_at == 456));
}

#[test]
fn downloader_handles_http_errors_and_parses_mock_official_document() {
    for (status, body, succeeds) in [(200, FIXTURE, true), (503, "unavailable", false)] {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}/pricing.md", server.server_addr());
        let handle = std::thread::spawn(move || {
            let request = server
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap();
            assert_eq!(request.method(), &tiny_http::Method::Get);
            request
                .respond(tiny_http::Response::from_string(body).with_status_code(status))
                .unwrap();
        });
        let result = download_prices(&url);
        assert_eq!(result.is_ok(), succeeds);
        if let Ok(document) = result {
            assert_eq!(parse_official_prices(&document, 123).unwrap().len(), 7);
        }
        handle.join().unwrap();
    }
}

#[test]
#[ignore = "manual live official pricing contract probe"]
fn live_official_pricing_contract() {
    let document = download_prices(PRICE_URL).unwrap();
    let rules = parse_official_prices(&document, now_ts()).unwrap();
    assert!(
        rules
            .iter()
            .filter(|rule| rule.billing_mode == "standard")
            .count()
            >= 30
    );
    let terra = rules
        .iter()
        .find(|rule| rule.model_pattern == "gpt-6.1-terra" && rule.billing_mode == "standard")
        .unwrap();
    assert_eq!(terra.input_price_per_1m, Some(1.0));
    assert_eq!(terra.cached_input_price_per_1m, Some(0.05));
    assert_eq!(terra.output_price_per_1m, Some(5.0));
    let count = rules
        .iter()
        .map(|rule| &rule.model_pattern)
        .collect::<BTreeSet<_>>()
        .len();
    eprintln!(
        "Live official website: {} models, {} tier rules",
        count,
        rules.len()
    );
    apply_prices(&storage(), &rules).unwrap();
}
