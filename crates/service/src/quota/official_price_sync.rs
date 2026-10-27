use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::sync::Mutex;
use std::time::Duration;

use codexmanager_core::storage::{now_ts, ModelPriceRule, Storage};
use serde::Serialize;

const PRICE_URL: &str = "https://developers.openai.com/api/docs/pricing.md";
const SOURCE_URL: &str = "https://developers.openai.com/api/docs/pricing";
const MAX_DOCUMENT_BYTES: u64 = 2 * 1024 * 1024;
static SYNC_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PriceSyncResult {
    model_count: usize,
    rule_count: usize,
    synced_at: i64,
    source_url: &'static str,
}

pub(crate) fn sync_openai_prices() -> Result<PriceSyncResult, String> {
    let _guard = SYNC_LOCK
        .try_lock()
        .map_err(|_| "官方价格同步正在进行中，请稍后重试".to_string())?;
    let document = download_prices(PRICE_URL)?;
    let rules = parse_official_prices(&document, now_ts())?;
    let storage = crate::storage_helpers::open_storage().ok_or("open storage failed")?;
    super::model_pricing::ensure_official_price_seed(&storage)?;
    apply_prices(&storage, &rules)
}

fn download_prices(url: &str) -> Result<String, String> {
    let mut builder = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("CodexManager/", env!("CARGO_PKG_VERSION")));
    if let Ok(proxy) = std::env::var("CODEXMANAGER_UPSTREAM_PROXY_URL") {
        if !proxy.trim().is_empty() {
            builder =
                builder.proxy(reqwest::Proxy::all(proxy.trim()).map_err(|_| "上游代理地址无效")?);
        }
    }
    let response = builder
        .build()
        .map_err(|err| format!("创建官价请求失败：{err}"))?
        .get(url)
        .send()
        .map_err(|err| format!("获取 OpenAI 官方价格失败：{err}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "获取 OpenAI 官方价格失败：HTTP {}",
            response.status()
        ));
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_DOCUMENT_BYTES)
    {
        return Err("官方价格文档过大，已保留现有价格".into());
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("读取官方价格失败：{err}"))?;
    if bytes.len() as u64 > MAX_DOCUMENT_BYTES {
        return Err("官方价格文档过大，已保留现有价格".into());
    }
    String::from_utf8(bytes).map_err(|_| "官方价格文档编码无效，已保留现有价格".into())
}

fn apply_prices(storage: &Storage, rules: &[ModelPriceRule]) -> Result<PriceSyncResult, String> {
    storage
        .replace_synced_openai_price_rules(rules)
        .map_err(|err| format!("保存官方价格失败，已保留现有价格：{err}"))?;
    Ok(PriceSyncResult {
        model_count: rules
            .iter()
            .map(|rule| &rule.model_pattern)
            .collect::<BTreeSet<_>>()
            .len(),
        rule_count: rules.len(),
        synced_at: rules[0].updated_at,
        source_url: SOURCE_URL,
    })
}

fn section<'a>(document: &'a str, heading: &str) -> Option<&'a str> {
    let start = document.find(heading)? + heading.len();
    let tail = &document[start..];
    let end = tail.find("\n#").unwrap_or(tail.len());
    Some(&tail[..end])
}

fn money(text: &str) -> Result<Option<f64>, String> {
    let text = text.trim();
    if matches!(text, "-" | "—" | "N/A") {
        return Ok(None);
    }
    let amount = text
        .strip_prefix('$')
        .ok_or("官方价格格式变化：金额缺少美元符号")?
        .replace(',', "")
        .parse::<f64>()
        .map_err(|_| "官方价格格式变化：金额无效")?;
    if !amount.is_finite() || !(0.0..=100_000.0).contains(&amount) {
        return Err("官方价格超出有效范围，已保留现有价格".into());
    }
    Ok(Some(amount))
}

