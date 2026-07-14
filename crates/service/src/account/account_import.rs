use codexmanager_core::auth::{
    extract_chatgpt_account_id, extract_chatgpt_user_id, extract_workspace_id,
    parse_id_token_claims, IdTokenClaims, DEFAULT_ISSUER,
};
use codexmanager_core::storage::{now_ts, Account, Storage, Token};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::account_identity::{
    build_account_storage_id, build_fallback_subject_key, clean_value,
    pick_existing_account_id_by_identity,
};
use crate::storage_helpers::{account_key, open_storage};

const MAX_ERROR_ITEMS: usize = 50;
const DEFAULT_IMPORT_BATCH_SIZE: usize = 200;
const IMPORT_BATCH_SIZE_ENV: &str = "CODEXMANAGER_ACCOUNT_IMPORT_BATCH_SIZE";
const ACCOUNT_SORT_STEP: i64 = 5;
const IMPORT_TOKEN_SUBJECT_PREFIX: &str = "import-token-";
const IMPORT_ACCESS_TOKEN_SUBJECT_PREFIX: &str = "import-access-";

#[derive(Debug, Serialize)]
pub(crate) struct AccountImportResult {
    total: usize,
    created: usize,
    updated: usize,
    failed: usize,
    errors: Vec<AccountImportError>,
}

#[derive(Debug, Serialize)]
struct AccountImportError {
    index: usize,
    message: String,
}

#[derive(Debug)]
struct ImportTokenPayload {
    access_token: String,
    id_token: String,
    refresh_token: String,
    account_id_hint: Option<String>,
    chatgpt_account_id_hint: Option<String>,
    user_id_hint: Option<String>,
}

#[derive(Debug)]
struct ImportedAccount {
    account_id: String,
    created: bool,
}

#[derive(Debug, Default)]
struct ImportAccountMeta {
    label: Option<String>,
    issuer: Option<String>,
    group_name: Option<String>,
    note: Option<String>,
    tags: Option<String>,
    workspace_id: Option<String>,
    chatgpt_account_id: Option<String>,
}

#[derive(Default)]
struct ExistingAccountIndex {
    by_id: HashMap<String, Account>,
    by_subject_storage_id: HashMap<String, String>,
    by_subject_key: HashMap<String, String>,
    ambiguous_subject_keys: HashSet<String>,
    next_sort: i64,
}

impl ExistingAccountIndex {
    /// 函数 `build`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - storage: 参数 storage
    ///
    /// # 返回
    /// 返回函数执行结果
    fn build(storage: &Storage) -> Result<Self, String> {
        let accounts = storage.list_accounts().map_err(|e| e.to_string())?;
        let mut idx = ExistingAccountIndex::default();
        for account in accounts {
            idx.next_sort = idx
                .next_sort
                .max(account.sort.saturating_add(ACCOUNT_SORT_STEP));
            idx.by_id.insert(account.id.clone(), account);
        }
        for token in storage.list_tokens().map_err(|e| e.to_string())? {
            if let Some(account) = idx.by_id.get(&token.account_id).cloned() {
                idx.index_token_subject(&account, &token);
            }
        }
        Ok(idx)
    }

    /// 函数 `find_existing_account_id`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - chatgpt_account_id: 参数 chatgpt_account_id
    /// - workspace_id: 参数 workspace_id
    /// - fallback_subject_key: 参数 fallback_subject_key
    /// - account_id_hint: 参数 account_id_hint
    ///
    /// # 返回
    /// 返回函数执行结果
    fn find_existing_account_id(
        &self,
        chatgpt_account_id: Option<&str>,
        workspace_id: Option<&str>,
        fallback_subject_key: Option<&str>,
        account_id_hint: Option<&str>,
    ) -> Option<String> {
        if let Some(found) =
            self.find_by_subject_identity(chatgpt_account_id, workspace_id, fallback_subject_key)
        {
            return Some(found);
        }
        if fallback_subject_key.is_some() {
            return None;
        }
        pick_existing_account_id_by_identity(
            self.by_id.values(),
            chatgpt_account_id,
            workspace_id,
            None,
            account_id_hint,
        )
    }

