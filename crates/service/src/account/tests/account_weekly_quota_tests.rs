use super::{
    effective_weekly_used_tokens, enforce_weekly_quota_for_account,
    filter_weekly_quota_candidates, is_weekly_quota_exceeded,
    recover_weekly_quota_limited_accounts, WEEKLY_QUOTA_EXCEEDED_REASON,
};
use crate::account_status::set_account_status;
use codexmanager_core::storage::{
    now_ts, Account, RequestLog, RequestTokenStat, Storage, Token,
};

fn test_account(id: &str, status: &str) -> Account {
    let now = now_ts();
    Account {
        id: id.to_string(),
        label: id.to_string(),
        issuer: "issuer".to_string(),
        chatgpt_account_id: None,
        workspace_id: None,
        group_name: None,
        sort: 0,
        status: status.to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn test_token(account_id: &str) -> Token {
    Token {
        account_id: account_id.to_string(),
        id_token: String::new(),
        access_token: "token".to_string(),
        refresh_token: String::new(),
        api_key_access_token: None,
        last_refresh: now_ts(),
    }
}

fn record_usage(storage: &Storage, account_id: &str, total_tokens: i64) {
    let now = now_ts();
    storage
        .insert_request_log_with_token_stat(
            &RequestLog {
                request_path: "/v1/responses".to_string(),
                method: "POST".to_string(),
                account_id: Some(account_id.to_string()),
                status_code: Some(200),
                created_at: now,
                ..RequestLog::default()
            },
            &RequestTokenStat {
                account_id: Some(account_id.to_string()),
                input_tokens: Some(total_tokens),
                output_tokens: Some(0),
                total_tokens: Some(total_tokens),
                created_at: now,
                ..RequestTokenStat::default()
            },
        )
        .expect("insert request log with stat");
}

fn setup_storage() -> Storage {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("init storage");
    storage
}

#[test]
fn weekly_quota_marks_account_limited_when_usage_reaches_limit() {
    let _guard = crate::test_env_guard();
    let storage = setup_storage();
    storage
        .insert_account(&test_account("acc-weekly", "active"))
        .expect("insert account");
    storage
        .upsert_account_quota_limit("acc-weekly", Some(1_000))
        .expect("set limit");

    record_usage(&storage, "acc-weekly", 500);
    assert!(!enforce_weekly_quota_for_account(&storage, "acc-weekly"));
    assert_eq!(
        storage
            .find_account_by_id("acc-weekly")
            .expect("find")
            .expect("exists")
            .status,
        "active"
    );

    record_usage(&storage, "acc-weekly", 600);
    assert!(enforce_weekly_quota_for_account(&storage, "acc-weekly"));
    let account = storage
        .find_account_by_id("acc-weekly")
        .expect("find")
        .expect("exists");
    assert_eq!(account.status, "limited");
    let reasons = storage
        .latest_account_status_reasons(&["acc-weekly".to_string()])
        .expect("reasons");
    assert_eq!(
        reasons.get("acc-weekly").map(String::as_str),
        Some(WEEKLY_QUOTA_EXCEEDED_REASON)
    );
}

#[test]
fn weekly_quota_does_not_touch_accounts_without_limit_or_manual_disable() {
    let _guard = crate::test_env_guard();
    let storage = setup_storage();
    storage
        .insert_account(&test_account("acc-free", "active"))
        .expect("insert account");
    record_usage(&storage, "acc-free", 10_000);
    assert!(!enforce_weekly_quota_for_account(&storage, "acc-free"));

    storage
        .insert_account(&test_account("acc-manual", "disabled"))
        .expect("insert account");
    storage
        .upsert_account_quota_limit("acc-manual", Some(100))
        .expect("set limit");
    record_usage(&storage, "acc-manual", 10_000);
    assert!(!enforce_weekly_quota_for_account(&storage, "acc-manual"));
    assert_eq!(
        storage
            .find_account_by_id("acc-manual")
            .expect("find")
            .expect("exists")
            .status,
        "disabled"
    );
}

#[test]
fn weekly_quota_recovers_account_after_limit_removed() {
    let _guard = crate::test_env_guard();
    let storage = setup_storage();
    storage
        .insert_account(&test_account("acc-recover", "active"))
        .expect("insert account");
    storage
        .upsert_account_quota_limit("acc-recover", Some(100))
        .expect("set limit");
    record_usage(&storage, "acc-recover", 200);
    assert!(enforce_weekly_quota_for_account(&storage, "acc-recover"));

    // 上限被移除后，恢复扫描应把账号重新置为 active。
    storage
        .upsert_account_quota_limit("acc-recover", None)
        .expect("clear limit");
    recover_weekly_quota_limited_accounts(&storage);
    assert_eq!(
        storage
            .find_account_by_id("acc-recover")
            .expect("find")
            .expect("exists")
            .status,
        "active"
    );
}

#[test]
fn weekly_quota_filter_removes_exceeded_candidates() {
    let _guard = crate::test_env_guard();
    let storage = setup_storage();
    storage
        .insert_account(&test_account("acc-a", "active"))
        .expect("insert account");
    storage
        .insert_account(&test_account("acc-b", "active"))
        .expect("insert account");
    storage
        .upsert_account_quota_limit("acc-a", Some(100))
        .expect("set limit");
    record_usage(&storage, "acc-a", 200);

    let mut candidates = vec![
        (test_account("acc-a", "active"), test_token("acc-a")),
        (test_account("acc-b", "active"), test_token("acc-b")),
    ];
    filter_weekly_quota_candidates(&storage, &mut candidates);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].0.id, "acc-b");
    assert_eq!(
        storage
            .find_account_by_id("acc-a")
            .expect("find")
            .expect("exists")
            .status,
        "limited"
    );
}

