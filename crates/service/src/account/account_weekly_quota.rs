use codexmanager_core::storage::{now_ts, Account, Storage, Token};

use crate::account_status::set_account_status;

pub(crate) const WEEKLY_QUOTA_EXCEEDED_REASON: &str = "weekly_quota_exceeded";
pub(crate) const WEEKLY_QUOTA_RECOVERED_REASON: &str = "weekly_quota_recovered";

/// 账号当前周窗口内的有效已用 token。
///
/// `account_usage_billing_stats` 的 secondary window 只在写入时自复位；
/// 被限流的账号不再有写入，因此这里把已过期窗口（resets_at <= now）视为用量 0，
/// 让额度限制能随周窗口重置自动解除。
pub(crate) fn effective_weekly_used_tokens(storage: &Storage, account_id: &str, now: i64) -> i64 {
    let Ok(Some(window)) = storage.find_account_usage_secondary_window(account_id) else {
        return 0;
    };
    match window.resets_at {
        Some(resets_at) if resets_at > now => window.total_tokens.max(0),
        _ => 0,
    }
}

/// 返回账号的周额度状态：`Some((limit, used))`；未配置上限时返回 None。
pub(crate) fn weekly_quota_state(
    storage: &Storage,
    account_id: &str,
    now: i64,
) -> Option<(i64, i64)> {
    let limit = storage
        .find_account_quota_limit(account_id)
        .ok()
        .flatten()
        .filter(|limit| *limit > 0)?;
    Some((limit, effective_weekly_used_tokens(storage, account_id, now)))
}

pub(crate) fn is_weekly_quota_exceeded(storage: &Storage, account_id: &str, now: i64) -> bool {
    weekly_quota_state(storage, account_id, now).is_some_and(|(limit, used)| used >= limit)
}

/// 请求计费落库后调用：周用量达到上限时把账号自动置为 limited。
/// 返回是否触发了状态变更。
pub(crate) fn enforce_weekly_quota_for_account(storage: &Storage, account_id: &str) -> bool {
    let account_id = account_id.trim();
    if account_id.is_empty() {
        return false;
    }
    let now = now_ts();
    let Some((limit, used)) = weekly_quota_state(storage, account_id, now) else {
        return false;
    };
    if used < limit {
        return false;
    }
    let Ok(Some(account)) = storage.find_account_by_id(account_id) else {
        return false;
    };
    // 只把可用账号自动置为 limited；已限流账号不重复打标，
    // 否则会覆盖其他限流原因（如 usage_limit_exhausted），导致恢复扫描误把
    // 上游仍在限流的账号恢复为 active。
    if !matches!(
        account.status.trim().to_ascii_lowercase().as_str(),
        "active" | "available"
    ) {
        return false;
    }
    set_account_status(storage, account_id, "limited", WEEKLY_QUOTA_EXCEEDED_REASON);
    true
}

/// 恢复被周额度限制的账号：窗口已重置、上限被调高或被移除时自动恢复为 active。
pub(crate) fn recover_weekly_quota_limited_accounts(storage: &Storage) {
    let Ok(accounts) = storage.list_accounts() else {
        return;
    };
    let limited_ids = accounts
        .iter()
        .filter(|account| account.status.trim().eq_ignore_ascii_case("limited"))
        .map(|account| account.id.clone())
        .collect::<Vec<_>>();
    if limited_ids.is_empty() {
        return;
    }
    let reasons = storage
        .latest_account_status_reasons(&limited_ids)
        .unwrap_or_default();
    let now = now_ts();
    for account_id in limited_ids {
        if reasons.get(&account_id).map(String::as_str) != Some(WEEKLY_QUOTA_EXCEEDED_REASON) {
            continue;
        }
        if !is_weekly_quota_exceeded(storage, &account_id, now) {
            set_account_status(storage, &account_id, "active", WEEKLY_QUOTA_RECOVERED_REASON);
        }
    }
}

/// 网关候选筛选：剔除周用量已达上限的账号，并把它们标记为 limited。
pub(crate) fn filter_weekly_quota_candidates(
    storage: &Storage,
    candidates: &mut Vec<(Account, Token)>,
) {
    if candidates.is_empty() {
        return;
    }
    let limits = storage.list_account_quota_limits().unwrap_or_default();
    if limits.is_empty() {
        return;
    }
    let now = now_ts();
    let mut exceeded_ids = Vec::new();
    candidates.retain(|(account, _)| {
        let Some(limit) = limits.get(account.id.as_str()).copied().filter(|v| *v > 0) else {
            return true;
        };
        let used = effective_weekly_used_tokens(storage, &account.id, now);
        if used >= limit {
            exceeded_ids.push(account.id.clone());
            false
        } else {
            true
        }
    });
    for account_id in exceeded_ids {
        set_account_status(storage, &account_id, "limited", WEEKLY_QUOTA_EXCEEDED_REASON);
    }
}

#[cfg(test)]
#[path = "tests/account_weekly_quota_tests.rs"]
mod tests;
