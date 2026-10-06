use crate::{
    compat_transport::{MessageKind, PublicEnvelope, PROTOCOL},
    effects::{
        EffectIntent, EffectObservation, EffectOutcome, EffectReceipt, EffectState, EffectStore,
    },
    enrollment::{EnrollmentError, PublicTlsSession},
    storage::{FencedStore, Migration, StorageError},
    transport::AuthenticatedIdentity,
};
use hm_context::{
    mobile_journal::{
        JournalBinding, JournalError, MobileJournal, MutationIdentity, MutationRecord,
        MutationState, NeutralReceipt, ReceiptOutcome,
    },
    types::digest_bytes,
    ContextError, Cursor, Scope,
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Debug, thiserror::Error)]
pub enum DeliveryError {
    #[error("encrypted delivery refusal: {0}")]
    Refused(String),
    #[error(transparent)]
    Journal(#[from] JournalError),
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error(transparent)]
    Enrollment(#[from] EnrollmentError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}
fn refuse(s: &str) -> DeliveryError {
    DeliveryError::Refused(s.into())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationRequest {
    pub version: u32,
    pub identity: MutationIdentity,
    pub expected_epoch: u64,
    pub dispatched_session_id: String,
    #[serde(default)]
    pub payload: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationEvidence {
    pub version: u32,
    pub identity: MutationIdentity,
    pub dispatched_session_id: String,
    pub observed_session_id: String,
    pub device_generation: u64,
    pub native_receipt_bytes: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    identity: MutationIdentity,
    dispatch_session: String,
}
const MIGRATIONS:&[Migration]=&[Migration{version:1,name:"encrypted-mobile-bindings-v1",sql:"CREATE TABLE owner(id INTEGER PRIMARY KEY CHECK(id=1),key BLOB NOT NULL,scope TEXT NOT NULL);CREATE TABLE bindings(key TEXT PRIMARY KEY,body TEXT NOT NULL);"}];
pub struct MobileEffectAuthority {
    bindings: FencedStore,
    effects: EffectStore,
    owner_key: [u8; 32],
    scope: Scope,
}
pub struct DispatchedMutation {
    intent: EffectIntent,
    binding: Binding,
}
impl DispatchedMutation {
    pub fn intent(&self) -> &EffectIntent {
        &self.intent
    }
}
impl MobileEffectAuthority {
    pub fn open(
        path: impl AsRef<Path>,
        owner: &AuthenticatedIdentity,
        effects: EffectStore,
    ) -> Result<Self, DeliveryError> {
        let mut bindings = FencedStore::open(path, MIGRATIONS)?;
        let owner_key = *owner.peer_key();
        let scope = owner.scope().clone();
        let encoded = serde_json::to_string(&scope).unwrap();
        let epoch = bindings.epoch();
        let ok = bindings.transaction(epoch, |tx| {
            tx.execute(
                "INSERT OR IGNORE INTO owner VALUES(1,?1,?2)",
                rusqlite::params![owner_key.as_slice(), encoded],
            )?;
            let row: (Vec<u8>, String) =
                tx.query_row("SELECT key,scope FROM owner WHERE id=1", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
            Ok(row.0 == owner_key && row.1 == encoded)
        })?;
        if !ok {
            return Err(refuse("authority owner mismatch"));
        }
        Ok(Self {
            bindings,
            effects,
            owner_key,
            scope,
        })
    }
    fn device(
        &self,
        session: &PublicTlsSession,
        identity: &MutationIdentity,
    ) -> Result<(), DeliveryError> {
        let device = session
            .device()
            .ok_or_else(|| refuse("accepted enrolled peer required"))?;
        if identity.binding.scope != self.scope
            || session.scope() != &self.scope
            || identity.binding.device_id != device.device().device_id
        {
            return Err(refuse("mutation scope or device mismatch"));
        }
        identity.binding.scope.validate()?;
        hm_context::types::validate_id(&identity.binding.session_id)?;
        hm_context::types::validate_id(&identity.operation_id)?;
        hm_context::types::validate_id(&identity.operation_kind)?;
        if identity.payload_digest.len() != 64
            || !identity
                .payload_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(refuse("payload digest"));
        }
        Ok(())
    }
    fn binding(&self, key: &str) -> Result<Option<Binding>, DeliveryError> {
        self.bindings
            .read(|db| {
                let row = db.query_row("SELECT body FROM bindings WHERE key=?1", [key], |r| {
                    r.get::<_, String>(0)
                });
                match row {
                    Ok(body) => serde_json::from_str(&body)
                        .map(Some)
                        .map_err(|_| StorageError::MigrationMismatch),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .map_err(Into::into)
    }
    pub fn begin_dispatch(
        &mut self,
        session: &PublicTlsSession,
        request: &MutationRequest,
    ) -> Result<DispatchedMutation, DeliveryError> {
        self.device(session, &request.identity)?;
        if request.version != 1
            || request.expected_epoch != 1
            || request.dispatched_session_id != session.session_id()
        {
            return Err(refuse("dispatch version, epoch or TLS attempt"));
        }
        let payload = request
            .payload
            .as_ref()
            .ok_or_else(|| refuse("dispatch payload required"))?;
        if payload.len() > 65536 || digest_bytes(payload) != request.identity.payload_digest {
            return Err(refuse("dispatch payload bounds or digest"));
        }
        if self.binding(&request.identity.operation_id)?.is_some()
            || self
                .effects
                .get(&self.scope, &request.identity.operation_id)?
                .is_some()
        {
            return Err(refuse("operation already admitted; inspect instead"));
        }
        let binding = Binding {
            identity: request.identity.clone(),
            dispatch_session: session.session_id().into(),
        };
        let body = serde_json::to_string(&binding).unwrap();
        let epoch = self.bindings.epoch();
        let admitted = self.bindings.transaction(epoch, |tx| {
            let n: i64 = tx.query_row("SELECT count(*) FROM bindings", [], |r| r.get(0))?;
            if n >= 1024 {
                return Ok(false);
            }
            tx.execute(
                "INSERT INTO bindings VALUES(?1,?2)",
                rusqlite::params![request.identity.operation_id, body],
            )?;
            Ok(true)
        })?;
        if !admitted {
            return Err(refuse("binding capacity"));
        }
        let prepared = self.effects.prepare(
            &self.scope,
            &request.identity.operation_id,
            &request.identity.operation_kind,
            payload,
        )?;
        let intent = self
            .effects
            .begin_dispatch(&self.scope, &prepared.key, prepared.version)?;
        Ok(DispatchedMutation { intent, binding })
    }
    pub fn complete_observed(
        &mut self,
        session: &PublicTlsSession,
        dispatch: DispatchedMutation,
        observation: EffectObservation,
    ) -> Result<(), DeliveryError> {
        self.device(session, &dispatch.binding.identity)?;
        if session.session_id() != dispatch.binding.dispatch_session {
            return Err(refuse("completion TLS attempt changed"));
        }
        self.effects.complete(
            &self.scope,
            &dispatch.intent.key,
            dispatch.intent.version,
            observation,
        )?;
        Ok(())
    }
    pub fn reconcile_observed(
        &mut self,
        owner: &AuthenticatedIdentity,
        key: &str,
        observation: EffectObservation,
    ) -> Result<(), DeliveryError> {
        if owner.peer_key() != &self.owner_key || owner.scope() != &self.scope {
            return Err(refuse("observed reconciliation owner"));
        }
        if self.binding(key)?.is_none() {
            return Err(refuse("unknown mutation binding"));
        }
        let intent = self
            .effects
            .get(&self.scope, key)?
            .ok_or_else(|| refuse("missing native intent"))?;
        self.effects
            .reconcile(&self.scope, key, intent.version, observation)?;
        Ok(())
    }
    pub fn evidence(
        &self,
        session: &PublicTlsSession,
        request: &MutationRequest,
    ) -> Result<MutationEvidence, DeliveryError> {
        self.device(session, &request.identity)?;
        if request.version != 1 || request.expected_epoch != 1 || request.payload.is_some() {
            return Err(refuse("inspection version, epoch or payload"));
        }
        let binding = self
            .binding(&request.identity.operation_id)?
            .ok_or_else(|| refuse("mutation not admitted"))?;
        if binding.identity != request.identity
            || binding.dispatch_session != request.dispatched_session_id
        {
            return Err(refuse("mutation identity or original attempt mismatch"));
        }
        let current = self
            .effects
            .get(&self.scope, &request.identity.operation_id)?
            .ok_or_else(|| refuse("native effect absent; outcome remains unknown"))?;
        let mut cursor = Cursor {
            epoch: 1,
            sequence: 0,
        };
        let mut selected = None;
        for _ in 0..4096 {
            let mut receipts = self.effects.receipts(&self.scope, cursor, 1)?;
            let Some(receipt) = receipts.pop() else { break };
            cursor = receipt.cursor;
            if receipt.intent == current {
                selected = Some(receipt);
            }
        }
        let receipt = selected
            .ok_or_else(|| refuse("authoritative receipt unavailable within scan bound"))?;
        let bytes = serde_json::to_vec(&receipt).map_err(|_| refuse("native receipt encoding"))?;
        if bytes.len() > 256 * 1024 {
            return Err(refuse("receipt capacity"));
        }
        Ok(MutationEvidence {
            version: 1,
            identity: binding.identity,
            dispatched_session_id: binding.dispatch_session,
            observed_session_id: session.session_id().into(),
            device_generation: session.device().unwrap().generation(),
            native_receipt_bytes: bytes,
        })
    }
    pub fn parse_request(
        session: &PublicTlsSession,
        envelope: &PublicEnvelope,
        operation: &str,
    ) -> Result<MutationRequest, DeliveryError> {
        if envelope.kind != MessageKind::Request
            || envelope.operation.as_deref() != Some(operation)
            || envelope.scope.as_ref() != Some(session.scope())
            || envelope.deadline_ms.is_none_or(|v| v <= now())
        {
            return Err(refuse("mutation request envelope"));
        }
        let request: MutationRequest = serde_json::from_value(
            envelope
                .payload
                .clone()
                .ok_or_else(|| refuse("missing mutation request"))?,
        )
        .map_err(|_| refuse("mutation request JSON"))?;
        if envelope.route_epoch != Some(request.expected_epoch) {
            return Err(refuse("request epoch fence"));
        }
        Ok(request)
    }
    pub async fn send_evidence(
        &self,
        session: &mut PublicTlsSession,
        envelope: &PublicEnvelope,
        request: &MutationRequest,
    ) -> Result<(), DeliveryError> {
        let mut inspection = request.clone();
        inspection.payload = None;
        let evidence = self.evidence(session, &inspection)?;
        let response = PublicEnvelope {
            protocol: PROTOCOL.into(),
            kind: MessageKind::Response,
            message_id: format!("mobile-receipt-{}", envelope.sequence),
            sequence: envelope.sequence,
            reply_to: Some(envelope.message_id.clone()),
            route_id: envelope.route_id.clone(),
            route_epoch: Some(request.expected_epoch),
            operation: envelope.operation.clone(),
            scope: Some(session.scope().clone()),
            trace: envelope.trace.clone(),
            deadline_ms: None,
            payload: Some(serde_json::to_value(evidence).unwrap()),
            error: None,
        };
        session.send(&response).await?;
        Ok(())
    }
}
pub struct EncryptedJournalSession {
    session: PublicTlsSession,
    binding: JournalBinding,
    server_certificate: String,
    outgoing: u64,
    incoming: u64,
}
impl EncryptedJournalSession {
    pub fn new(
        session: PublicTlsSession,
        binding: JournalBinding,
        server_certificate: &str,
    ) -> Result<Self, DeliveryError> {
        if session.scope() != &binding.scope
            || session.identity().certificate_sha256() != server_certificate
        {
            return Err(refuse("journal server pin or scope"));
        }
        Ok(Self {
            session,
            binding,
            server_certificate: server_certificate.into(),
            outgoing: 0,
            incoming: 0,
        })
    }
    pub fn session_id(&self) -> &str {
        self.session.session_id()
    }
    fn request(
        &self,
        record: &MutationRecord,
        payload: Option<Vec<u8>>,
        epoch: u64,
        operation: &str,
    ) -> Result<(PublicEnvelope, MutationRequest), DeliveryError> {
        if record.identity.binding != self.binding
            || self.session.identity().certificate_sha256() != self.server_certificate
        {
            return Err(refuse("journal binding changed"));
        }
        let request = MutationRequest {
            version: 1,
            identity: record.identity.clone(),
            expected_epoch: epoch,
            dispatched_session_id: record
                .dispatched_session_id
                .clone()
                .ok_or_else(|| refuse("missing dispatch attempt"))?,
            payload,
        };
        let frame = PublicEnvelope {
            protocol: PROTOCOL.into(),
            kind: MessageKind::Request,
            message_id: format!("mobile-request-{}", self.outgoing + 1),
            sequence: self.outgoing + 1,
            reply_to: None,
            route_id: Some(self.binding.session_id.clone()),
            route_epoch: Some(epoch),
            operation: Some(operation.into()),
            scope: Some(self.binding.scope.clone()),
            trace: None,
            deadline_ms: Some(now() + 10000),
            payload: Some(serde_json::to_value(&request).unwrap()),
            error: None,
        };
        Ok((frame, request))
    }
    async fn exchange(
        &mut self,
        journal: &mut MobileJournal,
        record: &MutationRecord,
        payload: Option<Vec<u8>>,
        operation: &str,
    ) -> Result<MutationRecord, DeliveryError> {
        let epoch = journal.cursor_receipt().map(|c| c.epoch).unwrap_or(1);
        let (frame, request) = self.request(record, payload, epoch, operation)?;
        self.session.send(&frame).await?;
        self.outgoing += 1;
        let response = self.session.receive().await?;
        if response.kind != MessageKind::Response
            || response.reply_to.as_deref() != Some(frame.message_id.as_str())
            || response.sequence != self.incoming + 1
            || response.scope.as_ref() != Some(&self.binding.scope)
            || response.route_epoch != Some(epoch)
            || response.error.is_some()
        {
            return Err(refuse("receipt frame correlation, epoch or scope"));
        }
        self.incoming = response.sequence;
        let evidence: MutationEvidence = serde_json::from_value(
            response
                .payload
                .ok_or_else(|| refuse("missing receipt evidence"))?,
        )
        .map_err(|_| refuse("receipt evidence JSON"))?;
        let receipt = self.validate_evidence(&evidence, &request, journal)?;
        Ok(journal.reconcile_before_next_write(&receipt)?)
    }
    fn validate_evidence(
        &self,
        evidence: &MutationEvidence,
        request: &MutationRequest,
        journal: &MobileJournal,
    ) -> Result<NeutralReceipt, DeliveryError> {
        if evidence.version != 1
            || evidence.identity != request.identity
            || evidence.dispatched_session_id != request.dispatched_session_id
            || evidence.observed_session_id != self.session.session_id()
            || evidence.device_generation == 0
            || evidence.native_receipt_bytes.len() > 256 * 1024
        {
            return Err(refuse("receipt identity or TLS attempt fence"));
        }
        let receipt: EffectReceipt = serde_json::from_slice(&evidence.native_receipt_bytes)
            .map_err(|_| refuse("native receipt JSON"))?;
        let intent = &receipt.intent;
        if receipt.contract_version != 1
            || receipt.cursor.epoch != request.expected_epoch
            || receipt.cursor.sequence == 0
            || receipt.cursor.sequence > 9_007_199_254_740_991
            || intent.version == 0
            || intent.version > 9_007_199_254_740_991
            || intent.scope != self.binding.scope
            || intent.key != request.identity.operation_id
            || intent.kind != request.identity.operation_kind
            || intent.payload_digest != request.identity.payload_digest
            || digest_bytes(&intent.payload) != request.identity.payload_digest
            || journal
                .cursor_receipt()
                .is_some_and(|c| receipt.cursor <= c)
        {
            return Err(refuse(
                "native receipt operation, payload, cursor or replay fence",
            ));
        }
        let outcome = match (&intent.state, &intent.observation) {
            (EffectState::Terminal, Some(observation))
                if !observation.evidence.trim().is_empty() =>
            {
                match observation.outcome {
                    EffectOutcome::Succeeded => ReceiptOutcome::Succeeded,
                    EffectOutcome::Failed => ReceiptOutcome::Failed,
                    EffectOutcome::NotApplied => ReceiptOutcome::NotApplied,
                }
            }
            (EffectState::Prepared | EffectState::Dispatched | EffectState::Uncertain, None) => {
                ReceiptOutcome::Unknown
            }
            _ => return Err(refuse("native receipt terminal observation")),
        };
        Ok(NeutralReceipt {
            identity: request.identity.clone(),
            intent_version: intent.version,
            cursor: receipt.cursor,
            receipt_digest: digest_bytes(&evidence.native_receipt_bytes),
            outcome,
            observed_session_id: evidence.observed_session_id.clone(),
        })
    }
    pub async fn dispatch_pending(
        &mut self,
        journal: &mut MobileJournal,
        payload: &[u8],
    ) -> Result<MutationRecord, DeliveryError> {
        let record = journal
            .pending()
            .cloned()
            .ok_or_else(|| refuse("no pending mutation"))?;
        if record.state != MutationState::Pending
            || record.identity.binding != self.binding
            || payload.len() > 65536
            || digest_bytes(payload) != record.identity.payload_digest
        {
            return Err(refuse("dispatch identity, state or payload"));
        }
        let record =
            journal.mark_dispatched(&record.identity.operation_id, self.session.session_id())?;
        self.exchange(journal, &record, Some(payload.to_vec()), "mobile.dispatch")
            .await
    }
    pub async fn reconcile_pending(
        &mut self,
        journal: &mut MobileJournal,
    ) -> Result<MutationRecord, DeliveryError> {
        let record = journal
            .pending()
            .cloned()
            .ok_or_else(|| refuse("no unresolved mutation"))?;
        if record.state != MutationState::Unknown
            || record.dispatched_session_id.as_deref() == Some(self.session.session_id())
        {
            return Err(refuse(
                "fresh encrypted session required for reconciliation",
            ));
        }
        self.exchange(journal, &record, None, "mobile.inspect")
            .await
    }
}
