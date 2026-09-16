#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_proj::vocabulary::VocabularyProjection;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg, VocabularyCategory,
    VocabularyImported, VocabularyTerm,
};
use tempfile::tempdir;

const VOCABULARY_ID: &[u8] = b"fleet-vocabulary";

fn frame(lsn: u64, kind: EventKind, payload: EventPayload) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap() * 1_000),
            actor: ActorId::new(7),
            conversation: ConversationId::new([0x61; 16]),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::UserAsserted,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: i64::try_from(lsn).unwrap() * 1_000,
        }),
    }
}

fn term(
    term_id: &str,
    canonical_name: &str,
    category: VocabularyCategory,
    aliases: &[&str],
) -> VocabularyTerm {
    VocabularyTerm {
        term_id: term_id.to_owned(),
        canonical_name: canonical_name.to_owned(),
        category,
        parent_term_id: None,
        aliases: Some(aliases.iter().map(|alias| (*alias).to_owned()).collect()),
    }
}

fn imported(lsn: u64, version: u16, terms: Vec<VocabularyTerm>) -> Frame {
    frame(
        lsn,
        EventKind::VocabularyImported,
        EventPayload::VocabularyImported(Box::new(VocabularyImported {
            vocabulary_id: VOCABULARY_ID.to_vec(),
            version,
            source_uri: "file:///vocabularies/fleet.nt".to_owned(),
            source_media_type: "application/n-triples".to_owned(),
            source_digest: vec![0x5a; 32],
            terms,
            ignored_triples: 4,
        })),
    )
}

fn frames() -> Vec<Frame> {
    vec![
        frame(
            1,
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"import the fleet vocabulary".to_vec(),
            })),
        ),
        imported(
            2,
            1,
            vec![
                term(
                    "hm:vocab/company",
                    "Company",
                    VocabularyCategory::EntityType,
                    &[],
                ),
                term(
                    "hm:vocab/works_for",
                    "works_for",
                    VocabularyCategory::Relation,
                    &["employs"],
                ),
            ],
        ),
        imported(
            3,
            1,
            vec![term(
                "hm:vocab/rival",
                "rival",
                VocabularyCategory::Relation,
                &["competes with"],
            )],
        ),
        imported(
            4,
            2,
            vec![
                term(
                    "hm:vocab/company",
                    "Company",
                    VocabularyCategory::EntityType,
                    &[],
                ),
                term(
                    "hm:vocab/works_for",
                    "works_for",
                    VocabularyCategory::Relation,
                    &["employed by"],
                ),
            ],
        ),
    ]
}

#[test]
fn vocabulary_projects_versioned_terms_and_exact_aliases() {
    let temporary = tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).unwrap();
    let frames = frames();

    rebuild_projection_stream(&store, &frames[..2], false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        snapshot.checkpoint(ProjectionId::Vocabulary).unwrap().get(),
        2
    );
    let head = VocabularyProjection::vocabulary(&snapshot, VOCABULARY_ID)
        .unwrap()
        .unwrap();
    assert_eq!(head.vocabulary_id, VOCABULARY_ID.to_vec());
    assert_eq!(head.version, 1);
    assert_eq!(head.source_uri, "file:///vocabularies/fleet.nt");
    assert_eq!(head.source_media_type, "application/n-triples");
    assert_eq!(head.source_digest, vec![0x5a; 32]);
    assert_eq!(head.term_count, 2);
    assert_eq!(head.ignored_triples, 4);
    assert_eq!(head.event_lsn, 2);

    let alias = VocabularyProjection::resolve(&snapshot, "employs", 8).unwrap();
    assert_eq!(alias.len(), 1);
    assert_eq!(alias[0].term_id, "hm:vocab/works_for");
    assert_eq!(alias[0].category, VocabularyCategory::Relation);
    assert_eq!(alias[0].version, 1);
    let canonical = VocabularyProjection::resolve(&snapshot, "Works For", 8).unwrap();
    assert_eq!(canonical, alias);
    assert!(
        VocabularyProjection::resolve(&snapshot, "employ", 8)
            .unwrap()
            .is_empty()
    );
    drop(snapshot);

    rebuild_projection_stream(&store, &frames[..3], false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        snapshot.checkpoint(ProjectionId::Vocabulary).unwrap().get(),
        3
    );
    assert_eq!(
        VocabularyProjection::vocabulary(&snapshot, VOCABULARY_ID)
            .unwrap()
            .unwrap(),
        head
    );
    assert!(
        VocabularyProjection::resolve(&snapshot, "rival", 8)
            .unwrap()
            .is_empty()
    );
    assert!(
        VocabularyProjection::resolve(&snapshot, "competes_with", 8)
            .unwrap()
            .is_empty()
    );
    let terms = VocabularyProjection::terms(&snapshot, 8).unwrap();
    assert_eq!(terms.len(), 2);
    assert_eq!(terms[0].term_id, "hm:vocab/company");
    assert_eq!(terms[1].term_id, "hm:vocab/works_for");
    drop(snapshot);

    rebuild_projection_stream(&store, &frames, false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        snapshot.checkpoint(ProjectionId::Vocabulary).unwrap().get(),
        4
    );
    let head = VocabularyProjection::vocabulary(&snapshot, VOCABULARY_ID)
        .unwrap()
        .unwrap();
    assert_eq!(head.version, 2);
    assert_eq!(head.event_lsn, 4);
    let revised = VocabularyProjection::resolve(&snapshot, "employed by", 8).unwrap();
    assert_eq!(revised.len(), 1);
    assert_eq!(revised[0].term_id, "hm:vocab/works_for");
    assert_eq!(revised[0].version, 2);
    assert_eq!(revised[0].aliases, vec!["employed by".to_owned()]);
    assert_eq!(
        VocabularyProjection::resolve(&snapshot, "employed_by", 8).unwrap(),
        revised
    );
    assert!(
        VocabularyProjection::resolve(&snapshot, "employs", 8)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        VocabularyProjection::resolve(&snapshot, "works_for", 8).unwrap(),
        revised
    );
    let terms = VocabularyProjection::terms(&snapshot, 8).unwrap();
    assert_eq!(terms.len(), 2);
    assert!(terms.iter().all(|record| record.version == 2));
    assert_eq!(terms[0].term_id, "hm:vocab/company");
    assert_eq!(terms[1].term_id, "hm:vocab/works_for");
    let listed = VocabularyProjection::list(&snapshot, 8).unwrap();
    assert_eq!(listed, vec![head]);

    for code in [
        VocabularyProjection::list(&snapshot, 0).unwrap_err().code,
        VocabularyProjection::terms(&snapshot, 0).unwrap_err().code,
        VocabularyProjection::resolve(&snapshot, "works_for", 0)
            .unwrap_err()
            .code,
        VocabularyProjection::vocabulary(&snapshot, &[])
            .unwrap_err()
            .code,
    ] {
        assert_eq!(code, ErrorCode::InvalidArgument);
    }
}

#[test]
fn vocabulary_rebuild_is_byte_identical() {
    let temporary = tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).unwrap();
    let frames = frames();

    let progress = rebuild_projection_stream(&store, &frames, false, usize::MAX).unwrap();
    assert!(progress.complete);
    let snapshot = store.begin_snapshot().unwrap();
    let incremental = snapshot.canonical_dump(ProjectionId::Vocabulary).unwrap();
    drop(snapshot);

    let progress = rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    assert!(progress.complete);
    let snapshot = store.begin_snapshot().unwrap();
    let rebuilt = snapshot.canonical_dump(ProjectionId::Vocabulary).unwrap();
    assert_eq!(rebuilt, incremental);
}
