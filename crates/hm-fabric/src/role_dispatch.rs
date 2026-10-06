use crate::{
    role_store::{
        DispatchTicket, DurableRoleRegistry, RoleError, RoleKind, RoleOutput, RoleWork,
        SourceFence, WorkRecord,
    },
    roles::{RunTerminal, TransformInput, TransformRegistry},
    runtime::{digest_contract, RuntimeError, RuntimeService, WorkerRequest},
    transport::{Frame, FrameKind, TypedFailure, UnixTransport},
};
use hm_context::types::{digest_bytes, ContextError, Scope};
use serde::{Deserialize, Serialize};
fn observed_clock_ns() -> Result<i64, DispatchError> {
    let value = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| ContextError::Invalid("clock before epoch".into()))?
        .as_nanos();
    i64::try_from(value).map_err(|_| ContextError::Capacity.into())
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error(transparent)]
    Role(#[from] RoleError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Transport(#[from] TypedFailure),
}
impl From<ContextError> for DispatchError {
    fn from(e: ContextError) -> Self {
        RoleError::Context(e).into()
    }
}
impl From<serde_json::Error> for DispatchError {
    fn from(e: serde_json::Error) -> Self {
        RoleError::from(e).into()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleWireRequest {
    pub ticket: DispatchTicket,
    pub work: RoleWork,
}

pub async fn handoff(
    registry: &mut DurableRoleRegistry,
    transport: &mut UnixTransport,
    id: &str,
    source: &SourceFence,
    now_ns: i64,
) -> Result<DispatchTicket, DispatchError> {
    let record = registry.get(id)?.ok_or(ContextError::Stale)?;
    if transport.identity().scope() != registry.scope() {
        return Err(ContextError::ScopeMismatch.into());
    }
    let ticket = registry.begin(id, registry.epoch(), source, now_ns)?;
    let request = RoleWireRequest {
        ticket: ticket.clone(),
        work: record.work,
    };
    let encoded = match serde_json::to_vec(&request) {
        Ok(bytes) => bytes,
        Err(error) => {
            registry.mark_uncertain(&ticket)?;
            return Err(error.into());
        }
    };
    let mut frame = Frame::request(id, encoded);
    if request.work.deadline_ns >= 0 {
        frame.deadline_unix_ms = Some((request.work.deadline_ns as u64) / 1_000_000);
    }
    if let Err(error) = transport.send(frame).await {
        registry.mark_uncertain(&ticket)?;
        return Err(error.into());
    }
    Ok(ticket)
}
pub async fn receive_outcome(
    registry: &mut DurableRoleRegistry,
    transport: &mut UnixTransport,
    ticket: &DispatchTicket,
    source: &SourceFence,
    now_ns: i64,
) -> Result<WorkRecord, DispatchError> {
    if transport.identity().scope() != registry.scope() {
        return Err(ContextError::ScopeMismatch.into());
    }
    let frame = match transport.receive().await {
        Ok(frame) => frame,
        Err(error) => {
            registry.mark_uncertain(ticket)?;
            return Err(error.into());
        }
    };
    if frame.kind != FrameKind::Response || frame.correlation_id != ticket.id {
        registry.mark_uncertain(ticket)?;
        return Err(ContextError::Conflict.into());
    }
    let output: RoleOutput = match serde_json::from_slice(&frame.payload) {
        Ok(output) => output,
        Err(error) => {
            registry.mark_uncertain(ticket)?;
            return Err(error.into());
        }
    };
    match registry.complete(ticket, source, output, now_ns.max(observed_clock_ns()?)) {
        Ok(record) => Ok(record),
        Err(error) => {
            registry.mark_uncertain(ticket)?;
            Err(error.into())
        }
    }
}
pub async fn dispatch_run(
    registry: &mut DurableRoleRegistry,
    transport: &mut UnixTransport,
    id: &str,
    source: &SourceFence,
    dispatch_ns: i64,
    observed_ns: i64,
) -> Result<WorkRecord, DispatchError> {
    let record = registry.get(id)?.ok_or(ContextError::Stale)?;
    if record.descriptor.kind != RoleKind::Runner {
        return Err(RoleError::Unsupported("runner role required".into()).into());
    }
    let ticket = handoff(registry, transport, id, source, dispatch_ns).await?;
    receive_outcome(registry, transport, &ticket, source, observed_ns).await
}
pub async fn dispatch_tool(
    registry: &mut DurableRoleRegistry,
    runtime: &mut RuntimeService,
    id: &str,
    source: &SourceFence,
    dispatch_ns: i64,
    observed_ns: i64,
) -> Result<WorkRecord, DispatchError> {
    let record = registry.get(id)?.ok_or(ContextError::Stale)?;
    if record.descriptor.kind != RoleKind::Tool
        || record.descriptor.id != "digest"
        || record.descriptor.pin != digest_contract().pin
    {
        return Err(RoleError::Unsupported(
            "embedded tool dispatch supports the declared digest contract".into(),
        )
        .into());
    }
    let ticket = registry.begin(id, registry.epoch(), source, dispatch_ns)?;
    let mut request = WorkerRequest::digest(id, record.work.payload.clone());
    if record.work.deadline_ns >= 0 {
        request.deadline_unix_ms = Some(record.work.deadline_ns as u64 / 1_000_000)
    }
    let result = match runtime.execute(request).await {
        Ok(result) => result,
        Err(error) => {
            registry.mark_uncertain(&ticket)?;
            return Err(error.into());
        }
    };
    let payload = serde_json::to_vec(&result)?;
    let output = RoleOutput {
        terminal: RunTerminal::Completed {
            result_digest: digest_bytes(&payload),
        },
        payload,
        blocks: vec![],
        grants: record.descriptor.grants,
    };
    Ok(registry.complete(
        &ticket,
        source,
        output,
        observed_ns.max(observed_clock_ns()?),
    )?)
}
pub fn dispatch_compact(
    registry: &mut DurableRoleRegistry,
    id: &str,
    source: &SourceFence,
    replacement: &[u8],
    now_ns: i64,
) -> Result<WorkRecord, DispatchError> {
    let record = registry.get(id)?.ok_or(ContextError::Stale)?;
    if record.descriptor.kind != RoleKind::Compaction {
        return Err(RoleError::Unsupported("compaction role required".into()).into());
    }
    let ticket = registry.begin(id, registry.epoch(), source, now_ns)?;
    let output = RoleOutput {
        terminal: RunTerminal::Completed {
            result_digest: digest_bytes(replacement),
        },
        payload: replacement.to_vec(),
        blocks: vec![],
        grants: record.descriptor.grants,
    };
    match registry.complete(&ticket, source, output, now_ns) {
        Ok(record) => Ok(record),
        Err(error) => {
            registry.mark_uncertain(&ticket)?;
            Err(error.into())
        }
    }
}
pub fn dispatch_transform(
    registry: &mut DurableRoleRegistry,
    id: &str,
    source: &SourceFence,
    scope: Scope,
    now_ns: i64,
) -> Result<WorkRecord, DispatchError> {
    if &scope != registry.scope() {
        return Err(ContextError::ScopeMismatch.into());
    }
    let record = registry.get(id)?.ok_or(ContextError::Stale)?;
    if record.descriptor.kind != RoleKind::Transform {
        return Err(RoleError::Unsupported("transform role required".into()).into());
    }
    let mut selector = TransformRegistry::default();
    selector.register(
        record
            .descriptor
            .transform
            .clone()
            .ok_or(ContextError::Stale)?,
    )?;
    let selected = selector.apply(
        &record.descriptor.id,
        &record.descriptor.pin,
        TransformInput {
            scope,
            hook: record.work.hook.clone(),
            blocks: record.work.blocks.clone(),
        },
    )?;
    let payload = serde_json::to_vec(&selected.blocks)?;
    let output = RoleOutput {
        terminal: RunTerminal::Completed {
            result_digest: digest_bytes(&payload),
        },
        payload,
        blocks: selected.blocks,
        grants: record.descriptor.grants,
    };
    let ticket = registry.begin(id, registry.epoch(), source, now_ns)?;
    match registry.complete(&ticket, source, output, now_ns) {
        Ok(record) => Ok(record),
        Err(error) => {
            registry.mark_uncertain(&ticket)?;
            Err(error.into())
        }
    }
}
