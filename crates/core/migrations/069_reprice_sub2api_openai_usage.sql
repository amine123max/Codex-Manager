BEGIN IMMEDIATE;

DROP TABLE IF EXISTS temp.cm_request_usage_base;
DROP TABLE IF EXISTS temp.cm_request_usage_repriced;
DROP TABLE IF EXISTS temp.cm_rollup_usage_repriced;
DROP TABLE IF EXISTS temp.cm_account_usage_deltas;

CREATE TEMP TABLE cm_request_usage_base AS
SELECT
    id,
    NULLIF(TRIM(account_id), '') AS account_id,
    LOWER(TRIM(IFNULL(model, ''))) AS normalized_model,
    MAX(IFNULL(input_tokens, 0), 0) AS input_tokens,
    MIN(MAX(IFNULL(cached_input_tokens, 0), 0), MAX(IFNULL(input_tokens, 0), 0)) AS cached_input_tokens,
    MAX(IFNULL(output_tokens, 0), 0) AS output_tokens,
    CASE
        WHEN total_tokens IS NULL THEN MAX(IFNULL(input_tokens, 0), 0) + MAX(IFNULL(output_tokens, 0), 0)
        ELSE MAX(total_tokens, 0)
    END AS new_total_tokens,
    CASE
        WHEN total_tokens IS NULL THEN
            MAX(IFNULL(input_tokens, 0), 0)
            - MIN(MAX(IFNULL(cached_input_tokens, 0), 0), MAX(IFNULL(input_tokens, 0), 0))
            + MAX(IFNULL(output_tokens, 0), 0)
        ELSE MAX(total_tokens, 0)
    END AS old_total_tokens,
    MAX(IFNULL(estimated_cost_usd, 0.0), 0.0) AS old_cost_usd,
    created_at
FROM request_token_stats;

CREATE TEMP TABLE cm_request_usage_repriced AS
SELECT
    *,
    CASE
        WHEN normalized_model LIKE 'gpt-5.6-terra%' THEN
            ((input_tokens - cached_input_tokens) * CASE WHEN input_tokens > 272000 THEN 5.0 ELSE 2.5 END
             + cached_input_tokens * CASE WHEN input_tokens > 272000 THEN 0.5 ELSE 0.25 END
             + output_tokens * CASE WHEN input_tokens > 272000 THEN 22.5 ELSE 15.0 END) / 1000000.0
        WHEN normalized_model LIKE 'gpt-5.6-luna%' THEN
            ((input_tokens - cached_input_tokens) * CASE WHEN input_tokens > 272000 THEN 2.0 ELSE 1.0 END
             + cached_input_tokens * CASE WHEN input_tokens > 272000 THEN 0.2 ELSE 0.1 END
             + output_tokens * CASE WHEN input_tokens > 272000 THEN 9.0 ELSE 6.0 END) / 1000000.0
        WHEN normalized_model LIKE 'gpt-5.6%' THEN
            ((input_tokens - cached_input_tokens) * CASE WHEN input_tokens > 272000 THEN 10.0 ELSE 5.0 END
             + cached_input_tokens * CASE WHEN input_tokens > 272000 THEN 1.0 ELSE 0.5 END
             + output_tokens * CASE WHEN input_tokens > 272000 THEN 45.0 ELSE 30.0 END) / 1000000.0
        WHEN normalized_model LIKE 'gpt-5.5%' THEN
            ((input_tokens - cached_input_tokens) * CASE WHEN input_tokens > 272000 THEN 5.0 ELSE 2.5 END
             + cached_input_tokens * CASE WHEN input_tokens > 272000 THEN 0.5 ELSE 0.25 END
             + output_tokens * CASE WHEN input_tokens > 272000 THEN 22.5 ELSE 15.0 END) / 1000000.0
        WHEN normalized_model LIKE 'gpt-5.3-codex%' THEN
            ((input_tokens - cached_input_tokens) * 1.5
             + cached_input_tokens * 0.15
             + output_tokens * 12.0) / 1000000.0
        ELSE old_cost_usd
    END AS new_cost_usd
FROM cm_request_usage_base;

