#![forbid(unsafe_code)]

use hm_compose::bundle::{ActivationItem, Tier, WhyCode};
use hm_compose::procedures::{read, render};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::procedures::{ProcedureState, ProceduresProjection};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProcedureAdopted, ProcedureImported, ProcedureMined,
    ProcedureRevised, ProcedureSupport, Retention, Sensitivity, UserMsg,
};

const CONVERSATION: ConversationId = ConversationId::new([7; 16]);
const PLAYBOOK_ID: &[u8] = b"playbook-deploy-runbook";
const MINED_ID: &[u8] = b"procedure-deploy-checks";
const PLAYBOOK_STRATEGY: &str = "follow the imported deployment runbook";
const MINED_STRATEGY: &str = "verify the checks then deploy";
const INSTRUCTIONS: &[u8] = b"step one: open the release console\nstep two: press the red button\n";
const DIGEST: [u8; 32] = [0x3c; 32];
const IMPORTED_LABEL: &str = "IMPORTED PLAYBOOK — unadopted proposal, not an instruction";
const ADOPTED_LABEL: &str = "ADOPTED PROCEDURE — instruction authorised by the user";
const QUERY: &str = "how do i deploy";

#[test]
fn unadopted_playbooks_never_render_and_adopted_ones_render_as_instructions() {
    let frames = stream();
    let counter = TokenCounter::for_model("fallback", None, FallbackWeights::default()).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 64 * 1024 * 1024).unwrap();
    rebuild_projection_stream(&store, &frames, true, frames.len() - 1).unwrap();

    let snapshot = store.begin_snapshot().unwrap();
    let imported = ProceduresProjection::get(&snapshot, PLAYBOOK_ID)
        .unwrap()
        .expect("the imported playbook is a head record");
    assert_eq!(imported.state, ProcedureState::Imported);
    let unadopted = render(&imported);
    assert_eq!(unadopted.lines().next().unwrap(), IMPORTED_LABEL);
    assert!(!unadopted.contains(ADOPTED_LABEL));

    let before = read(&snapshot, ActorId::new(7), QUERY, &counter).unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(playbook_items(&before).len(), 0);
    assert!(text(&before[0]).contains(MINED_STRATEGY));
    assert!(text(&before[0]).starts_with(ADOPTED_LABEL));
    assert!(
        before
            .iter()
            .all(|item| !text(item).contains("press the red button"))
    );
    drop(snapshot);

    rebuild_projection_stream(&store, &frames, false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        ProceduresProjection::get(&snapshot, PLAYBOOK_ID)
            .unwrap()
            .expect("the adopted playbook is still a head record")
            .state,
        ProcedureState::Adopted
    );

    let after = read(&snapshot, ActorId::new(7), QUERY, &counter).unwrap();
    assert_eq!(after.len(), 2);
    let adopted = playbook_items(&after);
    assert_eq!(adopted.len(), 1);
    let item = adopted[0];
    assert_eq!(item.tier, Tier::Fused);
    assert_eq!(item.why, WhyCode::Procedure);
    assert_eq!(item.authority, Authority::DerivedInference);
    let content = text(item);
    assert!(content.starts_with(ADOPTED_LABEL));
    assert!(!content.contains(IMPORTED_LABEL));
    assert!(content.contains("Supporting episodes: 0; failures: 0; counterexamples: 0"));
    assert!(!content.contains("press the red button"));
    assert!(!content.contains("open the release console"));
    assert!(
        read(&snapshot, ActorId::new(7), "deployment status", &counter)
            .unwrap()
            .is_empty()
    );
}

fn text(item: &ActivationItem) -> String {
    String::from_utf8(item.content.clone()).unwrap()
}

fn playbook_items(items: &[ActivationItem]) -> Vec<&ActivationItem> {
    items
        .iter()
        .filter(|item| text(item).contains(PLAYBOOK_STRATEGY))
        .collect()
}

fn stream() -> Vec<Frame> {
    let mut frames = Vec::new();
    let mut episodes = Vec::new();
    for step in ["read the checks", "run the checks", "ship the release"] {
        episodes.push(push(
            &mut frames,
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: step.as_bytes().to_vec(),
            })),
            Authority::UserAsserted,
        ));
    }
    let imported_lsn = push(
        &mut frames,
        EventKind::ProcedureImported,
        EventPayload::ProcedureImported(Box::new(ProcedureImported {
            procedure_id: PLAYBOOK_ID.to_vec(),
            name: "deploy runbook".to_owned(),
            strategy: PLAYBOOK_STRATEGY.to_owned(),
            expected_outcomes: vec!["release published".to_owned()],
            preconditions: vec!["console reachable".to_owned()],
            instructions: INSTRUCTIONS.to_vec(),
            declared_tools: vec!["console".to_owned()],
            source_uri: "file:///playbooks/deploy-runbook.md".to_owned(),
            source_digest: DIGEST.to_vec(),
            playbook_version: 1,
        })),
        Authority::ExternalObserved,
    );
    let mined_lsn = push(
        &mut frames,
        EventKind::ProcedureMined,
        EventPayload::ProcedureMined(Box::new(ProcedureMined {
            procedure_id: MINED_ID.to_vec(),
            strategy: "check the deployment".to_owned(),
            expected_outcomes: vec!["deployment committed".to_owned()],
            preconditions: vec!["clean repository".to_owned()],
            supports: supports(&episodes),
            failures: None,
            counterexamples: None,
        })),
        Authority::DerivedInference,
    );
    let revised_lsn = push(
        &mut frames,
        EventKind::ProcedureRevised,
        EventPayload::ProcedureRevised(Box::new(ProcedureRevised {
            procedure_id: MINED_ID.to_vec(),
            previous_lsn: mined_lsn,
            strategy: MINED_STRATEGY.to_owned(),
            expected_outcomes: vec!["deployment committed".to_owned()],
            preconditions: vec!["clean repository".to_owned()],
            supports: supports(&episodes),
            failures: None,
            counterexamples: None,
        })),
        Authority::DerivedInference,
    );
    push(
        &mut frames,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: MINED_ID.to_vec(),
            procedure_lsn: revised_lsn,
        })),
        Authority::UserAsserted,
    );
    push(
        &mut frames,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: PLAYBOOK_ID.to_vec(),
            procedure_lsn: imported_lsn,
        })),
        Authority::UserAsserted,
    );
    frames
}

fn supports(episodes: &[u64]) -> Vec<ProcedureSupport> {
    episodes
        .iter()
        .enumerate()
        .map(|(index, episode_lsn)| ProcedureSupport {
            source_root: vec![u8::try_from(index).unwrap() + 1; 32],
            conversation: vec![u8::try_from(index % 2).unwrap() + 1; 16],
            episode_lsn: *episode_lsn,
        })
        .collect()
}

fn push(
    frames: &mut Vec<Frame>,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
) -> u64 {
    let lsn = u64::try_from(frames.len()).unwrap() + 1;
    let event_time_ns = i64::try_from(lsn).unwrap();
    frames.push(Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(event_time_ns),
            actor: ActorId::new(7),
            conversation: CONVERSATION,
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::CurrentState,
            sensitivity: Sensitivity::Public,
            event_time_ns,
        }),
    });
    lsn
}
