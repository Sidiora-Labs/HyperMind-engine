#![allow(clippy::missing_errors_doc)]

use crate::tools::consolidate::hex;
use crate::{Envelope, RecallInput, authority_name};
use hm_core::{Error, ErrorCode};
use hm_cortex::repograph::{RepoFactKind, node_id};
use hm_schema::events::Authority;
use hm_serve::actor::{ActorEngine, GraphNeighbourhood};
use serde_json::json;
use std::collections::BTreeSet;

pub const MAXIMUM_GRAPH_ITEMS: usize = 256;

const NODE_ID_HEX_LENGTH: usize = 64;

pub(crate) async fn run(actor: &ActorEngine, input: RecallInput) -> Result<Envelope, Error> {
    let anchor = input
        .filters
        .anchor
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?
        .to_owned();
    if input.limit == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let limit = input.limit.min(MAXIMUM_GRAPH_ITEMS);
    let valid_at_ns = input.filters.temporal_from_ns.unwrap_or(i64::MAX);

    let mut generation = 0;
    let mut resolved: Option<(Vec<u8>, GraphNeighbourhood)> = None;
    for candidate in candidates(&anchor, input.query.trim())? {
        let found = actor
            .graph_neighbourhood(candidate.clone(), valid_at_ns, limit)
            .await?;
        generation = found.generation;
        if found.node.is_some() {
            resolved = Some((candidate, found));
            break;
        }
    }
    let Some((anchor_id, neighbourhood)) = resolved else {
        return Ok(unresolved(&anchor, generation));
    };

    let mut envelope = Envelope::empty();
    envelope.health["generation"] = json!(neighbourhood.generation);
    envelope.health["anchor"] = json!(hex(&anchor_id));
    if let Some(node) = &neighbourhood.node {
        envelope.health["anchor_name"] = json!(node.name);
    }
    let mut published = BTreeSet::new();
    for neighbour in &neighbourhood.neighbours {
        let edge = &neighbour.edge;
        let endpoint_id = if neighbour.outgoing {
            &edge.target_id
        } else {
            &edge.source_id
        };
        let mut support = Vec::new();
        for citation in &edge.citations {
            for lsn in [citation.first_lsn, citation.last_lsn] {
                if lsn != 0 && !support.contains(&lsn) {
                    support.push(lsn);
                }
            }
        }
        let uri = format!("hm://{}/lsn/{}", actor.actor(), edge.event_lsn);
        envelope.items.push(json!({
            "edge_id": hex(&edge.edge_id),
            "relation": edge.relation,
            "direction": direction(neighbour.outgoing),
            "node_id": hex(endpoint_id),
            "name": neighbour
                .endpoint
                .as_ref()
                .map_or_else(String::new, |record| record.name.clone()),
            "weight_micros": edge.weight_micros,
            "valid_from_ns": edge.valid_from_ns,
            "valid_to_ns": edge.valid_to_ns,
            "edge_lsn": edge.event_lsn,
            "generation": edge.generation,
            "authority": authority_name(Authority::DerivedInference),
            "support_lsns": support.clone(),
            "uri": uri.clone(),
        }));
        if published.insert(uri.clone()) {
            envelope.provenance.push(uri);
        }
        for lsn in support {
            let cited = format!("hm://{}/lsn/{lsn}", actor.actor());
            if published.insert(cited.clone()) {
                envelope.provenance.push(cited);
            }
        }
        if neighbour.endpoint.is_none() {
            envelope.gaps.push(json!({
                "kind": "graph_endpoint_unresolved",
                "node_id": hex(endpoint_id),
                "relation": edge.relation,
            }));
        }
    }
    if envelope.items.len() == limit {
        envelope
            .gaps
            .push(json!({"kind": "graph_neighbours_truncated", "limit": limit}));
    }
    envelope.warnings.push(
        "A repository graph answer describes facts a snapshot declared, not the current state of the repository."
            .to_owned(),
    );
    Ok(envelope)
}

fn unresolved(anchor: &str, generation: u64) -> Envelope {
    let mut envelope = Envelope::empty();
    envelope.health["generation"] = json!(generation);
    envelope
        .gaps
        .push(json!({"kind": "graph_anchor_unresolved", "anchor": anchor}));
    envelope.warnings.push(
        "The graph anchor matched no node visible in the active generation; nothing was traversed."
            .to_owned(),
    );
    envelope
}

const fn direction(outgoing: bool) -> &'static str {
    if outgoing { "outgoing" } else { "incoming" }
}

fn candidates(anchor: &str, name: &str) -> Result<Vec<Vec<u8>>, Error> {
    if let Some(id) = parse_node_id(anchor) {
        return Ok(vec![id.to_vec()]);
    }
    if name.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    Ok(RepoFactKind::ALL
        .into_iter()
        .map(|kind| node_id(anchor, kind, name).to_vec())
        .collect())
}

fn parse_node_id(anchor: &str) -> Option<[u8; 32]> {
    if anchor.len() != NODE_ID_HEX_LENGTH
        || !anchor
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return None;
    }
    let mut id = [0u8; 32];
    for (index, slot) in id.iter_mut().enumerate() {
        let start = index * 2;
        *slot = u8::from_str_radix(&anchor[start..start + 2], 16).ok()?;
    }
    Some(id)
}
