use rusqlite::{Result, Row};

use super::{now_ts, AccountAgentIdentity, Storage};

impl Storage {
    pub fn upsert_account_agent_identity(&self, identity: &AccountAgentIdentity) -> Result<()> {
        let existing_created_at = self
            .find_account_agent_identity(&identity.account_id)?
            .map(|value| value.created_at)
            .unwrap_or(identity.created_at);
        self.conn.execute(
            "INSERT INTO account_agent_identities (
                account_id,
                agent_runtime_id,
                agent_private_key,
                task_id,
                chatgpt_user_id,
                chatgpt_account_is_fedramp,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(account_id) DO UPDATE SET
                agent_runtime_id = excluded.agent_runtime_id,
                agent_private_key = excluded.agent_private_key,
                task_id = excluded.task_id,
                chatgpt_user_id = excluded.chatgpt_user_id,
                chatgpt_account_is_fedramp = excluded.chatgpt_account_is_fedramp,
                updated_at = excluded.updated_at",
            (
                &identity.account_id,
                &identity.agent_runtime_id,
                &identity.agent_private_key,
                normalize_optional_text(identity.task_id.as_deref()),
                &identity.chatgpt_user_id,
                if identity.chatgpt_account_is_fedramp {
                    1
                } else {
                    0
                },
                existing_created_at,
                identity.updated_at,
            ),
        )?;
        Ok(())
    }

    pub fn find_account_agent_identity(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountAgentIdentity>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, agent_runtime_id, agent_private_key, task_id,
                    chatgpt_user_id, chatgpt_account_is_fedramp, created_at, updated_at
             FROM account_agent_identities
             WHERE account_id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query([account_id])?;
        rows.next()?.map(map_account_agent_identity_row).transpose()
    }

    pub fn list_account_agent_identities(&self) -> Result<Vec<AccountAgentIdentity>> {
        let mut stmt = self.conn.prepare(
            "SELECT account_id, agent_runtime_id, agent_private_key, task_id,
                    chatgpt_user_id, chatgpt_account_is_fedramp, created_at, updated_at
             FROM account_agent_identities
             ORDER BY updated_at DESC, account_id ASC",
        )?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(map_account_agent_identity_row(row)?);
        }
        Ok(out)
    }

    pub fn update_account_agent_identity_task(
        &self,
        account_id: &str,
        task_id: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE account_agent_identities
             SET task_id = ?1, updated_at = ?2
             WHERE account_id = ?3",
            (task_id.trim(), now_ts(), account_id),
        )?;
        Ok(())
    }

    pub fn delete_account_agent_identity(&self, account_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM account_agent_identities WHERE account_id = ?1",
            [account_id],
        )?;
        Ok(())
    }

    pub(super) fn ensure_account_agent_identities_table(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS account_agent_identities (
                account_id TEXT PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
                agent_runtime_id TEXT NOT NULL,
                agent_private_key TEXT NOT NULL,
                task_id TEXT,
                chatgpt_user_id TEXT NOT NULL,
                chatgpt_account_is_fedramp INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_account_agent_identities_runtime
                ON account_agent_identities(agent_runtime_id);",
        )?;
        Ok(())
    }
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn map_account_agent_identity_row(row: &Row<'_>) -> Result<AccountAgentIdentity> {
    Ok(AccountAgentIdentity {
        account_id: row.get(0)?,
        agent_runtime_id: row.get(1)?,
        agent_private_key: row.get(2)?,
        task_id: row.get(3)?,
        chatgpt_user_id: row.get(4)?,
        chatgpt_account_is_fedramp: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}
