#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use super::gateway::DynError;
use super::pipeline::{BenchDocument, BenchQuestion, BenchRole};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const PROBE_SET_FORMAT: &str = "hypermind.beam-probe-set.v1";

const FIXTURE: &[u8] = include_bytes!("../../../../eval/fixtures/beam/probe-set.json");

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeKind {
    InformationExtraction,
    TemporalReasoning,
    MultiSessionReasoning,
    ContradictionResolution,
    EventOrdering,
    KnowledgeUpdate,
    Summarization,
    Abstention,
    PreferenceFollowing,
    InstructionFollowing,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: u64,
    pub role: BenchRole,
    pub speaker: String,
    pub text: String,
    pub occurred_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Session {
    pub index: usize,
    pub label: String,
    pub messages: Vec<Message>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Probe {
    pub id: String,
    pub kind: ProbeKind,
    pub question: String,
    pub reference_answer: String,
    pub criteria: Vec<String>,
    pub difficulty: String,
    pub evidence_message_ids: Vec<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Conversation {
    pub id: String,
    pub sessions: Vec<Session>,
    pub probes: Vec<Probe>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProbeSet {
    pub format: String,
    pub source_digest: String,
    pub conversations: Vec<Conversation>,
}

pub fn parse_probe_set(bytes: &[u8]) -> Result<ProbeSet, DynError> {
    let set: ProbeSet = serde_json::from_slice(bytes)?;
    validate(&set)?;
    Ok(set)
}

pub fn load_probe_set(path: &Path) -> Result<ProbeSet, DynError> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("probe set {}: {error}", path.display()))?;
    parse_probe_set(&bytes).map_err(|error| format!("probe set {}: {error}", path.display()).into())
}

pub fn fixture_probe_set() -> Result<ProbeSet, DynError> {
    parse_probe_set(FIXTURE)
}

pub fn probe_set_digest(set: &ProbeSet) -> Result<String, DynError> {
    Ok(blake3::hash(&serde_json::to_vec(set)?).to_hex().to_string())
}

pub fn adapt_published_artifact(bytes: &[u8]) -> Result<ProbeSet, DynError> {
    let rows: Vec<ArtifactRow> = serde_json::from_slice(bytes)
        .map_err(|error| format!("published artifact rows: {error}"))?;
    let mut conversations = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        if let Some(conversation) = adapt_row(index, row)? {
            conversations.push(conversation);
        }
    }
    if conversations.is_empty() {
        return Err(
            "published artifact yielded no conversation with gradable probes: a probe is kept only when it carries a question, a reference answer, grading criteria and, unless it is an abstention probe, evidence message ids that resolve inside its conversation"
                .into(),
        );
    }
    let set = ProbeSet {
        format: PROBE_SET_FORMAT.to_owned(),
        source_digest: blake3::hash(bytes).to_hex().to_string(),
        conversations,
    };
    validate(&set)?;
    Ok(set)
}

#[must_use]
pub fn to_documents(conversation: &Conversation) -> Vec<BenchDocument> {
    let mut documents = Vec::new();
    for session in &conversation.sessions {
        for message in &session.messages {
            documents.push(BenchDocument {
                id: document_id(&conversation.id, message.id),
                conversation: conversation.id.clone(),
                date: message.occurred_at.clone(),
                role: message.role,
                speaker: message.speaker.clone(),
                text: message.text.clone(),
            });
        }
    }
    documents
}

#[must_use]
pub fn to_question(conversation: &Conversation, probe: &Probe) -> BenchQuestion {
    let mut date = None;
    for session in &conversation.sessions {
        for message in &session.messages {
            if !message.occurred_at.trim().is_empty() {
                date = Some(message.occurred_at.clone());
            }
        }
    }
    BenchQuestion {
        id: probe.id.clone(),
        text: probe.question.clone(),
        date,
    }
}

#[must_use]
pub fn evidence_document_ids(conversation: &Conversation, probe: &Probe) -> Vec<String> {
    probe
        .evidence_message_ids
        .iter()
        .map(|message| document_id(&conversation.id, *message))
        .collect()
}

fn document_id(conversation: &str, message: u64) -> String {
    format!("{conversation}:{message}")
}

fn validate(set: &ProbeSet) -> Result<(), DynError> {
    if set.format != PROBE_SET_FORMAT {
        return Err(format!(
            "probe set format tag is {} and not {PROBE_SET_FORMAT}",
            set.format
        )
        .into());
    }
    if set.conversations.is_empty() {
        return Err("probe set carries no conversations".into());
    }
    let mut conversation_ids = BTreeSet::new();
    let mut probe_ids = BTreeSet::new();
    for conversation in &set.conversations {
        if conversation.id.trim().is_empty() {
            return Err("a conversation carries no identity".into());
        }
        if !conversation_ids.insert(conversation.id.as_str()) {
            return Err(format!(
                "conversation id {} appears more than once in the probe set",
                conversation.id
            )
            .into());
        }
        let message_ids = validate_messages(conversation)?;
        if conversation.probes.is_empty() {
            return Err(format!("conversation {} carries no probes", conversation.id).into());
        }
        for probe in &conversation.probes {
            if !probe_ids.insert(probe.id.as_str()) {
                return Err(format!(
                    "probe id {} appears more than once in the probe set",
                    probe.id
                )
                .into());
            }
            validate_probe(conversation, probe, &message_ids)?;
        }
    }
    Ok(())
}

fn validate_messages(conversation: &Conversation) -> Result<BTreeSet<u64>, DynError> {
    if conversation.sessions.is_empty() {
        return Err(format!("conversation {} carries no sessions", conversation.id).into());
    }
    let mut message_ids = BTreeSet::new();
    for session in &conversation.sessions {
        if session.messages.is_empty() {
            return Err(format!(
                "conversation {} session {} carries no messages",
                conversation.id, session.index
            )
            .into());
        }
        for message in &session.messages {
            if message.text.trim().is_empty() {
                return Err(format!(
                    "conversation {} message {} carries no text",
                    conversation.id, message.id
                )
                .into());
            }
            if !message_ids.insert(message.id) {
                return Err(format!(
                    "conversation {} repeats message id {}",
                    conversation.id, message.id
                )
                .into());
            }
        }
    }
    Ok(message_ids)
}

fn validate_probe(
    conversation: &Conversation,
    probe: &Probe,
    message_ids: &BTreeSet<u64>,
) -> Result<(), DynError> {
    if probe.id.trim().is_empty() {
        return Err(format!(
            "conversation {} carries a probe without an identity",
            conversation.id
        )
        .into());
    }
    if probe.question.trim().is_empty() {
        return Err(format!("probe {} carries no question", probe.id).into());
    }
    if probe.reference_answer.trim().is_empty() {
        return Err(format!("probe {} carries no reference answer", probe.id).into());
    }
    if probe.criteria.is_empty()
        || probe
            .criteria
            .iter()
            .any(|criterion| criterion.trim().is_empty())
    {
        return Err(format!("probe {} carries no grading criteria", probe.id).into());
    }
    for message in &probe.evidence_message_ids {
        if !message_ids.contains(message) {
            return Err(format!(
                "probe {} cites evidence message id {message}, which is absent from conversation {}",
                probe.id, conversation.id
            )
            .into());
        }
    }
    if probe.kind == ProbeKind::Abstention {
        if !probe.evidence_message_ids.is_empty() {
            return Err(
                format!("abstention probe {} carries evidence message ids", probe.id).into(),
            );
        }
    } else if probe.evidence_message_ids.is_empty() {
        return Err(format!(
            "probe {} of kind {:?} carries no evidence message ids",
            probe.id, probe.kind
        )
        .into());
    }
    Ok(())
}

#[derive(Deserialize)]
struct ArtifactRow {
    #[serde(default, alias = "conversation_id")]
    conversation: Option<String>,
    #[serde(default)]
    chat: Vec<Vec<ArtifactMessage>>,
    #[serde(default)]
    probing_questions: Value,
}

#[derive(Deserialize)]
struct ArtifactMessage {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    content: String,
    #[serde(default)]
    time_anchor: Option<String>,
}

#[derive(Deserialize)]
struct ArtifactProbe {
    #[serde(default)]
    question: String,
    #[serde(
        default,
        alias = "ideal_response",
        alias = "ideal_answer",
        alias = "ideal_summary"
    )]
    answer: Value,
    #[serde(default)]
    rubric: Value,
    #[serde(default)]
    difficulty: Option<String>,
    #[serde(default)]
    source_chat_ids: Value,
}

