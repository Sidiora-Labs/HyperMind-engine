#![forbid(unsafe_code)]

use hm_core::ActorId;
use hm_mcp::{InspectInput, McpServer};
use hm_schema::events::BeliefType;
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::Value;

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

async fn access(server: &McpServer) -> Value {
    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/access".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    let mut matched = envelope
        .items
        .into_iter()
        .filter(|item| item["surface"].as_str() == Some("access"))
        .collect::<Vec<_>>();
    assert_eq!(matched.len(), 1);
    matched.remove(0)
}

fn belief_type_name(value: BeliefType) -> &'static str {
    match value {
        BeliefType::Fact => "fact",
        BeliefType::Preference => "preference",
        BeliefType::Constraint => "constraint",
        BeliefType::Goal => "goal",
        BeliefType::Identity => "identity",
    }
}

#[tokio::test]
async fn access_surface_reports_server_authoritative_permissions() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor);

    let item = access(&server).await;
    assert_eq!(item["source"].as_str(), Some("server"));
    assert_eq!(item["actor"].as_u64(), Some(7));
    assert_eq!(item["isolation"].as_str(), Some("actor_scoped"));
    assert_eq!(item["admin_token_configured"].as_bool(), Some(false));

    let operations = item["admin_operations"].as_array().unwrap();
    assert_eq!(operations.len(), hm_serve::auth::ADMIN_OPERATIONS.len());
    for (reported, expected) in operations
        .iter()
        .zip(hm_serve::auth::ADMIN_OPERATIONS.iter())
    {
        assert_eq!(reported.as_str(), Some(*expected));
    }

    let verbs = item["verbs"].as_array().unwrap();
    assert_eq!(verbs.len(), hm_serve::rest::VERBS.len());
    for (reported, expected) in verbs.iter().zip(hm_serve::rest::VERBS.iter()) {
        let verb = reported["verb"].as_str().unwrap();
        assert_eq!(verb, *expected);
        assert_eq!(
            reported["mutating"].as_bool(),
            Some(hm_serve::uds::mutation_verb(verb)),
            "verb {verb} misreported its mutation class"
        );
        assert_eq!(reported["admin_listener_required"].as_bool(), Some(false));
        let actions = reported["admin_token_required_actions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|action| action.as_str().unwrap())
            .collect::<Vec<_>>();
        if verb == "forget" {
            assert_eq!(actions, vec!["crypto_shred"]);
        } else {
            assert!(actions.is_empty(), "verb {verb} claimed an admin action");
        }
    }

    let protected = item["protected_belief_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    for belief_type in BeliefType::ENUM_VALUES {
        let name = belief_type_name(belief_type);
        assert_eq!(
            protected.iter().any(|listed| listed == name),
            hm_proj::protected::is_protected(belief_type),
            "belief type {name} was misclassified"
        );
    }
    assert_eq!(
        protected,
        vec![
            "preference".to_owned(),
            "constraint".to_owned(),
            "identity".to_owned()
        ]
    );
}

#[tokio::test]
async fn access_surface_reports_a_configured_admin_token() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new_with_admin(actor, [9; 32]);

    let item = access(&server).await;
    assert_eq!(item["admin_token_configured"].as_bool(), Some(true));
    assert_eq!(item["source"].as_str(), Some("server"));
}
