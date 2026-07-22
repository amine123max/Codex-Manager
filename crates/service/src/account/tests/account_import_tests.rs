use super::{
    extract_token_payload, import_account_auth_json, import_single_item, parse_items_from_content,
    resolve_logical_account_id, ExistingAccountIndex, ImportTokenPayload,
};
use crate::account_identity::build_account_storage_id;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use codexmanager_core::storage::{now_ts, Account, Storage, Token};
use ed25519_dalek::pkcs8::EncodePrivateKey;
use ed25519_dalek::SigningKey;
use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_ID_TOKEN_WS_A: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzdWItMSIsImVtYWlsIjoidGVzdEBleGFtcGxlLmNvbSIsIndvcmtzcGFjZV9pZCI6IndzLWEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiY2dwdC0xIn19.sig";
const TEST_ID_TOKEN_META: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzdWItMSIsImVtYWlsIjoibWV0YUBleGFtcGxlLmNvbSIsIndvcmtzcGFjZV9pZCI6IndzLW1ldGEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiY2dwdC1tZXRhIn19.sig";
const TEST_ACCESS_TOKEN_TEAM_USER_A: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzdWJqZWN0LWEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoidGVhbS0xIiwiY2hhdGdwdF91c2VyX2lkIjoidXNlci1hIn19.sig";
const TEST_ACCESS_TOKEN_TEAM_USER_B: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzdWJqZWN0LWIiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoidGVhbS0xIiwiY2hhdGdwdF91c2VyX2lkIjoidXNlci1iIn19.sig";
const TEST_ID_TOKEN_SAME_SUB_TEAM: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzYW1lLXVzZXIiLCJlbWFpbCI6InNhbWVAZXhhbXBsZS5jb20iLCJ3b3Jrc3BhY2VfaWQiOiJ3cy1zaGFyZWQiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiY2dwdC10ZWFtIiwiY2hhdGdwdF9wbGFuX3R5cGUiOiJ0ZWFtIiwiY2hhdGdwdF91c2VyX2lkIjoic2FtZS11c2VyIn19.sig";
const TEST_ID_TOKEN_SAME_SUB_PLUS: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzYW1lLXVzZXIiLCJlbWFpbCI6InNhbWVAZXhhbXBsZS5jb20iLCJ3b3Jrc3BhY2VfaWQiOiJ3cy1zaGFyZWQiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiY2dwdC1wbHVzIiwiY2hhdGdwdF9wbGFuX3R5cGUiOiJwbHVzIiwiY2hhdGdwdF91c2VyX2lkIjoic2FtZS11c2VyIn19.sig";
const TEST_ID_TOKEN_SAME_SUB_TEAM_A: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzYW1lLXVzZXIiLCJlbWFpbCI6InNhbWVAZXhhbXBsZS5jb20iLCJ3b3Jrc3BhY2VfaWQiOiJ3cy10ZWFtLWEiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiY2dwdC10ZWFtLWEiLCJjaGF0Z3B0X3BsYW5fdHlwZSI6InRlYW0iLCJjaGF0Z3B0X3VzZXJfaWQiOiJzYW1lLXVzZXIifX0.sig";
const TEST_ID_TOKEN_SAME_SUB_TEAM_B: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJzYW1lLXVzZXIiLCJlbWFpbCI6InNhbWVAZXhhbXBsZS5jb20iLCJ3b3Jrc3BhY2VfaWQiOiJ3cy10ZWFtLWIiLCJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiY2dwdC10ZWFtLWIiLCJjaGF0Z3B0X3BsYW5fdHlwZSI6InRlYW0iLCJjaGF0Z3B0X3VzZXJfaWQiOiJzYW1lLXVzZXIifX0.sig";

/// 函数 `unique_temp_db_path`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn unique_temp_db_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("codexmanager-account-import-test-{unique}.db"))
}

/// 函数 `payload`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// 无
///
/// # 返回
/// 返回函数执行结果
fn payload() -> ImportTokenPayload {
    ImportTokenPayload {
        access_token: "access".to_string(),
        id_token: "id".to_string(),
        refresh_token: "refresh".to_string(),
        account_id_hint: None,
        chatgpt_account_id_hint: None,
        user_id_hint: None,
    }
}

fn test_agent_private_key() -> String {
    let signing_key = SigningKey::from_bytes(&[9_u8; 32]);
    let document = signing_key.to_pkcs8_der().expect("encode private key");
    STANDARD.encode(document.as_bytes())
}

fn agent_identity_value(
    runtime_id: &str,
    account_id: &str,
    user_id: &str,
    task_id: Option<&str>,
) -> serde_json::Value {
    let mut identity = json!({
        "agent_runtime_id": runtime_id,
        "agent_private_key": test_agent_private_key(),
        "account_id": account_id,
        "chatgpt_user_id": user_id,
        "email": "agent@example.com",
        "plan_type": "k12",
        "chatgpt_account_is_fedramp": false
    });
    if let Some(task_id) = task_id {
        identity["task_id"] = json!(task_id);
    }
    json!({
        "auth_mode": "agentIdentity",
        "agent_identity": identity
    })
}

