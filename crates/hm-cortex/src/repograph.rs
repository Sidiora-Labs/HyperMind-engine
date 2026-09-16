#![allow(clippy::missing_errors_doc)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const SNAPSHOT_CONTRACT: &str = "hypermind.repository-graph.v1";
pub const SNAPSHOT_DIGEST_DOMAIN: &str = "hypermind.repository-snapshot.v1";
pub const MAXIMUM_SNAPSHOT_FACTS: usize = 4_096;
pub const MAXIMUM_SNAPSHOT_EDGES: usize = 8_192;
pub const MAXIMUM_NAME_BYTES: usize = 1_024;
pub const RELATIONS: [&str; 8] = [
    "contains",
    "imports",
    "calls",
    "defines",
    "routes_to",
    "covers",
    "reads",
    "writes",
];

const NODE_DOMAIN: &str = "hypermind.repository-node.v1";
const EDGE_DOMAIN: &str = "hypermind.repository-edge.v1";
const EDGE_WEIGHT_STEP: u32 = 250_000;
const MAXIMUM_EDGE_WEIGHT: u32 = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RepoFactKind {
    File,
    Symbol,
    Dependency,
    Route,
    Test,
    Storage,
}

impl RepoFactKind {
    pub const ALL: [Self; 6] = [
        Self::File,
        Self::Symbol,
        Self::Dependency,
        Self::Route,
        Self::Test,
        Self::Storage,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Symbol => "symbol",
            Self::Dependency => "dependency",
            Self::Route => "route",
            Self::Test => "test",
            Self::Storage => "storage",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RepoCitation {
    pub lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RepoRelation {
    pub relation: String,
    pub target: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoFact {
    pub kind: RepoFactKind,
    pub name: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub end_line: Option<u32>,
    pub attributes: BTreeMap<String, String>,
    pub relations: Vec<RepoRelation>,
    pub source: RepoCitation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoSnapshotShard {
    pub lsn: u64,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoSnapshot {
    pub repository: String,
    pub digest: [u8; 32],
    pub facts: Vec<RepoFact>,
    pub skipped: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepoGraphError {
    Empty,
    Contract,
    Header,
    Truncated,
    Count,
    TooManyFacts,
}

pub fn parse_snapshot(shards: &[RepoSnapshotShard]) -> Result<RepoSnapshot, RepoGraphError> {
    if shards.is_empty() {
        return Err(RepoGraphError::Empty);
    }
    let mut ordered = shards.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|shard| shard.lsn);
    let digest = snapshot_digest(&ordered);

    let mut header: Option<(String, usize)> = None;
    let mut observed = 0usize;
    let mut skipped = 0u64;
    let mut parsed = Vec::new();
    for shard in &ordered {
        let text = std::str::from_utf8(&shard.content).map_err(|_| RepoGraphError::Truncated)?;
        u32::try_from(text.len()).map_err(|_| RepoGraphError::Truncated)?;
        let mut offset = 0usize;
        for line in text.split('\n') {
            let start = offset;
            offset += line.len() + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if header.is_none() {
                header = Some(parse_header(trimmed)?);
                continue;
            }
            observed += 1;
            if observed > MAXIMUM_SNAPSHOT_FACTS {
                return Err(RepoGraphError::TooManyFacts);
            }
            let lead = line.len() - line.trim_start().len();
            let byte_start = u32::try_from(start + lead).map_err(|_| RepoGraphError::Truncated)?;
            let byte_end = u32::try_from(start + lead + trimmed.len())
                .map_err(|_| RepoGraphError::Truncated)?;
            let source = RepoCitation {
                lsn: shard.lsn,
                byte_start,
                byte_end,
            };
            if let Some((fact, dropped)) = parse_fact(trimmed, source) {
                skipped += dropped;
                parsed.push(fact);
            } else {
                skipped += 1;
            }
        }
    }

    let (repository, declared) = header.ok_or(RepoGraphError::Empty)?;
    if declared != observed {
        return Err(RepoGraphError::Count);
    }

    parsed.sort_by(|left, right| {
        (
            left.kind,
            &left.name,
            left.source.lsn,
            left.source.byte_start,
        )
            .cmp(&(
                right.kind,
                &right.name,
                right.source.lsn,
                right.source.byte_start,
            ))
    });
    let mut seen = BTreeSet::new();
    let mut facts = Vec::with_capacity(parsed.len());
    for fact in parsed {
        if seen.insert((fact.kind, fact.name.clone())) {
            facts.push(fact);
        } else {
            skipped += 1;
        }
    }

    Ok(RepoSnapshot {
        repository,
        digest,
        facts,
        skipped,
    })
}

#[must_use]
pub fn node_id(repository: &str, kind: RepoFactKind, name: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(NODE_DOMAIN.as_bytes());
    hasher.update(&[0u8]);
    for component in [
        repository.as_bytes(),
        kind.as_str().as_bytes(),
        name.as_bytes(),
    ] {
        let length = u32::try_from(component.len()).unwrap_or(u32::MAX);
        hasher.update(&length.to_le_bytes());
        hasher.update(component);
    }
    *hasher.finalize().as_bytes()
}

#[must_use]
pub fn node_display_name(kind: RepoFactKind, name: &str) -> String {
    format!("{} {}", kind.as_str(), name)
}

#[must_use]
pub fn node_definition(fact: &RepoFact) -> Vec<u8> {
    let mut text = node_display_name(fact.kind, &fact.name);
    if let Some(path) = &fact.path {
        text.push_str(" at ");
        text.push_str(path);
    }
    if let Some(line) = fact.line {
        text.push_str(" line ");
        text.push_str(&line.to_string());
        if let Some(end_line) = fact.end_line {
            text.push_str(" to ");
            text.push_str(&end_line.to_string());
        }
    }
    for (key, value) in &fact.attributes {
        text.push_str("; ");
        text.push_str(key);
        text.push(' ');
        text.push_str(value);
    }
    text.push('.');
    text.into_bytes()
}

#[must_use]
pub fn node_tags(repository: &str, kind: RepoFactKind) -> Vec<String> {
    vec![
        format!("repository:{repository}"),
        format!("repository-kind:{}", kind.as_str()),
    ]
}

fn snapshot_digest(shards: &[&RepoSnapshotShard]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(SNAPSHOT_DIGEST_DOMAIN.as_bytes());
    hasher.update(&[0u8]);
    for shard in shards {
        hasher.update(&shard.content);
    }
    *hasher.finalize().as_bytes()
}

fn parse_header(line: &str) -> Result<(String, usize), RepoGraphError> {
    let value = serde_json::from_str::<Value>(line).map_err(|_| RepoGraphError::Header)?;
    let object = value.as_object().ok_or(RepoGraphError::Header)?;
    let contract = object
        .get("contract")
        .and_then(Value::as_str)
        .ok_or(RepoGraphError::Header)?;
    if contract != SNAPSHOT_CONTRACT {
        return Err(RepoGraphError::Contract);
    }
    let repository = object
        .get("repository")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty() && name.len() <= MAXIMUM_NAME_BYTES)
        .ok_or(RepoGraphError::Header)?;
    let declared = object
        .get("fact_count")
        .and_then(Value::as_u64)
        .ok_or(RepoGraphError::Header)?;
    let declared = usize::try_from(declared).map_err(|_| RepoGraphError::Header)?;
    Ok((repository.to_owned(), declared))
}

fn parse_fact(line: &str, source: RepoCitation) -> Option<(RepoFact, u64)> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    let object = value.as_object()?;
    let kind = RepoFactKind::parse(object.get("kind").and_then(Value::as_str)?)?;
    let name = object.get("name").and_then(Value::as_str)?;
    if name.is_empty() || name.len() > MAXIMUM_NAME_BYTES {
        return None;
    }
    let path = object
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty() && path.len() <= MAXIMUM_NAME_BYTES)
        .map(str::to_owned);
    let line_number = object
        .get("line")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok());
    let end_line = object
        .get("end_line")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok());
    let mut attributes = BTreeMap::new();
    if let Some(entries) = object.get("attributes").and_then(Value::as_object) {
        for (key, entry) in entries {
            if let Some(text) = entry.as_str()
                && !key.is_empty()
                && key.len() <= MAXIMUM_NAME_BYTES
            {
                attributes.insert(key.clone(), text.to_owned());
            }
        }
    }
    let mut relations = Vec::new();
    let mut dropped = 0u64;
    if let Some(entries) = object.get("relations").and_then(Value::as_array) {
        for entry in entries {
            if let Some(relation) = parse_relation(entry) {
                relations.push(relation);
            } else {
                dropped += 1;
            }
        }
    }
    let fact = RepoFact {
        kind,
        name: name.to_owned(),
        path,
        line: line_number,
        end_line,
        attributes,
        relations,
        source,
    };
    Some((fact, dropped))
}

fn parse_relation(entry: &Value) -> Option<RepoRelation> {
    let object = entry.as_object()?;
    let relation = object.get("relation").and_then(Value::as_str)?;
    let target = object.get("target").and_then(Value::as_str)?;
    if relation.is_empty()
        || target.is_empty()
        || relation.len() > MAXIMUM_NAME_BYTES
        || target.len() > MAXIMUM_NAME_BYTES
    {
        return None;
    }
    Some(RepoRelation {
        relation: relation.to_owned(),
        target: target.to_owned(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoNode {
    pub node_id: [u8; 32],
    pub kind: RepoFactKind,
    pub name: String,
    pub display_name: String,
    pub definition: Vec<u8>,
    pub tags: Vec<String>,
    pub citation: RepoCitation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoEdge {
    pub edge_id: [u8; 32],
    pub source_id: [u8; 32],
    pub target_id: [u8; 32],
    pub relation: String,
    pub weight_micros: u32,
    pub citation: RepoCitation,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RepoDropReason {
    UnknownRelation,
    UnresolvedTarget,
    AmbiguousTarget,
    SelfReference,
    NodeLimit,
    EdgeLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoDrop {
    pub source: String,
    pub relation: String,
    pub target: String,
    pub reason: RepoDropReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoGraph {
    pub repository: String,
    pub digest: [u8; 32],
    pub nodes: Vec<RepoNode>,
    pub edges: Vec<RepoEdge>,
    pub dropped: Vec<RepoDrop>,
    pub skipped: u64,
}

struct EdgeEvidence {
    occurrences: u32,
    citation: RepoCitation,
}

#[must_use]
pub fn resolve(snapshot: &RepoSnapshot) -> RepoGraph {
    let kept = snapshot.facts.len().min(MAXIMUM_SNAPSHOT_FACTS);
    let facts = &snapshot.facts[..kept];
    let mut dropped = Vec::new();
    for fact in &snapshot.facts[kept..] {
        dropped.push(RepoDrop {
            source: fact.name.clone(),
            relation: String::new(),
            target: String::new(),
            reason: RepoDropReason::NodeLimit,
        });
    }

    let mut nodes = Vec::with_capacity(kept);
    let mut exact: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut suffix: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, fact) in facts.iter().enumerate() {
        nodes.push(RepoNode {
            node_id: node_id(&snapshot.repository, fact.kind, &fact.name),
            kind: fact.kind,
            name: fact.name.clone(),
            display_name: node_display_name(fact.kind, &fact.name),
            definition: node_definition(fact),
            tags: node_tags(&snapshot.repository, fact.kind),
            citation: fact.source,
        });
        exact.entry(fact.name.as_str()).or_default().push(index);
        suffix
            .entry(name_suffix(fact.name.as_str()))
            .or_default()
            .push(index);
    }

    let mut evidence: BTreeMap<(usize, &'static str, usize), EdgeEvidence> = BTreeMap::new();
    for (index, fact) in facts.iter().enumerate() {
        for declared in &fact.relations {
            let Some(relation) = known_relation(&declared.relation) else {
                dropped.push(drop_of(fact, declared, RepoDropReason::UnknownRelation));
                continue;
            };
            let target = match resolve_target(&exact, &suffix, &declared.target) {
                Ok(target) => target,
                Err(reason) => {
                    dropped.push(drop_of(fact, declared, reason));
                    continue;
                }
            };
            if target == index {
                dropped.push(drop_of(fact, declared, RepoDropReason::SelfReference));
                continue;
            }
            let entry = evidence
                .entry((index, relation, target))
                .or_insert(EdgeEvidence {
                    occurrences: 0,
                    citation: fact.source,
                });
            entry.occurrences = entry.occurrences.saturating_add(1);
        }
    }

    let mut edges = Vec::new();
    for ((source, relation, target), found) in evidence {
        if edges.len() >= MAXIMUM_SNAPSHOT_EDGES {
            dropped.push(RepoDrop {
                source: nodes[source].name.clone(),
                relation: relation.to_owned(),
                target: nodes[target].name.clone(),
                reason: RepoDropReason::EdgeLimit,
            });
            continue;
        }
        edges.push(RepoEdge {
            edge_id: edge_id(&nodes[source].node_id, &nodes[target].node_id, relation),
            source_id: nodes[source].node_id,
            target_id: nodes[target].node_id,
            relation: relation.to_owned(),
            weight_micros: EDGE_WEIGHT_STEP
                .saturating_mul(found.occurrences)
                .min(MAXIMUM_EDGE_WEIGHT),
            citation: found.citation,
        });
    }

    RepoGraph {
        repository: snapshot.repository.clone(),
        digest: snapshot.digest,
        nodes,
        edges,
        dropped,
        skipped: snapshot.skipped,
    }
}

#[must_use]
pub fn edge_id(source_id: &[u8; 32], target_id: &[u8; 32], relation: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(EDGE_DOMAIN.as_bytes());
    hasher.update(&[0u8]);
    hasher.update(source_id);
    hasher.update(target_id);
    let length = u32::try_from(relation.len()).unwrap_or(u32::MAX);
    hasher.update(&length.to_le_bytes());
    hasher.update(relation.as_bytes());
    *hasher.finalize().as_bytes()
}

fn known_relation(relation: &str) -> Option<&'static str> {
    RELATIONS
        .into_iter()
        .find(|candidate| *candidate == relation)
}

fn name_suffix(name: &str) -> &str {
    match name.rfind(['/', '.']) {
        Some(position) if position + 1 < name.len() => &name[position + 1..],
        _ => name,
    }
}

fn resolve_target(
    exact: &BTreeMap<&str, Vec<usize>>,
    suffix: &BTreeMap<&str, Vec<usize>>,
    target: &str,
) -> Result<usize, RepoDropReason> {
    if let Some(candidates) = exact.get(target) {
        return match candidates.as_slice() {
            [only] => Ok(*only),
            _ => Err(RepoDropReason::AmbiguousTarget),
        };
    }
    match suffix.get(target).map(Vec::as_slice) {
        Some([only]) => Ok(*only),
        Some(_) => Err(RepoDropReason::AmbiguousTarget),
        None => Err(RepoDropReason::UnresolvedTarget),
    }
}

fn drop_of(fact: &RepoFact, declared: &RepoRelation, reason: RepoDropReason) -> RepoDrop {
    RepoDrop {
        source: fact.name.clone(),
        relation: declared.relation.clone(),
        target: declared.target.clone(),
        reason,
    }
}