/// Parse only the flagship/general text tables, never tool, image, audio or Batch prices.
fn parse_official_prices(document: &str, timestamp: i64) -> Result<Vec<ModelPriceRule>, String> {
    if !document.contains("Short context: ≤272K input tokens. Long context: >272K input tokens.")
    {
        return Err("官网长上下文阈值格式变化，已保留现有价格".into());
    }
    let mut rules = BTreeMap::new();
    for (heading, tier) in [
        ("### Standard pricing data", "standard"),
        ("#### Flex", "flex"),
        ("### Fast pricing data", "fast"),
        ("### Ultrafast pricing data", "ultrafast"),
    ] {
        let Some(section) = section(document, heading) else {
            if tier == "standard" {
                return Err("未找到官方文本价格表，已保留现有价格".into());
            }
            continue;
        };
        let mut header: Vec<&str> = Vec::new();
        for line in section.lines().map(str::trim) {
            if !line.starts_with('|') {
                continue;
            }
            let cells = line
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();
            if cells.first() == Some(&"Model") {
                let required: &[&str] = if cells.contains(&"Short context input") {
                    &[
                        "Short context input",
                        "Short context output",
                        "Short context cached input",
                        "Long context input",
                        "Long context output",
                        "Long context cached input",
                    ]
                } else {
                    &["Input", "Output", "Cached input"]
                };
                if required.iter().any(|column| !cells.contains(column)) {
                    return Err("官网价格表列格式变化，已保留现有价格".into());
                }
                header = cells;
                continue;
            }
            if cells.first().is_some_and(|cell| cell.starts_with('-')) {
                continue;
            }
            if header.is_empty() || cells.len() != header.len() {
                return Err("官网价格表列格式变化，已保留现有价格".into());
            }
            let name = cells[0]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('`');
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.'))
            {
                return Err("官网模型名称格式变化，已保留现有价格".into());
            }
            let value = |column: &str| -> Result<Option<f64>, String> {
                match header.iter().position(|name| *name == column) {
                    Some(index) => money(cells[index]),
                    None => Ok(None),
                }
            };
            let long = header.contains(&"Short context input");
            if !long && !header.contains(&"Input") {
                return Err("官网输入价格列格式变化，已保留现有价格".into());
            }
            let input = value(if long { "Short context input" } else { "Input" })?
                .ok_or("缺少官方输入单价")?;
            let output = value(if long {
                "Short context output"
            } else {
                "Output"
            })?
            .ok_or("缺少官方输出单价")?;
            let cached = value(if long {
                "Short context cached input"
            } else {
                "Cached input"
            })?;
            let long_input = value("Long context input")?;
            let long_output = value("Long context output")?;
            if long_input.is_some() != long_output.is_some() {
                return Err("官网长上下文价格不完整，已保留现有价格".into());
            }
            let rule = ModelPriceRule {
                id: format!("openai-sync-{tier}-{name}"),
                provider: "openai".into(),
                model_pattern: name.into(),
                match_type: "prefix".into(),
                billing_mode: tier.into(),
                currency: "USD".into(),
                unit: "per_1m_tokens".into(),
                input_price_per_1m: Some(input),
                cached_input_price_per_1m: cached,
                output_price_per_1m: Some(output),
                reasoning_output_price_per_1m: None,
                // Existing cache-write fields represent 5m/1h; do not mislabel official 30m/12h rates.
                cache_write_5m_price_per_1m: None,
                cache_write_1h_price_per_1m: None,
                cache_hit_price_per_1m: None,
                long_context_threshold_tokens: long.then_some(272_000),
                long_context_input_price_per_1m: long_input,
                long_context_cached_input_price_per_1m: value("Long context cached input")?,
                long_context_output_price_per_1m: long_output,
                source: "official_sync".into(),
                source_url: Some(SOURCE_URL.into()),
                seed_version: None,
                enabled: true,
                priority: 15_000,
                created_at: timestamp,
                updated_at: timestamp,
            };
            if rules
                .insert((tier.to_string(), name.to_string()), rule)
                .is_some()
            {
                return Err("官网价格表包含重复模型，已保留现有价格".into());
            }
        }
    }
    if !rules.keys().any(|(tier, _)| tier == "standard") {
        return Err("官方价格表为空，已保留现有价格".into());
    }
    if rules.len() > 2_000 {
        return Err("官网价格表行数异常，已保留现有价格".into());
    }
    Ok(rules.into_values().collect())
}

#[cfg(test)]
#[path = "tests/official_price_sync_tests.rs"]
mod tests;