    /// 函数 `find_by_subject_identity`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - chatgpt_account_id: 参数 chatgpt_account_id
    /// - workspace_id: 参数 workspace_id
    /// - fallback_subject_key: 参数 fallback_subject_key
    ///
    /// # 返回
    /// 返回函数执行结果
    fn find_by_subject_identity(
        &self,
        chatgpt_account_id: Option<&str>,
        workspace_id: Option<&str>,
        fallback_subject_key: Option<&str>,
    ) -> Option<String> {
        let subject_key = fallback_subject_key
            .map(str::trim)
            .filter(|v| !v.is_empty())?;
        let scoped_id =
            build_account_storage_id(subject_key, chatgpt_account_id, workspace_id, None);
        if let Some(account) = self.by_id.get(&scoped_id) {
            if account_scope_matches(account, chatgpt_account_id, workspace_id) {
                return Some(scoped_id);
            }
            return None;
        }
        if let Some(account_id) = self.by_subject_storage_id.get(&scoped_id) {
            return Some(account_id.clone());
        }
        if chatgpt_account_id.is_some() || workspace_id.is_some() {
            return None;
        }
        if self.by_id.contains_key(subject_key) {
            return Some(subject_key.to_string());
        }
        if let Some(account_id) = self.by_subject_key.get(subject_key) {
            return Some(account_id.clone());
        }
        None
    }

    /// 函数 `upsert_index`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - account: 参数 account
    ///
    /// # 返回
    /// 无
    fn upsert_index(&mut self, account: &Account) {
        self.by_id.insert(account.id.clone(), account.clone());
    }

    /// 函数 `index_token_subject`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - account: 参数 account
    /// - token: 参数 token
    ///
    /// # 返回
    /// 无
    fn index_token_subject(&mut self, account: &Account, token: &Token) {
        let Some(subject_account_id) = extract_import_subject_account_id(
            None,
            None,
            &token.id_token,
            &token.access_token,
            &token.refresh_token,
        ) else {
            return;
        };
        let Some(subject_key) =
            build_fallback_subject_key(Some(subject_account_id.as_str()), None::<&str>)
        else {
            return;
        };
        let scoped_id = build_account_storage_id(
            subject_key.as_str(),
            account.chatgpt_account_id.as_deref(),
            account.workspace_id.as_deref(),
            None,
        );
        self.by_subject_storage_id
            .insert(scoped_id, account.id.clone());
        self.record_subject_key(subject_key, account.id.clone());
    }

    /// 函数 `record_subject_key`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - subject_key: 参数 subject_key
    /// - account_id: 参数 account_id
    ///
    /// # 返回
    /// 无
    fn record_subject_key(&mut self, subject_key: String, account_id: String) {
        if self.ambiguous_subject_keys.contains(&subject_key) {
            return;
        }
        match self.by_subject_key.get(&subject_key) {
            Some(existing) if existing == &account_id => {}
            Some(_) => {
                self.by_subject_key.remove(&subject_key);
                self.ambiguous_subject_keys.insert(subject_key);
            }
            None => {
                self.by_subject_key.insert(subject_key, account_id);
            }
        }
    }
}

fn account_scope_matches(
    account: &Account,
    chatgpt_account_id: Option<&str>,
    workspace_id: Option<&str>,
) -> bool {
    if let Some(chatgpt) = chatgpt_account_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if account.chatgpt_account_id.as_deref().map(str::trim) != Some(chatgpt) {
            return false;
        }
    }
    if let Some(workspace) = workspace_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if account.workspace_id.as_deref().map(str::trim) != Some(workspace) {
            return false;
        }
    }
    true
}

fn resolve_scoped_account_id_collision(
    existing: &HashMap<String, Account>,
    account_id: String,
    chatgpt_account_id: Option<&str>,
    workspace_id: Option<&str>,
    payload: &ImportTokenPayload,
) -> String {
    let Some(account) = existing.get(&account_id) else {
        return account_id;
    };
    if account_scope_matches(account, chatgpt_account_id, workspace_id) {
        return account_id;
    }

    let fingerprint_source = payload
        .refresh_token
        .trim()
        .is_empty()
        .then_some(payload.access_token.as_str())
        .unwrap_or(payload.refresh_token.as_str());
    let fingerprint = token_fingerprint(fingerprint_source);
    let mut candidate = account_key(&account_id, Some(&format!("conflict_fp_{fingerprint}")));
    let mut suffix = 2usize;
    while existing.contains_key(&candidate) {
        candidate = account_key(
            &account_id,
            Some(&format!("conflict_fp_{fingerprint}_{suffix}")),
        );
        suffix = suffix.saturating_add(1);
    }
    candidate
}

