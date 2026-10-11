use serde_json::{json, Value};

/// Merge sanitized expiry metadata from the read-only reset-credit endpoint.
/// Card IDs and unrelated grant types must not enter the display snapshot.
pub(crate) fn merge_reset_credit_details(usage: &mut Value, details: &Value) -> bool {
    let reset = details
        .get("rate_limit_reset_credits")
        .or_else(|| details.get("rateLimitResetCredits"))
        .filter(|value| value.is_object())
        .unwrap_or(details);
    let list = reset.as_array().or_else(|| {
        ["credits", "rate_limit_reset_credits", "items", "data"]
            .iter()
            .find_map(|key| reset.get(key).and_then(Value::as_array))
    });
    let count = reset
        .get("available_count")
        .or_else(|| reset.get("availableCount"))
        .and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str()?.trim().parse().ok())
        })
        .filter(|count| *count >= 0);
    if list.is_none() && count.is_none() {
        return false;
    }
    let credits = list.map(|items| {
        items
            .iter()
            .filter(|item| {
                item.is_object()
                    && ["status"].iter().all(|key| {
                        item.get(key).and_then(Value::as_str).is_none_or(|status| {
                            status.trim().is_empty()
                                || status.trim().eq_ignore_ascii_case("available")
                        })
                    })
                    && item
                        .get("reset_type")
                        .or_else(|| item.get("resetType"))
                        .and_then(Value::as_str)
                        .is_none_or(|kind| {
                            kind.trim().is_empty()
                                || kind.trim().eq_ignore_ascii_case("codex_rate_limits")
                        })
            })
            .map(|item| {
                let expiry = item
                    .get("expires_at")
                    .or_else(|| item.get("expiresAt"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty());
                match expiry {
                    Some(expiry) => json!({ "expires_at": expiry }),
                    None => json!({}),
                }
            })
            .collect::<Vec<_>>()
    });
    let Some(usage) = usage.as_object_mut() else {
        return false;
    };
    let mut snapshot = usage
        .get("rate_limit_reset_credits")
        .or_else(|| usage.get("rateLimitResetCredits"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if let Some(count) = count.or_else(|| credits.as_ref().map(|credits| credits.len() as i64)) {
        snapshot.insert("available_count".into(), json!(count));
    }
    if let Some(credits) = credits {
        snapshot.insert("credits".into(), json!(credits));
    }
    usage.insert("rate_limit_reset_credits".into(), Value::Object(snapshot));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_expiries_and_filters_consumed_and_unrelated_cards() {
        let mut usage = json!({"rate_limit": {"primary_window": {"reset_at": 123}},
            "rate_limit_reset_credits": {"available_count": 9}});
        assert!(merge_reset_credit_details(
            &mut usage,
            &json!({"credits": [
                {"id": "private-id", "expires_at": "2026-11-01T00:00:00Z", "reset_type": "codex_rate_limits", "status": "available"},
                {"expiresAt": "2026-12-01T00:00:00Z"},
                {"status": "consumed", "expires_at": "2026-10-01T00:00:00Z"},
                {"reset_type": "other"}, null
            ]})
        ));
        assert_eq!(usage["rate_limit_reset_credits"]["available_count"], 2);
        assert_eq!(
            usage["rate_limit_reset_credits"]["credits"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(usage["rate_limit_reset_credits"]["credits"][0]
            .get("id")
            .is_none());
        assert_eq!(usage["rate_limit"]["primary_window"]["reset_at"], 123);
        let parsed = codexmanager_core::usage::parse_usage_snapshot(&usage);
        assert!(parsed.credits_json.unwrap().contains("2026-11-01"));
    }

    #[test]
    fn supports_arrays_camelcase_counts_and_empty_lists() {
        let mut usage = json!({});
        assert!(merge_reset_credit_details(
            &mut usage,
            &json!([{ "expiresAt": "2026-11-01T00:00:00Z" }])
        ));
        assert_eq!(usage["rate_limit_reset_credits"]["available_count"], 1);
        assert!(merge_reset_credit_details(
            &mut usage,
            &json!({"rateLimitResetCredits": {"availableCount": "3", "credits": [{}]}})
        ));
        assert_eq!(usage["rate_limit_reset_credits"]["available_count"], 3);
        assert!(merge_reset_credit_details(
            &mut usage,
            &json!({"credits": []})
        ));
        assert_eq!(usage["rate_limit_reset_credits"]["available_count"], 0);
    }

    #[test]
    fn invalid_details_preserve_the_usage_count() {
        let mut usage = json!({"rate_limit_reset_credits": {"available_count": 2}});
        let before = usage.clone();
        assert!(!merge_reset_credit_details(
            &mut usage,
            &json!({"credits": null, "available_count": -1})
        ));
        assert_eq!(usage, before);
    }
}
