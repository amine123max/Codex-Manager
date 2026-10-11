CREATE TABLE IF NOT EXISTS account_quota_limits (
  account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  weekly_limit_tokens INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_account_quota_limits_updated_at
  ON account_quota_limits(updated_at DESC);
