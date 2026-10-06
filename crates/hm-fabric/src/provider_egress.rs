use crate::{
    egress::{
        EgressHttpClient, EgressMethod, EgressPolicy, EgressRequest, EgressRoute, EgressRouteKind,
    },
    role_runner::RunnerDeclaration,
    secret_handles::{
        SecretError, SecretGrant, SecretHandle, SecretHandleService, SecretProviderResponse,
    },
    transport::AuthenticatedIdentity,
};
use hm_context::{Scope, types::digest_bytes};
use hm_llm::WireRequest;
use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct RunnerProviderGuard {
    client: Arc<EgressHttpClient>,
    secrets: Arc<SecretHandleService>,
    credential: Option<SecretHandle>,
}
#[derive(Clone)]
pub(crate) struct RunnerProviderSession {
    principal: String,
    revision: u64,
}
impl RunnerProviderGuard {
    pub fn new(
        policy: EgressPolicy,
        secrets: Arc<SecretHandleService>,
        credential: Option<SecretHandle>,
        owner: &str,
    ) -> Result<Self, SecretError> {
        if owner != policy.scope.owner_id {
            return Err(SecretError::Denied);
        }
        Ok(Self {
            client: Arc::new(EgressHttpClient::new(policy).map_err(SecretError::Egress)?),
            secrets,
            credential,
        })
    }
    pub fn policy(&self) -> &EgressPolicy {
        self.client.policy()
    }
    pub(crate) fn local(scope: &Scope, d: &RunnerDeclaration) -> Result<Self, SecretError> {
        let mut routes = Vec::new();
        for (id, url, method) in [
            ("runner-model", d.endpoint.clone(), EgressMethod::Post),
            (
                "runner-catalog",
                d.endpoint.replace("/api/chat", "/api/tags"),
                EgressMethod::Get,
            ),
        ] {
            let mut route = EgressRoute::for_url(
                id,
                EgressRouteKind::Destination,
                &url,
                BTreeSet::from([method]),
            )
            .map_err(SecretError::Egress)?;
            route.allowed_headers.insert("content-type".into());
            route.allowed_headers.insert("authorization".into());
            route
                .allowed_non_public_addresses
                .insert("127.0.0.1".parse().map_err(|_| SecretError::Invalid)?);
            routes.push(route);
        }
        Self::new(
            EgressPolicy {
                version: 1,
                scope: scope.clone(),
                routes,
                proxy: None,
                max_redirects: 0,
                max_request_bytes: 65536,
                max_response_bytes: d.max_response_bytes.max(65536),
                max_header_bytes: 8192,
                timeout_ms: d.timeout_ms,
            },
            Arc::new(SecretHandleService::new()),
            None,
            &scope.owner_id,
        )
    }
    pub(crate) async fn check_public(&self, bytes: &[u8]) -> Result<(), SecretError> {
        self.secrets.validate_public_bytes(bytes).await
    }
    pub(crate) async fn session(
        &self,
        identity: &AuthenticatedIdentity,
        module: &str,
        generation: u64,
    ) -> Result<RunnerProviderSession, SecretError> {
        if identity.scope() != &self.client.policy().scope || generation == 0 {
            return Err(SecretError::Denied);
        }
        let principal = format!(
            "runner-{}",
            digest_bytes(
                &serde_json::to_vec(&(
                    identity.scope(),
                    identity.peer_key(),
                    identity.session_id(),
                    module,
                    generation
                ))
                .map_err(|_| SecretError::Invalid)?
            )
        );
        self.secrets
            .set_principal(
                identity.scope(),
                &identity.scope().owner_id,
                &principal,
                Some(generation),
            )
            .await?;
        Ok(RunnerProviderSession {
            principal,
            revision: generation,
        })
    }
    pub(crate) async fn dispatch(
        &self,
        session: &RunnerProviderSession,
        operation: &str,
        request: &WireRequest,
    ) -> Result<SecretProviderResponse, SecretError> {
        if request.headers.contains_key("authorization")
            || request.headers.keys().any(|name| name != "content-type")
        {
            return Err(SecretError::Denied);
        }
        let method = match request.method.as_str() {
            "POST" => EgressMethod::Post,
            "GET" => EgressMethod::Get,
            _ => return Err(SecretError::Denied),
        };
        let body = if method == EgressMethod::Get {
            if !request.body.is_null() {
                return Err(SecretError::Denied);
            }
            Vec::new()
        } else {
            serde_json::to_vec(&request.body).map_err(|_| SecretError::Invalid)?
        };
        let egress = EgressRequest {
            url: request.url.clone(),
            method,
            headers: request
                .headers
                .iter()
                .map(|(name, value)| (name.clone(), value.as_bytes().to_vec()))
                .collect(),
            body,
        };
        let mut ids = Vec::new();
        if let Some(handle) = &self.credential {
            let resolved = self
                .client
                .authorize(&egress.url, egress.method)
                .await
                .map_err(SecretError::Egress)?;
            let scope = &self.client.policy().scope;
            let id = format!("dispatch-{}", digest_bytes(operation.as_bytes()));
            let expires = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| SecretError::Invalid)?
                .as_millis() as u64
                + self.client.policy().timeout_ms;
            self.secrets
                .grant(
                    scope,
                    &scope.owner_id,
                    SecretGrant {
                        id: id.clone(),
                        scope: scope.clone(),
                        principal: session.principal.clone(),
                        principal_revision: session.revision,
                        handle: handle.clone(),
                        operation_id: operation.into(),
                        request_digest: String::new(),
                        policy_digest: String::new(),
                        route_id: resolved.route_id,
                        header: "authorization".into(),
                        expires_unix_ms: expires,
                        remaining_uses: 1,
                    },
                    &self.client,
                    &egress,
                )
                .await?;
            ids.push(id);
        }
        self.secrets
            .dispatch_provider_response(
                &self.client,
                &self.client.policy().scope,
                &session.principal,
                session.revision,
                operation,
                egress,
                &ids,
            )
            .await
    }
}
