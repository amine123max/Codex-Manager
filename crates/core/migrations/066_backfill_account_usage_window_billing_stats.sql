WITH
current_time(now_ts) AS (
    SELECT CAST(strftime('%s', 'now') AS INTEGER)
),
window_stats AS (
    SELECT
        account_id,
        SUM(CASE WHEN created_at >= now_ts - 18000 THEN 1 ELSE 0 END) AS primary_request_count,
        SUM(CASE WHEN created_at >= now_ts - 18000 THEN
            CASE
                WHEN total_tokens IS NOT NULL AND total_tokens > 0 THEN total_tokens
                WHEN IFNULL(input_tokens, 0) - IFNULL(cached_input_tokens, 0) + IFNULL(output_tokens, 0) > 0
                    THEN IFNULL(input_tokens, 0) - IFNULL(cached_input_tokens, 0) + IFNULL(output_tokens, 0)
                ELSE 0
            END
        ELSE 0 END) AS primary_total_tokens,
        SUM(CASE WHEN created_at >= now_ts - 18000 AND estimated_cost_usd > 0
            THEN estimated_cost_usd ELSE 0.0 END) AS primary_estimated_cost_usd,
        COUNT(*) AS secondary_request_count,
        SUM(CASE
            WHEN total_tokens IS NOT NULL AND total_tokens > 0 THEN total_tokens
            WHEN IFNULL(input_tokens, 0) - IFNULL(cached_input_tokens, 0) + IFNULL(output_tokens, 0) > 0
                THEN IFNULL(input_tokens, 0) - IFNULL(cached_input_tokens, 0) + IFNULL(output_tokens, 0)
            ELSE 0
        END) AS secondary_total_tokens,
        SUM(CASE WHEN estimated_cost_usd > 0 THEN estimated_cost_usd ELSE 0.0 END) AS secondary_estimated_cost_usd,
        now_ts
    FROM request_token_stats, current_time
    WHERE TRIM(IFNULL(account_id, '')) <> ''
      AND created_at >= now_ts - 604800
    GROUP BY account_id
)
UPDATE account_usage_billing_stats
SET
    primary_window_resets_at = CASE
        WHEN COALESCE((SELECT primary_request_count FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id), 0) > 0
            THEN (SELECT now_ts + 18000 FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id)
        ELSE NULL
    END,
    primary_window_request_count = COALESCE((SELECT primary_request_count FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id), 0),
    primary_window_total_tokens = COALESCE((SELECT primary_total_tokens FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id), 0),
    primary_window_estimated_cost_usd = COALESCE((SELECT primary_estimated_cost_usd FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id), 0.0),
    secondary_window_resets_at = (SELECT now_ts + 604800 FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id),
    secondary_window_request_count = COALESCE((SELECT secondary_request_count FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id), 0),
    secondary_window_total_tokens = COALESCE((SELECT secondary_total_tokens FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id), 0),
    secondary_window_estimated_cost_usd = COALESCE((SELECT secondary_estimated_cost_usd FROM window_stats WHERE window_stats.account_id = account_usage_billing_stats.account_id), 0.0)
WHERE account_id IN (SELECT account_id FROM window_stats);