#[test]
fn import_agent_identity_without_oauth_token_persists_credentials_and_plan() {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("init storage");
    let mut index = ExistingAccountIndex::build(&storage).expect("index");
    let item = agent_identity_value("runtime-1", "team-1", "user-1", Some("task-1"));

    assert!(import_single_item(&storage, &mut index, &item, 1).expect("import"));

    let account = storage
        .find_account_by_id("team-1")
        .expect("find account")
        .expect("account");
    assert_eq!(account.label, "agent@example.com");
    let token = storage
        .find_token_by_account_id("team-1")
        .expect("find token")
        .expect("token");
    assert!(token.access_token.is_empty());
    assert!(token.refresh_token.is_empty());
    let identity = storage
        .find_account_agent_identity("team-1")
        .expect("find identity")
        .expect("identity");
    assert_eq!(identity.agent_runtime_id, "runtime-1");
    assert_eq!(identity.task_id.as_deref(), Some("task-1"));
    let subscription = storage
        .find_account_subscription("team-1")
        .expect("find subscription")
        .expect("subscription");
    assert_eq!(subscription.plan_type.as_deref(), Some("k12"));
}

#[test]
fn import_agent_identity_accepts_camel_case_and_updates_same_team() {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("init storage");
    let mut index = ExistingAccountIndex::build(&storage).expect("index");
    let first = agent_identity_value("runtime-1", "team-1", "user-1", Some("task-1"));
    import_single_item(&storage, &mut index, &first, 1).expect("first import");
    let second = json!({
        "authMode": "agentIdentity",
        "agentIdentity": {
            "agentRuntimeId": "runtime-2",
            "agentPrivateKey": test_agent_private_key(),
            "taskId": "task-2",
            "accountId": "team-1",
            "chatgptUserId": "user-1",
            "email": "agent@example.com",
            "planType": "team"
        }
    });

    assert!(!import_single_item(&storage, &mut index, &second, 2).expect("second import"));
    assert_eq!(storage.list_accounts().expect("accounts").len(), 1);
    let identity = storage
        .find_account_agent_identity("team-1")
        .expect("find identity")
        .expect("identity");
    assert_eq!(identity.agent_runtime_id, "runtime-2");
    assert_eq!(identity.task_id.as_deref(), Some("task-2"));
}

#[test]
fn import_agent_identity_accepts_sub2api_selected_accounts_export_shape() {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("init storage");
    let bundle = json!({
        "type": "sub2api-data",
        "version": 1,
        "accounts": [{
            "name": "agent@example.com",
            "type": "oauth",
            "platform": "openai",
            "credentials": {
                "auth_mode": "agentIdentity",
                "agent_runtime_id": "runtime-export",
                "agent_private_key": test_agent_private_key(),
                "task_id": "task-export",
                "account_id": "team-export",
                "chatgpt_account_id": "team-export",
                "chatgpt_user_id": "user-export",
                "email": "agent@example.com",
                "plan_type": "k12",
                "chatgpt_account_is_fedramp": false
            }
        }]
    });
    let items = parse_items_from_content(&bundle.to_string()).expect("parse bundle");
    assert_eq!(items.len(), 1);
    let mut index = ExistingAccountIndex::build(&storage).expect("index");

    assert!(import_single_item(&storage, &mut index, &items[0], 1).expect("import"));
    let identity = storage
        .find_account_agent_identity("team-export")
        .expect("find identity")
        .expect("identity");
    assert_eq!(identity.agent_runtime_id, "runtime-export");
    assert_eq!(identity.task_id.as_deref(), Some("task-export"));
    let account = storage
        .find_account_by_id("team-export")
        .expect("find account")
        .expect("account");
    assert_eq!(account.label, "agent@example.com");
}

