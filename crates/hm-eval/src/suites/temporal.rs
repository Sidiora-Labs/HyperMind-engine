#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use hm_compose::bundle::ActivationBundle;
use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, Error, ErrorCode, LSN};
use hm_mcp::{
    BeliefClaimInput, BeliefTypeInput, BelieveInput, ClaimInput, McpServer, ProvenanceInput,
    RememberInput, RememberKind,
};
use hm_proj::beliefs::BeliefAsOf;
use hm_schema::events::BeliefType;
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    conversation: String,
    canonical_identity: String,
    conflict_domain: String,
    evidence: String,
    versions: Vec<Version>,
    valid_query: Query,
    known_query: Query,
}

#[derive(Clone, Debug, Deserialize)]
struct Version {
    belief_id: String,
    value: String,
    valid_from_ns: i64,
    valid_to_ns: i64,
}

#[derive(Clone, Debug, Deserialize)]
struct Query {
    at: i64,
    expected: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemporalResult {
    pub axis_cases: usize,
    pub axis_correct: usize,
    pub stale_claims: usize,
    pub stated_claims: usize,
}

impl TemporalResult {
    #[must_use]
    pub fn stale_fact_rate(self) -> f64 {
        if self.stated_claims == 0 {
            0.0
        } else {
            self.stale_claims as f64 / self.stated_claims as f64
        }
    }
}

pub async fn run() -> Result<TemporalResult, Error> {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../eval/fixtures/temporal/supersessions.json"
    ))
    .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    if fixture.versions.len() < 2 {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let temporary = tempfile::tempdir().map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let actor = open_actor(temporary.path()).await?;
    let server = McpServer::new(actor.clone());
    write_fixture(&server, &fixture).await?;
    let valid = actor
        .as_of(
            BeliefType::Fact,
            fixture.canonical_identity.clone(),
            BeliefAsOf::ValidAt(fixture.valid_query.at),
        )
        .await?
        .record;
    let known = actor
        .as_of(
            BeliefType::Fact,
            fixture.canonical_identity.clone(),
            BeliefAsOf::KnownAt(LSN::new(
                u64::try_from(fixture.known_query.at)
                    .map_err(|_| Error::new(ErrorCode::InvalidArgument))?,
            )),
        )
        .await?
        .record;
    let valid_value = valid
        .as_ref()
        .and_then(|record| std::str::from_utf8(&record.value).ok());
    let known_value = known
        .as_ref()
        .and_then(|record| std::str::from_utf8(&record.value).ok());
    let axis_correct = usize::from(valid_value == Some(fixture.valid_query.expected.as_str()))
        + usize::from(known_value == Some(fixture.known_query.expected.as_str()));
    if valid_value == known_value {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let bundle = actor
        .activate(ActivateRequest {
            conversation: hm_core::ConversationId::derive(&fixture.conversation),
            query: "current deployment".to_owned(),
            turn_text: String::new(),
            budget_tokens: 4096,
            token_weights: FallbackWeights::default(),
        })
        .await?;
    let current = fixture
        .versions
        .last()
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
    let superseded: Vec<&str> = fixture.versions[..fixture.versions.len() - 1]
        .iter()
        .map(|version| version.value.as_str())
        .collect();
    if count_stale([superseded[0].as_bytes()], &superseded, &current.value) != (1, 1) {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let (stale_claims, stated_claims) = bundle_stale(&bundle, &superseded, &current.value);
    drop(server);
    actor.shutdown().await?;
    Ok(TemporalResult {
        axis_cases: 2,
        axis_correct,
        stale_claims,
        stated_claims,
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

async fn write_fixture(server: &McpServer, fixture: &Fixture) -> Result<(), Error> {
    let evidence = server
        .remember_envelope(RememberInput {
            conversation: fixture.conversation.clone(),
            content: fixture.evidence.clone(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
            document: None,
            source_sync: None,
        })
        .await;
    let evidence_lsn = envelope_lsn(&evidence)?;
    for version in &fixture.versions {
        let written = server
            .believe_envelope(BelieveInput {
                conversation: fixture.conversation.clone(),
                belief: BeliefClaimInput {
                    belief_id: version.belief_id.clone(),
                    belief_type: BeliefTypeInput::Fact,
                    canonical_identity: fixture.canonical_identity.clone(),
                    value: version.value.clone(),
                    valid_from_ns: version.valid_from_ns,
                    valid_to_ns: version.valid_to_ns,
                    provenance: vec![ProvenanceInput {
                        first_lsn: evidence_lsn,
                        last_lsn: evidence_lsn,
                        byte_start: 0,
                        byte_end: u32::try_from(fixture.evidence.len())
                            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
                    }],
                    conflict_domain: Some(fixture.conflict_domain.clone()),
                    claim: ClaimInput::Affirmative,
                },
                run_id: None,
            })
            .await;
        envelope_lsn(&written)?;
    }
    Ok(())
}

fn envelope_lsn(envelope: &hm_mcp::Envelope) -> Result<u64, Error> {
    if !envelope.ok {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    envelope
        .items
        .first()
        .and_then(|item| item.get("lsn").or_else(|| item.get("first_lsn")))
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))
}

fn bundle_stale(bundle: &ActivationBundle, superseded: &[&str], current: &str) -> (usize, usize) {
    count_stale(
        bundle
            .sections
            .iter()
            .flat_map(|section| section.items.iter())
            .map(|item| item.content.as_slice()),
        superseded,
        current,
    )
}

fn count_stale<'a>(
    contents: impl IntoIterator<Item = &'a [u8]>,
    superseded: &[&str],
    current: &str,
) -> (usize, usize) {
    let mut stale = 0;
    let mut stated = 0;
    for content in contents {
        let Ok(content) = std::str::from_utf8(content) else {
            continue;
        };
        if content.contains(current) {
            stated += 1;
        }
        for value in superseded {
            if content.contains(value) {
                stale += 1;
                stated += 1;
            }
        }
    }
    (stale, stated)
}
