#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId};
use hm_index::entity_rules::extract_entities;
use hm_mcp::{InspectInput, McpServer, RememberInput, RememberKind};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::Value;

const ALPHA_ONE: &str =
    "Northwind Traders shipped the ledger to https://example.com/reports today.";
const ALPHA_TWO: &str =
    "An audit for Northwind Traders lists example.com and /var/log/hypermind.log.";
const BETA_ONE: &str =
    "Contoso Logistics reviewed example.com for the Northwind Traders migration.";

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

async fn seeded(path: &std::path::Path) -> McpServer {
    let actor = ActorEngine::open(actor_config(path)).await.unwrap();
    let server = McpServer::new(actor);
    for (conversation, content) in [
        ("alpha", ALPHA_ONE),
        ("alpha", ALPHA_TWO),
        ("beta", BETA_ONE),
    ] {
        let stored = server
            .remember_envelope(RememberInput {
                conversation: conversation.to_owned(),
                content: content.to_owned(),
                kind: RememberKind::Document,
                chunk_bytes: None,
                anchor: None,
                retention: None,
                sensitivity: None,
            })
            .await;
        assert!(stored.ok, "remember failed: {:?}", stored.items);
    }
    server
}

fn surfaces<'a>(items: &'a [Value], surface: &str) -> Vec<&'a Value> {
    items
        .iter()
        .filter(|item| item["surface"].as_str() == Some(surface))
        .collect()
}

#[tokio::test]
async fn sources_index_groups_the_ledger_by_conversation() {
    let temporary = tempfile::tempdir().unwrap();
    let server = seeded(temporary.path()).await;

    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/sources".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "{:?}", envelope.items);
    let items = surfaces(&envelope.items, "sources");
    assert_eq!(items.len(), 2);

    let alpha_hex = ConversationId::derive("alpha").to_string();
    let beta_hex = ConversationId::derive("beta").to_string();
    assert_eq!(items[0]["conversation"].as_str(), Some(alpha_hex.as_str()));
    assert_eq!(items[1]["conversation"].as_str(), Some(beta_hex.as_str()));
    assert_eq!(alpha_hex.len(), 32);
    assert_eq!(items[0]["records"].as_u64(), Some(2));
    assert_eq!(items[1]["records"].as_u64(), Some(1));
    assert_eq!(items[0]["kinds"]["usermsg"].as_u64(), Some(2));
    assert_eq!(items[1]["kinds"]["usermsg"].as_u64(), Some(1));
    assert!(
        items[0]["first_lsn"].as_u64().unwrap() < items[1]["first_lsn"].as_u64().unwrap(),
        "conversations are ordered by ascending first_lsn"
    );
    assert!(items[0]["last_lsn"].as_u64().unwrap() >= items[0]["first_lsn"].as_u64().unwrap());
    assert!(items[0]["last_wall_timestamp_ns"].as_i64().unwrap() > 0);
    assert_eq!(items[0]["truncated"].as_bool(), Some(false));

    assert_eq!(envelope.provenance.len(), 2);
    assert_eq!(
        envelope.provenance[0],
        format!("hm://7/lsn/{}", items[0]["last_lsn"].as_u64().unwrap())
    );
    assert_eq!(
        envelope.provenance[1],
        format!("hm://7/lsn/{}", items[1]["last_lsn"].as_u64().unwrap())
    );
}

#[tokio::test]
async fn source_detail_reports_entities_and_shared_entities() {
    let temporary = tempfile::tempdir().unwrap();
    let server = seeded(temporary.path()).await;

    let alpha_hex = ConversationId::derive("alpha").to_string();
    let beta_hex = ConversationId::derive("beta").to_string();
    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://7/sources/{alpha_hex}")),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "{:?}", envelope.items);
    let items = surfaces(&envelope.items, "source_detail");
    assert_eq!(items.len(), 1);
    let item = items[0];

    assert_eq!(item["conversation"].as_str(), Some(alpha_hex.as_str()));
    assert_eq!(item["records"].as_u64(), Some(2));
    assert_eq!(item["extractor"].as_str(), Some("entity_rules"));
    assert_eq!(item["truncated"].as_bool(), Some(false));

    let expected = extract_entities(&format!("{ALPHA_ONE}\n{ALPHA_TWO}"));
    let reported = item["entities"].as_array().unwrap();
    assert_eq!(reported.len(), expected.len());
    for (entity, value) in expected.iter().zip(reported) {
        assert_eq!(value["canonical"].as_str(), Some(entity.canonical.as_str()));
        let aliases: Vec<&str> = value["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect();
        assert_eq!(aliases, entity.aliases);
        assert!(!value["kind"].as_str().unwrap().is_empty());
    }
    let first_lsn = item["first_lsn"].as_u64().unwrap();
    let last_lsn = item["last_lsn"].as_u64().unwrap();
    let carrying: Vec<&Value> = reported
        .iter()
        .filter(|value| !value["lsns"].as_array().unwrap().is_empty())
        .collect();
    assert!(!carrying.is_empty(), "entities cite contributing records");
    for value in carrying {
        for lsn in value["lsns"].as_array().unwrap() {
            let lsn = lsn.as_u64().unwrap();
            assert!((first_lsn..=last_lsn).contains(&lsn));
        }
    }

    let shared = item["shared_entities"].as_array().unwrap();
    let domain = shared
        .iter()
        .find(|value| value["canonical"].as_str() == Some("example.com"))
        .expect("example.com is shared with the other conversation");
    let conversations: Vec<&str> = domain["conversations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert!(conversations.len() >= 2);
    assert!(conversations.contains(&alpha_hex.as_str()));
    assert!(conversations.contains(&beta_hex.as_str()));
    for value in shared {
        assert!(value["conversations"].as_array().unwrap().len() >= 2);
    }

    let citations: Vec<&str> = item["citations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(citations.len(), 2);
    for citation in citations {
        assert!(citation.starts_with("hm://7/"), "{citation}");
        assert!(
            citation.starts_with(&format!("hm://7/{alpha_hex}/")),
            "{citation}"
        );
    }
}

#[tokio::test]
async fn source_detail_reports_an_empty_conversation() {
    let temporary = tempfile::tempdir().unwrap();
    let server = seeded(temporary.path()).await;

    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some(format!(
                "hm://7/sources/{}",
                ConversationId::derive("gamma")
            )),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "{:?}", envelope.items);
    let items = surfaces(&envelope.items, "source_detail");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["records"].as_u64(), Some(0));
    assert_eq!(items[0]["first_lsn"].as_u64(), Some(0));
    assert_eq!(items[0]["last_lsn"].as_u64(), Some(0));
    assert!(items[0]["entities"].as_array().unwrap().is_empty());
    assert!(items[0]["shared_entities"].as_array().unwrap().is_empty());
    assert!(items[0]["citations"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn source_detail_rejects_a_malformed_conversation() {
    let temporary = tempfile::tempdir().unwrap();
    let server = seeded(temporary.path()).await;

    for suffix in ["zz", "ABCDEF01234567890123456789ABCDEF", ""] {
        let envelope = server
            .inspect_envelope(InspectInput {
                uri: Some(format!("hm://7/sources/{suffix}")),
                ..InspectInput::default()
            })
            .await;
        assert!(!envelope.ok, "{suffix} must be refused");
        assert_eq!(
            envelope.items[0]["error"].as_str(),
            Some("kInvalidArgument")
        );
    }
}
