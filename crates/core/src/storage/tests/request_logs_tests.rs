use super::{RequestLog, RequestTokenStat, Storage};
use crate::storage::now_ts;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn account_billing_totals_survive_request_log_clear() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let log = RequestLog {
        account_id: Some("acc-billing".to_string()),
        request_path: "/v1/responses".to_string(),
        method: "POST".to_string(),
        model: Some("gpt-5.4".to_string()),
        status_code: Some(200),
        created_at: 100,
        ..Default::default()
    };
    let stat = RequestTokenStat {
        request_log_id: 0,
        key_id: None,
        account_id: log.account_id.clone(),
        model: log.model.clone(),
        input_tokens: Some(1_000),
        cached_input_tokens: Some(200),
        output_tokens: Some(300),
        total_tokens: Some(1_300),
        reasoning_output_tokens: Some(100),
        estimated_cost_usd: Some(0.25),
        created_at: 100,
    };
    storage
        .insert_request_log_with_token_stat(&log, &stat)
        .expect("insert log and billing stats");

    storage.clear_request_logs().expect("clear request logs");

    let items = storage
        .summarize_request_token_stats_by_account()
        .expect("read account billing totals");
    let item = items
        .iter()
        .find(|item| item.account_id == "acc-billing")
        .expect("account billing total");
    assert_eq!(item.usage.request_count, 1);
    assert_eq!(item.usage.total_tokens, 1_300);
    assert_eq!(item.usage.estimated_cost_usd, 0.25);
    assert_eq!(item.primary_window.request_count, 1);
    assert_eq!(item.primary_window.total_tokens, 1_300);
    assert_eq!(item.primary_window.estimated_cost_usd, 0.25);
    assert_eq!(item.secondary_window.request_count, 1);
    assert_eq!(item.secondary_window.total_tokens, 1_300);
    assert_eq!(item.secondary_window.estimated_cost_usd, 0.25);
}

#[test]
fn account_billing_window_stats_roll_over_independently() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");

    for (index, created_at) in [100_i64, 200, 18_100].into_iter().enumerate() {
        let log = RequestLog {
            trace_id: Some(format!("trc-window-{index}")),
            account_id: Some("acc-window".to_string()),
            request_path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            status_code: Some(200),
            created_at,
            ..Default::default()
        };
        let stat = RequestTokenStat {
            account_id: log.account_id.clone(),
            total_tokens: Some(100),
            estimated_cost_usd: Some(0.10),
            created_at,
            ..Default::default()
        };
        storage
            .insert_request_log_with_token_stat(&log, &stat)
            .expect("insert window billing stat");
    }

    let items = storage
        .summarize_request_token_stats_by_account()
        .expect("read account window billing stats");
    let item = items
        .iter()
        .find(|item| item.account_id == "acc-window")
        .expect("account window billing stats");
    assert_eq!(item.usage.request_count, 3);
    assert_eq!(item.primary_window.request_count, 1);
    assert_eq!(item.primary_window.total_tokens, 100);
    assert!((item.primary_window.estimated_cost_usd - 0.10).abs() < f64::EPSILON);
    assert_eq!(item.secondary_window.request_count, 3);
    assert_eq!(item.secondary_window.total_tokens, 300);
    assert!((item.secondary_window.estimated_cost_usd - 0.30).abs() < f64::EPSILON);
}