#[test]
fn weekly_quota_does_not_clobber_accounts_limited_for_other_reasons() {
    let _guard = crate::test_env_guard();
    let storage = setup_storage();
    storage
        .insert_account(&test_account("acc-upstream", "active"))
        .expect("insert account");
    // 上游用量耗尽导致的限流
    set_account_status(&storage, "acc-upstream", "limited", "usage_limit_exhausted");
    storage
        .upsert_account_quota_limit("acc-upstream", Some(100))
        .expect("set limit");
    record_usage(&storage, "acc-upstream", 200);

    // 周额度强制不应覆盖其他原因的限流标记
    assert!(!enforce_weekly_quota_for_account(&storage, "acc-upstream"));
    let reasons = storage
        .latest_account_status_reasons(&["acc-upstream".to_string()])
        .expect("reasons");
    assert_eq!(
        reasons.get("acc-upstream").map(String::as_str),
        Some("usage_limit_exhausted")
    );

    // 周上限被移除后，恢复扫描也不能误恢复该账号
    storage
        .upsert_account_quota_limit("acc-upstream", None)
        .expect("clear limit");
    recover_weekly_quota_limited_accounts(&storage);
    assert_eq!(
        storage
            .find_account_by_id("acc-upstream")
            .expect("find")
            .expect("exists")
            .status,
        "limited"
    );
}

#[test]
fn weekly_quota_effective_usage_ignores_expired_window() {
    let _guard = crate::test_env_guard();
    let storage = setup_storage();
    storage
        .insert_account(&test_account("acc-window", "active"))
        .expect("insert account");
    record_usage(&storage, "acc-window", 500);

    let now = now_ts();
    assert_eq!(effective_weekly_used_tokens(&storage, "acc-window", now), 500);
    // 周窗口重置时间过后，用量视为 0，额度限制自动解除。
    let after_window = now + 8 * 24 * 3600;
    assert_eq!(
        effective_weekly_used_tokens(&storage, "acc-window", after_window),
        0
    );
    storage
        .upsert_account_quota_limit("acc-window", Some(100))
        .expect("set limit");
    assert!(is_weekly_quota_exceeded(&storage, "acc-window", now));
    assert!(!is_weekly_quota_exceeded(&storage, "acc-window", after_window));
}
