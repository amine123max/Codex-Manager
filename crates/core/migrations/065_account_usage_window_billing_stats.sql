ALTER TABLE account_usage_billing_stats ADD COLUMN primary_window_resets_at INTEGER;
ALTER TABLE account_usage_billing_stats ADD COLUMN primary_window_request_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE account_usage_billing_stats ADD COLUMN primary_window_total_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE account_usage_billing_stats ADD COLUMN primary_window_estimated_cost_usd REAL NOT NULL DEFAULT 0.0;
ALTER TABLE account_usage_billing_stats ADD COLUMN secondary_window_resets_at INTEGER;
ALTER TABLE account_usage_billing_stats ADD COLUMN secondary_window_request_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE account_usage_billing_stats ADD COLUMN secondary_window_total_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE account_usage_billing_stats ADD COLUMN secondary_window_estimated_cost_usd REAL NOT NULL DEFAULT 0.0;
