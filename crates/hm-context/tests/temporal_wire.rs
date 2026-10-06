use hm_context::{
    notes::{
        AttributeProposal, Note, NoteKind, NotesCommand, NotesEvent, NotesProjection, Predicate,
    },
    temporal::{ClockProvenance, SourceIdentity, TemporalContext, TemporalSource, TemporalTimes},
    Authority, MessagePart, MessageRole, Scope, SourceMessage,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const CURRENT: i64 = 1_791_288_000_123_456_789;
const NS_FIELDS: [&str; 5] = [
    "occurred_at_ns",
    "recorded_at_ns",
    "valid_from_ns",
    "valid_until_ns",
    "verified_at_ns",
];
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn times() -> TemporalTimes {
    TemporalTimes {
        occurred_at_ns: Some(CURRENT - 1),
        recorded_at_ns: CURRENT,
        valid_from_ns: Some(i64::MIN),
        valid_until_ns: Some(i64::MAX),
        verified_at_ns: Some(CURRENT + 1),
    }
}
fn source() -> SourceMessage {
    let mut message = SourceMessage {
        id: "m1".into(),
        ordinal: 0,
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            text: "hello λ".into(),
        }],
        occurred_at_ns: Some(1_791_287_999_123_456_789),
        recorded_at_ns: CURRENT,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    message
}

