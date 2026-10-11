use codexmanager_core::storage::{now_ts, Event};

use crate::{account_status, storage_helpers::open_storage};

/// 函数 `update_account`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn update_account(
    account_id: &str,
    sort: Option<i64>,
    preferred: Option<bool>,
    status: Option<&str>,
    label: Option<&str>,
    note: Option<&str>,
    tags: Option<&str>,
    model_slugs: Option<Vec<String>>,
    quota_capacity_primary_window_tokens: Option<i64>,
    quota_capacity_secondary_window_tokens: Option<i64>,
    weekly_quota_limit_tokens: Option<i64>,
) -> Result<(), String> {
    // 更新账号排序或状态并记录事件
    let normalized_account_id = account_id.trim();
    if normalized_account_id.is_empty() {
        return Err("missing accountId".to_string());
    }

    let normalized_status = status.map(normalize_account_status).transpose()?;
    let normalized_label = normalize_optional_label(label)?;
    let normalized_note = normalize_optional_text(note);
    let normalized_tags = normalize_optional_tags(tags);
    let metadata_requested = note.is_some() || tags.is_some();
    let model_assignment_requested = model_slugs.is_some();
    let quota_override_requested = quota_capacity_primary_window_tokens.is_some()
        || quota_capacity_secondary_window_tokens.is_some();
    let weekly_quota_limit_requested = weekly_quota_limit_tokens.is_some();

    if sort.is_none()
        && preferred.is_none()
        && normalized_status.is_none()
        && normalized_label.is_none()
        && !metadata_requested
        && !model_assignment_requested
        && !quota_override_requested
        && !weekly_quota_limit_requested
    {
        return Err("missing account update fields".to_string());
    }

    let mut storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let now = now_ts();
    if let Some(preferred) = preferred {
        if preferred {
            let found = storage
                .find_account_by_id(normalized_account_id)
                .map_err(|err| err.to_string())?
                .is_some();
            if !found {
                return Err("account not found".to_string());
            }
            storage
                .set_preferred_account(Some(normalized_account_id))
                .map_err(|e| e.to_string())?;
        } else {
            storage
                .clear_preferred_account_if(normalized_account_id)
                .map_err(|e| e.to_string())?;
        }
        let _ = storage.insert_event(&Event {
            account_id: Some(normalized_account_id.to_string()),
            event_type: "account_preferred_update".to_string(),
            message: format!("preferred={preferred}"),
            created_at: now,
        });
    }

    if let Some(sort) = sort {
        storage
            .update_account_sort(normalized_account_id, sort)
            .map_err(|e| e.to_string())?;
        let _ = storage.insert_event(&Event {
            account_id: Some(normalized_account_id.to_string()),
            event_type: "account_sort_update".to_string(),
            message: format!("sort={sort}"),
            created_at: now,
        });
    }

    if let Some(status) = normalized_status {
        let reason = if status == "disabled" {
            "manual_disable"
        } else {
            "manual_enable"
        };
        account_status::set_account_status(&storage, normalized_account_id, status, reason);
    }

    if let Some(label) = normalized_label {
        storage
            .update_account_label(normalized_account_id, label)
            .map_err(|e| e.to_string())?;
        let _ = storage.insert_event(&Event {
            account_id: Some(normalized_account_id.to_string()),
            event_type: "account_profile_update".to_string(),
            message: format!("label={label}"),
            created_at: now,
        });
    }

    if metadata_requested {
        storage
            .upsert_account_metadata(
                normalized_account_id,
                normalized_note.as_deref(),
                normalized_tags.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        storage
            .touch_account_updated_at(normalized_account_id)
            .map_err(|e| e.to_string())?;
        let _ = storage.insert_event(&Event {
            account_id: Some(normalized_account_id.to_string()),
            event_type: "account_profile_update".to_string(),
            message: format!(
                "note={} tags={}",
                normalized_note.as_deref().unwrap_or("-"),
                normalized_tags.as_deref().unwrap_or("-"),
            ),
            created_at: now,
        });
    }

    if let Some(model_slugs) = model_slugs {
        storage
            .set_quota_source_model_assignments(
                "openai_account",
                normalized_account_id,
                model_slugs.as_slice(),
            )
            .map_err(|e| e.to_string())?;
        storage
            .touch_account_updated_at(normalized_account_id)
            .map_err(|e| e.to_string())?;
        let _ = storage.insert_event(&Event {
            account_id: Some(normalized_account_id.to_string()),
            event_type: "account_quota_models_update".to_string(),
            message: format!("models={}", model_slugs.join(",")),
            created_at: now,
        });
    }

    if quota_override_requested {
        storage
            .upsert_account_quota_capacity_override(
                normalized_account_id,
                quota_capacity_primary_window_tokens,
                quota_capacity_secondary_window_tokens,
            )
            .map_err(|e| e.to_string())?;
        storage
            .touch_account_updated_at(normalized_account_id)
            .map_err(|e| e.to_string())?;
        let _ = storage.insert_event(&Event {
            account_id: Some(normalized_account_id.to_string()),
            event_type: "account_quota_capacity_update".to_string(),
            message: format!(
                "primary={:?} secondary={:?}",
                quota_capacity_primary_window_tokens, quota_capacity_secondary_window_tokens
            ),
            created_at: now,
        });
    }

    if let Some(weekly_limit) = weekly_quota_limit_tokens {
        // <= 0 表示清除周额度上限
        storage
            .upsert_account_quota_limit(normalized_account_id, Some(weekly_limit))
            .map_err(|e| e.to_string())?;
        storage
            .touch_account_updated_at(normalized_account_id)
            .map_err(|e| e.to_string())?;
        let _ = storage.insert_event(&Event {
            account_id: Some(normalized_account_id.to_string()),
            event_type: "account_weekly_quota_limit_update".to_string(),
            message: format!("weeklyLimitTokens={weekly_limit}"),
            created_at: now,
        });
        // 上限变化后立即重新评估：可能需要立刻限流，也可能需要解除限流。
        if !crate::account_weekly_quota::enforce_weekly_quota_for_account(
            &storage,
            normalized_account_id,
        ) {
            crate::account_weekly_quota::recover_weekly_quota_limited_accounts(&storage);
        }
    }

    Ok(())
}

/// 函数 `normalize_account_status`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - status: 参数 status
///
/// # 返回
/// 返回函数执行结果
fn normalize_account_status(status: &str) -> Result<&'static str, String> {
    let normalized = status.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "active" => Ok("active"),
        "disabled" | "inactive" => Ok("disabled"),
        _ => Err(format!("unsupported account status: {status}")),
    }
}

/// 函数 `normalize_optional_label`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - label: 参数 label
///
/// # 返回
/// 返回函数执行结果
fn normalize_optional_label(label: Option<&str>) -> Result<Option<&str>, String> {
    let Some(label) = label else {
        return Ok(None);
    };
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err("label cannot be empty".to_string());
    }
    Ok(Some(trimmed))
}

/// 函数 `normalize_optional_text`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

/// 函数 `normalize_optional_tags`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
///
/// # 返回
/// 返回函数执行结果
fn normalize_optional_tags(value: Option<&str>) -> Option<String> {
    let Some(value) = value else {
        return None;
    };
    let parts = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(","))
    }
}