#[test]
fn account_billing_window_stats_survive_database_reopen() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let db_path =
        std::env::temp_dir().join(format!("codexmanager-account-window-reopen-{nonce}.db"));
    let created_at = now_ts();

    {
        let storage = Storage::open(&db_path).expect("open");
        storage.init().expect("init");
        let log = RequestLog {
            account_id: Some("acc-reopen".to_string()),
            request_path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            status_code: Some(200),
            created_at,
            ..Default::default()
        };
        let stat = RequestTokenStat {
            account_id: log.account_id.clone(),
            total_tokens: Some(321),
            estimated_cost_usd: Some(0.45),
            created_at,
            ..Default::default()
        };
        storage
            .insert_request_log_with_token_stat(&log, &stat)
            .expect("insert persisted window billing stat");
    }

    {
        let storage = Storage::open(&db_path).expect("reopen");
        storage.init().expect("reinit");
        let items = storage
            .summarize_request_token_stats_by_account()
            .expect("read reopened account window billing stats");
        let item = items
            .iter()
            .find(|item| item.account_id == "acc-reopen")
            .expect("reopened account window billing stats");
        assert_eq!(item.primary_window.request_count, 1);
        assert_eq!(item.primary_window.total_tokens, 321);
        assert_eq!(item.primary_window.estimated_cost_usd, 0.45);
        assert_eq!(item.secondary_window.request_count, 1);
        assert_eq!(item.secondary_window.total_tokens, 321);
        assert_eq!(item.secondary_window.estimated_cost_usd, 0.45);
    }

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}

#[test]
fn inclusive_openai_token_fallback_survives_database_reopen() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!("codexmanager-token-fallback-{nonce}.db"));
    let created_at = now_ts();

    {
        let storage = Storage::open(&db_path).expect("open");
        storage.init().expect("init");
        let log = RequestLog {
            account_id: Some("acc-inclusive-input".to_string()),
            request_path: "/v1/responses".to_string(),
            method: "POST".to_string(),
            status_code: Some(200),
            created_at,
            ..Default::default()
        };
        let stat = RequestTokenStat {
            account_id: log.account_id.clone(),
            input_tokens: Some(1_000),
            cached_input_tokens: Some(800),
            output_tokens: Some(50),
            total_tokens: None,
            estimated_cost_usd: Some(0.25),
            created_at,
            ..Default::default()
        };
        storage
            .insert_request_log_with_token_stat(&log, &stat)
            .expect("insert inclusive input usage");
    }

    for _ in 0..2 {
        let storage = Storage::open(&db_path).expect("reopen");
        storage.init().expect("reinit");
        let item = storage
            .summarize_request_token_stats_by_account()
            .expect("read account usage")
            .into_iter()
            .find(|item| item.account_id == "acc-inclusive-input")
            .expect("persisted account usage");
        assert_eq!(item.usage.total_tokens, 1_050);
        assert_eq!(item.primary_window.total_tokens, 1_050);
        assert_eq!(item.secondary_window.total_tokens, 1_050);
        assert_eq!(item.usage.estimated_cost_usd, 0.25);
    }

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
}

#[test]
fn sub2api_repricing_migration_is_idempotent_and_updates_windows() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let created_at = now_ts();
    let old_cost = 0.00085_f64;
    let expected_cost = 0.0029_f64;
    let log = RequestLog {
        account_id: Some("acc-gpt56-reprice".to_string()),
        request_path: "/v1/responses".to_string(),
        method: "POST".to_string(),
        model: Some("gpt-5.6-sol".to_string()),
        status_code: Some(200),
        created_at,
        ..Default::default()
    };
    let stat = RequestTokenStat {
        account_id: log.account_id.clone(),
        model: log.model.clone(),
        input_tokens: Some(1_000),
        cached_input_tokens: Some(800),
        output_tokens: Some(50),
        total_tokens: None,
        estimated_cost_usd: Some(old_cost),
        created_at,
        ..Default::default()
    };
    storage
        .insert_request_log_with_token_stat(&log, &stat)
        .expect("insert old priced usage");

    storage
        .conn
        .execute(
            "UPDATE account_usage_billing_stats
             SET total_tokens = 250,
                 primary_window_total_tokens = 250,
                 secondary_window_total_tokens = 250
             WHERE account_id = 'acc-gpt56-reprice'",
            [],
        )
        .expect("simulate old cached-token fallback");

    let migration = include_str!("../../../migrations/069_reprice_sub2api_openai_usage.sql");
    for _ in 0..2 {
        storage
            .conn
            .execute_batch(migration)
            .expect("apply repricing migration");
        let item = storage
            .summarize_request_token_stats_by_account()
            .expect("read repriced account usage")
            .into_iter()
            .find(|item| item.account_id == "acc-gpt56-reprice")
            .expect("repriced account usage");
        assert_eq!(item.usage.total_tokens, 1_050);
        assert_eq!(item.primary_window.total_tokens, 1_050);
        assert_eq!(item.secondary_window.total_tokens, 1_050);
        assert!((item.usage.estimated_cost_usd - expected_cost).abs() < 1e-12);
        assert!((item.primary_window.estimated_cost_usd - expected_cost).abs() < 1e-12);
        assert!((item.secondary_window.estimated_cost_usd - expected_cost).abs() < 1e-12);
    }
}

