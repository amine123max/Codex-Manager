CREATE TABLE IF NOT EXISTS account_usage_billing_stats (
    account_id TEXT PRIMARY KEY,
    request_count INTEGER NOT NULL DEFAULT 0,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    cached_input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    reasoning_output_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL DEFAULT 0.0,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_account_usage_billing_stats_updated_at
    ON account_usage_billing_stats(updated_at DESC);

INSERT INTO account_usage_billing_stats (
    account_id, request_count, input_tokens, cached_input_tokens,
    output_tokens, reasoning_output_tokens, total_tokens,
    estimated_cost_usd, updated_at
)
SELECT
    account_id,
    SUM(request_count),
    SUM(input_tokens),
    SUM(cached_input_tokens),
    SUM(output_tokens),
    SUM(reasoning_output_tokens),
    SUM(total_tokens),
    SUM(estimated_cost_usd),
    CAST(strftime('%s', 'now') AS INTEGER)
FROM (
    SELECT
        account_id,
        source_rows AS request_count,
        input_tokens,
        cached_input_tokens,
        output_tokens,
        reasoning_output_tokens,
        total_tokens,
        estimated_cost_usd
    FROM request_token_stat_rollups
    WHERE TRIM(account_id) <> ''
    UNION ALL
    SELECT
        account_id,
        1,
        CASE WHEN input_tokens > 0 THEN input_tokens ELSE 0 END,
        CASE WHEN cached_input_tokens > 0 THEN cached_input_tokens ELSE 0 END,
        CASE WHEN output_tokens > 0 THEN output_tokens ELSE 0 END,
        CASE WHEN reasoning_output_tokens > 0 THEN reasoning_output_tokens ELSE 0 END,
        CASE
            WHEN total_tokens IS NOT NULL AND total_tokens > 0 THEN total_tokens
            WHEN IFNULL(input_tokens, 0) + IFNULL(output_tokens, 0) > 0
                THEN IFNULL(input_tokens, 0) + IFNULL(output_tokens, 0)
            ELSE 0
        END,
        CASE WHEN estimated_cost_usd > 0 THEN estimated_cost_usd ELSE 0.0 END
    FROM request_token_stats
    WHERE TRIM(IFNULL(account_id, '')) <> ''
) combined
GROUP BY account_id
ON CONFLICT(account_id) DO NOTHING;
