#![allow(clippy::missing_errors_doc)]

use hm_cortex::citations::{FrozenCandidate, SourceKind};
use hm_cortex::nrem::cluster::{ObservationCluster, PendingObservation};
use hm_cortex::nrem::merge::{DropReason, ExistingMemory, consolidate_clusters, merge_request};
use hm_cortex::quality::check_rewrite;
use hm_cortex::rem::r#abstract::{
    AbstractDrop, AbstractError, AbstractOptions, ExistingAbstract, abstract_request, synthesize,
};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, StructuredRequest, WireFixture,
    WireRequest, WireResponse,
};
use hm_schema::events::Authority;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DreamResult {
    pub regression_cases: usize,
    pub lossy_rewrites: usize,
    pub preserved_cases: usize,
    pub duplicate_abstractions: usize,
    pub valid_mints: usize,
    pub ungrounded_mints: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct Incident {
    id: String,
    name: String,
    original: String,
    concern: String,
    lossy_revision: String,
    required: Vec<String>,
}

pub fn run() -> Result<DreamResult, Box<dyn std::error::Error>> {
    let incidents: Vec<Incident> = serde_json::from_slice(&std::fs::read(fixture_path())?)?;
    let mut result = DreamResult {
        regression_cases: incidents.len(),
        ..DreamResult::default()
    };
    for incident in &incidents {
        let lossy = check_rewrite(
            &incident.name,
            &incident.original,
            &incident.lossy_revision,
            &[&incident.concern],
        );
        result.lossy_rewrites += usize::from(lossy.accepted);
        let preserved = format!("{} {}", incident.original, incident.concern);
        let guard = check_rewrite(
            &incident.name,
            &incident.original,
            &preserved,
            &[&incident.concern],
        );
        if guard.accepted
            && incident
                .required
                .iter()
                .all(|required| preserved.contains(required))
        {
            result.preserved_cases += 1;
        }
        if incident.id.is_empty() {
            return Err("dream incident id is empty".into());
        }
    }

    result.duplicate_abstractions = duplicate_abstraction_count()?;
    let (valid_mints, ungrounded_mints) = mint_independence_counts()?;
    result.valid_mints = valid_mints;
    result.ungrounded_mints = ungrounded_mints;
    Ok(result)
}

fn duplicate_abstraction_count() -> Result<usize, Box<dyn std::error::Error>> {
    let sources = [
        source(20, 1, 1, "Refresh tokens rotate after every use."),
        source(21, 2, 2, "Rotation failures are recorded in the ops log."),
    ];
    let output = json!({
        "name": "Audited token rotation",
        "definition": "Refresh tokens rotate after every use, and rotation failures are recorded in the ops log.",
        "citations": [
            {"lsn": 20, "byte_start": 0, "byte_end": 38, "quote": "tokens rotate"},
            {"lsn": 21, "byte_start": 0, "byte_end": 46, "quote": "Rotation failures"}
        ]
    });
    let request = abstract_request("auth", &sources)
        .map_err(|error| format!("abstract fixture request failed: {error:?}"))?;
    let provider = recorded_provider(vec![fixture(&request, &output)]);
    let first = synthesize(
        &provider,
        b"dream-run",
        "auth",
        &sources,
        &[],
        &[],
        AbstractOptions {
            maximum_similarity_micros: 800_000,
            minimum_grounding_micros: 100_000,
        },
    )
    .map_err(|error| format!("abstract fixture synthesis failed: {error:?}"))?;
    let provider = recorded_provider(vec![fixture(&request, &output)]);
    let second = synthesize(
        &provider,
        b"dream-run",
        "auth",
        &sources,
        &[ExistingAbstract {
            definition: String::from_utf8(first.definition)?,
            faded: false,
        }],
        &[],
        AbstractOptions {
            maximum_similarity_micros: 800_000,
            minimum_grounding_micros: 100_000,
        },
    );
    Ok(usize::from(!matches!(
        second,
        Err(AbstractError::Drop(AbstractDrop::Duplicate { .. }))
    )))
}

fn mint_independence_counts() -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let valid = ObservationCluster {
        cluster_id: [1; 32],
        priority: 2,
        observations: vec![
            observation(1, 1, 1, "Refresh tokens rotate after every use."),
            observation(2, 2, 2, "Rotation failures are recorded in the ops log."),
            observation(3, 2, 3, "Refresh tokens are stored hashed in SQLite."),
        ],
    };
    let insufficient = ObservationCluster {
        cluster_id: [2; 32],
        priority: 1,
        observations: vec![
            observation(4, 3, 4, "The primary region is eu-central."),
            observation(5, 4, 5, "The deployment targets eu-central."),
        ],
    };
    let outputs = [
        json!({
            "action": "mint",
            "target": null,
            "name": "Audited token rotation",
            "definition": "Refresh tokens rotate after every use, are stored hashed in SQLite, and rotation failures are recorded in the ops log.",
            "tags": ["auth"],
            "salience_micros": 800000,
            "citations": [
                {"lsn": 1, "byte_start": 0, "byte_end": 38, "quote": "tokens rotate"},
                {"lsn": 2, "byte_start": 0, "byte_end": 46, "quote": "Rotation failures"},
                {"lsn": 3, "byte_start": 0, "byte_end": 43, "quote": "stored hashed in SQLite"}
            ]
        }),
        json!({
            "action": "mint",
            "target": null,
            "name": "Deployment region",
            "definition": "The deployment targets eu-central as its primary region.",
            "tags": ["deployment"],
            "salience_micros": 700000,
            "citations": [
                {"lsn": 4, "byte_start": 0, "byte_end": 33, "quote": "primary region"},
                {"lsn": 5, "byte_start": 0, "byte_end": 34, "quote": "targets eu-central"}
            ]
        }),
    ];
    let clusters = [valid, insufficient];
    let fixtures = clusters
        .iter()
        .zip(&outputs)
        .map(|(cluster, output)| fixture(&merge_request(cluster, &[]).unwrap(), output))
        .collect();
    let provider = recorded_provider(fixtures);
    let report = consolidate_clusters(&provider, b"dream-run", &clusters, &[] as &[ExistingMemory])
        .map_err(|error| format!("NREM fixture failed: {error:?}"))?;
    let ungrounded = report
        .dropped
        .iter()
        .filter(|drop| {
            !matches!(
                drop.reason,
                DropReason::InsufficientIndependentRoots | DropReason::InsufficientConversations
            )
        })
        .count();
    Ok((report.decisions.len(), ungrounded))
}