/// 函数 `import_account_auth_json`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - crate: 参数 crate
///
/// # 返回
/// 返回函数执行结果
pub(crate) fn import_account_auth_json(
    contents: Vec<String>,
) -> Result<AccountImportResult, String> {
    let storage = open_storage().ok_or_else(|| "storage unavailable".to_string())?;
    let mut index = ExistingAccountIndex::build(&storage)?;
    let mut result = AccountImportResult {
        total: 0,
        created: 0,
        updated: 0,
        failed: 0,
        errors: Vec::new(),
    };
    let mut progress = AccountImportProgress::new();
    let batch_size = import_batch_size();

    for content in contents {
        match parse_items_from_content(&content) {
            Ok(items) => {
                import_items_in_batches(
                    &storage,
                    &mut index,
                    &mut result,
                    &mut progress,
                    items,
                    batch_size,
                );
            }
            Err(err) => {
                record_import_error(&mut result, &mut progress, err);
            }
        }
    }

    progress.finish();
    Ok(result)
}

/// 函数 `import_batch_size`
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
fn import_batch_size() -> usize {
    std::env::var(IMPORT_BATCH_SIZE_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_IMPORT_BATCH_SIZE)
}

/// 函数 `import_items_in_batches`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - index: 参数 index
/// - result: 参数 result
/// - progress: 参数 progress
/// - items: 参数 items
/// - batch_size: 参数 batch_size
///
/// # 返回
/// 无
fn import_items_in_batches(
    storage: &Storage,
    index: &mut ExistingAccountIndex,
    result: &mut AccountImportResult,
    progress: &mut AccountImportProgress,
    items: Vec<Value>,
    batch_size: usize,
) {
    if items.is_empty() {
        return;
    }
    let total_batches = items.len().div_ceil(batch_size);
    for (batch_index, batch) in items.chunks(batch_size).enumerate() {
        progress.begin_batch(batch_index + 1, total_batches, batch.len());
        for item in batch {
            result.total += 1;
            let current_index = result.total;
            match import_single_item_with_account_id(storage, index, item, current_index) {
                Ok(imported) => {
                    if imported.created {
                        result.created += 1;
                    } else {
                        result.updated += 1;
                    }
                    progress.on_item_success(imported.created);
                    let _ = crate::usage_refresh::enqueue_usage_refresh_after_account_add(
                        &imported.account_id,
                    );
                }
                Err(err) => {
                    result.failed += 1;
                    progress.on_item_failure();
                    if result.errors.len() < MAX_ERROR_ITEMS {
                        result.errors.push(AccountImportError {
                            index: current_index,
                            message: err,
                        });
                    }
                }
            }
        }
        progress.finish_batch();
    }
}

/// 函数 `record_import_error`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - result: 参数 result
/// - progress: 参数 progress
/// - message: 参数 message
///
/// # 返回
/// 无
fn record_import_error(
    result: &mut AccountImportResult,
    progress: &mut AccountImportProgress,
    message: String,
) {
    result.total += 1;
    result.failed += 1;
    progress.on_item_failure();
    if result.errors.len() < MAX_ERROR_ITEMS {
        result.errors.push(AccountImportError {
            index: result.total,
            message,
        });
    }
}

#[derive(Debug)]
struct AccountImportProgress {
    started_at: Instant,
    processed: usize,
    created: usize,
    updated: usize,
    failed: usize,
    active_batch: Option<AccountImportBatchProgress>,
}

#[derive(Debug)]
struct AccountImportBatchProgress {
    index: usize,
    total: usize,
    size: usize,
    processed: usize,
    created: usize,
    updated: usize,
    failed: usize,
}

impl AccountImportProgress {
    /// 函数 `new`
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
    fn new() -> Self {
        Self {
            started_at: Instant::now(),
            processed: 0,
            created: 0,
            updated: 0,
            failed: 0,
            active_batch: None,
        }
    }

    /// 函数 `begin_batch`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - index: 参数 index
    /// - total: 参数 total
    /// - size: 参数 size
    ///
    /// # 返回
    /// 无
    fn begin_batch(&mut self, index: usize, total: usize, size: usize) {
        self.active_batch = Some(AccountImportBatchProgress {
            index,
            total,
            size,
            processed: 0,
            created: 0,
            updated: 0,
            failed: 0,
        });
    }