#[test]
fn account_billing_window_backfill_restores_recent_persisted_stats() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let created_at = now_ts();
    storage
        .insert_request_token_stat(&RequestTokenStat {
            account_id: Some("acc-backfill".to_string()),
            total_tokens: Some(654),
            estimated_cost_usd: Some(0.78),
            created_at,
            ..Default::default()
        })
        .expect("insert historical token stat");
    storage
        .conn
        .execute(
            "INSERT INTO account_usage_billing_stats (
                account_id, request_count, total_tokens, estimated_cost_usd, updated_at
             ) VALUES (?1, 1, ?2, ?3, ?4)",
            ("acc-backfill", 654_i64, 0.78_f64, created_at),
        )
        .expect("insert historical account billing total");

    storage
        .backfill_account_usage_billing_windows()
        .expect("backfill account billing windows");
    let items = storage
        .summarize_request_token_stats_by_account()
        .expect("read backfilled account window billing stats");
    let item = items
        .iter()
        .find(|item| item.account_id == "acc-backfill")
        .expect("backfilled account window billing stats");
    assert_eq!(item.primary_window.request_count, 1);
    assert_eq!(item.primary_window.total_tokens, 654);
    assert_eq!(item.primary_window.estimated_cost_usd, 0.78);
    assert_eq!(item.secondary_window.request_count, 1);
    assert_eq!(item.secondary_window.total_tokens, 654);
    assert_eq!(item.secondary_window.estimated_cost_usd, 0.78);
}

/// 函数 `collect_query_plan_details`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - sql: 参数 sql
///
/// # 返回
/// 返回函数执行结果
fn collect_query_plan_details(storage: &Storage, sql: &str) -> Vec<String> {
    let mut stmt = storage.conn.prepare(sql).expect("prepare explain");
    let mut rows = stmt.query([]).expect("query explain");
    let mut details = Vec::new();
    while let Some(row) = rows.next().expect("next explain row") {
        let detail: String = row.get(3).expect("detail");
        details.push(detail.to_ascii_lowercase());
    }
    details
}

/// 函数 `method_exact_query_matches_composite_index`
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
fn method_exact_query_matches_composite_index() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let details = collect_query_plan_details(
        &storage,
        "EXPLAIN QUERY PLAN
         SELECT key_id, account_id, request_path, method, model, reasoning_effort, upstream_url, status_code, error, created_at
         FROM request_logs
         WHERE method = 'POST'
         ORDER BY created_at DESC, id DESC
         LIMIT 100",
    );
    assert!(details
        .iter()
        .any(|detail| detail.contains("idx_request_logs_method_created_at")));
}

/// 函数 `key_exact_query_matches_composite_index`
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
fn key_exact_query_matches_composite_index() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");
    let details = collect_query_plan_details(
        &storage,
        "EXPLAIN QUERY PLAN
         SELECT key_id, account_id, request_path, method, model, reasoning_effort, upstream_url, status_code, error, created_at
         FROM request_logs
         WHERE key_id = 'gk_1'
         ORDER BY created_at DESC, id DESC
         LIMIT 100",
    );
    assert!(details
        .iter()
        .any(|detail| detail.contains("idx_request_logs_key_id_created_at")));
}