#[test]
fn exact_nanoseconds_round_trip_all_temporal_fields_and_source_digest() {
    let original = times();
    original.validate().unwrap();
    let wire = serde_json::to_value(&original).unwrap();
    for (field, value) in [
        ("occurred_at_ns", CURRENT - 1),
        ("recorded_at_ns", CURRENT),
        ("valid_from_ns", i64::MIN),
        ("valid_until_ns", i64::MAX),
        ("verified_at_ns", CURRENT + 1),
    ] {
        assert_eq!(wire[field], value.to_string());
    }
    assert_eq!(
        serde_json::from_value::<TemporalTimes>(wire).unwrap(),
        original
    );
    let original = source();
    assert_eq!(
        original.source_digest,
        "78af0610090d418ca2335976c873cfcc7647f9ce431f98696040d8fa1660500c"
    );
    let wire = serde_json::to_vec(&original).unwrap();
    let decoded: SourceMessage = serde_json::from_slice(&wire).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn unsafe_legacy_numbers_and_noncanonical_dates_fail_for_every_field() {
    let base = serde_json::to_value(times()).unwrap();
    let bad = [
        json!(CURRENT),
        json!(-CURRENT),
        json!(9_007_199_254_740_992_i64),
        json!(1.5),
        json!("01"),
        json!("-0"),
        json!("+1"),
        json!(" 1"),
        json!("1e3"),
        json!("9223372036854775808"),
        json!("-9223372036854775809"),
    ];
    for field in NS_FIELDS {
        for value in &bad {
            let mut wire = base.clone();
            wire[field] = value.clone();
            assert!(
                serde_json::from_value::<TemporalTimes>(wire).is_err(),
                "{field}: {value}"
            );
        }
        for number in [0, 9_007_199_254_740_991_i64, -9_007_199_254_740_991_i64] {
            let mut wire = base.clone();
            wire[field] = json!(number);
            let decoded: TemporalTimes = serde_json::from_value(wire).unwrap();
            assert_eq!(
                serde_json::to_value(decoded).unwrap()[field],
                number.to_string()
            );
        }
        let mut wire = base.clone();
        wire[field] = Value::Null;
        assert_eq!(
            serde_json::from_value::<TemporalTimes>(wire).is_ok(),
            field != "recorded_at_ns"
        );
        let mut wire = base.clone();
        wire[field] = json!(1.0);
        assert!(serde_json::from_value::<TemporalTimes>(wire).is_err());
    }
    let unknown: TemporalTimes = serde_json::from_value(json!({"recorded_at_ns":"0"})).unwrap();
    assert_eq!(unknown.occurred_at_ns, None);
    assert_eq!(unknown.verified_at_ns, None);
    assert_eq!(unknown.valid_from_ns, None);
    assert_eq!(unknown.valid_until_ns, None);
}

#[test]
fn clocks_offsets_and_unknown_dates_preserve_their_meaning() {
    let mut message = source();
    message.occurred_at_ns = None;
    message.source_digest = message.computed_digest().unwrap();
    let temporal = TemporalSource::from_message(&message).unwrap();
    let expected = vec![
        temporal.source.clone(),
        SourceIdentity {
            id: "missing".into(),
            digest: "a".repeat(64),
            ordinal: 1,
        },
    ];
    let unknown = TemporalContext::build_with_offset(
        scope(),
        "session".into(),
        None,
        vec![temporal.clone()],
        expected.clone(),
    )
    .unwrap();
    assert_eq!(unknown.utc_offset_seconds, None);
    assert_eq!(
        unknown.clock_provenance[0].occurred_at,
        ClockProvenance::Unknown
    );
    assert_eq!(
        unknown.clock_provenance[0].recorded_at,
        ClockProvenance::SourceDeclaredUtc
    );
    assert_eq!(
        unknown.clock_provenance[0].verified_at,
        ClockProvenance::Unknown
    );
    let local = TemporalContext::build_with_offset(
        scope(),
        "session".into(),
        Some(19800),
        vec![temporal.clone()],
        expected.clone(),
    )
    .unwrap();
    assert_eq!(local.utc_offset_seconds, Some(19800));
    assert_eq!(local.gaps, unknown.gaps);
    assert_eq!(local.sources[0].times.occurred_at_ns, None);
    let wire = serde_json::to_value(&unknown).unwrap();
    assert_eq!(wire["utc_offset_seconds"], Value::Null);
    assert_eq!(
        wire["clock_provenance"][0]["recorded_at"],
        "source_declared_utc"
    );
    assert_eq!(
        serde_json::from_value::<TemporalContext>(wire).unwrap(),
        unknown
    );
    for offset in [-86401, 86401] {
        assert!(TemporalContext::build_with_offset(
            scope(),
            "session".into(),
            Some(offset),
            vec![temporal.clone()],
            expected.clone()
        )
        .is_err());
    }
    let known = times();
    assert_eq!(
        known.local_occurrence_ns(-3600).unwrap(),
        Some(CURRENT - 1 - 3_600_000_000_000)
    );
}

fn note(id: &str, kind: NoteKind, expires: Option<i64>) -> Note {
    Note {
        id: id.into(),
        kind,
        revision: 1,
        text: "Exact evidence".into(),
        parents: BTreeSet::new(),
        contradictions: BTreeSet::new(),
        expires_at_ns: expires,
        predicate: Predicate::True,
        tombstoned: false,
    }
}
#[test]
fn notes_anchors_primers_and_revisions_persist_exact_expiry() {
    let mut state = NotesProjection::new(scope()).unwrap();
    for (id, kind) in [
        ("anchor", NoteKind::Anchor),
        ("note", NoteKind::Note),
        ("primer", NoteKind::Primer),
    ] {
        let event = state
            .plan(
                "owner",
                NotesCommand::Create(note(id, kind, Some(CURRENT + 1))),
            )
            .unwrap();
        state.replay(event).unwrap();
    }
    let mut revised = note("note", NoteKind::Note, Some(CURRENT + 2));
    revised.revision = 2;
    let event = state
        .plan(
            "owner",
            NotesCommand::Revise {
                note: revised,
                expected_revision: 1,
            },
        )
        .unwrap();
    state.replay(event).unwrap();
    let events: Vec<NotesEvent> = state
        .events("owner")
        .unwrap()
        .into_iter()
        .cloned()
        .collect();
    let wire = serde_json::to_value(&events).unwrap();
    assert_eq!(
        wire[0]["command"]["Create"]["expires_at_ns"],
        (CURRENT + 1).to_string()
    );
    assert_eq!(
        wire[3]["command"]["Revise"]["note"]["expires_at_ns"],
        (CURRENT + 2).to_string()
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("notes.json");
    std::fs::write(&path, serde_json::to_vec(&events).unwrap()).unwrap();
    let restored = NotesProjection::restore(
        scope(),
        serde_json::from_slice::<Vec<NotesEvent>>(&std::fs::read(&path).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(restored, state);
    assert!(restored
        .read("owner", "anchor", CURRENT, &BTreeMap::new())
        .unwrap()
        .is_some());
    assert!(restored
        .read("owner", "anchor", CURRENT + 1, &BTreeMap::new())
        .unwrap()
        .is_none());
    assert!(restored
        .read("owner", "note", CURRENT + 1, &BTreeMap::new())
        .unwrap()
        .is_some());
    assert!(restored
        .read("owner", "note", CURRENT + 2, &BTreeMap::new())
        .unwrap()
        .is_none());
    let mut legacy = serde_json::to_value(note("legacy", NoteKind::Note, None)).unwrap();
    legacy["expires_at_ns"] = json!(CURRENT);
    assert!(serde_json::from_value::<Note>(legacy.clone()).is_err());
    legacy["expires_at_ns"] = json!(42);
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<Note>(legacy.clone()).unwrap()).unwrap()
            ["expires_at_ns"],
        "42"
    );
    legacy.as_object_mut().unwrap().remove("expires_at_ns");
    assert_eq!(
        serde_json::from_value::<Note>(legacy)
            .unwrap()
            .expires_at_ns,
        None
    );
    let proposal = AttributeProposal {
        id: "proposal".into(),
        key: "preference".into(),
        value: "blue".into(),
        base_revision: 0,
        proposer: "owner".into(),
    };
    assert!(serde_json::to_value(proposal)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .all(|key| !key.ends_with("_ns")));
}
