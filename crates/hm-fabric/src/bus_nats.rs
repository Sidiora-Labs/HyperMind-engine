use crate::bus::{BackendCapabilities, DeadLetter, Delivery, Disposition, Event, Register};
use crate::bus_contract::{
    validate_token, BusTopology, RegisterSnapshot, RegisterUpdate, StreamFamily,
};
use async_nats::{Client, HeaderMap};
use base64::Engine;
use hm_context::{ContextError, Scope};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NatsCredentials {
    pub username: String,
    pub password: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NatsBinding {
    pub scope: Scope,
    pub family: StreamFamily,
    pub stream: String,
    pub consumer: String,
    pub publish_subject: String,
    pub dead_subject: String,
    pub dead_stream: String,
    pub inbox_prefix: String,
    pub register_bucket: String,
    pub max_attempts: u32,
    pub max_scan: u32,
    pub max_inflight: usize,
    pub ack_wait_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatsReplay {
    pub events: Vec<Event>,
    pub gaps: Vec<u64>,
    pub cursor: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatsRegisterReplay {
    pub updates: Vec<RegisterUpdate>,
    pub gaps: Vec<u64>,
    pub cursor: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatsCapabilityMatrix {
    pub preprovisioned_consumers_only: bool,
    pub native_register_cas: bool,
    pub bounded_gap_replay: bool,
    pub private_inboxes: bool,
    pub replicas: u64,
    pub broker_leaf_qualified: bool,
    pub broker_system_account_qualified: bool,
    pub cross_process_late_ack_fence: bool,
}
#[derive(Clone)]
pub struct NatsBus {
    client: Client,
    binding: NatsBinding,
    flights: Arc<Mutex<BTreeMap<(u64, u32), Flight>>>,
    pull: Arc<Mutex<()>>,
}
struct Flight {
    delivery: Delivery,
    reply: String,
    deadline: Instant,
}
struct Stored {
    sequence: u64,
    subject: String,
    payload: Vec<u8>,
    headers: BTreeMap<String, String>,
}
fn unavailable() -> ContextError {
    ContextError::Unavailable("NATS operation unavailable or denied".into())
}
fn validate_digest(value: &[u8]) -> Result<(), ContextError> {
    if value.len() != 64
        || !value
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err(ContextError::Invalid(
            "expected lowercase SHA256 reference".into(),
        ));
    }
    Ok(())
}
fn number(value: &Value, key: &str) -> Result<u64, ContextError> {
    value[key]
        .as_u64()
        .ok_or_else(|| ContextError::Invalid("invalid broker response".into()))
}
fn subject_valid(subject: &str, root: &str, family: StreamFamily) -> Result<(), ContextError> {
    let prefix = format!("{root}.{}.", family.token());
    let suffix = subject
        .strip_prefix(&prefix)
        .ok_or(ContextError::ScopeMismatch)?;
    let parts: Vec<_> = suffix.split('.').collect();
    if parts.len() != 2 {
        return Err(ContextError::Invalid("invalid event subject".into()));
    }
    for p in parts {
        validate_token(p)?;
    }
    Ok(())
}
impl NatsBus {
    pub async fn connect(
        url: &str,
        binding: NatsBinding,
        credentials: NatsCredentials,
    ) -> Result<Self, ContextError> {
        let topology = BusTopology::new(binding.scope.clone())?;
        if binding.stream != topology.stream_name(binding.family)
            || binding.dead_stream != topology.stream_name(StreamFamily::DeadLetterEffects)
            || binding.register_bucket != topology.census_bucket
        {
            return Err(ContextError::ScopeMismatch);
        }
        subject_valid(&binding.publish_subject, &topology.root, binding.family)?;
        subject_valid(
            &binding.dead_subject,
            &topology.root,
            StreamFamily::DeadLetterEffects,
        )?;
        if binding.inbox_prefix.is_empty()
            || binding.inbox_prefix.len() > 256
            || binding.inbox_prefix.split('.').any(|p| {
                p.is_empty()
                    || !p
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            })
            || binding.max_attempts == 0
            || binding.max_attempts > 1000
            || binding.max_scan == 0
            || binding.max_scan > 10000
            || binding.max_inflight == 0
            || binding.max_inflight > 1000
            || binding.ack_wait_ms == 0
            || binding.ack_wait_ms > 60000
        {
            return Err(ContextError::Invalid("invalid NATS binding bounds".into()));
        }
        if binding.consumer.is_empty()
            || binding.consumer.len() > 400
            || !binding
                .consumer
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(ContextError::Invalid(
                "invalid preprovisioned consumer".into(),
            ));
        }
        let client = async_nats::ConnectOptions::new()
            .user_and_password(credentials.username, credentials.password)
            .custom_inbox_prefix(binding.inbox_prefix.clone())
            .request_timeout(Some(Duration::from_secs(2)))
            .connection_timeout(Duration::from_secs(3))
            .connect(url)
            .await
            .map_err(|_| unavailable())?;
        let bus = Self {
            client,
            binding,
            flights: Arc::new(Mutex::new(BTreeMap::new())),
            pull: Arc::new(Mutex::new(())),
        };
        bus.verify_binding(&bus.binding.stream, bus.binding.family)
            .await?;
        bus.verify_binding(
            &format!("KV_{}", bus.binding.register_bucket),
            bus.binding.family,
        )
        .await?;
        let info = bus
            .api(
                &format!(
                    "$JS.API.CONSUMER.INFO.{}.{}",
                    bus.binding.stream, bus.binding.consumer
                ),
                json!({}),
            )
            .await?;
        let config = &info["config"];
        if config["ack_policy"] != "explicit"
            || config["max_deliver"].as_i64() != Some(-1)
            || config["ack_wait"].as_u64() != Some(bus.binding.ack_wait_ms * 1_000_000)
            || config["filter_subject"] != topology.stream_subject(bus.binding.family)
            || config["max_ack_pending"].as_u64() != Some(bus.binding.max_inflight as u64)
        {
            return Err(ContextError::Conflict);
        }
        Ok(bus)
    }
    async fn verify_binding(&self, stream: &str, family: StreamFamily) -> Result<(), ContextError> {
        let info = self
            .api(&format!("$JS.API.STREAM.INFO.{stream}"), json!({}))
            .await?;
        if info["config"]["metadata"]["scope_digest"].as_str()
            != Some(self.binding.scope.digest()?.as_str())
            || info["config"]["storage"] != "file"
            || info["config"]["num_replicas"].as_u64() != Some(1)
        {
            return Err(ContextError::ScopeMismatch);
        }
        let expected = if stream.starts_with("KV_") {
            format!("$KV.{}.>", self.binding.register_bucket)
        } else {
            BusTopology::new(self.binding.scope.clone())?.stream_subject(family)
        };
        if info["config"]["subjects"] != json!([expected]) {
            return Err(ContextError::Conflict);
        }
        Ok(())
    }
    async fn api(&self, subject: &str, body: Value) -> Result<Value, ContextError> {
        let response = self
            .client
            .request(subject.to_string(), serde_json::to_vec(&body)?.into())
            .await
            .map_err(|_| unavailable())?;
        if response.status.is_some_and(|s| s.as_u16() >= 400) {
            return Err(unavailable());
        }
        let value: Value = serde_json::from_slice(&response.payload)?;
        if let Some(error) = value.get("error") {
            let code = error["code"].as_u64().unwrap_or(0);
            let kind = error["err_code"].as_u64().unwrap_or(0);
            return Err(if code == 404 {
                ContextError::Stale
            } else if code == 409 || kind == 10071 {
                ContextError::Conflict
            } else {
                unavailable()
            });
        }
        Ok(value)
    }
    async fn stored(&self, stream: &str, query: Value) -> Result<Option<Stored>, ContextError> {
        let value = match self
            .api(&format!("$JS.API.STREAM.MSG.GET.{stream}"), query)
            .await
        {
            Ok(v) => v,
            Err(ContextError::Stale) => return Ok(None),
            Err(e) => return Err(e),
        };
        let message = &value["message"];
        let sequence = number(message, "seq")?;
        let payload = base64::engine::general_purpose::STANDARD
            .decode(message["data"].as_str().unwrap_or(""))
            .map_err(|_| ContextError::Invalid("invalid broker payload".into()))?;
        let mut headers = BTreeMap::new();
        if let Some(raw) = message["hdrs"].as_str() {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(raw)
                .map_err(|_| ContextError::Invalid("invalid broker headers".into()))?;
            if bytes.len() > 8192 {
                return Err(ContextError::Capacity);
            }
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| ContextError::Invalid("invalid broker headers".into()))?;
            for line in text.lines().skip(1) {
                if let Some((key, value)) = line.split_once(':') {
                    headers.insert(key.to_string(), value.trim().to_string());
                }
            }
        }
        Ok(Some(Stored {
            sequence,
            subject: message["subject"]
                .as_str()
                .ok_or_else(|| ContextError::Invalid("missing broker subject".into()))?
                .into(),
            payload,
            headers,
        }))
    }
    pub fn binding(&self) -> &NatsBinding {
        &self.binding
    }
    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend: "nats".into(),
            durable: true,
            replay: true,
            register_cas: true,
            distributed: true,
        }
    }
    pub fn capability_matrix(&self) -> NatsCapabilityMatrix {
        NatsCapabilityMatrix {
            preprovisioned_consumers_only: true,
            native_register_cas: true,
            bounded_gap_replay: true,
            private_inboxes: true,
            replicas: 1,
            broker_leaf_qualified: false,
            broker_system_account_qualified: false,
            cross_process_late_ack_fence: false,
        }
    }
    async fn publish_ref(
        &self,
        subject: &str,
        stream: &str,
        id: &str,
        payload: &[u8],
        mut headers: HeaderMap,
    ) -> Result<Event, ContextError> {
        validate_digest(payload)?;
        hm_context::types::validate_id(id)?;
        headers.insert("Nats-Msg-Id", id);
        let response = self
            .client
            .request_with_headers(subject.to_string(), headers, payload.to_vec().into())
            .await
            .map_err(|_| unavailable())?;
        let ack: Value = serde_json::from_slice(&response.payload)?;
        if ack.get("error").is_some() {
            return Err(unavailable());
        }
        if ack["stream"].as_str() != Some(stream) {
            return Err(ContextError::ScopeMismatch);
        }
        let sequence = number(&ack, "seq")?;
        if ack["duplicate"].as_bool() == Some(true) {
            let stored = self
                .stored(stream, json!({"seq":sequence}))
                .await?
                .ok_or(ContextError::Stale)?;
            if stored.payload != payload || stored.subject != subject {
                return Err(ContextError::Conflict);
            }
        }
        Ok(Event {
            sequence,
            stream: stream.into(),
            payload: payload.into(),
        })
    }
    pub async fn append_once(&self, id: &str, digest: &[u8]) -> Result<Event, ContextError> {
        self.publish_ref(
            &self.binding.publish_subject,
            &self.binding.stream,
            id,
            digest,
            HeaderMap::new(),
        )
        .await
    }
    pub async fn replay(&self, after: u64, limit: u32) -> Result<NatsReplay, ContextError> {
        if limit == 0 || limit > self.binding.max_scan {
            return Err(ContextError::Capacity);
        }
        let info = self
            .api(
                &format!("$JS.API.STREAM.INFO.{}", self.binding.stream),
                json!({}),
            )
            .await?;
        let upper = number(&info["state"], "last_seq")?;
        if after > upper {
            return Err(ContextError::Stale);
        }
        let mut result = NatsReplay {
            events: Vec::new(),
            gaps: Vec::new(),
            cursor: after,
        };
        while result.cursor < upper && result.events.len() + result.gaps.len() < limit as usize {
            result.cursor += 1;
            match self
                .stored(&self.binding.stream, json!({"seq":result.cursor}))
                .await?
            {
                Some(value) => {
                    validate_digest(&value.payload)?;
                    result.events.push(Event {
                        sequence: value.sequence,
                        stream: self.binding.stream.clone(),
                        payload: value.payload,
                    });
                }
                None => result.gaps.push(result.cursor),
            }
        }
        Ok(result)
    }
    pub async fn next(&self) -> Result<Option<Delivery>, ContextError> {
        let _pull = self.pull.lock().await;
        let mut flights = self.flights.lock().await;
        flights.retain(|_, f| f.deadline > Instant::now());
        if flights.len() >= self.binding.max_inflight {
            return Err(ContextError::Capacity);
        }
        let response = self
            .client
            .send_request(
                format!(
                    "$JS.API.CONSUMER.MSG.NEXT.{}.{}",
                    self.binding.stream, self.binding.consumer
                ),
                async_nats::Request::new()
                    .inbox(self.client.new_inbox())
                    .payload(
                        serde_json::to_vec(&json!({"batch":1,"expires":1_000_000_000u64}))?.into(),
                    ),
            )
            .await
            .map_err(|_| unavailable())?;
        if response
            .status
            .is_some_and(|s| s.as_u16() == 408 || s.as_u16() == 404)
        {
            return Ok(None);
        }
        if response.status.is_some() {
            return Err(unavailable());
        }
        validate_digest(&response.payload)?;
        let reply = response
            .reply
            .ok_or_else(|| ContextError::Invalid("delivery lacks acknowledgment address".into()))?
            .to_string();
        let prefix = format!("$JS.ACK.{}.{}.", self.binding.stream, self.binding.consumer);
        let parts: Vec<_> = reply
            .strip_prefix(&prefix)
            .ok_or(ContextError::ScopeMismatch)?
            .split('.')
            .collect();
        if parts.len() != 5 {
            return Err(ContextError::Invalid(
                "unsupported acknowledgment address".into(),
            ));
        }
        let attempt: u32 = parts[0]
            .parse()
            .map_err(|_| ContextError::Invalid("invalid delivery attempt".into()))?;
        let sequence: u64 = parts[1]
            .parse()
            .map_err(|_| ContextError::Invalid("invalid delivery sequence".into()))?;
        let delivery = Delivery {
            event: Event {
                sequence,
                stream: self.binding.stream.clone(),
                payload: response.payload.to_vec(),
            },
            subscriber: self.binding.consumer.clone(),
            attempt,
        };
        if attempt > self.binding.max_attempts {
            self.dead_letter(&delivery).await?;
            self.ack_sync(&reply, b"+TERM").await?;
            return Ok(None);
        }
        flights.insert(
            (sequence, attempt),
            Flight {
                delivery: delivery.clone(),
                reply,
                deadline: Instant::now() + Duration::from_millis(self.binding.ack_wait_ms),
            },
        );
        Ok(Some(delivery))
    }
    async fn ack_sync(&self, reply: &str, payload: &[u8]) -> Result<(), ContextError> {
        let response = self
            .client
            .request(reply.to_string(), payload.to_vec().into())
            .await
            .map_err(|_| unavailable())?;
        if response.status.is_some() || !response.payload.is_empty() {
            return Err(unavailable());
        }
        Ok(())
    }
    async fn dead_letter(&self, delivery: &Delivery) -> Result<(), ContextError> {
        let mut headers = HeaderMap::new();
        headers.insert("Hm-Original-Stream", delivery.event.stream.as_str());
        headers.insert("Hm-Original-Sequence", delivery.event.sequence.to_string());
        headers.insert("Hm-Consumer", delivery.subscriber.as_str());
        headers.insert("Hm-Attempt", delivery.attempt.to_string());
        let id = hm_context::types::digest_bytes(
            format!(
                "{}:{}:{}",
                delivery.event.stream, delivery.event.sequence, delivery.subscriber
            )
            .as_bytes(),
        );
        self.publish_ref(
            &self.binding.dead_subject,
            &self.binding.dead_stream,
            &id,
            &delivery.event.payload,
            headers,
        )
        .await?;
        Ok(())
    }
    pub async fn disposition(
        &self,
        delivery: &Delivery,
        disposition: Disposition,
    ) -> Result<(), ContextError> {
        let mut flights = self.flights.lock().await;
        let key = (delivery.event.sequence, delivery.attempt);
        let flight = flights.get_mut(&key).ok_or(ContextError::Stale)?;
        if flight.delivery != *delivery || flight.deadline <= Instant::now() {
            return Err(ContextError::Stale);
        }
        match disposition {
            Disposition::Progress { lease_ms } => {
                if lease_ms != self.binding.ack_wait_ms as i64 {
                    return Err(ContextError::Invalid(
                        "progress must use provisioned acknowledgment interval".into(),
                    ));
                }
                self.client
                    .publish(flight.reply.clone(), "+WPI".into())
                    .await
                    .map_err(|_| unavailable())?;
                self.client.flush().await.map_err(|_| unavailable())?;
                flight.deadline = Instant::now() + Duration::from_millis(self.binding.ack_wait_ms);
                return Ok(());
            }
            Disposition::Ack => self.ack_sync(&flight.reply, b"+ACK").await?,
            Disposition::Nak => {
                if delivery.attempt >= self.binding.max_attempts {
                    self.dead_letter(delivery).await?;
                    self.ack_sync(&flight.reply, b"+TERM").await?;
                } else {
                    self.ack_sync(&flight.reply, b"-NAK").await?;
                }
            }
            Disposition::Term => {
                self.dead_letter(delivery).await?;
                self.ack_sync(&flight.reply, b"+TERM").await?;
            }
        }
        flights.remove(&key);
        Ok(())
    }
    pub async fn dead_letters(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<Vec<DeadLetter>, ContextError> {
        if limit == 0 || limit > self.binding.max_scan {
            return Err(ContextError::Capacity);
        }
        let info = self
            .api(
                &format!("$JS.API.STREAM.INFO.{}", self.binding.dead_stream),
                json!({}),
            )
            .await?;
        let upper = number(&info["state"], "last_seq")?;
        let mut result = Vec::new();
        for sequence in after.saturating_add(1)..=upper.min(after.saturating_add(limit as u64)) {
            if let Some(value) = self
                .stored(&self.binding.dead_stream, json!({"seq":sequence}))
                .await?
            {
                if value.subject != self.binding.dead_subject {
                    continue;
                }
                validate_digest(&value.payload)?;
                let sequence = value
                    .headers
                    .get("Hm-Original-Sequence")
                    .and_then(|v| v.parse().ok())
                    .ok_or(ContextError::Invalid("invalid dead letter".into()))?;
                let attempt = value
                    .headers
                    .get("Hm-Attempt")
                    .and_then(|v| v.parse().ok())
                    .ok_or(ContextError::Invalid("invalid dead letter".into()))?;
                result.push(DeadLetter {
                    delivery: Delivery {
                        event: Event {
                            sequence,
                            stream: value
                                .headers
                                .get("Hm-Original-Stream")
                                .ok_or(ContextError::Invalid("invalid dead letter".into()))?
                                .clone(),
                            payload: value.payload,
                        },
                        subscriber: value
                            .headers
                            .get("Hm-Consumer")
                            .ok_or(ContextError::Invalid("invalid dead letter".into()))?
                            .clone(),
                        attempt,
                    },
                    reason: "terminated".into(),
                });
            }
        }
        Ok(result)
    }
    fn key_subject(&self, key: &str) -> Result<String, ContextError> {
        validate_token(key)?;
        Ok(format!("$KV.{}.{}", self.binding.register_bucket, key))
    }
    pub async fn register_get(&self, key: &str) -> Result<Option<Register>, ContextError> {
        let subject = self.key_subject(key)?;
        let Some(value) = self
            .stored(
                &format!("KV_{}", self.binding.register_bucket),
                json!({"last_by_subj":subject}),
            )
            .await?
        else {
            return Ok(None);
        };
        if value
            .headers
            .get("KV-Operation")
            .is_some_and(|v| v == "DEL" || v == "PURGE")
        {
            return Ok(None);
        }
        validate_digest(&value.payload)?;
        Ok(Some(Register {
            revision: value.sequence,
            value: value.payload,
        }))
    }
    async fn register_write(
        &self,
        key: &str,
        expected: u64,
        value: &[u8],
        delete: bool,
    ) -> Result<u64, ContextError> {
        let subject = self.key_subject(key)?;
        if !delete {
            validate_digest(value)?;
        }
        let mut headers = HeaderMap::new();
        let mut expected = expected;
        if expected == 0 && !delete {
            if let Some(previous) = self
                .stored(
                    &format!("KV_{}", self.binding.register_bucket),
                    json!({"last_by_subj":subject}),
                )
                .await?
            {
                if previous
                    .headers
                    .get("KV-Operation")
                    .is_some_and(|v| v == "DEL" || v == "PURGE")
                {
                    expected = previous.sequence;
                }
            }
        }
        headers.insert("Nats-Expected-Last-Subject-Sequence", expected.to_string());
        if delete {
            headers.insert("KV-Operation", "DEL");
        }
        let response = self
            .client
            .request_with_headers(subject, headers, value.to_vec().into())
            .await
            .map_err(|_| unavailable())?;
        let ack: Value = serde_json::from_slice(&response.payload)?;
        if let Some(error) = ack.get("error") {
            let code = error["err_code"].as_u64();
            return Err(if code == Some(10071) {
                ContextError::Conflict
            } else {
                unavailable()
            });
        }
        if ack["stream"] != format!("KV_{}", self.binding.register_bucket) {
            return Err(ContextError::ScopeMismatch);
        }
        number(&ack, "seq")
    }
    pub async fn register_cas(
        &self,
        key: &str,
        expected: u64,
        value: &[u8],
    ) -> Result<Register, ContextError> {
        let revision = self.register_write(key, expected, value, false).await?;
        Ok(Register {
            revision,
            value: value.into(),
        })
    }
    pub async fn register_delete(&self, key: &str, expected: u64) -> Result<u64, ContextError> {
        self.register_write(key, expected, &[], true).await
    }
    pub async fn register_watch_after(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<NatsRegisterReplay, ContextError> {
        if limit == 0 || limit > self.binding.max_scan {
            return Err(ContextError::Capacity);
        }
        let stream = format!("KV_{}", self.binding.register_bucket);
        let info = self
            .api(&format!("$JS.API.STREAM.INFO.{stream}"), json!({}))
            .await?;
        let upper = number(&info["state"], "last_seq")?;
        if after > upper {
            return Err(ContextError::Stale);
        }
        let mut result = NatsRegisterReplay {
            updates: Vec::new(),
            gaps: Vec::new(),
            cursor: after,
        };
        while result.cursor < upper && result.updates.len() + result.gaps.len() < limit as usize {
            result.cursor += 1;
            match self.stored(&stream, json!({"seq":result.cursor})).await? {
                None => result.gaps.push(result.cursor),
                Some(value) => {
                    let prefix = format!("$KV.{}.", self.binding.register_bucket);
                    let key = value
                        .subject
                        .strip_prefix(&prefix)
                        .ok_or(ContextError::ScopeMismatch)?
                        .to_string();
                    validate_token(&key)?;
                    if value
                        .headers
                        .get("KV-Operation")
                        .is_some_and(|v| v == "DEL" || v == "PURGE")
                    {
                        result.updates.push(RegisterUpdate::Delete {
                            key,
                            revision: value.sequence,
                        });
                    } else {
                        validate_digest(&value.payload)?;
                        result.updates.push(RegisterUpdate::Put {
                            key,
                            register: Register {
                                revision: value.sequence,
                                value: value.payload,
                            },
                        });
                    }
                }
            }
        }
        Ok(result)
    }
    pub async fn register_snapshot(&self) -> Result<RegisterSnapshot, ContextError> {
        let page = self.register_watch_after(0, self.binding.max_scan).await?;
        if !page.gaps.is_empty() {
            return Err(ContextError::Stale);
        }
        let stream = format!("KV_{}", self.binding.register_bucket);
        let info = self
            .api(&format!("$JS.API.STREAM.INFO.{stream}"), json!({}))
            .await?;
        if number(&info["state"], "last_seq")? != page.cursor {
            return Err(ContextError::Stale);
        }
        let mut entries = BTreeMap::new();
        for update in page.updates {
            match update {
                RegisterUpdate::Put { key, register } => {
                    entries.insert(key, register);
                }
                RegisterUpdate::Delete { key, .. } => {
                    entries.remove(&key);
                }
            }
        }
        Ok(RegisterSnapshot {
            revision: page.cursor,
            entries: entries.into_iter().collect(),
        })
    }
}