    /// 函数 `on_item_success`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    /// - created: 参数 created
    ///
    /// # 返回
    /// 无
    fn on_item_success(&mut self, created: bool) {
        self.processed += 1;
        if created {
            self.created += 1;
        } else {
            self.updated += 1;
        }
        if let Some(batch) = self.active_batch.as_mut() {
            batch.processed += 1;
            if created {
                batch.created += 1;
            } else {
                batch.updated += 1;
            }
        }
    }

    /// 函数 `on_item_failure`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 无
    fn on_item_failure(&mut self) {
        self.processed += 1;
        self.failed += 1;
        if let Some(batch) = self.active_batch.as_mut() {
            batch.processed += 1;
            batch.failed += 1;
        }
    }

    /// 函数 `finish_batch`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 无
    fn finish_batch(&mut self) {
        if let Some(batch) = self.active_batch.take() {
            log::info!(
                "account import batch finished: {}/{} size={} processed={} created={} updated={} failed={} total_processed={} elapsed_ms={}",
                batch.index,
                batch.total,
                batch.size,
                batch.processed,
                batch.created,
                batch.updated,
                batch.failed,
                self.processed,
                self.started_at.elapsed().as_millis()
            );
        }
    }

    /// 函数 `finish`
    ///
    /// 作者: gaohongshun
    ///
    /// 时间: 2026-04-02
    ///
    /// # 参数
    /// - self: 参数 self
    ///
    /// # 返回
    /// 无
    fn finish(&self) {
        log::info!(
            "account import finished: processed={} created={} updated={} failed={} elapsed_ms={}",
            self.processed,
            self.created,
            self.updated,
            self.failed,
            self.started_at.elapsed().as_millis()
        );
    }
}

/// 函数 `parse_items_from_content`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - content: 参数 content
///
/// # 返回
/// 返回函数执行结果
fn parse_items_from_content(content: &str) -> Result<Vec<Value>, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if looks_like_json(trimmed) {
        match decode_json_stream(trimmed) {
            Ok(values) => return Ok(flatten_import_values(values)),
            Err(err) if trimmed.contains('\n') => {
                if let Ok(values) = parse_import_lines(trimmed) {
                    return Ok(values);
                }
                return Err(err);
            }
            Err(err) => return Err(err),
        }
    }

    parse_import_lines(trimmed)
}

fn parse_import_lines(content: &str) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for (line_index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if looks_like_json(line) {
            let values = decode_json_stream(line)
                .map_err(|err| format!("line {}: {err}", line_index + 1))?;
            out.extend(flatten_import_values(values));
        } else {
            out.push(Value::String(line.to_string()));
        }
    }
    Ok(out)
}

fn looks_like_json(content: &str) -> bool {
    matches!(content.as_bytes().first(), Some(b'{') | Some(b'['))
}

fn decode_json_stream(content: &str) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    let stream = serde_json::Deserializer::from_str(content).into_iter::<Value>();
    for value in stream {
        out.push(value.map_err(|err| format!("invalid JSON object stream: {err}"))?);
    }
    if out.is_empty() {
        return Err("invalid JSON object stream: empty JSON content".to_string());
    }
    Ok(out)
}

fn flatten_import_values(values: Vec<Value>) -> Vec<Value> {
    let mut out = Vec::new();
    for value in values {
        flatten_import_value(value, &mut out);
    }
    out
}

fn flatten_import_value(value: Value, out: &mut Vec<Value>) {
    if let Some(accounts) = sub2api_accounts(&value) {
        for account in accounts.iter().cloned() {
            flatten_import_value(account, out);
        }
        return;
    }

    match value {
        Value::Array(items) => {
            for item in items {
                flatten_import_value(item, out);
            }
        }
        other => out.push(other),
    }
}

fn sub2api_accounts(value: &Value) -> Option<&Vec<Value>> {
    let object = value.as_object()?;
    if let Some(data) = object.get("data") {
        if let Some(accounts) = sub2api_accounts(data) {
            return Some(accounts);
        }
    }

    let accounts = object.get("accounts")?.as_array()?;
    let data_type = object
        .get("type")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let is_sub2api_type = matches!(data_type, "sub2api-data" | "sub2api-bundle");
    let has_export_shape = object.contains_key("proxies") || object.contains_key("exported_at");
    (is_sub2api_type || has_export_shape).then_some(accounts)
}

