#![forbid(unsafe_code)]

use hm_cortex::repograph::{
    MAXIMUM_SNAPSHOT_EDGES, MAXIMUM_SNAPSHOT_FACTS, RELATIONS, RepoCitation, RepoDrop,
    RepoDropReason, RepoFact, RepoFactKind, RepoRelation, RepoSnapshot, RepoSnapshotShard,
    SNAPSHOT_CONTRACT, edge_id, node_display_name, node_id, parse_snapshot, resolve,
};
use std::collections::BTreeMap;

fn shard(lsn: u64, lines: &[String]) -> RepoSnapshotShard {
    let mut content = String::new();
    for line in lines {
        content.push_str(line);
        content.push('\n');
    }
    RepoSnapshotShard {
        lsn,
        content: content.into_bytes(),
    }
}

fn header(repository: &str, fact_count: usize) -> String {
    format!(
        "{{\"contract\":\"{SNAPSHOT_CONTRACT}\",\"repository\":\"{repository}\",\"fact_count\":{fact_count}}}"
    )
}

fn symbol(name: &str, relations: &[(&str, &str)]) -> String {
    let rendered = relations
        .iter()
        .map(|(relation, target)| {
            format!("{{\"relation\":\"{relation}\",\"target\":\"{target}\"}}")
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"kind\":\"symbol\",\"name\":\"{name}\",\"path\":\"app.py\",\"relations\":[{rendered}]}}"
    )
}

fn snapshot(repository: &str, facts: &[String]) -> RepoSnapshot {
    let mut lines = vec![header(repository, facts.len())];
    lines.extend(facts.iter().cloned());
    parse_snapshot(&[shard(7, &lines)]).expect("snapshot parses")
}

fn citation_of(parsed: &RepoSnapshot, name: &str) -> RepoCitation {
    parsed
        .facts
        .iter()
        .find(|fact| fact.name == name)
        .expect("declared fact")
        .source
}

#[test]
fn relations_resolve_exactly_then_by_unique_suffix() {
    let parsed = snapshot(
        "engine",
        &[
            symbol("app/api.exact", &[("calls", "app/db.Database")]),
            symbol("app/api.bare", &[("calls", "Database")]),
            symbol("app/db.Database", &[]),
        ],
    );
    let graph = resolve(&parsed);

    let names = graph
        .nodes
        .iter()
        .map(|node| node.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["app/api.bare", "app/api.exact", "app/db.Database"]);
    for node in &graph.nodes {
        assert_eq!(
            node.node_id,
            node_id("engine", RepoFactKind::Symbol, &node.name)
        );
        assert_eq!(
            node.display_name,
            node_display_name(RepoFactKind::Symbol, &node.name)
        );
        assert!(!node.definition.is_empty());
        assert!(!node.tags.is_empty());
        assert_eq!(node.citation, citation_of(&parsed, &node.name));
    }

    assert!(graph.dropped.is_empty());
    assert_eq!(graph.edges.len(), 2);
    let target = node_id("engine", RepoFactKind::Symbol, "app/db.Database");
    for source in ["app/api.bare", "app/api.exact"] {
        let source_id = node_id("engine", RepoFactKind::Symbol, source);
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.source_id == source_id)
            .expect("edge for declaring fact");
        assert_eq!(edge.relation, "calls");
        assert_eq!(edge.target_id, target);
        assert_eq!(edge.weight_micros, 250_000);
        assert_eq!(edge.citation, citation_of(&parsed, source));
        assert_eq!(edge.edge_id, edge_id(&source_id, &target, "calls"));
    }
}

#[test]
fn ambiguous_unknown_and_self_targets_are_dropped_with_reasons() {
    let parsed = snapshot(
        "engine",
        &[
            symbol("x/Handler", &[]),
            symbol("y/Handler", &[]),
            symbol(
                "app/main",
                &[
                    ("calls", "Handler"),
                    ("imports", "missing/Thing"),
                    ("teleports", "x/Handler"),
                ],
            ),
            symbol("app/loop", &[("calls", "app/loop")]),
        ],
    );
    let graph = resolve(&parsed);

    assert!(graph.edges.is_empty());
    assert_eq!(
        graph.dropped,
        vec![
            RepoDrop {
                source: "app/loop".to_owned(),
                relation: "calls".to_owned(),
                target: "app/loop".to_owned(),
                reason: RepoDropReason::SelfReference,
            },
            RepoDrop {
                source: "app/main".to_owned(),
                relation: "calls".to_owned(),
                target: "Handler".to_owned(),
                reason: RepoDropReason::AmbiguousTarget,
            },
            RepoDrop {
                source: "app/main".to_owned(),
                relation: "imports".to_owned(),
                target: "missing/Thing".to_owned(),
                reason: RepoDropReason::UnresolvedTarget,
            },
            RepoDrop {
                source: "app/main".to_owned(),
                relation: "teleports".to_owned(),
                target: "x/Handler".to_owned(),
                reason: RepoDropReason::UnknownRelation,
            },
        ]
    );
    assert!(!RELATIONS.contains(&"teleports"));
}