/// 函数 `insert_request_log_with_token_stat_is_visible_via_join`
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
fn insert_request_log_with_token_stat_is_visible_via_join() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");

    let created_at = 123456_i64;
    let log = RequestLog {
        trace_id: Some("trc-1".to_string()),
        key_id: Some("gk_1".to_string()),
        account_id: Some("acc_1".to_string()),
        initial_account_id: Some("acc_1".to_string()),
        attempted_account_ids_json: Some(r#"["acc_1"]"#.to_string()),
        request_path: "/v1/responses".to_string(),
        original_path: Some("/v1/chat/completions".to_string()),
        adapted_path: Some("/v1/responses".to_string()),
        method: "POST".to_string(),
        request_type: Some("http".to_string()),
        model: Some("gpt-5".to_string()),
        upstream_model: Some("gpt-provider-5".to_string()),
        actual_source_kind: Some("openai_account".to_string()),
        actual_source_id: Some("acc_1".to_string()),
        reasoning_effort: Some("medium".to_string()),
        service_tier: Some("fast".to_string()),
        effective_service_tier: Some("priority".to_string()),
        response_adapter: Some("OpenAIChatCompletionsJson".to_string()),
        upstream_url: Some("https://example.test".to_string()),
        aggregate_api_supplier_name: None,
        aggregate_api_url: None,
        status_code: Some(200),
        duration_ms: Some(1234),
        first_response_ms: Some(456),
        input_tokens: None,
        cached_input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        reasoning_output_tokens: None,
        estimated_cost_usd: None,
        error: None,
        created_at,
        ..Default::default()
    };

    let stat = RequestTokenStat {
        request_log_id: 0,
        key_id: log.key_id.clone(),
        account_id: log.account_id.clone(),
        model: log.model.clone(),
        input_tokens: Some(10),
        cached_input_tokens: Some(1),
        output_tokens: Some(2),
        total_tokens: Some(12),
        reasoning_output_tokens: Some(3),
        estimated_cost_usd: Some(0.123),
        created_at,
    };

    let (_request_log_id, token_err) = storage
        .insert_request_log_with_token_stat(&log, &stat)
        .expect("insert request log with token stat");
    assert!(token_err.is_none(), "token stat should insert");

    let logs = storage
        .list_request_logs(None, 10)
        .expect("list request logs");
    assert_eq!(logs.len(), 1);
    let row = &logs[0];
    assert_eq!(row.trace_id.as_deref(), Some("trc-1"));
    assert_eq!(row.initial_account_id.as_deref(), Some("acc_1"));
    assert_eq!(
        row.attempted_account_ids_json.as_deref(),
        Some(r#"["acc_1"]"#)
    );
    assert_eq!(row.request_path, log.request_path);
    assert_eq!(row.original_path.as_deref(), Some("/v1/chat/completions"));
    assert_eq!(row.adapted_path.as_deref(), Some("/v1/responses"));
    assert_eq!(row.request_type.as_deref(), Some("http"));
    assert_eq!(row.model.as_deref(), Some("gpt-5"));
    assert_eq!(row.upstream_model.as_deref(), Some("gpt-provider-5"));
    assert_eq!(row.actual_source_kind.as_deref(), Some("openai_account"));
    assert_eq!(row.actual_source_id.as_deref(), Some("acc_1"));
    assert_eq!(row.service_tier.as_deref(), Some("fast"));
    assert_eq!(row.effective_service_tier.as_deref(), Some("priority"));
    assert_eq!(row.first_response_ms, Some(456));
    assert_eq!(
        row.response_adapter.as_deref(),
        Some("OpenAIChatCompletionsJson")
    );
    assert_eq!(row.input_tokens, Some(10));
    assert_eq!(row.cached_input_tokens, Some(1));
    assert_eq!(row.output_tokens, Some(2));
    assert_eq!(row.total_tokens, Some(12));
    assert_eq!(row.reasoning_output_tokens, Some(3));
    assert_eq!(row.estimated_cost_usd, Some(0.123));
}

/// 函数 `token_stat_failure_still_commits_request_log`
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
fn token_stat_failure_still_commits_request_log() {
    let storage = Storage::open_in_memory().expect("open");
    // Only create request_logs table, so request_token_stats insert fails.
    storage
        .ensure_request_logs_table()
        .expect("ensure logs table");

    let created_at = 42_i64;
    let log = RequestLog {
        trace_id: Some("trc-2".to_string()),
        key_id: Some("gk_1".to_string()),
        account_id: Some("acc_1".to_string()),
        initial_account_id: Some("acc_1".to_string()),
        attempted_account_ids_json: Some(r#"["acc_1"]"#.to_string()),
        request_path: "/v1/responses".to_string(),
        original_path: Some("/v1/responses".to_string()),
        adapted_path: Some("/v1/responses".to_string()),
        method: "POST".to_string(),
        model: Some("gpt-5".to_string()),
        reasoning_effort: None,
        response_adapter: Some("Passthrough".to_string()),
        upstream_url: None,
        aggregate_api_supplier_name: None,
        aggregate_api_url: None,
        status_code: Some(200),
        duration_ms: None,
        first_response_ms: None,
        input_tokens: None,
        cached_input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        reasoning_output_tokens: None,
        estimated_cost_usd: None,
        error: None,
        created_at,
        ..Default::default()
    };

    let stat = RequestTokenStat {
        request_log_id: 0,
        key_id: log.key_id.clone(),
        account_id: log.account_id.clone(),
        model: log.model.clone(),
        input_tokens: Some(1),
        cached_input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        reasoning_output_tokens: None,
        estimated_cost_usd: None,
        created_at,
    };

    let (_request_log_id, token_err) = storage
        .insert_request_log_with_token_stat(&log, &stat)
        .expect("insert request log with token stat");
    assert!(token_err.is_some(), "token stat insert should fail");

    let count: i64 = storage
        .conn
        .query_row("SELECT COUNT(1) FROM request_logs", [], |row| row.get(0))
        .expect("count request_logs");
    assert_eq!(count, 1);
}

/// 函数 `request_logs_support_backend_pagination_and_status_filters`
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
fn request_logs_support_backend_pagination_and_status_filters() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");

    for index in 0..5_i64 {
        let created_at = 1_000 + index;
        let status_code = match index {
            0 | 1 => Some(200),
            2 => Some(404),
            _ => Some(502),
        };
        let error = if status_code.unwrap_or_default() >= 500 {
            Some("upstream interrupted".to_string())
        } else {
            None
        };
        let request_log_id = storage
            .insert_request_log(&RequestLog {
                trace_id: Some(format!("trc-{index}")),
                key_id: Some("gk-log".to_string()),
                account_id: Some("acc-log".to_string()),
                initial_account_id: Some("acc-log".to_string()),
                attempted_account_ids_json: Some(r#"["acc-log"]"#.to_string()),
                request_path: format!("/v1/responses/{index}"),
                original_path: Some("/v1/responses".to_string()),
                adapted_path: Some("/v1/responses".to_string()),
                method: "POST".to_string(),
                model: Some("gpt-5".to_string()),
                reasoning_effort: Some("high".to_string()),
                response_adapter: Some("Passthrough".to_string()),
                upstream_url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
                aggregate_api_supplier_name: None,
                aggregate_api_url: None,
                status_code,
                duration_ms: Some(200 + index),
                first_response_ms: None,
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error,
                created_at,
                ..Default::default()
            })
            .expect("insert request log");
        storage
            .insert_request_token_stat(&RequestTokenStat {
                request_log_id,
                key_id: Some("gk-log".to_string()),
                account_id: Some("acc-log".to_string()),
                model: Some("gpt-5".to_string()),
                input_tokens: Some(10 + index),
                cached_input_tokens: Some(1),
                output_tokens: Some(2),
                total_tokens: Some(20 + index),
                reasoning_output_tokens: Some(0),
                estimated_cost_usd: Some(0.01),
                created_at,
            })
            .expect("insert token stat");
    }

    let page = storage
        .list_request_logs_paginated(None, Some("5xx"), None, None, 0, 1)
        .expect("list paginated logs");
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].trace_id.as_deref(), Some("trc-4"));

    let total_5xx = storage
        .count_request_logs(None, Some("5xx"), None, None)
        .expect("count 5xx logs");
    assert_eq!(total_5xx, 2);
}

