use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use codexmanager_core::storage::{AccountAgentIdentity, Storage, Token};
use crypto_box::SecretKey as BoxSecretKey;
use ed25519_dalek::pkcs8::DecodePrivateKey;
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

const AGENT_IDENTITY_AUTH_BASE_URL: &str = "https://auth.openai.com/api/accounts";
const AGENT_IDENTITY_TASK_REGISTRATION_TIMEOUT: Duration = Duration::from_secs(30);

static AGENT_IDENTITY_TASK_LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    OnceLock::new();

#[derive(Debug, Serialize)]
struct AgentAssertionEnvelope<'a> {
    agent_runtime_id: &'a str,
    task_id: &'a str,
    timestamp: &'a str,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct AgentTaskRegistrationResponse {
    #[serde(default, alias = "taskId")]
    task_id: String,
    #[serde(default, alias = "encryptedTaskId")]
    encrypted_task_id: String,
}

pub(crate) fn validate_agent_identity_private_key(encoded: &str) -> Result<(), String> {
    decode_agent_identity_private_key(encoded).map(|_| ())
}

pub(crate) fn resolve_chatgpt_authorization(
    storage: &Storage,
    account_id: &str,
    token: &Token,
) -> Result<String, String> {
    if storage
        .find_account_agent_identity(account_id)
        .map_err(|err| err.to_string())?
        .is_some()
    {
        return build_account_agent_assertion(storage, account_id);
    }

    let access_token = token.access_token.trim();
    if access_token.is_empty() {
        return Err("missing chatgpt access token".to_string());
    }
    Ok(access_token.to_string())
}

pub(crate) fn authorization_header_value(credential: &str) -> String {
    let credential = credential.trim();
    if credential.starts_with("AgentAssertion ") || credential.starts_with("Bearer ") {
        credential.to_string()
    } else {
        format!("Bearer {credential}")
    }
}

pub(crate) fn build_account_agent_assertion(
    storage: &Storage,
    account_id: &str,
) -> Result<String, String> {
    ensure_agent_identity_task(storage, account_id)?;
    let identity = storage
        .find_account_agent_identity(account_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "agent identity credentials are unavailable".to_string())?;
    build_agent_assertion(&identity, chrono::Utc::now())
}

fn ensure_agent_identity_task(storage: &Storage, account_id: &str) -> Result<(), String> {
    let identity = storage
        .find_account_agent_identity(account_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "agent identity credentials are unavailable".to_string())?;
    if identity
        .task_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return Ok(());
    }

    let task_lock = account_task_lock(account_id);
    let _guard = crate::lock_utils::lock_recover(task_lock.as_ref(), "agent_identity_task_lock");
    let refreshed = storage
        .find_account_agent_identity(account_id)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| "agent identity credentials are unavailable".to_string())?;
    if refreshed
        .task_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
    {
        return Ok(());
    }

    let task_id = register_agent_identity_task(&refreshed)?;
    storage
        .update_account_agent_identity_task(account_id, &task_id)
        .map_err(|err| err.to_string())
}