#[test]
fn import_agent_identity_reuses_existing_oauth_account_for_same_chatgpt_team() {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("init storage");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "legacy-storage-id".to_string(),
            label: "old@example.com".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("team-1".to_string()),
            workspace_id: Some("legacy-workspace".to_string()),
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert account");
    storage
        .insert_token(&Token {
            account_id: "legacy-storage-id".to_string(),
            id_token: String::new(),
            access_token: "oauth-access".to_string(),
            refresh_token: "oauth-refresh".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert token");
    let mut index = ExistingAccountIndex::build(&storage).expect("index");
    let item = agent_identity_value("runtime-1", "team-1", "user-1", Some("task-1"));

    assert!(!import_single_item(&storage, &mut index, &item, 1).expect("import"));
    assert_eq!(storage.list_accounts().expect("accounts").len(), 1);
    assert!(storage
        .find_account_agent_identity("legacy-storage-id")
        .expect("identity")
        .is_some());
}

#[test]
fn import_agent_identity_keeps_different_teams_for_same_user_separate() {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("init storage");
    let mut index = ExistingAccountIndex::build(&storage).expect("index");
    let first = agent_identity_value("runtime-a", "team-a", "same-user", Some("task-a"));
    let second = agent_identity_value("runtime-b", "team-b", "same-user", Some("task-b"));

    import_single_item(&storage, &mut index, &first, 1).expect("first import");
    import_single_item(&storage, &mut index, &second, 2).expect("second import");

    assert_eq!(storage.list_accounts().expect("accounts").len(), 2);
    assert!(storage
        .find_account_agent_identity("team-a")
        .expect("team a")
        .is_some());
    assert!(storage
        .find_account_agent_identity("team-b")
        .expect("team b")
        .is_some());
}

#[test]
fn import_agent_identity_rejects_invalid_private_key() {
    let storage = Storage::open_in_memory().expect("open storage");
    storage.init().expect("init storage");
    let mut index = ExistingAccountIndex::build(&storage).expect("index");
    let item = json!({
        "auth_mode": "agentIdentity",
        "agent_identity": {
            "agent_runtime_id": "runtime-1",
            "agent_private_key": "bm90LWEtcHJpdmF0ZS1rZXk=",
            "account_id": "team-1",
            "chatgpt_user_id": "user-1"
        }
    });

    let err = import_single_item(&storage, &mut index, &item, 1).expect_err("invalid key");
    assert!(err.contains("PKCS#8"));
}

#[test]
fn imported_agent_identity_survives_database_reopen() {
    let path = unique_temp_db_path();
    {
        let storage = Storage::open(&path).expect("open storage");
        storage.init().expect("init storage");
        let mut index = ExistingAccountIndex::build(&storage).expect("index");
        let item = agent_identity_value("runtime-1", "team-1", "user-1", Some("task-1"));
        import_single_item(&storage, &mut index, &item, 1).expect("import");
    }

    let reopened = Storage::open(&path).expect("reopen storage");
    reopened.init().expect("reinit storage");
    let identity = reopened
        .find_account_agent_identity("team-1")
        .expect("find identity")
        .expect("identity");
    assert_eq!(identity.agent_runtime_id, "runtime-1");
    assert_eq!(identity.task_id.as_deref(), Some("task-1"));
    drop(reopened);
    let _ = std::fs::remove_file(path);
}

/// 函数 `resolve_logical_account_id_distinguishes_workspace_under_same_chatgpt`
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
fn resolve_logical_account_id_distinguishes_workspace_under_same_chatgpt() {
    let input = payload();
    let a = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-a"),
        Some("same-fp"),
    )
    .expect("resolve ws-a");
    let b = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-b"),
        Some("same-fp"),
    )
    .expect("resolve ws-b");

    assert_ne!(a, b);
}

/// 函数 `resolve_logical_account_id_is_stable_when_scope_is_stable`
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
fn resolve_logical_account_id_is_stable_when_scope_is_stable() {
    let input = payload();
    let first = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-a"),
        Some("fp-1"),
    )
    .expect("resolve first");
    let second = resolve_logical_account_id(
        &input,
        Some("sub-1"),
        Some("cgpt-1"),
        Some("ws-a"),
        Some("fp-2"),
    )
    .expect("resolve second");

    assert_eq!(first, second);
    assert_eq!(
        first,
        build_account_storage_id("sub-1", Some("cgpt-1"), Some("ws-a"), None)
    );
}

