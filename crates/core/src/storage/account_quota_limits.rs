use std::collections::HashMap;

use rusqlite::{OptionalExtension, Result};

use super::{now_ts, AccountUsageWindowRollup, Storage};

impl Storage {
    /// 设置或清除账号周额度上限；`weekly_limit_tokens` 为 None 或 <= 0 时删除该账号的上限。
    pub fn upsert_account_quota_limit(
        &self,
        account_id: &str,
        weekly_limit_tokens: Option<i64>,
    ) -> Result<()> {
        let normalized = weekly_limit_tokens.filter(|value| *value > 0);
        let Some(limit) = normalized else {
            self.conn.execute(
                "DELETE FROM account_quota_limits WHERE account_id = ?1",
                [account_id],
            )?;
            return Ok(());
        };

        let now = now_ts();
        self.conn.execute(
            "INSERT INTO account_quota_limits (
                account_id, weekly_limit_tokens, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(account_id) DO UPDATE SET
                weekly_limit_tokens = excluded.weekly_limit_tokens,
                updated_at = excluded.updated_at",
            (account_id, limit, now),
        )?;
        Ok(())
    }

    pub fn find_account_quota_limit(&self, account_id: &str) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT weekly_limit_tokens
                 FROM account_quota_limits
                 WHERE account_id = ?1
                 LIMIT 1",
                [account_id],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn list_account_quota_limits(&self) -> Result<HashMap<String, i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, weekly_limit_tokens
             FROM account_quota_limits
             WHERE weekly_limit_tokens > 0",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = HashMap::new();
        while let Some(row) = rows.next()? {
            out.insert(row.get(0)?, row.get(1)?);
        }
        Ok(out)
    }

    /// 读取单个账号本地计费统计里的周窗口（secondary window）用量。
    pub fn find_account_usage_secondary_window(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountUsageWindowRollup>> {
        self.conn
            .query_row(
                "SELECT secondary_window_resets_at, secondary_window_request_count,
                        secondary_window_total_tokens, secondary_window_estimated_cost_usd
                 FROM account_usage_billing_stats
                 WHERE account_id = ?1
                 LIMIT 1",
                [account_id],
                |row| {
                    Ok(AccountUsageWindowRollup {
                        resets_at: row.get(0)?,
                        request_count: row.get::<_, i64>(1)?.max(0),
                        total_tokens: row.get::<_, i64>(2)?.max(0),
                        estimated_cost_usd: row.get::<_, f64>(3)?.max(0.0),
                    })
                },
            )
            .optional()
    }

    pub(super) fn ensure_account_quota_limits_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS account_quota_limits (
                account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
                weekly_limit_tokens INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_account_quota_limits_updated_at
             ON account_quota_limits(updated_at DESC)",
            [],
        )?;
        Ok(())
    }
}