#[test]
fn repeated_relations_collapse_and_count_evidence() {
    let parsed = snapshot(
        "engine",
        &[
            symbol("app/once", &[("calls", "app/target")]),
            symbol("app/twice", &[("calls", "app/target"), ("calls", "target")]),
            symbol(
                "app/five",
                &[
                    ("calls", "app/target"),
                    ("calls", "app/target"),
                    ("calls", "target"),
                    ("calls", "app/target"),
                    ("calls", "target"),
                ],
            ),
            symbol("app/target", &[]),
        ],
    );
    let graph = resolve(&parsed);

    assert!(graph.dropped.is_empty());
    assert_eq!(graph.edges.len(), 3);
    let weights = graph
        .edges
        .iter()
        .map(|edge| {
            let source = graph
                .nodes
                .iter()
                .find(|node| node.node_id == edge.source_id)
                .expect("source node");
            (source.name.clone(), edge.weight_micros)
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(weights["app/once"], 250_000);
    assert_eq!(weights["app/twice"], 500_000);
    assert_eq!(weights["app/five"], 1_000_000);
    for edge in &graph.edges {
        assert!((1..=1_000_000).contains(&edge.weight_micros));
    }
}

#[test]
fn node_and_edge_caps_are_reported() {
    let overflow = MAXIMUM_SNAPSHOT_FACTS + 2;
    let mut facts = Vec::with_capacity(overflow);
    for index in 0..overflow {
        let relations = if index < MAXIMUM_SNAPSHOT_FACTS {
            (1..=3)
                .map(|step| RepoRelation {
                    relation: "calls".to_owned(),
                    target: format!("mod{:04}/item", (index + step) % MAXIMUM_SNAPSHOT_FACTS),
                })
                .collect()
        } else {
            Vec::new()
        };
        facts.push(RepoFact {
            kind: RepoFactKind::Symbol,
            name: format!("mod{index:04}/item"),
            path: None,
            line: None,
            end_line: None,
            attributes: BTreeMap::new(),
            relations,
            source: RepoCitation {
                lsn: 9,
                byte_start: 0,
                byte_end: 1,
            },
        });
    }
    let graph = resolve(&RepoSnapshot {
        repository: "engine".to_owned(),
        digest: [3u8; 32],
        facts,
        skipped: 4,
    });

    assert_eq!(graph.nodes.len(), MAXIMUM_SNAPSHOT_FACTS);
    assert_eq!(graph.edges.len(), MAXIMUM_SNAPSHOT_EDGES);
    assert_eq!(graph.skipped, 4);
    let node_limit = graph
        .dropped
        .iter()
        .filter(|drop| drop.reason == RepoDropReason::NodeLimit)
        .collect::<Vec<_>>();
    assert_eq!(node_limit.len(), 2);
    assert_eq!(
        node_limit[0].source,
        format!("mod{MAXIMUM_SNAPSHOT_FACTS:04}/item")
    );
    let edge_limit = graph
        .dropped
        .iter()
        .filter(|drop| drop.reason == RepoDropReason::EdgeLimit)
        .count();
    assert_eq!(
        edge_limit,
        3 * MAXIMUM_SNAPSHOT_FACTS - MAXIMUM_SNAPSHOT_EDGES
    );
    assert_eq!(graph.dropped.len(), 2 + edge_limit);
}

#[test]
fn resolution_is_byte_identical_across_runs() {
    let parsed = snapshot(
        "engine",
        &[
            symbol("app/api.exact", &[("calls", "app/db.Database")]),
            symbol("app/api.bare", &[("calls", "Database"), ("teleports", "x")]),
            symbol("app/db.Database", &[("reads", "store/rows")]),
            symbol("store/rows", &[("writes", "nowhere")]),
        ],
    );
    let first = resolve(&parsed);
    let second = resolve(&parsed);

    assert_eq!(
        first
            .nodes
            .iter()
            .map(|node| node.node_id)
            .collect::<Vec<_>>(),
        second
            .nodes
            .iter()
            .map(|node| node.node_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        first
            .edges
            .iter()
            .map(|edge| edge.edge_id)
            .collect::<Vec<_>>(),
        second
            .edges
            .iter()
            .map(|edge| edge.edge_id)
            .collect::<Vec<_>>()
    );
    assert_eq!(first.dropped, second.dropped);
    assert_eq!(first, second);
    assert_eq!(first.edges.len(), 3);
    assert_eq!(first.dropped.len(), 2);
}