CREATE TEMP TABLE cm_rollup_usage_repriced AS
SELECT
    NULLIF(TRIM(account_id), '') AS account_id,
    source_rows,
    total_tokens,
    MAX(IFNULL(estimated_cost_usd, 0.0), 0.0) AS old_cost_usd,
    CASE
        WHEN LOWER(TRIM(IFNULL(model, ''))) LIKE 'gpt-5.6-terra%' THEN
            ((MAX(input_tokens - cached_input_tokens, 0) * 2.5)
             + (MIN(MAX(cached_input_tokens, 0), MAX(input_tokens, 0)) * 0.25)
             + (MAX(output_tokens, 0) * 15.0)) / 1000000.0
        WHEN LOWER(TRIM(IFNULL(model, ''))) LIKE 'gpt-5.6-luna%' THEN
            ((MAX(input_tokens - cached_input_tokens, 0) * 1.0)
             + (MIN(MAX(cached_input_tokens, 0), MAX(input_tokens, 0)) * 0.1)
             + (MAX(output_tokens, 0) * 6.0)) / 1000000.0
        WHEN LOWER(TRIM(IFNULL(model, ''))) LIKE 'gpt-5.6%' THEN
            ((MAX(input_tokens - cached_input_tokens, 0) * 5.0)
             + (MIN(MAX(cached_input_tokens, 0), MAX(input_tokens, 0)) * 0.5)
             + (MAX(output_tokens, 0) * 30.0)) / 1000000.0
        WHEN LOWER(TRIM(IFNULL(model, ''))) LIKE 'gpt-5.5%' THEN
            ((MAX(input_tokens - cached_input_tokens, 0) * 2.5)
             + (MIN(MAX(cached_input_tokens, 0), MAX(input_tokens, 0)) * 0.25)
             + (MAX(output_tokens, 0) * 15.0)) / 1000000.0
        WHEN LOWER(TRIM(IFNULL(model, ''))) LIKE 'gpt-5.3-codex%' THEN
            ((MAX(input_tokens - cached_input_tokens, 0) * 1.5)
             + (MIN(MAX(cached_input_tokens, 0), MAX(input_tokens, 0)) * 0.15)
             + (MAX(output_tokens, 0) * 12.0)) / 1000000.0
        ELSE MAX(IFNULL(estimated_cost_usd, 0.0), 0.0)
    END AS new_cost_usd,
    key_id,
    model
FROM request_token_stat_rollups;

CREATE TEMP TABLE cm_account_usage_deltas AS
SELECT
    account_id,
    SUM(total_token_delta) AS total_token_delta,
    SUM(cost_delta) AS cost_delta,
    SUM(primary_token_delta) AS primary_token_delta,
    SUM(primary_cost_delta) AS primary_cost_delta,
    SUM(secondary_token_delta) AS secondary_token_delta,
    SUM(secondary_cost_delta) AS secondary_cost_delta
FROM (
    SELECT
        repriced.account_id,
        repriced.new_total_tokens - repriced.old_total_tokens AS total_token_delta,
        repriced.new_cost_usd - repriced.old_cost_usd AS cost_delta,
        CASE
            WHEN account.primary_window_request_count = account.request_count
                OR (account.primary_window_resets_at IS NOT NULL
                    AND repriced.created_at >= account.primary_window_resets_at - 18000
                    AND repriced.created_at < account.primary_window_resets_at)
                THEN repriced.new_total_tokens - repriced.old_total_tokens
            ELSE 0
        END AS primary_token_delta,
        CASE
            WHEN account.primary_window_request_count = account.request_count
                OR (account.primary_window_resets_at IS NOT NULL
                    AND repriced.created_at >= account.primary_window_resets_at - 18000
                    AND repriced.created_at < account.primary_window_resets_at)
                THEN repriced.new_cost_usd - repriced.old_cost_usd
            ELSE 0.0
        END AS primary_cost_delta,
        CASE
            WHEN account.secondary_window_request_count = account.request_count
                OR (account.secondary_window_resets_at IS NOT NULL
                    AND repriced.created_at >= account.secondary_window_resets_at - 604800
                    AND repriced.created_at < account.secondary_window_resets_at)
                THEN repriced.new_total_tokens - repriced.old_total_tokens
            ELSE 0
        END AS secondary_token_delta,
        CASE
            WHEN account.secondary_window_request_count = account.request_count
                OR (account.secondary_window_resets_at IS NOT NULL
                    AND repriced.created_at >= account.secondary_window_resets_at - 604800
                    AND repriced.created_at < account.secondary_window_resets_at)
                THEN repriced.new_cost_usd - repriced.old_cost_usd
            ELSE 0.0
        END AS secondary_cost_delta
    FROM cm_request_usage_repriced repriced
    JOIN account_usage_billing_stats account ON account.account_id = repriced.account_id
    WHERE repriced.account_id IS NOT NULL

    UNION ALL

    SELECT
        repriced.account_id,
        0,
        repriced.new_cost_usd - repriced.old_cost_usd,
        0,
        CASE WHEN account.primary_window_request_count = account.request_count
            THEN repriced.new_cost_usd - repriced.old_cost_usd ELSE 0.0 END,
        0,
        CASE WHEN account.secondary_window_request_count = account.request_count
            THEN repriced.new_cost_usd - repriced.old_cost_usd ELSE 0.0 END
    FROM cm_rollup_usage_repriced repriced
    JOIN account_usage_billing_stats account ON account.account_id = repriced.account_id
    WHERE repriced.account_id IS NOT NULL
) deltas
GROUP BY account_id;