fn adapt_row(index: usize, row: &ArtifactRow) -> Result<Option<Conversation>, DynError> {
    let id = row
        .conversation
        .as_ref()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("conversation-{index}"));
    let mut sessions = Vec::new();
    let mut identifiers = BTreeMap::new();
    let mut used = BTreeSet::new();
    let mut next = 1;
    for batch in &row.chat {
        let mut messages = Vec::new();
        for message in batch {
            if message.content.trim().is_empty() {
                continue;
            }
            let assigned = match message.id {
                Some(original) if !used.contains(&original) => original,
                _ => {
                    while used.contains(&next) {
                        next += 1;
                    }
                    next
                }
            };
            used.insert(assigned);
            if let Some(original) = message.id {
                identifiers.entry(original).or_insert(assigned);
            }
            let role = message.role.as_deref().unwrap_or_default().trim();
            messages.push(Message {
                id: assigned,
                role: match role.to_ascii_lowercase().as_str() {
                    "user" => BenchRole::User,
                    "assistant" => BenchRole::Assistant,
                    _ => BenchRole::Other,
                },
                speaker: if role.is_empty() {
                    "unknown".to_owned()
                } else {
                    role.to_owned()
                },
                text: message.content.trim().to_owned(),
                occurred_at: message
                    .time_anchor
                    .as_ref()
                    .map(|value| value.trim().to_owned())
                    .unwrap_or_default(),
            });
        }
        if messages.is_empty() {
            continue;
        }
        sessions.push(Session {
            index: sessions.len(),
            label: format!("session-{}", sessions.len() + 1),
            messages,
        });
    }
    if sessions.is_empty() {
        return Ok(None);
    }
    let probes = adapt_probes(&id, &row.probing_questions, &identifiers)?;
    if probes.is_empty() {
        return Ok(None);
    }
    Ok(Some(Conversation {
        id,
        sessions,
        probes,
    }))
}