/// 函数 `import_single_item`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - storage: 参数 storage
/// - index: 参数 index
/// - item: 参数 item
/// - sequence: 参数 sequence
///
/// # 返回
/// 返回函数执行结果
#[cfg(test)]
fn import_single_item(
    storage: &Storage,
    index: &mut ExistingAccountIndex,
    item: &Value,
    sequence: usize,
) -> Result<bool, String> {
    import_single_item_with_account_id(storage, index, item, sequence)
        .map(|imported| imported.created)
}

fn import_single_item_with_account_id(
    storage: &Storage,
    index: &mut ExistingAccountIndex,
    item: &Value,
    sequence: usize,
) -> Result<ImportedAccount, String> {
    let payload = extract_token_payload(&item)?;
    let meta = extract_account_meta(item);
    let claims = parse_id_token_claims(&payload.id_token).ok();
    let fingerprint_source = if payload.refresh_token.trim().is_empty() {
        payload.access_token.as_str()
    } else {
        payload.refresh_token.as_str()
    };
    let token_fingerprint = token_fingerprint(fingerprint_source);
    let subject_account_id = extract_import_subject_account_id(
        claims.as_ref(),
        payload.user_id_hint.as_deref(),
        &payload.id_token,
        &payload.access_token,
        &payload.refresh_token,
    );
    let chatgpt_account_id = clean_value(
        meta.chatgpt_account_id
            .clone()
            .or_else(|| payload.chatgpt_account_id_hint.clone())
            .or_else(|| {
                claims
                    .as_ref()
                    .and_then(|c| c.auth.as_ref()?.chatgpt_account_id.clone())
            })
            .or_else(|| extract_chatgpt_account_id(&payload.id_token))
            .or_else(|| extract_chatgpt_account_id(&payload.access_token)),
    );

    let workspace_id = clean_value(
        meta.workspace_id
            .clone()
            .or_else(|| claims.as_ref().and_then(|c| c.workspace_id.clone()))
            .or_else(|| extract_workspace_id(&payload.id_token))
            .or_else(|| extract_workspace_id(&payload.access_token))
            .or_else(|| payload.account_id_hint.clone())
            .or_else(|| chatgpt_account_id.clone()),
    );
    let fallback_subject_key =
        build_fallback_subject_key(subject_account_id.as_deref(), None::<&str>);
    let token_fingerprint_for_id = match subject_account_id.as_deref() {
        Some(subject)
            if subject.starts_with(IMPORT_TOKEN_SUBJECT_PREFIX)
                || subject.starts_with(IMPORT_ACCESS_TOKEN_SUBJECT_PREFIX) =>
        {
            None
        }
        _ => Some(token_fingerprint.as_str()),
    };
    let account_id = index
        .find_existing_account_id(
            chatgpt_account_id.as_deref(),
            workspace_id.as_deref(),
            fallback_subject_key.as_deref(),
            payload.account_id_hint.as_deref(),
        )
        .unwrap_or(resolve_logical_account_id(
            &payload,
            subject_account_id.as_deref(),
            chatgpt_account_id.as_deref(),
            workspace_id.as_deref(),
            token_fingerprint_for_id,
        )?);
    let account_id = resolve_scoped_account_id_collision(
        &index.by_id,
        account_id,
        chatgpt_account_id.as_deref(),
        workspace_id.as_deref(),
        &payload,
    );

    let label = meta
        .label
        .clone()
        .or_else(|| {
            claims
                .as_ref()
                .and_then(|c| c.email.clone())
                .filter(|v| !v.trim().is_empty())
        })
        .or_else(|| {
            item.get("email")
                .and_then(Value::as_str)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
        .or_else(|| {
            optional_string_paths(
                item,
                &[
                    &["user", "email"],
                    &["credentials", "email"],
                    &["account", "email"],
                ],
            )
        })
        .unwrap_or_else(|| format!("导入账号{:04}", sequence));
    let default_issuer =
        std::env::var("CODEXMANAGER_ISSUER").unwrap_or_else(|_| DEFAULT_ISSUER.to_string());
    let issuer = meta
        .issuer
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(default_issuer);
    let group_name = meta
        .group_name
        .clone()
        .filter(|value| !value.trim().is_empty());
    let note = meta.note.clone().filter(|value| !value.trim().is_empty());
    let tags = meta.tags.clone().filter(|value| !value.trim().is_empty());

    let now = now_ts();
    let (account_id, account, created) =
        if let Some(existing) = index.by_id.get(&account_id).cloned() {
            let merged_chatgpt_account_id = chatgpt_account_id
                .clone()
                .or_else(|| clean_value(existing.chatgpt_account_id.clone()));
            let merged_workspace_id = workspace_id
                .clone()
                .or_else(|| clean_value(existing.workspace_id.clone()));
            let updated = Account {
                id: existing.id.clone(),
                label: if existing.label.trim().is_empty() {
                    label
                } else {
                    existing.label.clone()
                },
                issuer: if existing.issuer.trim().is_empty() {
                    issuer
                } else {
                    existing.issuer.clone()
                },
                chatgpt_account_id: merged_chatgpt_account_id,
                workspace_id: merged_workspace_id,
                group_name: existing
                    .group_name
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .or(group_name)
                    .or_else(|| Some("IMPORT".to_string())),
                sort: existing.sort,
                status: "active".to_string(),
                created_at: existing.created_at,
                updated_at: now,
            };
            (existing.id.clone(), updated, false)
        } else {
            let next_sort = index.next_sort;
            index.next_sort = index.next_sort.saturating_add(ACCOUNT_SORT_STEP);
            let created = Account {
                id: account_id.clone(),
                label,
                issuer,
                chatgpt_account_id: chatgpt_account_id.clone(),
                workspace_id,
                group_name: group_name.or_else(|| Some("IMPORT".to_string())),
                sort: next_sort,
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
            };
            (account_id.clone(), created, true)
        };

    storage
        .insert_account(&account)
        .map_err(|e| e.to_string())?;
    let existing_metadata = storage
        .find_account_metadata(&account_id)
        .map_err(|e| e.to_string())?;
    let merged_note = note.or_else(|| {
        existing_metadata
            .as_ref()
            .and_then(|value| value.note.clone())
    });
    let merged_tags = tags.or_else(|| {
        existing_metadata
            .as_ref()
            .and_then(|value| value.tags.clone())
    });
    storage
        .upsert_account_metadata(&account_id, merged_note.as_deref(), merged_tags.as_deref())
        .map_err(|e| e.to_string())?;
    let mut refresh_token = payload.refresh_token;
    if refresh_token.trim().is_empty() && !created {
        if let Some(existing_token) = storage
            .find_token_by_account_id(&account_id)
            .map_err(|e| e.to_string())?
        {
            refresh_token = existing_token.refresh_token;
        }
    }
    let token = Token {
        account_id: account_id.clone(),
        id_token: payload.id_token,
        access_token: payload.access_token,
        refresh_token,
        api_key_access_token: None,
        last_refresh: now,
    };
    storage.insert_token(&token).map_err(|e| e.to_string())?;
    index.upsert_index(&account);
    index.index_token_subject(&account, &token);
    Ok(ImportedAccount {
        account_id,
        created,
    })
}

/// 函数 `extract_import_subject_account_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - claims: 参数 claims
/// - id_token: 参数 id_token
/// - access_token: 参数 access_token
/// - refresh_token: 参数 refresh_token
///
/// # 返回
/// 返回函数执行结果
fn extract_import_subject_account_id(
    claims: Option<&IdTokenClaims>,
    user_id_hint: Option<&str>,
    id_token: &str,
    access_token: &str,
    refresh_token: &str,
) -> Option<String> {
    clean_value(
        user_id_hint
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                claims.and_then(|c| {
                    c.auth.as_ref().and_then(|auth| {
                        auth.chatgpt_user_id
                            .clone()
                            .or_else(|| auth.user_id.clone())
                    })
                })
            })
            .or_else(|| {
                claims
                    .map(|c| c.sub.trim().to_string())
                    .filter(|v| !v.is_empty())
            })
            .or_else(|| extract_chatgpt_user_id(id_token))
            .or_else(|| extract_chatgpt_user_id(access_token))
            .or_else(|| {
                if refresh_token.trim().is_empty() {
                    None
                } else {
                    let token_fingerprint = token_fingerprint(refresh_token);
                    Some(format!("{IMPORT_TOKEN_SUBJECT_PREFIX}{token_fingerprint}"))
                }
            })
            .or_else(|| {
                if access_token.trim().is_empty() {
                    None
                } else {
                    let token_fingerprint = token_fingerprint(access_token);
                    Some(format!(
                        "{IMPORT_ACCESS_TOKEN_SUBJECT_PREFIX}{token_fingerprint}"
                    ))
                }
            }),
    )
}

