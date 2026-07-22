use super::{
    classify_usage_refresh_error, mark_usage_unreachable_if_needed,
    should_record_failure_event_with_state, FailureThrottleKey,
};
use codexmanager_core::storage::{now_ts, Account, AccountAgentIdentity, Storage};
use std::collections::HashMap;

#[test]
fn agent_identity_usage_401_does_not_disable_account() {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("initialize storage");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "agent-401".to_string(),
            label: "agent@example.com".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("team-agent".to_string()),
            workspace_id: Some("team-agent".to_string()),
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .upsert_account_agent_identity(&AccountAgentIdentity {
            account_id: "agent-401".to_string(),
            agent_runtime_id: "runtime-agent".to_string(),
            agent_private_key: "private-key".to_string(),
            task_id: Some("task-agent".to_string()),
            chatgpt_user_id: "user-agent".to_string(),
            chatgpt_account_is_fedramp: false,
            created_at: now,
            updated_at: now,
        })
        .expect("insert agent identity");

    mark_usage_unreachable_if_needed(
        &storage,
        "agent-401",
        "usage endpoint failed: status=401 Unauthorized body=some non-task 401",
    );

    assert_eq!(
        storage
            .find_account_by_id("agent-401")
            .expect("load account")
            .expect("account exists")
            .status,
        "active"
    );
}

/// 函数 `usage_refresh_error_class_groups_by_status_code`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn usage_refresh_error_class_groups_by_status_code() {
    assert_eq!(
        classify_usage_refresh_error("usage endpoint status 500 Internal Server Error"),
        "usage_status_500"
    );
    assert_eq!(
        classify_usage_refresh_error("usage endpoint status 503 Service Unavailable"),
        "usage_status_503"
    );
    assert_eq!(
        classify_usage_refresh_error("subscription endpoint status 401 Unauthorized"),
        "usage_status_401"
    );
    assert_eq!(
        classify_usage_refresh_error(
            "subscription endpoint failed: status=503 Service Unavailable body=upstream unavailable"
        ),
        "usage_status_503"
    );
}

/// 函数 `usage_refresh_error_class_catches_timeout_and_connection`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn usage_refresh_error_class_catches_timeout_and_connection() {
    assert_eq!(
        classify_usage_refresh_error("request timeout while calling usage"),
        "timeout"
    );
    assert_eq!(
        classify_usage_refresh_error("connection reset by peer"),
        "connection"
    );
    assert_eq!(classify_usage_refresh_error("unknown error"), "other");
}

/// 函数 `failure_event_throttle_dedupes_within_window`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn failure_event_throttle_dedupes_within_window() {
    let mut state = HashMap::new();
    let key = FailureThrottleKey {
        account_id: "acc-1".to_string(),
        error_class: "usage_status_500".to_string(),
    };

    assert!(should_record_failure_event_with_state(
        &mut state,
        key.clone(),
        100,
        60
    ));
    assert!(!should_record_failure_event_with_state(
        &mut state,
        key.clone(),
        120,
        60
    ));
    assert!(should_record_failure_event_with_state(
        &mut state, key, 161, 60
    ));
}

/// 函数 `failure_event_throttle_isolated_by_error_class`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn failure_event_throttle_isolated_by_error_class() {
    let mut state = HashMap::new();
    let key_500 = FailureThrottleKey {
        account_id: "acc-1".to_string(),
        error_class: "usage_status_500".to_string(),
    };
    let key_timeout = FailureThrottleKey {
        account_id: "acc-1".to_string(),
        error_class: "timeout".to_string(),
    };

    assert!(should_record_failure_event_with_state(
        &mut state, key_500, 100, 60
    ));
    assert!(should_record_failure_event_with_state(
        &mut state,
        key_timeout,
        110,
        60
    ));
}