/// 函数 `request_logs_filtered_summary_aggregates_counts_and_tokens`
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
fn request_logs_filtered_summary_aggregates_counts_and_tokens() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");

    for (index, status_code, total_tokens, error) in [
        (0_i64, Some(200_i64), Some(30_i64), None),
        (1_i64, Some(200_i64), Some(50_i64), None),
        (2_i64, Some(502_i64), Some(70_i64), Some("upstream error")),
    ] {
        let created_at = 2_000 + index;
        let request_log_id = storage
            .insert_request_log(&RequestLog {
                trace_id: Some(format!("trc-sum-{index}")),
                key_id: Some("gk-sum".to_string()),
                account_id: Some("acc-sum".to_string()),
                initial_account_id: Some("acc-sum".to_string()),
                attempted_account_ids_json: Some(r#"["acc-sum"]"#.to_string()),
                request_path: "/v1/responses".to_string(),
                original_path: Some("/v1/responses".to_string()),
                adapted_path: Some("/v1/responses".to_string()),
                method: "POST".to_string(),
                model: Some("gpt-5".to_string()),
                reasoning_effort: Some("medium".to_string()),
                response_adapter: Some("Passthrough".to_string()),
                upstream_url: Some("https://chatgpt.com/backend-api/codex/responses".to_string()),
                aggregate_api_supplier_name: None,
                aggregate_api_url: None,
                status_code,
                duration_ms: Some(900),
                first_response_ms: None,
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                reasoning_output_tokens: None,
                estimated_cost_usd: None,
                error: error.map(|value| value.to_string()),
                created_at,
                ..Default::default()
            })
            .expect("insert request log");
        storage
            .insert_request_token_stat(&RequestTokenStat {
                request_log_id,
                key_id: Some("gk-sum".to_string()),
                account_id: Some("acc-sum".to_string()),
                model: Some("gpt-5".to_string()),
                input_tokens: None,
                cached_input_tokens: None,
                output_tokens: None,
                total_tokens,
                reasoning_output_tokens: Some(0),
                estimated_cost_usd: Some(0.01),
                created_at,
            })
            .expect("insert token stat");
    }

    let summary = storage
        .summarize_request_logs_filtered(None, Some("all"), None, None)
        .expect("summarize filtered logs");
    assert_eq!(summary.count, 3);
    assert_eq!(summary.success_count, 2);
    assert_eq!(summary.error_count, 1);
    assert_eq!(summary.total_tokens, 150);
    assert_eq!(summary.estimated_cost_usd, 0.03);
}