/// 函数 `extract_token_payload`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - item: 参数 item
///
/// # 返回
/// 返回函数执行结果
fn extract_token_payload(item: &Value) -> Result<ImportTokenPayload, String> {
    if let Some(raw_token) = item.as_str() {
        let access_token = raw_token.trim();
        if access_token.is_empty() {
            return Err("empty field: access_token/accessToken/token".to_string());
        }
        return Ok(ImportTokenPayload {
            access_token: access_token.to_string(),
            id_token: String::new(),
            refresh_token: String::new(),
            account_id_hint: None,
            chatgpt_account_id_hint: None,
            user_id_hint: None,
        });
    }

    let access_token = required_string_paths(
        item,
        &[
            &["tokens", "access_token"],
            &["tokens", "accessToken"],
            &["credentials", "access_token"],
            &["credentials", "accessToken"],
            &["auth", "access_token"],
            &["auth", "accessToken"],
            &["access_token"],
            &["accessToken"],
            &["token"],
        ],
        "access_token/accessToken/token",
    )?;
    let id_token = optional_string_paths(
        item,
        &[
            &["tokens", "id_token"],
            &["tokens", "idToken"],
            &["credentials", "id_token"],
            &["credentials", "idToken"],
            &["auth", "id_token"],
            &["auth", "idToken"],
            &["id_token"],
            &["idToken"],
        ],
    )
    .unwrap_or_default();
    let refresh_token = optional_string_paths(
        item,
        &[
            &["tokens", "refresh_token"],
            &["tokens", "refreshToken"],
            &["credentials", "refresh_token"],
            &["credentials", "refreshToken"],
            &["auth", "refresh_token"],
            &["auth", "refreshToken"],
            &["refresh_token"],
            &["refreshToken"],
        ],
    )
    .unwrap_or_default();
    let account_id_hint = optional_string_paths(
        item,
        &[
            &["tokens", "account_id"],
            &["tokens", "accountId"],
            &["credentials", "account_id"],
            &["credentials", "accountId"],
            &["account_id"],
            &["accountId"],
        ],
    );
    let chatgpt_account_id_hint = optional_string_paths(
        item,
        &[
            &["tokens", "chatgpt_account_id"],
            &["tokens", "chatgptAccountId"],
            &["credentials", "chatgpt_account_id"],
            &["credentials", "chatgptAccountId"],
            &["chatgpt_account_id"],
            &["chatgptAccountId"],
            &["account", "chatgpt_account_id"],
            &["account", "chatgptAccountId"],
            &["account", "account_id"],
            &["account", "accountId"],
            &["account", "id"],
        ],
    );
    let user_id_hint = optional_string_paths(
        item,
        &[
            &["tokens", "chatgpt_user_id"],
            &["tokens", "chatgptUserId"],
            &["credentials", "chatgpt_user_id"],
            &["credentials", "chatgptUserId"],
            &["chatgpt_user_id"],
            &["chatgptUserId"],
            &["user_id"],
            &["userId"],
            &["user", "id"],
        ],
    );
    Ok(ImportTokenPayload {
        access_token,
        id_token,
        refresh_token,
        account_id_hint,
        chatgpt_account_id_hint,
        user_id_hint,
    })
}

