use crate::types::{
    digest_bytes, validate_id, ContextBlock, ContextError, MessagePart, Omission, SourceMessage,
    SourceSpan, TokenBudget,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummaryTier {
    Detailed,
    Condensed,
    Brief,
    Outline,
}
impl SummaryTier {
    fn index(self) -> usize {
        self as usize
    }
    fn at(index: usize) -> Self {
        [Self::Detailed, Self::Condensed, Self::Brief, Self::Outline][index]
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReductionItem {
    pub original: ContextBlock,
    pub summaries: [Option<ContextBlock>; 4],
    pub importance: u16,
    pub age: u64,
    pub current_work: bool,
    pub dependencies: Vec<String>,
}
impl ReductionItem {
    pub fn original(block: ContextBlock) -> Self {
        Self {
            original: block,
            summaries: [None, None, None, None],
            importance: 0,
            age: 0,
            current_work: false,
            dependencies: Vec::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReductionPolicy {
    pub half_life: u64,
}
impl Default for ReductionPolicy {
    fn default() -> Self {
        Self { half_life: 100 }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureBand {
    Low,
    Moderate,
    High,
    Critical,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Selection {
    pub id: String,
    pub tier: Option<SummaryTier>,
    pub protected: bool,
    pub recovery: Vec<SourceSpan>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReductionRequest {
    pub id: String,
    pub original_digest: String,
    pub tier: SummaryTier,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReductionPlan {
    pub blocks: Vec<ContextBlock>,
    pub selections: Vec<Selection>,
    pub omitted: Vec<Omission>,
    pub tokens: u64,
    pub pressure: PressureBand,
    pub pending: Vec<ReductionRequest>,
    pub recovery: BTreeMap<String, Vec<SourceSpan>>,
}
#[derive(Debug, thiserror::Error)]
pub enum ReductionError {
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error("protected context requires {required} tokens but only {available} are available")]
    Overflow { required: u64, available: u64 },
    #[error("invalid reduction: {0}")]
    Invalid(String),
    #[error("stale reduction")]
    Stale,
}
fn fingerprint(block: &ContextBlock) -> Result<String, ReductionError> {
    Ok(digest_bytes(
        &serde_json::to_vec(block).map_err(ContextError::from)?,
    ))
}
fn validate_item(item: &ReductionItem) -> Result<(), ReductionError> {
    validate_id(&item.original.id)?;
    if (item.summaries.iter().any(Option::is_some) && item.original.provenance.is_empty())
        || item.original.tokens == 0
        || item
            .original
            .provenance
            .iter()
            .any(|s| s.byte_end < s.byte_start || s.source_digest.is_empty())
    {
        return Err(ReductionError::Invalid(
            "invalid original or recovery range".into(),
        ));
    }
    let mut previous = item.original.tokens;
    for block in item.summaries.iter().flatten() {
        if block.id != item.original.id
            || block.tokens == 0
            || block.tokens > previous
            || block.provenance != item.original.provenance
            || block.required != item.original.required
            || block.authority != crate::types::Authority::DerivedInference
        {
            return Err(ReductionError::Invalid(
                "summary identity, authority, coverage or size".into(),
            ));
        }
        previous = block.tokens;
    }
    Ok(())
}
pub fn select(
    items: &[ReductionItem],
    budget: TokenBudget,
    policy: ReductionPolicy,
) -> Result<ReductionPlan, ReductionError> {
    if policy.half_life == 0 {
        return Err(ReductionError::Invalid("zero half life".into()));
    }
    let available = budget.available()?;
    let mut ids = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        validate_item(item)?;
        if ids.insert(item.original.id.clone(), index).is_some() {
            return Err(ReductionError::Invalid("duplicate item".into()));
        }
    }
    let mut protected: BTreeSet<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, i)| i.original.required || i.current_work)
        .map(|(n, _)| n)
        .collect();
    for item in items {
        for dependency in &item.dependencies {
            if !ids.contains_key(dependency) {
                return Err(ReductionError::Invalid("missing dependency".into()));
            }
        }
    }
    loop {
        let before = protected.len();
        for index in protected.clone() {
            for dependency in &items[index].dependencies {
                protected.insert(ids[dependency]);
            }
        }
        if before == protected.len() {
            break;
        }
    }
    let total = items.iter().try_fold(0u64, |sum, i| {
        sum.checked_add(i.original.tokens)
            .ok_or(ContextError::Capacity)
    })?;
    let required = protected.iter().try_fold(0u64, |sum, index| {
        sum.checked_add(items[*index].original.tokens)
            .ok_or(ContextError::Capacity)
    })?;
    if required > available {
        return Err(ReductionError::Overflow {
            required,
            available,
        });
    }
    let ratio = if available == 0 {
        u128::MAX
    } else {
        total as u128 * 100 / available as u128
    };
    let pressure = if ratio <= 70 {
        PressureBand::Low
    } else if ratio <= 90 {
        PressureBand::Moderate
    } else if ratio <= 110 {
        PressureBand::High
    } else {
        PressureBand::Critical
    };
    let mut order: Vec<usize> = (0..items.len())
        .filter(|i| !protected.contains(i))
        .collect();
    order.sort_by(|a, b| {
        let score = |i: usize| {
            (items[i].importance as u128 + 1) * policy.half_life as u128 * 1_000_000
                / (policy.half_life as u128 + items[i].age as u128)
        };
        score(*a)
            .cmp(&score(*b))
            .then_with(|| items[*b].age.cmp(&items[*a].age))
            .then_with(|| items[*a].original.id.cmp(&items[*b].original.id))
    });
    let mut chosen: Vec<Option<usize>> = vec![Some(0); items.len()];
    let mut tokens = total;
    let mut pending = Vec::new();
    for tier in 0..4 {
        for &index in &order {
            if tokens <= available {
                break;
            }
            let item = &items[index];
            let current = chosen[index].unwrap_or(0);
            if let Some(summary) = &item.summaries[tier] {
                let old = if current == 0 {
                    item.original.tokens
                } else {
                    item.summaries[current - 1].as_ref().unwrap().tokens
                };
                if summary.tokens < old {
                    tokens -= old - summary.tokens;
                    chosen[index] = Some(tier + 1);
                }
            } else {
                pending.push(ReductionRequest {
                    id: item.original.id.clone(),
                    original_digest: fingerprint(&item.original)?,
                    tier: SummaryTier::at(tier),
                });
            }
        }
    }
    // Connected evidence is admitted or omitted together, including dependency cycles.
    for &index in &order {
        if tokens <= available {
            break;
        }
        let mut group = BTreeSet::from([index]);
        loop {
            let before = group.len();
            for (other, item) in items.iter().enumerate() {
                if group.contains(&other)
                    || item.dependencies.iter().any(|d| group.contains(&ids[d]))
                {
                    group.insert(other);
                    for d in &item.dependencies {
                        group.insert(ids[d]);
                    }
                }
            }
            if before == group.len() {
                break;
            }
        }
        if group.iter().any(|i| protected.contains(i)) {
            continue;
        }
        for member in group {
            if let Some(tier) = chosen[member].take() {
                tokens -= if tier == 0 {
                    items[member].original.tokens
                } else {
                    items[member].summaries[tier - 1].as_ref().unwrap().tokens
                };
            }
        }
    }
    if tokens > available {
        return Err(ReductionError::Overflow {
            required: tokens,
            available,
        });
    }
    let mut plan = ReductionPlan {
        blocks: Vec::new(),
        selections: Vec::new(),
        omitted: Vec::new(),
        tokens,
        pressure,
        pending,
        recovery: items
            .iter()
            .map(|item| (item.original.id.clone(), item.original.provenance.clone()))
            .collect(),
    };
    for (index, item) in items.iter().enumerate() {
        if let Some(tier) = chosen[index] {
            plan.blocks.push(if tier == 0 {
                item.original.clone()
            } else {
                item.summaries[tier - 1].as_ref().unwrap().clone()
            });
            plan.selections.push(Selection {
                id: item.original.id.clone(),
                tier: if tier == 0 {
                    None
                } else {
                    Some(SummaryTier::at(tier - 1))
                },
                protected: protected.contains(&index),
                recovery: item.original.provenance.clone(),
            });
        } else {
            plan.omitted.push(Omission {
                id: item.original.id.clone(),
                reason: "context pressure; original remains recoverable".into(),
            });
        }
    }
    Ok(plan)
}

pub fn expand(
    reference: &SourceSpan,
    source_id: &str,
    source: &[u8],
) -> Result<Vec<u8>, ReductionError> {
    if reference.source_id != source_id || reference.source_digest != digest_bytes(source) {
        return Err(ReductionError::Stale);
    }
    let start = usize::try_from(reference.byte_start)
        .map_err(|_| ReductionError::Invalid("range".into()))?;
    let end =
        usize::try_from(reference.byte_end).map_err(|_| ReductionError::Invalid("range".into()))?;
    source
        .get(start..end)
        .map(|b| b.to_vec())
        .ok_or_else(|| ReductionError::Invalid("range".into()))
}

pub fn expand_history(
    history: &crate::history::SourceHistory,
    scope: &crate::types::Scope,
    reference: &SourceSpan,
) -> Result<Vec<u8>, ReductionError> {
    Ok(history.recover(scope, reference)?)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReductionQueue {
    requests: BTreeMap<String, ReductionRequest>,
    completed: BTreeMap<String, String>,
}
impl ReductionQueue {
    fn key(request: &ReductionRequest) -> Result<String, ReductionError> {
        Ok(digest_bytes(
            &serde_json::to_vec(request).map_err(ContextError::from)?,
        ))
    }
    pub fn enqueue(&mut self, request: ReductionRequest) -> Result<bool, ReductionError> {
        validate_id(&request.id)?;
        if request.original_digest.is_empty() {
            return Err(ReductionError::Invalid("missing original digest".into()));
        }
        let key = Self::key(&request)?;
        if self.completed.contains_key(&key) || self.requests.contains_key(&key) {
            return Ok(false);
        }
        self.requests.insert(key, request);
        Ok(true)
    }
    pub fn pending(&self) -> Vec<ReductionRequest> {
        self.requests.values().cloned().collect()
    }
    pub fn apply(
        &mut self,
        request: &ReductionRequest,
        item: &mut ReductionItem,
        summary: ContextBlock,
    ) -> Result<bool, ReductionError> {
        let key = Self::key(request)?;
        let digest = fingerprint(&summary)?;
        if request.id != item.original.id || request.original_digest != fingerprint(&item.original)?
        {
            return Err(ReductionError::Stale);
        }
        if let Some(previous) = self.completed.get(&key) {
            return if previous == &digest {
                Ok(false)
            } else {
                Err(ReductionError::Stale)
            };
        }
        if self.requests.get(&key) != Some(request) {
            return Err(ReductionError::Stale);
        }
        let mut candidate = item.clone();
        candidate.summaries[request.tier.index()] = Some(summary);
        validate_item(&candidate)?;
        *item = candidate;
        self.requests.remove(&key);
        self.completed.insert(key, digest);
        Ok(true)
    }
}

pub fn protect_tool_dependencies(
    items: &mut [ReductionItem],
    messages: &[SourceMessage],
) -> Result<(), ReductionError> {
    let indices: BTreeMap<String, usize> = items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.original.id.clone(), i))
        .collect();
    let mut open: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for message in messages {
        message.validate()?;
        let index = *indices
            .get(&message.id)
            .ok_or_else(|| ReductionError::Invalid("missing tool message".into()))?;
        for part in &message.parts {
            match part {
                MessagePart::ToolCall { call_id, .. } => {
                    open.entry(call_id.clone()).or_default().push(index)
                }
                MessagePart::ToolResult { call_id, .. } => {
                    let call = open
                        .get_mut(call_id)
                        .and_then(|calls| calls.pop())
                        .ok_or_else(|| ReductionError::Invalid("unpaired tool result".into()))?;
                    if call != index {
                        let call_id = items[call].original.id.clone();
                        let result_id = items[index].original.id.clone();
                        if !items[index].dependencies.contains(&call_id) {
                            items[index].dependencies.push(call_id);
                        }
                        if !items[call].dependencies.contains(&result_id) {
                            items[call].dependencies.push(result_id);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    for index in open.values().flatten() {
        items[*index].current_work = true;
    }
    Ok(())
}