#[test]
fn request_logs_support_time_range_filters() {
    let storage = Storage::open_in_memory().expect("open");
    storage.init().expect("init");

    for (index, created_at) in [1_000_i64, 1_900_i64, 3_100_i64].into_iter().enumerate() {
        let request_log_id = storage
            .insert_request_log(&RequestLog {
                trace_id: Some(format!("trc-time-{index}")),
                key_id: Some("gk-time".to_string()),
                account_id: Some("acc-time".to_string()),
                request_path: "/v1/responses".to_string(),
                method: "POST".to_string(),
                status_code: Some(200),
                created_at,
                ..Default::default()
            })
            .expect("insert request log");
        storage
            .insert_request_token_stat(&RequestTokenStat {
                request_log_id,
                key_id: Some("gk-time".to_string()),
                account_id: Some("acc-time".to_string()),
                model: Some("gpt-5".to_string()),
                total_tokens: Some(10),
                estimated_cost_usd: Some(0.01),
                created_at,
                ..Default::default()
            })
            .expect("insert token stat");
    }

    let page = storage
        .list_request_logs_paginated(None, None, Some(1_500), Some(3_000), 0, 10)
        .expect("list paginated logs");
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].trace_id.as_deref(), Some("trc-time-1"));

    let total = storage
        .count_request_logs(None, None, Some(1_500), Some(3_000))
        .expect("count logs");
    assert_eq!(total, 1);

    let summary = storage
        .summarize_request_logs_filtered(None, None, Some(900), Some(2_000))
        .expect("summarize time range");
    assert_eq!(summary.count, 2);
    assert_eq!(summary.total_tokens, 20);
}