/// 函数 `resolve_logical_account_id`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - payload: 参数 payload
/// - subject_account_id: 参数 subject_account_id
/// - chatgpt_account_id: 参数 chatgpt_account_id
/// - workspace_id: 参数 workspace_id
/// - token_fingerprint: 参数 token_fingerprint
///
/// # 返回
/// 返回函数执行结果
fn resolve_logical_account_id(
    payload: &ImportTokenPayload,
    subject_account_id: Option<&str>,
    chatgpt_account_id: Option<&str>,
    workspace_id: Option<&str>,
    token_fingerprint: Option<&str>,
) -> Result<String, String> {
    let account_id_hint = payload
        .account_id_hint
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty());
    let hint_suffix = account_id_hint.and_then(|value| {
        value
            .split_once("::")
            .map(|(_, suffix)| suffix.trim())
            .filter(|suffix| !suffix.is_empty())
    });

    if let Some(sub) = subject_account_id.map(str::trim).filter(|v| !v.is_empty()) {
        let scoped_id = build_account_storage_id(sub, chatgpt_account_id, workspace_id, None);
        if scoped_id != sub {
            return Ok(scoped_id);
        }
        if let Some(v) = hint_suffix {
            return Ok(account_key(sub, Some(&format!("hint={v}"))));
        }
        if let Some(fp) = token_fingerprint.map(str::trim).filter(|v| !v.is_empty()) {
            return Ok(account_key(sub, Some(&format!("fp_{fp}"))));
        }
        return Ok(sub.to_string());
    }

    let chatgpt = chatgpt_account_id
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .or_else(|| extract_chatgpt_account_id(&payload.id_token))
        .or_else(|| extract_chatgpt_account_id(&payload.access_token));
    let workspace = workspace_id
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    if let Some(chatgpt) = chatgpt.as_ref() {
        if let Some(workspace) = workspace.as_ref() {
            if chatgpt != workspace {
                return Ok(account_key(chatgpt, Some(workspace)));
            }
        }
        return Ok(chatgpt.to_string());
    }

    if let Some(value) = account_id_hint {
        return Ok(value.to_string());
    }

    if let Some(workspace) = workspace {
        return Ok(workspace);
    }

    Err("unable to resolve account id from tokens.account_id / id_token / access_token".to_string())
}

