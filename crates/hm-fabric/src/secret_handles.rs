use crate::egress::{EgressError, EgressHttpClient, EgressReceipt, EgressRequest, EgressRouteKind};
use hm_context::{Scope, validate_id};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretHandle(String);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecretGrant {
    pub id: String,
    pub scope: Scope,
    pub principal: String,
    pub principal_revision: u64,
    pub handle: SecretHandle,
    pub operation_id: String,
    pub request_digest: String,
    pub policy_digest: String,
    pub route_id: String,
    pub header: String,
    pub expires_unix_ms: u64,
    pub remaining_uses: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SecretInventory {
    pub version: u32,
    pub handles: Vec<SecretHandle>,
    pub grants: Vec<SecretGrant>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SecretDispatchReceipt {
    pub version: u32,
    pub operation_id: String,
    pub grant_ids: Vec<String>,
    pub status: u16,
    pub egress: EgressReceipt,
}
pub struct SecretProviderResponse {
    pub receipt: SecretDispatchReceipt,
    pub original_bytes: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SecretError {
    #[error("secret operation denied")]
    Denied,
    #[error("invalid secret operation")]
    Invalid,
    #[error("secret capacity exceeded")]
    Capacity,
    #[error("secret randomness unavailable")]
    Randomness,
    #[error("secret dispatch: {0}")]
    Egress(EgressError),
}
struct StoredSecret {
    scope: Scope,
    bytes: Zeroizing<Vec<u8>>,
}
#[derive(Default)]
struct State {
    secrets: BTreeMap<SecretHandle, StoredSecret>,
    principals: BTreeMap<(Scope, String), u64>,
    grants: BTreeMap<(Scope, String), SecretGrant>,
}
#[derive(Default)]
pub struct SecretHandleService {
    state: Mutex<State>,
}
impl std::fmt::Debug for SecretHandleService {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretHandleService([redacted])")
    }
}
impl SecretHandleService {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn set_principal(
        &self,
        scope: &Scope,
        owner: &str,
        principal: &str,
        revision: Option<u64>,
    ) -> Result<(), SecretError> {
        owner_check(scope, owner)?;
        identifier(principal)?;
        if revision == Some(0) {
            return Err(SecretError::Invalid);
        }
        let mut state = self.state.lock().await;
        metadata_check(
            &state,
            &serde_json::to_vec(&(scope, principal)).map_err(|_| SecretError::Invalid)?,
        )?;
        let key = (scope.clone(), principal.into());
        if let Some(revision) = revision {
            if state
                .principals
                .get(&key)
                .is_some_and(|current| revision <= *current)
            {
                return Err(SecretError::Denied);
            }
            if state.principals.len() >= 1024 && !state.principals.contains_key(&key) {
                return Err(SecretError::Capacity);
            }
            state.principals.insert(key, revision);
        } else {
            state.principals.remove(&key);
        }
        state
            .grants
            .retain(|_, grant| grant.scope != *scope || grant.principal != principal);
        Ok(())
    }
    pub async fn insert(
        &self,
        scope: &Scope,
        owner: &str,
        bytes: Zeroizing<Vec<u8>>,
    ) -> Result<SecretHandle, SecretError> {
        owner_check(scope, owner)?;
        if bytes.is_empty()
            || bytes.len() > 8192
            || reqwest::header::HeaderValue::from_bytes(&bytes).is_err()
        {
            return Err(SecretError::Invalid);
        }
        let mut state = self.state.lock().await;
        if state.secrets.len() >= 256 {
            return Err(SecretError::Capacity);
        }
        let metadata = serde_json::to_vec(scope).map_err(|_| SecretError::Invalid)?;
        metadata_check(&state, &metadata)?;
        if contains(&metadata, &bytes) {
            return Err(SecretError::Invalid);
        }
        let existing = serde_json::to_vec(&(
            state.grants.values().collect::<Vec<_>>(),
            state.principals.keys().collect::<Vec<_>>(),
            state.secrets.keys().collect::<Vec<_>>(),
        ))
        .map_err(|_| SecretError::Invalid)?;
        if contains(&existing, &bytes) {
            return Err(SecretError::Invalid);
        }
        let mut selected = None;
        for _ in 0..16 {
            let mut random = [0u8; 32];
            getrandom::fill(&mut random).map_err(|_| SecretError::Randomness)?;
            let handle = SecretHandle(random.iter().map(|byte| format!("{byte:02x}")).collect());
            if !state.secrets.contains_key(&handle)
                && !contains(handle.0.as_bytes(), &bytes)
                && metadata_check(&state, handle.0.as_bytes()).is_ok()
            {
                selected = Some(handle);
                break;
            }
        }
        let handle = selected.ok_or(SecretError::Randomness)?;
        state.secrets.insert(
            handle.clone(),
            StoredSecret {
                scope: scope.clone(),
                bytes,
            },
        );
        Ok(handle)
    }
    pub async fn revoke_handle(
        &self,
        scope: &Scope,
        owner: &str,
        handle: &SecretHandle,
    ) -> Result<(), SecretError> {
        owner_check(scope, owner)?;
        let mut state = self.state.lock().await;
        if !state
            .secrets
            .get(handle)
            .is_some_and(|secret| secret.scope == *scope)
        {
            return Err(SecretError::Denied);
        }
        state.secrets.remove(handle);
        state.grants.retain(|_, grant| &grant.handle != handle);
        Ok(())
    }
    pub async fn grant(
        &self,
        scope: &Scope,
        owner: &str,
        mut grant: SecretGrant,
        client: &EgressHttpClient,
        request: &EgressRequest,
    ) -> Result<SecretGrant, SecretError> {
        owner_check(scope, owner)?;
        for id in [
            &grant.id,
            &grant.principal,
            &grant.operation_id,
            &grant.route_id,
        ] {
            identifier(id)?;
        }
        if grant.scope != *scope
            || client.policy().scope != *scope
            || grant.remaining_uses == 0
            || grant.remaining_uses > 1024
            || grant.expires_unix_ms <= now()?
            || grant.expires_unix_ms.saturating_sub(now()?) > 86400000
        {
            return Err(SecretError::Invalid);
        }
        let name = reqwest::header::HeaderName::from_bytes(grant.header.as_bytes())
            .map_err(|_| SecretError::Invalid)?;
        if name.as_str() != grant.header
            || !client.policy().routes.iter().any(|route| {
                route.kind == EgressRouteKind::Destination
                    && route.id == grant.route_id
                    && route.methods.contains(&request.method)
                    && route.allowed_headers.contains(&grant.header)
            })
        {
            return Err(SecretError::Denied);
        }
        let mut state = self.state.lock().await;
        if state.grants.len() >= 1024
            || state
                .grants
                .contains_key(&(scope.clone(), grant.id.clone()))
            || state
                .principals
                .get(&(scope.clone(), grant.principal.clone()))
                != Some(&grant.principal_revision)
            || !state
                .secrets
                .get(&grant.handle)
                .is_some_and(|secret| secret.scope == *scope)
        {
            return Err(SecretError::Denied);
        }
        request_check(&state, request)?;
        metadata_check(
            &state,
            &serde_json::to_vec(client.policy()).map_err(|_| SecretError::Invalid)?,
        )?;
        metadata_check(
            &state,
            &serde_json::to_vec(&grant).map_err(|_| SecretError::Invalid)?,
        )?;
        grant.request_digest = request_digest(request);
        grant.policy_digest = policy_digest(client)?;
        metadata_check(
            &state,
            &serde_json::to_vec(&grant).map_err(|_| SecretError::Invalid)?,
        )?;
        state
            .grants
            .insert((scope.clone(), grant.id.clone()), grant.clone());
        Ok(grant)
    }
    pub async fn revoke_grant(
        &self,
        scope: &Scope,
        owner: &str,
        id: &str,
    ) -> Result<(), SecretError> {
        owner_check(scope, owner)?;
        if self
            .state
            .lock()
            .await
            .grants
            .remove(&(scope.clone(), id.into()))
            .is_none()
        {
            return Err(SecretError::Denied);
        }
        Ok(())
    }
    pub async fn inventory(
        &self,
        scope: &Scope,
        owner: &str,
    ) -> Result<SecretInventory, SecretError> {
        owner_check(scope, owner)?;
        let state = self.state.lock().await;
        Ok(SecretInventory {
            version: 1,
            handles: state
                .secrets
                .iter()
                .filter(|(_, secret)| secret.scope == *scope)
                .map(|(handle, _)| handle.clone())
                .collect(),
            grants: state
                .grants
                .values()
                .filter(|grant| grant.scope == *scope)
                .cloned()
                .collect(),
        })
    }
    pub async fn validate_public_bytes(&self, bytes: &[u8]) -> Result<(), SecretError> {
        let state = self.state.lock().await;
        metadata_check(&state, bytes)?;
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) {
            if !json_secret_free(&state, &value) {
                return Err(SecretError::Denied);
            }
        }
        Ok(())
    }
    pub async fn dispatch(
        &self,
        client: &EgressHttpClient,
        scope: &Scope,
        principal: &str,
        principal_revision: u64,
        operation_id: &str,
        request: EgressRequest,
        grant_ids: &[String],
    ) -> Result<SecretDispatchReceipt, SecretError> {
        self.dispatch_inner(
            client,
            scope,
            principal,
            principal_revision,
            operation_id,
            request,
            grant_ids,
            false,
        )
        .await
        .map(|response| response.receipt)
    }
    pub async fn dispatch_provider_response(
        &self,
        client: &EgressHttpClient,
        scope: &Scope,
        principal: &str,
        principal_revision: u64,
        operation_id: &str,
        request: EgressRequest,
        grant_ids: &[String],
    ) -> Result<SecretProviderResponse, SecretError> {
        self.dispatch_inner(
            client,
            scope,
            principal,
            principal_revision,
            operation_id,
            request,
            grant_ids,
            true,
        )
        .await
    }
    async fn dispatch_inner(
        &self,
        client: &EgressHttpClient,
        scope: &Scope,
        principal: &str,
        principal_revision: u64,
        operation_id: &str,
        mut request: EgressRequest,
        grant_ids: &[String],
        provider_response: bool,
    ) -> Result<SecretProviderResponse, SecretError> {
        scope.validate().map_err(|_| SecretError::Invalid)?;
        identifier(principal)?;
        identifier(operation_id)?;
        if client.policy().scope != *scope
            || (!provider_response && grant_ids.is_empty())
            || grant_ids.len() > 8
        {
            return Err(SecretError::Denied);
        }
        let mut state = self.state.lock().await;
        if state.principals.get(&(scope.clone(), principal.into())) != Some(&principal_revision) {
            return Err(SecretError::Denied);
        }
        request_check(&state, &request)?;
        metadata_check(
            &state,
            &serde_json::to_vec(client.policy()).map_err(|_| SecretError::Invalid)?,
        )?;
        metadata_check(
            &state,
            &serde_json::to_vec(&(scope, principal, operation_id, grant_ids))
                .map_err(|_| SecretError::Invalid)?,
        )?;
        let digest = request_digest(&request);
        let policy_digest = policy_digest(client)?;
        let timestamp = now()?;
        let mut names = request
            .headers
            .iter()
            .map(|(name, _)| name.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        let mut unique = BTreeSet::new();
        for id in grant_ids {
            identifier(id)?;
            let grant = state
                .grants
                .get(&(scope.clone(), id.clone()))
                .ok_or(SecretError::Denied)?;
            if !unique.insert(id)
                || grant.principal != principal
                || grant.principal_revision != principal_revision
                || grant.operation_id != operation_id
                || grant.remaining_uses == 0
                || grant.expires_unix_ms <= timestamp
                || grant.request_digest != digest
                || grant.policy_digest != policy_digest
                || !names.insert(grant.header.clone())
                || !state
                    .secrets
                    .get(&grant.handle)
                    .is_some_and(|secret| secret.scope == *scope)
            {
                return Err(SecretError::Denied);
            }
        }
        let resolved = client
            .authorize(&request.url, request.method)
            .await
            .map_err(SecretError::Egress)?;
        for id in grant_ids {
            let grant = &state.grants[&(scope.clone(), id.clone())];
            if grant.route_id != resolved.route_id || grant.expires_unix_ms <= now()? {
                return Err(SecretError::Denied);
            }
        }
        // Cancellation or an uncertain write consumes authority; reconciliation must precede a fresh grant.
        for id in grant_ids {
            let key = (scope.clone(), id.clone());
            let grant = state.grants.get_mut(&key).unwrap();
            grant.remaining_uses -= 1;
            let header = grant.header.clone();
            let handle = grant.handle.clone();
            request
                .headers
                .push((header, state.secrets[&handle].bytes.to_vec()));
        }
        let mut dispatch_policy = client.policy().clone();
        dispatch_policy.max_redirects = 0;
        let dispatch_client =
            EgressHttpClient::new(dispatch_policy).map_err(SecretError::Egress)?;
        let response = dispatch_client
            .execute(request)
            .await
            .map_err(SecretError::Egress)?;
        let original_bytes = if provider_response {
            if metadata_check(&state, &response.body).is_err() {
                return Err(SecretError::Egress(EgressError::OutcomeUncertain));
            }
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&response.body) {
                if !json_secret_free(&state, &json) {
                    return Err(SecretError::Egress(EgressError::OutcomeUncertain));
                }
            }
            response.body
        } else {
            Vec::new()
        };
        Ok(SecretProviderResponse {
            original_bytes,
            receipt: SecretDispatchReceipt {
                version: 1,
                operation_id: operation_id.into(),
                grant_ids: grant_ids.to_vec(),
                status: response.status,
                egress: response.receipt,
            },
        })
    }
}
fn identifier(value: &str) -> Result<(), SecretError> {
    validate_id(value).map_err(|_| SecretError::Invalid)
}
fn owner_check(scope: &Scope, owner: &str) -> Result<(), SecretError> {
    scope.validate().map_err(|_| SecretError::Invalid)?;
    if owner != scope.owner_id {
        return Err(SecretError::Denied);
    }
    Ok(())
}
fn now() -> Result<u64, SecretError> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| SecretError::Invalid)?
            .as_millis(),
    )
    .map_err(|_| SecretError::Invalid)
}
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
fn metadata_check(state: &State, bytes: &[u8]) -> Result<(), SecretError> {
    if state
        .secrets
        .values()
        .any(|secret| contains(bytes, &secret.bytes))
    {
        return Err(SecretError::Denied);
    }
    Ok(())
}
fn request_check(state: &State, request: &EgressRequest) -> Result<(), SecretError> {
    metadata_check(state, request.url.as_bytes())?;
    metadata_check(state, &request.body)?;
    if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&request.body) {
        if !json_secret_free(state, &json) {
            return Err(SecretError::Denied);
        }
    }
    for (name, value) in &request.headers {
        metadata_check(state, name.as_bytes())?;
        metadata_check(state, value)?;
    }
    Ok(())
}
pub fn request_digest(request: &EgressRequest) -> String {
    let mut hash = Sha256::new();
    let method = format!("{:?}", request.method);
    for bytes in [
        request.url.as_bytes(),
        method.as_bytes(),
        request.body.as_slice(),
    ] {
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
    }
    for (name, value) in &request.headers {
        for bytes in [name.as_bytes(), value.as_slice()] {
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(bytes);
        }
    }
    format!("{:x}", hash.finalize())
}
fn policy_digest(client: &EgressHttpClient) -> Result<String, SecretError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(client.policy()).map_err(|_| SecretError::Invalid)?)
    ))
}

fn json_secret_free(state: &State, value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(text) => metadata_check(state, text.as_bytes()).is_ok(),
        serde_json::Value::Array(values) => {
            values.iter().all(|value| json_secret_free(state, value))
        }
        serde_json::Value::Object(values) => values.iter().all(|(key, value)| {
            metadata_check(state, key.as_bytes()).is_ok() && json_secret_free(state, value)
        }),
        _ => true,
    }
}