/// 函数 `existing_account_index_next_sort_uses_step_five`
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
fn existing_account_index_next_sort_uses_step_five() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "acc-1".to_string(),
            label: "acc-1".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("cgpt-1".to_string()),
            workspace_id: Some("ws-1".to_string()),
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert acc-1");
    storage
        .insert_account(&Account {
            id: "acc-2".to_string(),
            label: "acc-2".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("cgpt-2".to_string()),
            workspace_id: Some("ws-2".to_string()),
            group_name: None,
            sort: 9,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert acc-2");

    let idx = ExistingAccountIndex::build(&storage).expect("build index");
    assert_eq!(idx.next_sort, 14);
}

/// 函数 `extract_token_payload_supports_flat_codex_format`
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
fn extract_token_payload_supports_flat_codex_format() {
    let value = json!({
        "type": "codex",
        "email": "u@example.com",
        "id_token": "id.flat",
        "account_id": "acc-flat",
        "access_token": "access.flat",
        "refresh_token": "refresh.flat"
    });

    let payload = extract_token_payload(&value).expect("parse flat payload");
    assert_eq!(payload.access_token, "access.flat");
    assert_eq!(payload.id_token, "id.flat");
    assert_eq!(payload.refresh_token, "refresh.flat");
    assert_eq!(payload.account_id_hint.as_deref(), Some("acc-flat"));
    assert_eq!(payload.chatgpt_account_id_hint, None);
}

/// 函数 `extract_token_payload_supports_camel_case_fields`
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
fn extract_token_payload_supports_camel_case_fields() {
    let value = json!({
        "tokens": {
            "idToken": "id.camel",
            "accessToken": "access.camel",
            "refreshToken": "refresh.camel",
            "accountId": "acc-camel",
            "chatgptAccountId": "cgpt-camel"
        }
    });

    let payload = extract_token_payload(&value).expect("parse camel payload");
    assert_eq!(payload.access_token, "access.camel");
    assert_eq!(payload.id_token, "id.camel");
    assert_eq!(payload.refresh_token, "refresh.camel");
    assert_eq!(payload.account_id_hint.as_deref(), Some("acc-camel"));
    assert_eq!(
        payload.chatgpt_account_id_hint.as_deref(),
        Some("cgpt-camel")
    );
}

#[test]
fn parse_items_supports_sub2api_data_exports() {
    let values = parse_items_from_content(
        &json!({
            "type": "sub2api-data",
            "version": 1,
            "exported_at": "2026-07-14T00:00:00Z",
            "proxies": [],
            "accounts": [
                {
                    "name": "BugTeam A",
                    "platform": "openai",
                    "type": "oauth",
                    "credentials": {
                        "access_token": "access.bugteam.a",
                        "chatgpt_account_id": "team-a",
                        "chatgpt_user_id": "user-a"
                    }
                },
                [{
                    "name": "BugTeam B",
                    "platform": "openai",
                    "type": "oauth",
                    "credentials": {
                        "access_token": "access.bugteam.b",
                        "refresh_token": "refresh.b"
                    }
                }]
            ]
        })
        .to_string(),
    )
    .expect("parse sub2api export");

    assert_eq!(values.len(), 2);
    assert_eq!(
        values[0].get("name").and_then(|value| value.as_str()),
        Some("BugTeam A")
    );
    assert_eq!(
        values[1].get("name").and_then(|value| value.as_str()),
        Some("BugTeam B")
    );
}

#[test]
fn parse_items_supports_raw_tokens_and_mixed_lines() {
    let values = parse_items_from_content(
        "raw-access-token\n{\"token\":\"json-access-token\"}\n[[\"nested-access-token\"]]",
    )
    .expect("parse mixed content");

    assert_eq!(values.len(), 3);
    assert_eq!(values[0].as_str(), Some("raw-access-token"));
    assert_eq!(
        values[1].get("token").and_then(|value| value.as_str()),
        Some("json-access-token")
    );
    assert_eq!(values[2].as_str(), Some("nested-access-token"));
}

#[test]
fn extract_token_payload_supports_sub2api_credentials_and_session_identity() {
    let exported = json!({
        "name": "BugTeam Account",
        "notes": "from sub2api",
        "platform": "openai",
        "type": "oauth",
        "credentials": {
            "access_token": "access.sub2api",
            "id_token": "id.sub2api",
            "chatgpt_account_id": "team-sub2api",
            "chatgpt_user_id": "user-sub2api",
            "email": "sub2api@example.com"
        }
    });

    let payload = extract_token_payload(&exported).expect("parse sub2api credentials");
    assert_eq!(payload.access_token, "access.sub2api");
    assert_eq!(payload.id_token, "id.sub2api");
    assert_eq!(payload.refresh_token, "");
    assert_eq!(
        payload.chatgpt_account_id_hint.as_deref(),
        Some("team-sub2api")
    );
    assert_eq!(payload.user_id_hint.as_deref(), Some("user-sub2api"));

    let session = json!({
        "user": {
            "id": "session-user",
            "name": "Session User",
            "email": "session@example.com"
        },
        "account": {
            "id": "session-team",
            "planType": "team"
        },
        "accessToken": "access.session"
    });
    let payload = extract_token_payload(&session).expect("parse session payload");
    assert_eq!(payload.access_token, "access.session");
    assert_eq!(
        payload.chatgpt_account_id_hint.as_deref(),
        Some("session-team")
    );
    assert_eq!(payload.user_id_hint.as_deref(), Some("session-user"));
}

/// 函数 `extract_token_payload_allows_missing_id_and_refresh_tokens`
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
fn extract_token_payload_allows_missing_id_and_refresh_tokens() {
    let value = json!({
        "tokens": {
            "access_token": "access.only",
            "account_id": "acc-only"
        }
    });

    let payload = extract_token_payload(&value).expect("parse optional token payload");
    assert_eq!(payload.access_token, "access.only");
    assert_eq!(payload.id_token, "");
    assert_eq!(payload.refresh_token, "");
    assert_eq!(payload.account_id_hint.as_deref(), Some("acc-only"));
}

/// 函数 `import_single_item_reuses_existing_login_account_by_scope_identity`
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
fn import_single_item_reuses_existing_login_account_by_scope_identity() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let now = now_ts();
    let existing_id = build_account_storage_id("sub-1", Some("cgpt-1"), Some("ws-a"), None);
    storage
        .insert_account(&Account {
            id: existing_id.clone(),
            label: "existing".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("cgpt-1".to_string()),
            workspace_id: Some("ws-a".to_string()),
            group_name: Some("LOGIN".to_string()),
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert existing account");

    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let item = json!({
        "tokens": {
            "access_token": "access.import",
            "id_token": TEST_ID_TOKEN_WS_A,
            "refresh_token": "refresh.import",
            "account_id": "legacy-import-id"
        }
    });

    let created = import_single_item(&storage, &mut idx, &item, 1).expect("import item");
    assert!(!created);

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, existing_id);
    assert_eq!(accounts[0].group_name, None);
    assert!(storage
        .find_account_metadata(&accounts[0].id)
        .expect("find metadata")
        .is_none());

    let token = storage
        .find_token_by_account_id(&accounts[0].id)
        .expect("find token")
        .expect("token");
    assert_eq!(token.account_id, accounts[0].id);
}

/// 函数 `import_single_item_distinguishes_team_members_sharing_account_hint`
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
fn import_single_item_distinguishes_team_members_sharing_account_hint() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");

    let user_a = json!({
        "tokens": {
            "access_token": TEST_ACCESS_TOKEN_TEAM_USER_A,
            "account_id": "team-1",
            "refresh_token": "refresh.user-a"
        }
    });
    let user_b = json!({
        "tokens": {
            "access_token": TEST_ACCESS_TOKEN_TEAM_USER_B,
            "account_id": "team-1",
            "refresh_token": "refresh.user-b"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &user_a, 1).expect("import user a"));
    assert!(import_single_item(&storage, &mut idx, &user_b, 2).expect("import user b"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 2);
    assert!(accounts
        .iter()
        .any(|account| account.id.starts_with("user-a::")));
    assert!(accounts
        .iter()
        .any(|account| account.id.starts_with("user-b::")));
    assert!(accounts
        .iter()
        .all(|account| account.workspace_id.as_deref() == Some("team-1")));

    assert!(!import_single_item(&storage, &mut idx, &user_a, 3).expect("reimport user a"));
    assert_eq!(storage.list_accounts().expect("list accounts").len(), 2);
}

/// 函数 `import_single_item_distinguishes_same_subject_with_different_chatgpt_accounts`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-08
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn import_single_item_distinguishes_same_subject_with_different_chatgpt_accounts() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");

    let team = json!({
        "tokens": {
            "access_token": "team.access",
            "id_token": TEST_ID_TOKEN_SAME_SUB_TEAM,
            "refresh_token": "team.refresh"
        }
    });
    let plus = json!({
        "tokens": {
            "access_token": "plus.access",
            "id_token": TEST_ID_TOKEN_SAME_SUB_PLUS,
            "refresh_token": "plus.refresh"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &team, 1).expect("import team"));
    assert!(import_single_item(&storage, &mut idx, &plus, 2).expect("import plus"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 2);
    assert!(accounts
        .iter()
        .any(|account| account.chatgpt_account_id.as_deref() == Some("cgpt-team")));
    assert!(accounts
        .iter()
        .any(|account| account.chatgpt_account_id.as_deref() == Some("cgpt-plus")));

    assert!(!import_single_item(&storage, &mut idx, &team, 3).expect("reimport team"));
    assert_eq!(storage.list_accounts().expect("list accounts").len(), 2);
}

/// 函数 `import_single_item_distinguishes_same_subject_across_team_workspaces`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-08
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn import_single_item_distinguishes_same_subject_across_team_workspaces() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");

    let team_a = json!({
        "tokens": {
            "access_token": "team-a.access",
            "id_token": TEST_ID_TOKEN_SAME_SUB_TEAM_A,
            "refresh_token": "team-a.refresh"
        }
    });
    let team_b = json!({
        "tokens": {
            "access_token": "team-b.access",
            "id_token": TEST_ID_TOKEN_SAME_SUB_TEAM_B,
            "refresh_token": "team-b.refresh"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &team_a, 1).expect("import team a"));
    assert!(import_single_item(&storage, &mut idx, &team_b, 2).expect("import team b"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 2);
    assert!(accounts
        .iter()
        .any(|account| account.workspace_id.as_deref() == Some("ws-team-a")));
    assert!(accounts
        .iter()
        .any(|account| account.workspace_id.as_deref() == Some("ws-team-b")));
}

/// 函数 `import_single_item_restores_account_when_old_import_overwrote_scoped_identity`
///
/// 作者: gaohongshun
///
/// 时间: 2026-05-08
///
/// # 参数
/// 无
///
/// # 返回
/// 无
#[test]
fn import_single_item_restores_account_when_old_import_overwrote_scoped_identity() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let now = now_ts();
    let old_team_scoped_id =
        build_account_storage_id("same-user", Some("cgpt-team"), Some("ws-shared"), None);
    storage
        .insert_account(&Account {
            id: old_team_scoped_id.clone(),
            label: "same@example.com".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("cgpt-plus".to_string()),
            workspace_id: Some("ws-shared".to_string()),
            group_name: Some("IMPORT".to_string()),
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert old overwritten account");
    storage
        .insert_token(&Token {
            account_id: old_team_scoped_id.clone(),
            id_token: TEST_ID_TOKEN_SAME_SUB_PLUS.to_string(),
            access_token: "plus.access.old".to_string(),
            refresh_token: "plus.refresh.old".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert plus token");

    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let team = json!({
        "tokens": {
            "access_token": "team.access",
            "id_token": TEST_ID_TOKEN_SAME_SUB_TEAM,
            "refresh_token": "team.refresh"
        }
    });
    let plus = json!({
        "tokens": {
            "access_token": "plus.access",
            "id_token": TEST_ID_TOKEN_SAME_SUB_PLUS,
            "refresh_token": "plus.refresh"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &team, 1).expect("restore team"));
    assert!(!import_single_item(&storage, &mut idx, &plus, 2).expect("refresh plus"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 2);
    assert!(accounts
        .iter()
        .any(|account| account.chatgpt_account_id.as_deref() == Some("cgpt-team")));
    assert!(accounts
        .iter()
        .any(|account| account.chatgpt_account_id.as_deref() == Some("cgpt-plus")));
}

/// 函数 `import_single_item_reuses_legacy_team_account_when_token_subject_matches`
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
fn import_single_item_reuses_legacy_team_account_when_token_subject_matches() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let now = now_ts();
    storage
        .insert_account(&Account {
            id: "team-1".to_string(),
            label: "legacy team account".to_string(),
            issuer: "https://auth.openai.com".to_string(),
            chatgpt_account_id: Some("team-1".to_string()),
            workspace_id: Some("team-1".to_string()),
            group_name: None,
            sort: 0,
            status: "active".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("insert legacy account");
    storage
        .insert_token(&Token {
            account_id: "team-1".to_string(),
            id_token: "".to_string(),
            access_token: TEST_ACCESS_TOKEN_TEAM_USER_A.to_string(),
            refresh_token: "refresh.user-a.old".to_string(),
            api_key_access_token: None,
            last_refresh: now,
        })
        .expect("insert legacy token");

    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let item = json!({
        "tokens": {
            "access_token": TEST_ACCESS_TOKEN_TEAM_USER_A,
            "account_id": "team-1",
            "refresh_token": "refresh.user-a.new"
        }
    });

    let created = import_single_item(&storage, &mut idx, &item, 1).expect("import item");
    assert!(!created);

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, "team-1");
    let token = storage
        .find_token_by_account_id("team-1")
        .expect("find token")
        .expect("token");
    assert_eq!(token.refresh_token, "refresh.user-a.new");
}

/// 函数 `import_single_item_prefers_meta_fields_for_new_account`
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
fn import_single_item_prefers_email_and_meta_fields_for_new_account() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let item = json!({
        "tokens": {
            "access_token": "access.meta",
            "id_token": TEST_ID_TOKEN_META,
            "refresh_token": "refresh.meta",
            "account_id": "exported-account-id"
        },
        "meta": {
            "label": "Meta Label",
            "issuer": "https://issuer.example",
            "note": "Meta Note",
            "tags": ["高频", "团队A"],
            "workspace_id": "ws-manual",
            "chatgpt_account_id": "cgpt-manual"
        }
    });

    let created = import_single_item(&storage, &mut idx, &item, 1).expect("import item");
    assert!(created);

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(
        accounts[0].id,
        build_account_storage_id("sub-1", Some("cgpt-manual"), Some("ws-manual"), None)
    );
    assert_eq!(accounts[0].label, "meta@example.com");
    assert_eq!(accounts[0].issuer, "https://issuer.example");
    assert_eq!(accounts[0].group_name, None);
    assert_eq!(
        accounts[0].chatgpt_account_id.as_deref(),
        Some("cgpt-manual")
    );
    assert_eq!(accounts[0].workspace_id.as_deref(), Some("ws-manual"));
    let metadata = storage
        .find_account_metadata(&accounts[0].id)
        .expect("find metadata")
        .expect("metadata");
    assert_eq!(metadata.note.as_deref(), Some("Meta Note"));
    assert_eq!(metadata.tags.as_deref(), Some("高频,团队A"));
}

/// 函数 `import_single_item_allows_missing_id_and_refresh_tokens`
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
fn import_single_item_allows_missing_id_and_refresh_tokens() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let item = json!({
        "tokens": {
            "access_token": "access.only",
            "account_id": "legacy-import-id"
        },
        "meta": {
            "label": "Only Access Token",
            "workspace_id": "ws-manual",
            "chatgpt_account_id": "cgpt-manual"
        }
    });

    let created = import_single_item(&storage, &mut idx, &item, 1).expect("import item");
    assert!(created);

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].label, "Only Access Token");
    assert_eq!(
        accounts[0].chatgpt_account_id.as_deref(),
        Some("cgpt-manual")
    );
    assert_eq!(accounts[0].workspace_id.as_deref(), Some("ws-manual"));

    let token = storage
        .find_token_by_account_id(&accounts[0].id)
        .expect("find token")
        .expect("token");
    assert_eq!(token.access_token, "access.only");
    assert_eq!(token.id_token, "");
    assert_eq!(token.refresh_token, "");
}

#[test]
fn import_single_item_supports_bugteam_access_token_only_account() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let item = json!({
        "name": "BugTeam Access Only",
        "platform": "openai",
        "type": "oauth",
        "credentials": {
            "access_token": "opaque-bugteam-access-token",
            "chatgpt_account_id": "bugteam-workspace",
            "chatgpt_user_id": "bugteam-user"
        }
    });

    let created = import_single_item(&storage, &mut idx, &item, 1).expect("import bugteam");
    assert!(created);

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].label, "BugTeam Access Only");
    assert_eq!(
        accounts[0].chatgpt_account_id.as_deref(),
        Some("bugteam-workspace")
    );
    let token = storage
        .find_token_by_account_id(&accounts[0].id)
        .expect("find token")
        .expect("token");
    assert_eq!(token.access_token, "opaque-bugteam-access-token");
    assert_eq!(token.refresh_token, "");
}

#[test]
fn import_single_item_prefers_bugteam_email_over_display_name() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let user_email = json!({
        "name": "Mary Jones",
        "user": {
            "name": "Ignored User Name",
            "email": "mary@example.com"
        },
        "credentials": {
            "access_token": "access.mary",
            "chatgpt_account_id": "team-mary",
            "chatgpt_user_id": "user-mary"
        }
    });
    let credentials_email = json!({
        "name": "Emma Smith",
        "credentials": {
            "access_token": "access.emma",
            "email": "emma@example.com",
            "chatgpt_account_id": "team-emma",
            "chatgpt_user_id": "user-emma"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &user_email, 1).expect("import mary"));
    assert!(import_single_item(&storage, &mut idx, &credentials_email, 2).expect("import emma"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 2);
    assert!(accounts
        .iter()
        .any(|account| account.label == "mary@example.com"));
    assert!(accounts
        .iter()
        .any(|account| account.label == "emma@example.com"));
}

#[test]
fn import_single_item_persists_sub2api_account_type_labels() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let credentials_plan = json!({
        "name": "K12 Credentials Account",
        "credentials": {
            "access_token": "access.k12.credentials",
            "email": "k12-credentials@example.com",
            "plan_type": "k12",
            "chatgpt_account_id": "team-k12-credentials",
            "chatgpt_user_id": "user-k12-credentials"
        }
    });
    let tags_plan = json!({
        "name": "K12 Tagged Account",
        "tags": ["BugTeam", "K12"],
        "credentials": {
            "access_token": "access.k12.tags",
            "email": "k12-tags@example.com",
            "chatgpt_account_id": "team-k12-tags",
            "chatgpt_user_id": "user-k12-tags"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &credentials_plan, 1)
        .expect("import credentials plan"));
    assert!(import_single_item(&storage, &mut idx, &tags_plan, 2).expect("import tags plan"));

    let subscriptions = storage
        .list_account_subscriptions()
        .expect("list subscriptions");
    assert_eq!(subscriptions.len(), 2);
    assert!(subscriptions
        .iter()
        .all(|item| item.account_plan_type.as_deref() == Some("k12")));
    assert!(subscriptions
        .iter()
        .all(|item| item.plan_type.as_deref() == Some("k12")));
}

#[test]
fn import_single_item_reimport_upgrades_old_name_label_to_email() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let original = json!({
        "name": "Mary Jones",
        "credentials": {
            "access_token": "access.mary.old",
            "chatgpt_account_id": "team-mary",
            "chatgpt_user_id": "user-mary"
        }
    });
    let updated = json!({
        "name": "Mary Jones",
        "credentials": {
            "access_token": "access.mary.new",
            "email": "mary@example.com",
            "chatgpt_account_id": "team-mary",
            "chatgpt_user_id": "user-mary"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &original, 1).expect("initial import"));
    assert!(!import_single_item(&storage, &mut idx, &updated, 2).expect("reimport"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].label, "mary@example.com");
}

#[test]
fn import_single_item_reimport_replaces_manually_edited_label_with_email() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let original = json!({
        "name": "Emma Smith",
        "credentials": {
            "access_token": "access.emma.old",
            "chatgpt_account_id": "team-emma",
            "chatgpt_user_id": "user-emma"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &original, 1).expect("initial import"));
    let mut account = storage
        .list_accounts()
        .expect("list accounts")
        .into_iter()
        .next()
        .expect("account");
    account.label = "My Team Account".to_string();
    storage
        .insert_account(&account)
        .expect("update manual account label");
    let mut idx = ExistingAccountIndex::build(&storage).expect("rebuild index");

    let updated = json!({
        "name": "Emma Smith",
        "credentials": {
            "access_token": "access.emma.new",
            "email": "emma@example.com",
            "chatgpt_account_id": "team-emma",
            "chatgpt_user_id": "user-emma"
        }
    });
    assert!(!import_single_item(&storage, &mut idx, &updated, 2).expect("reimport"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].label, "emma@example.com");
}

#[test]
fn import_single_item_email_overrides_explicit_label() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let item = json!({
        "name": "Account Name",
        "meta": {
            "label": "Custom Label"
        },
        "credentials": {
            "access_token": "access.custom",
            "email": "custom@example.com",
            "chatgpt_account_id": "team-custom",
            "chatgpt_user_id": "user-custom"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &item, 1).expect("import custom"));
    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].label, "custom@example.com");
}

#[test]
fn import_single_item_raw_access_token_uses_stable_fingerprint_identity() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let item = json!("opaque-raw-access-token");

    assert!(import_single_item(&storage, &mut idx, &item, 1).expect("first import"));
    assert!(!import_single_item(&storage, &mut idx, &item, 2).expect("second import"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert!(accounts[0].id.starts_with("import-access-"));
}

#[test]
fn import_single_item_access_only_update_preserves_existing_refresh_token() {
    let storage = Storage::open_in_memory().expect("open in memory");
    storage.init().expect("init");
    let mut idx = ExistingAccountIndex::build(&storage).expect("build index");
    let with_refresh = json!({
        "credentials": {
            "access_token": "access.old",
            "refresh_token": "refresh.keep",
            "chatgpt_account_id": "team-preserve",
            "chatgpt_user_id": "user-preserve"
        }
    });
    let access_only = json!({
        "credentials": {
            "access_token": "access.new",
            "chatgpt_account_id": "team-preserve",
            "chatgpt_user_id": "user-preserve"
        }
    });

    assert!(import_single_item(&storage, &mut idx, &with_refresh, 1).expect("initial import"));
    assert!(!import_single_item(&storage, &mut idx, &access_only, 2).expect("access-only update"));

    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    let token = storage
        .find_token_by_account_id(&accounts[0].id)
        .expect("find token")
        .expect("token");
    assert_eq!(token.access_token, "access.new");
    assert_eq!(token.refresh_token, "refresh.keep");
}

/// 函数 `import_account_auth_json_keeps_valid_items_when_one_content_is_invalid`
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
fn import_account_auth_json_keeps_valid_items_when_one_content_is_invalid() {
    let _guard = crate::test_env_guard();
    let db_path = unique_temp_db_path();
    let previous_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
    std::env::set_var("CODEXMANAGER_DB_PATH", &db_path);

    let storage = Storage::open(&db_path).expect("open storage");
    storage.init().expect("init storage");
    drop(storage);

    let result = import_account_auth_json(vec![
        json!({
            "type": "codex",
            "email": "valid@example.com",
            "id_token": TEST_ID_TOKEN_META,
            "account_id": "valid-account",
            "access_token": "access.valid",
            "refresh_token": "refresh.valid"
        })
        .to_string(),
        "{\"accessToken\":".to_string(),
    ])
    .expect("import account auth json");

    assert_eq!(result.total, 2);
    assert_eq!(result.created, 1);
    assert_eq!(result.updated, 0);
    assert_eq!(result.failed, 1);
    assert!(result
        .errors
        .iter()
        .any(|item| { item.message.contains("invalid JSON object stream") }));

    let storage = Storage::open(&db_path).expect("reopen storage");
    let accounts = storage.list_accounts().expect("list accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].label, "meta@example.com");

    if let Some(value) = previous_db_path {
        std::env::set_var("CODEXMANAGER_DB_PATH", value);
    } else {
        std::env::remove_var("CODEXMANAGER_DB_PATH");
    }
    let _ = std::fs::remove_file(&db_path);
}