fn account_task_lock(account_id: &str) -> Arc<Mutex<()>> {
    let locks = AGENT_IDENTITY_TASK_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut locks = crate::lock_utils::lock_recover(locks, "agent_identity_task_locks");
    locks
        .entry(account_id.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

fn decode_agent_identity_private_key(encoded: &str) -> Result<SigningKey, String> {
    let der = STANDARD
        .decode(encoded.trim())
        .map_err(|_| "agent identity private key is not valid base64".to_string())?;
    SigningKey::from_pkcs8_der(&der)
        .map_err(|_| "agent identity private key is not valid PKCS#8 Ed25519".to_string())
}

fn build_agent_assertion(
    identity: &AccountAgentIdentity,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<String, String> {
    let runtime_id = identity.agent_runtime_id.trim();
    let task_id = identity.task_id.as_deref().unwrap_or_default().trim();
    if runtime_id.is_empty() || task_id.is_empty() {
        return Err("agent identity runtime or task id is missing".to_string());
    }
    let timestamp = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let signing_key = decode_agent_identity_private_key(&identity.agent_private_key)?;
    let payload = format!("{runtime_id}:{task_id}:{timestamp}");
    let signature = signing_key.sign(payload.as_bytes());
    let envelope = AgentAssertionEnvelope {
        agent_runtime_id: runtime_id,
        task_id,
        timestamp: &timestamp,
        signature: STANDARD.encode(signature.to_bytes()),
    };
    let encoded = serde_json::to_vec(&envelope)
        .map_err(|_| "failed to serialize agent assertion".to_string())?;
    Ok(format!(
        "AgentAssertion {}",
        URL_SAFE_NO_PAD.encode(encoded)
    ))
}

fn sign_agent_task_registration(
    identity: &AccountAgentIdentity,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(String, String), String> {
    let runtime_id = identity.agent_runtime_id.trim();
    if runtime_id.is_empty() {
        return Err("agent identity runtime id is missing".to_string());
    }
    let timestamp = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let signing_key = decode_agent_identity_private_key(&identity.agent_private_key)?;
    let signature = signing_key.sign(format!("{runtime_id}:{timestamp}").as_bytes());
    Ok((timestamp, STANDARD.encode(signature.to_bytes())))
}

fn register_agent_identity_task(identity: &AccountAgentIdentity) -> Result<String, String> {
    let (timestamp, signature) = sign_agent_task_registration(identity, chrono::Utc::now())?;
    let url = format!(
        "{}/v1/agent/{}/task/register",
        AGENT_IDENTITY_AUTH_BASE_URL.trim_end_matches('/'),
        identity.agent_runtime_id.trim()
    );
    let mut client_builder =
        reqwest::blocking::Client::builder().timeout(AGENT_IDENTITY_TASK_REGISTRATION_TIMEOUT);
    if let Some(proxy_url) =
        crate::gateway::current_upstream_proxy_url_for_account(identity.account_id.as_str())
    {
        if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
            client_builder = client_builder.proxy(proxy);
        }
    }
    let client = client_builder
        .build()
        .map_err(|_| "failed to create agent task registration client".to_string())?;
    let response = client
        .post(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .json(&serde_json::json!({
            "timestamp": timestamp,
            "signature": signature,
        }))
        .send()
        .map_err(|_| "agent task registration request failed".to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "agent task registration returned status {}",
            status.as_u16()
        ));
    }
    let result = response
        .json::<AgentTaskRegistrationResponse>()
        .map_err(|_| "agent task registration response is invalid".to_string())?;
    let task_id = result.task_id.trim();
    if !task_id.is_empty() {
        return Ok(task_id.to_string());
    }
    if result.encrypted_task_id.trim().is_empty() {
        return Err("agent task registration response omitted task id".to_string());
    }
    decrypt_agent_task_id(identity, &result.encrypted_task_id)
}

fn decrypt_agent_task_id(identity: &AccountAgentIdentity, encoded: &str) -> Result<String, String> {
    let ciphertext = STANDARD
        .decode(encoded.trim())
        .map_err(|_| "encrypted agent task id is not valid base64".to_string())?;
    let signing_key = decode_agent_identity_private_key(&identity.agent_private_key)?;
    let digest = Sha512::digest(signing_key.to_bytes());
    let mut curve_private = [0_u8; 32];
    curve_private.copy_from_slice(&digest[..32]);
    curve_private[0] &= 248;
    curve_private[31] &= 127;
    curve_private[31] |= 64;
    let secret_key = BoxSecretKey::from_bytes(curve_private);
    let plaintext = secret_key
        .unseal(&ciphertext)
        .map_err(|_| "failed to decrypt encrypted agent task id".to_string())?;
    let task_id = String::from_utf8(plaintext)
        .map_err(|_| "decrypted agent task id is not valid UTF-8".to_string())?;
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err("decrypted agent task id is empty".to_string());
    }
    Ok(task_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_box::aead::OsRng;
    use ed25519_dalek::pkcs8::EncodePrivateKey;
    use ed25519_dalek::Signature;

    fn sample_identity() -> AccountAgentIdentity {
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let private_key = signing_key.to_pkcs8_der().expect("encode private key");
        AccountAgentIdentity {
            account_id: "account-1".to_string(),
            agent_runtime_id: "runtime-1".to_string(),
            agent_private_key: STANDARD.encode(private_key.as_bytes()),
            task_id: Some("task-1".to_string()),
            chatgpt_user_id: "user-1".to_string(),
            chatgpt_account_is_fedramp: false,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn assertion_uses_agent_assertion_scheme_and_signed_fields() {
        let identity = sample_identity();
        let now = chrono::DateTime::parse_from_rfc3339("2026-07-22T10:20:30Z")
            .expect("time")
            .with_timezone(&chrono::Utc);
        let assertion = build_agent_assertion(&identity, now).expect("assertion");
        assert!(assertion.starts_with("AgentAssertion "));
        let encoded = assertion.trim_start_matches("AgentAssertion ");
        let envelope: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).expect("decode envelope"))
                .expect("parse envelope");
        assert_eq!(envelope["agent_runtime_id"], "runtime-1");
        assert_eq!(envelope["task_id"], "task-1");
        assert_eq!(envelope["timestamp"], "2026-07-22T10:20:30Z");
        let signature = STANDARD
            .decode(envelope["signature"].as_str().expect("signature"))
            .expect("decode signature");
        let signature = Signature::from_slice(&signature).expect("parse signature");
        let signing_key = decode_agent_identity_private_key(&identity.agent_private_key)
            .expect("decode private key");
        signing_key
            .verifying_key()
            .verify_strict(b"runtime-1:task-1:2026-07-22T10:20:30Z", &signature)
            .expect("verify signature");
    }

    #[test]
    fn private_key_validation_rejects_non_pkcs8_input() {
        assert!(validate_agent_identity_private_key("bm90LWEtcHJpdmF0ZS1rZXk=").is_err());
    }

    #[test]
    fn encrypted_task_id_uses_nacl_sealed_box_compatible_key_derivation() {
        let identity = sample_identity();
        let signing_key = decode_agent_identity_private_key(&identity.agent_private_key)
            .expect("decode private key");
        let digest = Sha512::digest(signing_key.to_bytes());
        let mut curve_private = [0_u8; 32];
        curve_private.copy_from_slice(&digest[..32]);
        curve_private[0] &= 248;
        curve_private[31] &= 127;
        curve_private[31] |= 64;
        let secret_key = BoxSecretKey::from_bytes(curve_private);
        let ciphertext = secret_key
            .public_key()
            .seal(&mut OsRng, b"task-encrypted")
            .expect("seal task id");
        let decrypted = decrypt_agent_task_id(&identity, &STANDARD.encode(ciphertext))
            .expect("decrypt task id");
        assert_eq!(decrypted, "task-encrypted");
    }

    #[test]
    fn authorization_header_does_not_prefix_agent_assertion_with_bearer() {
        assert_eq!(
            authorization_header_value("AgentAssertion abc"),
            "AgentAssertion abc"
        );
        assert_eq!(
            authorization_header_value("oauth-token"),
            "Bearer oauth-token"
        );
    }
}