/// 函数 `token_fingerprint`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - refresh_token: 参数 refresh_token
///
/// # 返回
/// 返回函数执行结果
fn token_fingerprint(refresh_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(refresh_token.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(12);
    for b in digest.iter().take(6) {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// 函数 `extract_account_meta`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - item: 参数 item
///
/// # 返回
/// 返回函数执行结果
fn extract_account_meta(item: &Value) -> ImportAccountMeta {
    ImportAccountMeta {
        label: optional_string_paths(
            item,
            &[&["meta", "label"], &["label"], &["name"], &["user", "name"]],
        ),
        issuer: optional_string_paths(
            item,
            &[&["meta", "issuer"], &["issuer"], &["credentials", "issuer"]],
        ),
        group_name: optional_string_paths(
            item,
            &[
                &["meta", "group_name"],
                &["meta", "groupName"],
                &["group_name"],
                &["groupName"],
            ],
        ),
        note: optional_string_paths(item, &[&["meta", "note"], &["note"], &["notes"]]),
        tags: optional_tags_paths(item, &[&["meta", "tags"], &["tags"]]),
        workspace_id: optional_string_paths(
            item,
            &[
                &["meta", "workspace_id"],
                &["meta", "workspaceId"],
                &["credentials", "workspace_id"],
                &["credentials", "workspaceId"],
                &["workspace_id"],
                &["workspaceId"],
                &["account", "workspace_id"],
                &["account", "workspaceId"],
            ],
        ),
        chatgpt_account_id: optional_string_paths(
            item,
            &[
                &["meta", "chatgpt_account_id"],
                &["meta", "chatgptAccountId"],
                &["credentials", "chatgpt_account_id"],
                &["credentials", "chatgptAccountId"],
                &["chatgpt_account_id"],
                &["chatgptAccountId"],
                &["account", "chatgpt_account_id"],
                &["account", "chatgptAccountId"],
                &["account", "account_id"],
                &["account", "accountId"],
                &["account", "id"],
            ],
        ),
    }
}

fn required_string_paths(value: &Value, paths: &[&[&str]], label: &str) -> Result<String, String> {
    optional_string_paths(value, paths).ok_or_else(|| format!("missing field: {label}"))
}

fn optional_string_paths(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| value_at_path(value, path).and_then(value_to_string))
}

fn value_at_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn value_to_string(value: &Value) -> Option<String> {
    let raw = match value {
        Value::String(value) => value.trim().to_string(),
        Value::Number(value) => value.to_string(),
        _ => return None,
    };
    (!raw.is_empty()).then_some(raw)
}

/// 函数 `optional_tags`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - value: 参数 value
/// - key: 参数 key
///
/// # 返回
/// 返回函数执行结果
fn optional_tags(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        let normalized = text
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.join(","))
        }
    } else if let Some(items) = value.as_array() {
        let normalized = items
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.join(","))
        }
    } else {
        None
    }
}

/// 函数 `optional_tags_any`
///
/// 作者: gaohongshun
///
/// 时间: 2026-04-02
///
/// # 参数
/// - candidates: 参数 candidates
///
/// # 返回
/// 返回函数执行结果
fn optional_tags_paths(value: &Value, paths: &[&[&str]]) -> Option<String> {
    paths
        .iter()
        .find_map(|path| value_at_path(value, path).and_then(optional_tags))
}

#[cfg(test)]
#[path = "tests/account_import_tests.rs"]
mod tests;
