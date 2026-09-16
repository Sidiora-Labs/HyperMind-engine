#![forbid(unsafe_code)]

use hm_cortex::repograph::{
    MAXIMUM_NAME_BYTES, MAXIMUM_SNAPSHOT_FACTS, RepoFactKind, RepoGraphError, RepoSnapshotShard,
    SNAPSHOT_CONTRACT, node_definition, node_display_name, node_id, node_tags, parse_snapshot,
};
use serde_json::Value;
use std::collections::BTreeMap;

fn shard(lsn: u64, lines: &[&str]) -> RepoSnapshotShard {
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

fn lines_by_name(shards: &[RepoSnapshotShard]) -> BTreeMap<String, String> {
    let mut indexed = BTreeMap::new();
    for shard in shards {
        let text = std::str::from_utf8(&shard.content).expect("utf-8 shard");
        for line in text.split('\n') {
            if line.trim().is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if let Some(name) = value.get("name").and_then(Value::as_str) {
                indexed.insert(name.to_owned(), line.to_owned());
            }
        }
    }
    indexed
}

#[test]
fn snapshot_parses_every_fact_kind_with_exact_byte_ranges() {
    let shards = vec![
        shard(
            11,
            &[
                &header("engine", 6),
                "{\"kind\":\"file\",\"name\":\"app/db.py\",\"path\":\"app/db.py\"}",
                "{\"kind\":\"symbol\",\"name\":\"app/db.Database\",\"path\":\"app/db.py\",\"line\":10,\"end_line\":42,\"attributes\":{\"role\":\"class\"},\"relations\":[{\"relation\":\"defines\",\"target\":\"app/db.query\"}]}",
                "{\"kind\":\"dependency\",\"name\":\"sqlite3\"}",
            ],
        ),
        shard(
            12,
            &[
                "{\"kind\":\"route\",\"name\":\"GET /health\",\"relations\":[{\"relation\":\"routes_to\",\"target\":\"app/db.query\"}]}",
                "{\"kind\":\"test\",\"name\":\"tests/db::query\"}",
                "{\"kind\":\"storage\",\"name\":\"sqlite:main\"}",
            ],
        ),
    ];

    let snapshot = parse_snapshot(&shards).expect("snapshot parses");
    assert_eq!(snapshot.repository, "engine");
    assert_eq!(snapshot.skipped, 0);
    assert_eq!(snapshot.facts.len(), 6);
    assert_ne!(snapshot.digest, [0u8; 32]);

    let kinds = snapshot
        .facts
        .iter()
        .map(|fact| fact.kind)
        .collect::<Vec<_>>();
    assert_eq!(kinds, RepoFactKind::ALL.to_vec());

    let expected = lines_by_name(&shards);
    for fact in &snapshot.facts {
        let shard = shards
            .iter()
            .find(|shard| shard.lsn == fact.source.lsn)
            .expect("citation names an observed shard");
        assert!(fact.source.byte_end > fact.source.byte_start);
        let slice = &shard.content[fact.source.byte_start as usize..fact.source.byte_end as usize];
        let line = expected.get(&fact.name).expect("fact name was declared");
        assert_eq!(slice, line.as_bytes());
    }

    let symbol = snapshot
        .facts
        .iter()
        .find(|fact| fact.kind == RepoFactKind::Symbol)
        .expect("symbol fact");
    assert_eq!(symbol.path.as_deref(), Some("app/db.py"));
    assert_eq!(symbol.line, Some(10));
    assert_eq!(symbol.end_line, Some(42));
    assert_eq!(
        symbol.attributes.get("role").map(String::as_str),
        Some("class")
    );
    assert_eq!(symbol.relations.len(), 1);
    assert_eq!(symbol.relations[0].relation, "defines");
    assert_eq!(symbol.relations[0].target, "app/db.query");
    assert_eq!(
        node_display_name(symbol.kind, &symbol.name),
        "symbol app/db.Database"
    );
    assert_eq!(
        node_definition(symbol),
        b"symbol app/db.Database at app/db.py line 10 to 42; role class.".to_vec()
    );
    assert_eq!(
        node_tags("engine", symbol.kind),
        vec![
            "repository:engine".to_owned(),
            "repository-kind:symbol".to_owned()
        ]
    );
}

#[test]
fn unknown_kinds_and_malformed_lines_are_counted_not_fatal() {
    let shards = vec![shard(
        7,
        &[
            &header("engine", 5),
            "{\"kind\":\"widget\",\"name\":\"app/widget.py\"}",
            "not json at all",
            "{\"kind\":\"file\",\"name\":\"\"}",
            "{\"kind\":\"file\",\"name\":\"app/db.py\",\"relations\":[{\"relation\":\"contains\"}]}",
            "{\"kind\":\"symbol\",\"name\":\"app/db.Database\"}",
        ],
    )];

    let snapshot = parse_snapshot(&shards).expect("snapshot parses");
    assert_eq!(snapshot.skipped, 4);
    assert_eq!(snapshot.facts.len(), 2);
    assert_eq!(snapshot.facts[0].kind, RepoFactKind::File);
    assert_eq!(snapshot.facts[0].name, "app/db.py");
    assert!(snapshot.facts[0].relations.is_empty());
    assert_eq!(snapshot.facts[1].kind, RepoFactKind::Symbol);
    assert_eq!(snapshot.facts[1].name, "app/db.Database");

    let oversized = "x".repeat(MAXIMUM_NAME_BYTES + 1);
    let shards = vec![shard(
        7,
        &[
            &header("engine", 2),
            &format!("{{\"kind\":\"file\",\"name\":\"{oversized}\"}}"),
            "{\"kind\":\"file\",\"name\":\"app/db.py\"}",
        ],
    )];
    let snapshot = parse_snapshot(&shards).expect("snapshot parses");
    assert_eq!(snapshot.skipped, 1);
    assert_eq!(snapshot.facts.len(), 1);
}

#[test]
fn snapshot_contract_and_count_fail_closed() {
    assert_eq!(parse_snapshot(&[]), Err(RepoGraphError::Empty));

    let wrong_contract = vec![shard(
        3,
        &[
            "{\"contract\":\"hypermind.repository-graph.v0\",\"repository\":\"engine\",\"fact_count\":1}",
            "{\"kind\":\"file\",\"name\":\"app/db.py\"}",
        ],
    )];
    assert_eq!(
        parse_snapshot(&wrong_contract),
        Err(RepoGraphError::Contract)
    );

    let missing_repository = vec![shard(
        3,
        &[&format!(
            "{{\"contract\":\"{SNAPSHOT_CONTRACT}\",\"fact_count\":0}}"
        )],
    )];
    assert_eq!(
        parse_snapshot(&missing_repository),
        Err(RepoGraphError::Header)
    );

    let wrong_count = vec![shard(
        3,
        &[
            &header("engine", 2),
            "{\"kind\":\"file\",\"name\":\"app/db.py\"}",
        ],
    )];
    assert_eq!(parse_snapshot(&wrong_count), Err(RepoGraphError::Count));

    let invalid_utf8 = vec![RepoSnapshotShard {
        lsn: 3,
        content: vec![0xff, 0xfe, b'\n'],
    }];
    assert_eq!(
        parse_snapshot(&invalid_utf8),
        Err(RepoGraphError::Truncated)
    );

    let mut lines = vec![header("engine", MAXIMUM_SNAPSHOT_FACTS + 1)];
    for index in 0..=MAXIMUM_SNAPSHOT_FACTS {
        lines.push(format!(
            "{{\"kind\":\"file\",\"name\":\"app/file{index}.py\"}}"
        ));
    }
    let borrowed = lines.iter().map(String::as_str).collect::<Vec<_>>();
    let oversize = vec![shard(3, &borrowed)];
    assert_eq!(parse_snapshot(&oversize), Err(RepoGraphError::TooManyFacts));
}

#[test]
fn node_identity_is_stable_and_length_prefixed() {
    assert_eq!(
        node_id("engine", RepoFactKind::Symbol, "app/db.Database"),
        node_id("engine", RepoFactKind::Symbol, "app/db.Database")
    );
    assert_ne!(
        node_id("a", RepoFactKind::Symbol, "bc"),
        node_id("ab", RepoFactKind::Symbol, "c")
    );
    assert_ne!(
        node_id("engine", RepoFactKind::Symbol, "app/db.Database"),
        node_id("other", RepoFactKind::Symbol, "app/db.Database")
    );
    assert_ne!(
        node_id("engine", RepoFactKind::Symbol, "one"),
        node_id("engine", RepoFactKind::Symbol, "two")
    );

    let mut identities = std::collections::BTreeSet::new();
    for kind in RepoFactKind::ALL {
        assert!(identities.insert(node_id("engine", kind, "shared")));
        assert_eq!(RepoFactKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(identities.len(), 6);
    assert_eq!(RepoFactKind::parse("widget"), None);
}

#[test]
fn parsing_is_byte_identical_across_runs() {
    let shards = vec![
        shard(
            21,
            &[
                &header("engine", 4),
                "{\"kind\":\"storage\",\"name\":\"sqlite:main\"}",
                "{\"kind\":\"symbol\",\"name\":\"app/db.Database\"}",
            ],
        ),
        shard(
            22,
            &[
                "{\"kind\":\"file\",\"name\":\"app/db.py\"}",
                "{\"kind\":\"symbol\",\"name\":\"app/db.Database\"}",
            ],
        ),
    ];

    let first = parse_snapshot(&shards).expect("snapshot parses");
    let second = parse_snapshot(&shards).expect("snapshot parses");
    assert_eq!(first, second);
    assert_eq!(first.digest, second.digest);
    assert_eq!(first.skipped, 1);
    assert_eq!(first.facts.len(), 3);

    let ordering = first
        .facts
        .iter()
        .map(|fact| (fact.kind, fact.name.clone()))
        .collect::<Vec<_>>();
    let mut sorted = ordering.clone();
    sorted.sort();
    assert_eq!(ordering, sorted);
    assert_eq!(first.facts[1].source.lsn, 21);

    let left = first
        .facts
        .iter()
        .map(|fact| node_id(&first.repository, fact.kind, &fact.name))
        .collect::<Vec<_>>();
    let right = second
        .facts
        .iter()
        .map(|fact| node_id(&second.repository, fact.kind, &fact.name))
        .collect::<Vec<_>>();
    assert_eq!(left, right);
}
