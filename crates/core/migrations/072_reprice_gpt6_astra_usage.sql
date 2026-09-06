BEGIN IMMEDIATE;

DROP TABLE IF EXISTS temp.cm_gpt6_request_repriced;
DROP TABLE IF EXISTS temp.cm_gpt6_rollup_repriced;
DROP TABLE IF EXISTS temp.cm_gpt6_account_cost_deltas;

CREATE TEMP TABLE cm_gpt6_request_repriced AS
WITH source AS (
    SELECT
        t.id,
        NULLIF(TRIM(t.account_id), '') AS account_id,
        REPLACE(LOWER(TRIM(IFNULL(t.model, ''))), '_', '-') AS normalized_model,
        MAX(IFNULL(t.input_tokens, 0), 0) AS input_tokens,
        MIN(MAX(IFNULL(t.cached_input_tokens, 0), 0), MAX(IFNULL(t.input_tokens, 0), 0)) AS cached_input_tokens,
        MAX(IFNULL(t.output_tokens, 0), 0) AS output_tokens,
        MAX(IFNULL(t.estimated_cost_usd, 0.0), 0.0) AS old_cost_usd,
        t.created_at,
        CASE LOWER(TRIM(COALESCE(NULLIF(r.effective_service_tier, ''), NULLIF(r.service_tier, ''), '')))
            WHEN 'priority' THEN 2.0
            WHEN 'fast' THEN 2.0
            WHEN 'ultrafast' THEN 2.0
            WHEN 'flex' THEN 0.5
            ELSE 1.0
        END AS tier_multiplier
    FROM request_token_stats t
    LEFT JOIN request_logs r ON r.id = t.request_log_id
), matched AS (
    SELECT *
    FROM source
    WHERE normalized_model IN ('gpt-6', 'gpt-6-astra')
       OR normalized_model LIKE 'gpt-6-astra-%'
       OR normalized_model LIKE '%/gpt-6'
       OR normalized_model LIKE '%/gpt-6-astra'
       OR normalized_model LIKE '%/gpt-6-astra-%'
)
SELECT
    *,
    (((input_tokens - cached_input_tokens)
        * CASE WHEN input_tokens > 272000 THEN 20.0 ELSE 10.0 END)
      + (cached_input_tokens
        * CASE WHEN input_tokens > 272000 THEN 2.0 ELSE 1.0 END)
      + (output_tokens
        * CASE WHEN input_tokens > 272000 THEN 75.0 ELSE 50.0 END))
      * tier_multiplier / 1000000.0 AS new_cost_usd
FROM matched;

CREATE TEMP TABLE cm_gpt6_rollup_repriced AS
WITH source AS (
    SELECT
        key_id,
        NULLIF(TRIM(account_id), '') AS account_id,
        model,
        REPLACE(LOWER(TRIM(IFNULL(model, ''))), '_', '-') AS normalized_model,
        MAX(input_tokens, 0) AS input_tokens,
        MIN(MAX(cached_input_tokens, 0), MAX(input_tokens, 0)) AS cached_input_tokens,
        MAX(output_tokens, 0) AS output_tokens,
        MAX(IFNULL(estimated_cost_usd, 0.0), 0.0) AS old_cost_usd
    FROM request_token_stat_rollups
), matched AS (
    SELECT *
    FROM source
    WHERE normalized_model IN ('gpt-6', 'gpt-6-astra')
       OR normalized_model LIKE 'gpt-6-astra-%'
       OR normalized_model LIKE '%/gpt-6'
       OR normalized_model LIKE '%/gpt-6-astra'
       OR normalized_model LIKE '%/gpt-6-astra-%'
)
SELECT
    *,
    (((input_tokens - cached_input_tokens) * 10.0)
      + (cached_input_tokens * 1.0)
      + (output_tokens * 50.0)) / 1000000.0 AS new_cost_usd
FROM matched;

CREATE TEMP TABLE cm_gpt6_account_cost_deltas AS
SELECT
    account_id,
    SUM(cost_delta) AS cost_delta,
    SUM(primary_cost_delta) AS primary_cost_delta,
    SUM(secondary_cost_delta) AS secondary_cost_delta
FROM (
    SELECT
        repriced.account_id,
        repriced.new_cost_usd - repriced.old_cost_usd AS cost_delta,
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
                THEN repriced.new_cost_usd - repriced.old_cost_usd
            ELSE 0.0
        END AS secondary_cost_delta
    FROM cm_gpt6_request_repriced repriced
    JOIN account_usage_billing_stats account ON account.account_id = repriced.account_id
    WHERE repriced.account_id IS NOT NULL

    UNION ALL

    SELECT
        repriced.account_id,
        repriced.new_cost_usd - repriced.old_cost_usd,
        CASE WHEN account.primary_window_request_count = account.request_count
            THEN repriced.new_cost_usd - repriced.old_cost_usd ELSE 0.0 END,
        CASE WHEN account.secondary_window_request_count = account.request_count
            THEN repriced.new_cost_usd - repriced.old_cost_usd ELSE 0.0 END
    FROM cm_gpt6_rollup_repriced repriced
    JOIN account_usage_billing_stats account ON account.account_id = repriced.account_id
    WHERE repriced.account_id IS NOT NULL
) deltas
GROUP BY account_id;

UPDATE request_token_stats
SET estimated_cost_usd = (
    SELECT new_cost_usd
    FROM cm_gpt6_request_repriced
    WHERE cm_gpt6_request_repriced.id = request_token_stats.id
)
WHERE id IN (SELECT id FROM cm_gpt6_request_repriced);

UPDATE request_token_stat_rollups
SET estimated_cost_usd = (
    SELECT new_cost_usd
    FROM cm_gpt6_rollup_repriced repriced
    WHERE repriced.key_id = request_token_stat_rollups.key_id
      AND COALESCE(repriced.account_id, '') = request_token_stat_rollups.account_id
      AND repriced.model = request_token_stat_rollups.model
)
WHERE EXISTS (
    SELECT 1
    FROM cm_gpt6_rollup_repriced repriced
    WHERE repriced.key_id = request_token_stat_rollups.key_id
      AND COALESCE(repriced.account_id, '') = request_token_stat_rollups.account_id
      AND repriced.model = request_token_stat_rollups.model
);

UPDATE account_usage_billing_stats
SET
    estimated_cost_usd = MAX(estimated_cost_usd + COALESCE((
        SELECT cost_delta FROM cm_gpt6_account_cost_deltas
        WHERE cm_gpt6_account_cost_deltas.account_id = account_usage_billing_stats.account_id
    ), 0.0), 0.0),
    primary_window_estimated_cost_usd = MAX(primary_window_estimated_cost_usd + COALESCE((
        SELECT primary_cost_delta FROM cm_gpt6_account_cost_deltas
        WHERE cm_gpt6_account_cost_deltas.account_id = account_usage_billing_stats.account_id
    ), 0.0), 0.0),
    secondary_window_estimated_cost_usd = MAX(secondary_window_estimated_cost_usd + COALESCE((
        SELECT secondary_cost_delta FROM cm_gpt6_account_cost_deltas
        WHERE cm_gpt6_account_cost_deltas.account_id = account_usage_billing_stats.account_id
    ), 0.0), 0.0)
WHERE account_id IN (SELECT account_id FROM cm_gpt6_account_cost_deltas);

DROP TABLE temp.cm_gpt6_account_cost_deltas;
DROP TABLE temp.cm_gpt6_rollup_repriced;
DROP TABLE temp.cm_gpt6_request_repriced;

COMMIT;
