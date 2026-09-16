#![allow(clippy::missing_errors_doc)]

use hm_core::{ActorId, Error, ErrorCode};
use hm_mcp::{
    ActivateInput, BeliefClaimInput, BeliefTypeInput, BelieveInput, ClaimInput, McpServer,
    ProvenanceInput, RememberInput, RememberKind,
};
use hm_serve::actor::{ActorConfig, ActorEngine};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtectedResult {
    pub attempts: usize,
    pub rejected: usize,
    pub surfaced_proposals: usize,
}

pub async fn run() -> Result<ProtectedResult, Error> {
    let temporary = tempfile::tempdir().map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let actor = open_actor(temporary.path()).await?;
    let server = McpServer::new(actor.clone());
    let evidence = server
        .remember_envelope(RememberInput {
            conversation: "protected-suite".to_owned(),
            content: "run-derived candidate about the protected self".to_owned(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
        })
        .await;
    let evidence_lsn = evidence
        .items
        .first()
        .and_then(|item| item["first_lsn"].as_u64())
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
    let protected = [
        (BeliefTypeInput::Identity, "self:name", "HyperMind"),
        (
            BeliefTypeInput::Constraint,
            "self:constraint:network",
            "never access the network on recall",
        ),
        (
            BeliefTypeInput::Preference,
            "self:preference:format",
            "prefer concise output",
        ),
    ];
    let mut rejected = 0;
    for (index, (belief_type, identity, value)) in protected.into_iter().enumerate() {
        let result = server
            .believe_envelope(BelieveInput {
                conversation: "protected-suite".to_owned(),
                belief: BeliefClaimInput {
                    belief_id: format!("run-protected-{index}"),
                    belief_type,
                    canonical_identity: identity.to_owned(),
                    value: value.to_owned(),
                    valid_from_ns: 0,
                    valid_to_ns: 0,
                    provenance: vec![ProvenanceInput {
                        first_lsn: evidence_lsn,
                        last_lsn: evidence_lsn,
                        byte_start: 0,
                        byte_end: 46,
                    }],
                    conflict_domain: Some("protected-self".to_owned()),
                    claim: ClaimInput::Affirmative,
                },
                run_id: Some(format!("protected-run-{index}")),
            })
            .await;
        if !result.ok
            && result.effect_state.as_deref() == Some("rejected")
            && result
                .items
                .first()
                .is_some_and(|item| item["error"] == ErrorCode::ProtectedTypeWrite.as_str())
        {
            rejected += 1;
        }
    }
    let bundle = server
        .activate_envelope(ActivateInput {
            conversation: "protected-suite".to_owned(),
            query: "protected proposals".to_owned(),
            turn_text: String::new(),
            budget_tokens: 8192,
        })
        .await;
    if !bundle.ok {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let surfaced_proposals = bundle
        .items
        .iter()
        .filter(|item| {
            item["tier"] == "conflicts"
                && item["uri"]
                    .as_str()
                    .is_some_and(|uri| uri.contains("src=proposal"))
        })
        .count();
    drop(server);
    actor.shutdown().await?;
    Ok(ProtectedResult {
        attempts: protected.len(),
        rejected,
        surfaced_proposals,
    })
}

async fn open_actor(path: &std::path::Path) -> Result<ActorEngine, Error> {
    ActorEngine::open(ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 32 * 1024 * 1024,
    })
    .await
}
