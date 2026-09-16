#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_cortex::relations::{
    RelationDrop, RelationOptions, RelationSource, relation_key, represent,
};

const OPTIONS: RelationOptions = RelationOptions {
    maximum_relations: 16,
    maximum_text_bytes: 64,
};

fn source(
    edge_id: &str,
    edge_lsn: u64,
    source_id: &str,
    relation: &str,
    target_id: &str,
    support_lsns: &[u64],
) -> RelationSource {
    RelationSource {
        edge_id: edge_id.as_bytes().to_vec(),
        edge_lsn,
        source_id: source_id.as_bytes().to_vec(),
        target_id: target_id.as_bytes().to_vec(),
        relation: relation.to_owned(),
        support_lsns: support_lsns.to_vec(),
    }
}

fn generated(count: usize) -> Vec<RelationSource> {
    (0..count)
        .map(|index| {
            let edge_lsn = u64::try_from(count - index).expect("index fits a u64");
            RelationSource {
                edge_id: format!("edge-{index}").into_bytes(),
                edge_lsn,
                source_id: format!("node-{index}").into_bytes(),
                target_id: format!("node-{}", index + 1).into_bytes(),
                relation: "links to".to_owned(),
                support_lsns: vec![edge_lsn],
            }
        })
        .collect()
}

#[test]
fn relationship_representations_are_deterministic_and_carry_edge_and_source_provenance() {
    let sources = vec![
        source(
            "edge-ada",
            12,
            "ada",
            "collaborates with",
            "grace",
            &[9, 4, 9],
        ),
        source("edge-grace", 18, " grace ", " mentors ", " alan ", &[11]),
        source("edge-alan", 25, "alan", "reports to", "ada", &[3, 21]),
    ];

    let first = represent(&sources, OPTIONS).expect("bounded inputs produce a plan");
    let second = represent(&sources, OPTIONS).expect("bounded inputs produce a plan");
    assert_eq!(first, second);

    assert_eq!(first.representations.len(), 3);
    assert!(first.dropped.is_empty());
    assert_eq!(
        first
            .representations
            .iter()
            .map(|representation| representation.edge_lsn)
            .collect::<Vec<_>>(),
        vec![12, 18, 25]
    );

    let head = &first.representations[0];
    assert_eq!(head.support_lsns, vec![4, 9]);
    assert_eq!(head.edge_lsn, 12);
    assert!(head.text.contains("ada"));
    assert!(head.text.contains("collaborates with"));
    assert!(head.text.contains("grace"));
    assert_eq!(head.text, "ada collaborates with grace");
    assert_eq!(
        head.relation_key,
        relation_key(b"edge-ada", b"ada", "collaborates with", b"grace")
    );

    assert_eq!(first.representations[1].text, "grace mentors alan");
    assert_eq!(first.representations[1].support_lsns, vec![11]);
    assert_eq!(first.representations[2].support_lsns, vec![3, 21]);
}

#[test]
fn relationship_representations_drop_unusable_edges_with_a_typed_reason() {
    let long_relation = "elaborates".repeat(20);
    let sources = vec![
        source("edge-zero", 0, "ada", "knows", "grace", &[4]),
        source("edge-blank", 2, "ada", "   ", "grace", &[4]),
        source("edge-endpoint", 3, "ada", "knows", "", &[4]),
        source("edge-support", 4, "ada", "knows", "grace", &[]),
        source("edge-long", 5, "ada", &long_relation, "grace", &[4]),
    ];

    let plan = represent(&sources, OPTIONS).expect("bounded inputs produce a plan");

    assert!(plan.representations.is_empty());
    assert_eq!(
        plan.dropped,
        vec![
            (0, RelationDrop::ZeroLsn),
            (2, RelationDrop::EmptyRelation),
            (3, RelationDrop::EmptyEndpoint),
            (4, RelationDrop::NoSupport),
            (5, RelationDrop::TextTooLong),
        ]
    );
}

#[test]
fn duplicate_relationships_collapse_to_the_latest_assertion() {
    let sources = vec![
        source("edge-ada", 5, "ada", "collaborates with", "grace", &[4, 9]),
        source("edge-ada", 9, "ada", "collaborates with", "grace", &[2, 4]),
    ];

    let plan = represent(&sources, OPTIONS).expect("bounded inputs produce a plan");

    assert!(plan.dropped.is_empty());
    assert_eq!(plan.representations.len(), 1);
    let representation = &plan.representations[0];
    assert_eq!(representation.edge_lsn, 9);
    assert_eq!(representation.support_lsns, vec![2, 4, 9]);
    assert_eq!(representation.text, "ada collaborates with grace");
    assert_eq!(
        representation.relation_key,
        relation_key(b"edge-ada", b"ada", "collaborates with", b"grace")
    );
}

#[test]
fn relationship_representation_arguments_are_bounded() {
    let sources = generated(10);

    let no_relations = represent(
        &sources,
        RelationOptions {
            maximum_relations: 0,
            maximum_text_bytes: 64,
        },
    )
    .expect_err("a zero relation ceiling is rejected");
    assert_eq!(no_relations.code, ErrorCode::InvalidArgument);

    let short_text = represent(
        &sources,
        RelationOptions {
            maximum_relations: 16,
            maximum_text_bytes: 8,
        },
    )
    .expect_err("a nonsensical text ceiling is rejected");
    assert_eq!(short_text.code, ErrorCode::InvalidArgument);

    let overflowing = represent(&generated(4_097), OPTIONS).expect_err("4097 inputs are refused");
    assert_eq!(overflowing.code, ErrorCode::CapacityExceeded);

    assert!(represent(&generated(4_096), OPTIONS).is_ok());

    let truncated = represent(
        &sources,
        RelationOptions {
            maximum_relations: 3,
            maximum_text_bytes: 64,
        },
    )
    .expect("bounded inputs produce a plan");
    assert_eq!(truncated.representations.len(), 3);
    assert!(truncated.dropped.is_empty());
    assert_eq!(
        truncated
            .representations
            .iter()
            .map(|representation| representation.edge_lsn)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}
