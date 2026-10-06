use hm_context::types::{digest_bytes, validate_id, Authority, ContextBlock, ContextError, Scope};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const ROLE_VERSION: u32 = 1;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoleVersion {
    pub version: u32,
    pub schema_digest: String,
    pub semantic_digest: String,
}
impl RoleVersion {
    pub fn new(schema: &[u8], semantics: &[u8]) -> Self {
        Self {
            version: ROLE_VERSION,
            schema_digest: digest_bytes(schema),
            semantic_digest: digest_bytes(semantics),
        }
    }
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.version != ROLE_VERSION
            || ![&self.schema_digest, &self.semantic_digest]
                .iter()
                .all(|d| {
                    d.len() == 64
                        && d.bytes()
                            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                })
        {
            return Err(ContextError::Invalid("invalid role pin".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolContract {
    pub name: String,
    pub pin: RoleVersion,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ToolReceipt {
    Available {
        contract: ToolContract,
    },
    Withdrawn {
        contract: ToolContract,
        reason: String,
    },
}
#[derive(Default)]
pub struct ToolRegistry {
    entries: BTreeMap<String, ToolReceipt>,
}
impl ToolRegistry {
    pub fn register(&mut self, contract: ToolContract) -> Result<(), ContextError> {
        validate_id(&contract.name)?;
        contract.pin.validate()?;
        if let Some(old) = self.entries.get(&contract.name) {
            return if old
                == &(ToolReceipt::Available {
                    contract: contract.clone(),
                }) {
                Ok(())
            } else {
                Err(ContextError::Conflict)
            };
        }
        self.entries
            .insert(contract.name.clone(), ToolReceipt::Available { contract });
        Ok(())
    }
    pub fn resolve(&self, name: &str, pin: &RoleVersion) -> Result<ToolReceipt, ContextError> {
        let receipt = self
            .entries
            .get(name)
            .ok_or_else(|| ContextError::Unavailable("unknown tool".into()))?;
        let contract = match receipt {
            ToolReceipt::Available { contract } | ToolReceipt::Withdrawn { contract, .. } => {
                contract
            }
        };
        if &contract.pin != pin {
            return Err(ContextError::Conflict);
        }
        Ok(receipt.clone())
    }
    pub fn withdraw(&mut self, name: &str, reason: String) -> Result<ToolReceipt, ContextError> {
        if reason.is_empty() {
            return Err(ContextError::Invalid("empty withdrawal reason".into()));
        }
        let old = self
            .entries
            .get(name)
            .ok_or_else(|| ContextError::Unavailable("unknown tool".into()))?
            .clone();
        match old {
            ToolReceipt::Withdrawn { .. } => Ok(old),
            ToolReceipt::Available { contract } => {
                let receipt = ToolReceipt::Withdrawn { contract, reason };
                self.entries.insert(name.into(), receipt.clone());
                Ok(receipt)
            }
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunTerminal {
    Completed { result_digest: String },
    Failed { reason: String },
    Cancelled,
}
struct RunState {
    scope: Scope,
    pin: RoleVersion,
    terminal: Option<RunTerminal>,
    deliveries: BTreeMap<String, String>,
}
#[derive(Default)]
pub struct RunRegistry {
    runs: BTreeMap<String, RunState>,
}
impl RunRegistry {
    pub fn start(
        &mut self,
        id: String,
        scope: Scope,
        pin: RoleVersion,
    ) -> Result<(), ContextError> {
        validate_id(&id)?;
        scope.validate()?;
        pin.validate()?;
        if let Some(old) = self.runs.get(&id) {
            return if old.scope == scope && old.pin == pin {
                Ok(())
            } else {
                Err(ContextError::Conflict)
            };
        }
        self.runs.insert(
            id,
            RunState {
                scope,
                pin,
                terminal: None,
                deliveries: BTreeMap::new(),
            },
        );
        Ok(())
    }
    pub fn finish(
        &mut self,
        id: &str,
        scope: &Scope,
        terminal: RunTerminal,
    ) -> Result<RunTerminal, ContextError> {
        let run = self.runs.get_mut(id).ok_or(ContextError::Stale)?;
        if &run.scope != scope {
            return Err(ContextError::ScopeMismatch);
        }
        if let Some(old) = &run.terminal {
            return if old == &terminal {
                Ok(old.clone())
            } else {
                Err(ContextError::Conflict)
            };
        }
        match &terminal {
            RunTerminal::Completed { result_digest } => {
                validate_id(result_digest)?;
            }
            RunTerminal::Failed { reason } if reason.is_empty() => {
                return Err(ContextError::Invalid("empty failure".into()))
            }
            _ => {}
        }
        run.terminal = Some(terminal.clone());
        Ok(terminal)
    }
    pub fn deliver(
        &mut self,
        id: &str,
        scope: &Scope,
        delivery_id: String,
        payload: &[u8],
    ) -> Result<bool, ContextError> {
        validate_id(&delivery_id)?;
        let run = self.runs.get_mut(id).ok_or(ContextError::Stale)?;
        if &run.scope != scope {
            return Err(ContextError::ScopeMismatch);
        }
        if run.terminal.is_none() {
            return Err(ContextError::Conflict);
        }
        let digest = digest_bytes(payload);
        if let Some(old) = run.deliveries.get(&delivery_id) {
            return if old == &digest {
                Ok(false)
            } else {
                Err(ContextError::Conflict)
            };
        }
        run.deliveries.insert(delivery_id, digest);
        Ok(true)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompactionRequest {
    pub id: String,
    pub scope: Scope,
    pub generation: u64,
    pub deadline_ns: i64,
    pub start: u64,
    pub end: u64,
    pub source_digest: String,
    pub pin: RoleVersion,
}
#[derive(Default)]
pub struct CompactionRegistry {
    pending: BTreeMap<String, CompactionRequest>,
    published: BTreeMap<String, String>,
}
impl CompactionRegistry {
    pub fn request(&mut self, request: CompactionRequest, now_ns: i64) -> Result<(), ContextError> {
        validate_id(&request.id)?;
        request.scope.validate()?;
        request.pin.validate()?;
        validate_id(&request.source_digest)?;
        if request.start >= request.end || now_ns >= request.deadline_ns {
            return Err(ContextError::Stale);
        }
        if let Some(old) = self.pending.get(&request.id) {
            return if old == &request {
                Ok(())
            } else {
                Err(ContextError::Conflict)
            };
        }
        if self.pending.values().any(|old| {
            old.scope == request.scope
                && old.generation == request.generation
                && old.start < request.end
                && request.start < old.end
        }) {
            return Err(ContextError::Conflict);
        }
        self.pending.insert(request.id.clone(), request);
        Ok(())
    }
    pub fn publish(
        &mut self,
        request: &CompactionRequest,
        current_generation: u64,
        current_source_digest: &str,
        now_ns: i64,
        replacement: &[u8],
    ) -> Result<bool, ContextError> {
        if self.pending.get(&request.id) != Some(request)
            || request.generation != current_generation
            || request.source_digest != current_source_digest
            || now_ns >= request.deadline_ns
        {
            return Err(ContextError::Stale);
        }
        let digest = digest_bytes(replacement);
        if let Some(old) = self.published.get(&request.id) {
            return if old == &digest {
                Ok(false)
            } else {
                Err(ContextError::Conflict)
            };
        }
        self.published.insert(request.id.clone(), digest);
        Ok(true)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransformDeclaration {
    pub id: String,
    pub pin: RoleVersion,
    pub authorities: Vec<Authority>,
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub hooks: BTreeSet<String>,
    pub may_reorder: bool,
}
#[derive(Clone, Debug)]
pub struct TransformInput {
    pub scope: Scope,
    pub hook: String,
    pub blocks: Vec<ContextBlock>,
}
#[derive(Clone, Debug)]
pub struct TransformOutput {
    pub blocks: Vec<ContextBlock>,
}
#[derive(Default)]
pub struct TransformRegistry {
    declarations: BTreeMap<String, TransformDeclaration>,
}
impl TransformRegistry {
    pub fn register(&mut self, declaration: TransformDeclaration) -> Result<(), ContextError> {
        validate_id(&declaration.id)?;
        declaration.pin.validate()?;
        if declaration.authorities.is_empty()
            || declaration.hooks.is_empty()
            || declaration.max_input_tokens == 0
            || declaration.max_output_tokens == 0
        {
            return Err(ContextError::Invalid("empty transform bounds".into()));
        }
        for hook in &declaration.hooks {
            validate_id(hook)?;
        }
        if let Some(old) = self.declarations.get(&declaration.id) {
            return if old == &declaration {
                Ok(())
            } else {
                Err(ContextError::Conflict)
            };
        }
        self.declarations
            .insert(declaration.id.clone(), declaration);
        Ok(())
    }
    pub fn validate_output(
        &self,
        id: &str,
        pin: &RoleVersion,
        input: &TransformInput,
        output: &TransformOutput,
    ) -> Result<(), ContextError> {
        input.scope.validate()?;
        let d = self.declarations.get(id).ok_or(ContextError::Stale)?;
        if &d.pin != pin || !d.hooks.contains(&input.hook) {
            return Err(ContextError::Conflict);
        }
        let count = |blocks: &[ContextBlock]| {
            blocks.iter().try_fold(0u64, |n, b| {
                n.checked_add(b.tokens).ok_or(ContextError::Capacity)
            })
        };
        if count(&input.blocks)? > d.max_input_tokens
            || count(&output.blocks)? > d.max_output_tokens
        {
            return Err(ContextError::Capacity);
        }
        let mut seen = BTreeSet::new();
        let mut previous = None;
        for block in &output.blocks {
            if !seen.insert(&block.id) {
                return Err(ContextError::Conflict);
            }
            let index = input
                .blocks
                .iter()
                .position(|b| b.id == block.id)
                .ok_or(ContextError::Conflict)?;
            let source = &input.blocks[index];
            if !d.authorities.contains(&source.authority)
                || block.provenance != source.provenance
                || block.required != source.required
            {
                return Err(ContextError::Conflict);
            }
            if block.authority != source.authority && block.authority != Authority::DerivedInference
            {
                return Err(ContextError::Invalid("authority elevation".into()));
            }
            if block.text != source.text && block.authority != Authority::DerivedInference {
                return Err(ContextError::Invalid(
                    "changed text requires derived authority".into(),
                ));
            }
            if !d.may_reorder && previous.is_some_and(|old| index <= old) {
                return Err(ContextError::Conflict);
            }
            previous = Some(index);
        }
        if input
            .blocks
            .iter()
            .any(|b| b.required && !seen.contains(&b.id))
        {
            return Err(ContextError::Invalid("required block omitted".into()));
        }
        Ok(())
    }
    pub fn apply(
        &self,
        id: &str,
        pin: &RoleVersion,
        input: TransformInput,
    ) -> Result<TransformOutput, ContextError> {
        input.scope.validate()?;
        let d = self.declarations.get(id).ok_or(ContextError::Stale)?;
        if &d.pin != pin || !d.hooks.contains(&input.hook) {
            return Err(ContextError::Conflict);
        }
        let total = input.blocks.iter().try_fold(0u64, |n, b| {
            n.checked_add(b.tokens).ok_or(ContextError::Capacity)
        })?;
        if total > d.max_input_tokens {
            return Err(ContextError::Capacity);
        }
        let mut blocks = Vec::new();
        let mut used = 0u64;
        let mut ids = BTreeSet::new();
        for block in input.blocks {
            if !ids.insert(block.id.clone()) {
                return Err(ContextError::Conflict);
            }
            if !d.authorities.contains(&block.authority) {
                if block.required {
                    return Err(ContextError::Invalid(
                        "required authority outside declaration".into(),
                    ));
                }
                continue;
            }
            let next = used
                .checked_add(block.tokens)
                .ok_or(ContextError::Capacity)?;
            if next > d.max_output_tokens {
                if block.required {
                    return Err(ContextError::Capacity);
                }
                continue;
            }
            used = next;
            blocks.push(block);
        }
        Ok(TransformOutput { blocks })
    }
}