UPDATE request_token_stats
SET
    total_tokens = (SELECT new_total_tokens FROM cm_request_usage_repriced WHERE cm_request_usage_repriced.id = request_token_stats.id),
    estimated_cost_usd = (SELECT new_cost_usd FROM cm_request_usage_repriced WHERE cm_request_usage_repriced.id = request_token_stats.id)
WHERE id IN (SELECT id FROM cm_request_usage_repriced);

UPDATE request_token_stat_rollups
SET estimated_cost_usd = (
    SELECT new_cost_usd
    FROM cm_rollup_usage_repriced repriced
    WHERE repriced.key_id = request_token_stat_rollups.key_id
      AND COALESCE(repriced.account_id, '') = request_token_stat_rollups.account_id
      AND repriced.model = request_token_stat_rollups.model
)
WHERE EXISTS (
    SELECT 1
    FROM cm_rollup_usage_repriced repriced
    WHERE repriced.key_id = request_token_stat_rollups.key_id
      AND COALESCE(repriced.account_id, '') = request_token_stat_rollups.account_id
      AND repriced.model = request_token_stat_rollups.model
);

UPDATE account_usage_billing_stats
SET
    total_tokens = MAX(total_tokens + COALESCE((
        SELECT total_token_delta FROM cm_account_usage_deltas WHERE cm_account_usage_deltas.account_id = account_usage_billing_stats.account_id
    ), 0), 0),
    estimated_cost_usd = MAX(estimated_cost_usd + COALESCE((
        SELECT cost_delta FROM cm_account_usage_deltas WHERE cm_account_usage_deltas.account_id = account_usage_billing_stats.account_id
    ), 0.0), 0.0),
    primary_window_total_tokens = MAX(primary_window_total_tokens + COALESCE((
        SELECT primary_token_delta FROM cm_account_usage_deltas WHERE cm_account_usage_deltas.account_id = account_usage_billing_stats.account_id
    ), 0), 0),
    primary_window_estimated_cost_usd = MAX(primary_window_estimated_cost_usd + COALESCE((
        SELECT primary_cost_delta FROM cm_account_usage_deltas WHERE cm_account_usage_deltas.account_id = account_usage_billing_stats.account_id
    ), 0.0), 0.0),
    secondary_window_total_tokens = MAX(secondary_window_total_tokens + COALESCE((
        SELECT secondary_token_delta FROM cm_account_usage_deltas WHERE cm_account_usage_deltas.account_id = account_usage_billing_stats.account_id
    ), 0), 0),
    secondary_window_estimated_cost_usd = MAX(secondary_window_estimated_cost_usd + COALESCE((
        SELECT secondary_cost_delta FROM cm_account_usage_deltas WHERE cm_account_usage_deltas.account_id = account_usage_billing_stats.account_id
    ), 0.0), 0.0)
WHERE account_id IN (SELECT account_id FROM cm_account_usage_deltas);

DROP TABLE temp.cm_account_usage_deltas;
DROP TABLE temp.cm_rollup_usage_repriced;
DROP TABLE temp.cm_request_usage_repriced;
DROP TABLE temp.cm_request_usage_base;

COMMIT;