fn observation(lsn: u64, conversation: u8, root: u8, content: &str) -> PendingObservation {
    PendingObservation {
        source: source(lsn, conversation, root, content),
        salience_micros: 800_000,
        event_time_ns: i64::try_from(lsn).unwrap_or(i64::MAX),
        embedding: vec![10, 1],
        entities: vec!["fixture".to_owned()],
    }
}

fn source(lsn: u64, conversation: u8, root: u8, content: &str) -> FrozenCandidate {
    let mut conversation_id = [0; 16];
    conversation_id[0] = conversation;
    let mut source_root = [0; 32];
    source_root[0] = root;
    FrozenCandidate {
        lsn,
        conversation: conversation_id,
        source_root,
        content: content.as_bytes().to_vec(),
        kind: SourceKind::Declarative,
        authority: Authority::UserAsserted,
    }
}

fn recorded_provider(fixtures: Vec<WireFixture>) -> OpenAiCompatible<RecordedTransport> {
    OpenAiCompatible::new(
        ProviderConfig {
            endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
            api_key: Some("fixture-key".to_owned()),
            model: "fixture-model".to_owned(),
            tier: ModelTier::Capable,
            pricing: Pricing {
                input_microusd_per_million_tokens: 1_000_000,
                output_microusd_per_million_tokens: 2_000_000,
            },
        },
        RecordedTransport::new(fixtures),
    )
    .expect("valid recorded provider")
}

fn fixture(request: &StructuredRequest, output: &Value) -> WireFixture {
    WireFixture {
        request: WireRequest {
            method: "POST".to_owned(),
            url: "https://fixture.invalid/v1/chat/completions".to_owned(),
            headers: BTreeMap::from([
                ("authorization".to_owned(), "Bearer fixture-key".to_owned()),
                ("content-type".to_owned(), "application/json".to_owned()),
            ]),
            body: json!({
                "model": "fixture-model",
                "messages": [
                    {"role": "system", "content": request.system},
                    {"role": "user", "content": request.prompt}
                ],
                "max_tokens": request.maximum_output_tokens,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "name": request.prompt_id,
                        "strict": true,
                        "schema": request.json_schema
                    }
                }
            }),
        },
        response: WireResponse {
            status: 200,
            body: json!({
                "choices": [{"message": {"content": output.to_string()}}],
                "usage": {
                    "prompt_tokens": 100,
                    "completion_tokens": 25,
                    "prompt_tokens_details": {"cached_tokens": 0}
                }
            }),
        },
    }
}

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/fixtures/dream/cortex-incidents.json")
}