fn adapt_probes(
    conversation: &str,
    probing: &Value,
    identifiers: &BTreeMap<u64, u64>,
) -> Result<Vec<Probe>, DynError> {
    let mut probes = Vec::new();
    for (category, entries) in probing_map(probing)? {
        let Ok(kind) = serde_json::from_value::<ProbeKind>(Value::String(category.clone())) else {
            continue;
        };
        let Ok(entries) = serde_json::from_value::<Vec<ArtifactProbe>>(entries) else {
            continue;
        };
        for (ordinal, entry) in entries.iter().enumerate() {
            let question = entry.question.trim();
            let reference = match &entry.answer {
                Value::String(text) => text.trim().to_owned(),
                Value::Number(number) => number.to_string(),
                _ => String::new(),
            };
            let criteria = criteria_list(&entry.rubric);
            if question.is_empty() || reference.is_empty() || criteria.is_empty() {
                continue;
            }
            let mut evidence = Vec::new();
            if kind != ProbeKind::Abstention {
                let mut resolved = BTreeSet::new();
                let mut cited = Vec::new();
                collect_identifiers(&entry.source_chat_ids, &mut cited);
                for original in cited {
                    if let Some(assigned) = identifiers.get(&original) {
                        resolved.insert(*assigned);
                    }
                }
                if resolved.is_empty() {
                    continue;
                }
                evidence = resolved.into_iter().collect();
            }
            probes.push(Probe {
                id: format!("{conversation}-{category}-{}", ordinal + 1),
                kind,
                question: question.to_owned(),
                reference_answer: reference,
                criteria,
                difficulty: entry
                    .difficulty
                    .as_ref()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "unknown".to_owned()),
                evidence_message_ids: evidence,
            });
        }
    }
    Ok(probes)
}

fn probing_map(value: &Value) -> Result<Map<String, Value>, DynError> {
    match value {
        Value::Null => Ok(Map::new()),
        Value::Object(map) => Ok(map.clone()),
        Value::String(text) if text.trim().is_empty() => Ok(Map::new()),
        Value::String(text) => match serde_json::from_str::<Value>(text)
            .map_err(|error| format!("embedded probing question map: {error}"))?
        {
            Value::Object(map) => Ok(map),
            _ => Err("embedded probing question map is not an object".into()),
        },
        _ => Err("probing questions must be an object or an embedded JSON object".into()),
    }
}

fn criteria_list(rubric: &Value) -> Vec<String> {
    match rubric {
        Value::String(text) if !text.trim().is_empty() => vec![text.trim().to_owned()],
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn collect_identifiers(value: &Value, into: &mut Vec<u64>) {
    match value {
        Value::Number(number) => {
            if let Some(identifier) = number.as_u64() {
                into.push(identifier);
            }
        }
        Value::String(text) => {
            if let Ok(identifier) = text.trim().parse() {
                into.push(identifier);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_identifiers(item, into);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                collect_identifiers(item, into);
            }
        }
        Value::Null | Value::Bool(_) => {}
    }
}
