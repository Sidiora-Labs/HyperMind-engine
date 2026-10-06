use hm_context::{historian::*, types::*};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn sources() -> Vec<SourceMessage> {
    (0..3)
        .map(|ordinal| {
            let mut message = SourceMessage {
                id: format!("m{ordinal}"),
                ordinal,
                role: MessageRole::User,
                parts: vec![MessagePart::Text {
                    text: format!("Measurement {ordinal}: temperature 20 C"),
                }],
                occurred_at_ns: None,
                recorded_at_ns: 1,
                authority: Authority::UserAsserted,
                source_digest: String::new(),
            };
            message.source_digest = message.computed_digest().unwrap();
            message
        })
        .collect()
}
fn result(chunk: &SourceChunk) -> HistorianResult {
    HistorianResult {
        source_digest: chunk.digest.clone(),
        tiers: [
            "Recorded temperature measurements: 20 C",
            "Temperature measurements: 20 C",
            "Temperature: 20 C",
            "20 C",
        ]
        .map(|text| SummaryTier {
            text: text.into(),
            coverage: chunk.coverage().unwrap(),
        }),
    }
}
#[test]
fn bounded_chunks_exact_coverage_and_digest() {
    let source = sources();
    let chunks = select_chunks(
        &source,
        ChunkLimits {
            max_messages: 2,
            max_bytes: 10000,
        },
    )
    .unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].sources.len(), 2);
    assert!(select_chunks(
        &source,
        ChunkLimits {
            max_messages: 2,
            max_bytes: 1
        }
    )
    .is_err());
    let mut summary = result(&chunks[0]);
    summary.validate(&chunks[0]).unwrap();
    summary.tiers[0].coverage.pop();
    assert!(summary.validate(&chunks[0]).is_err());
    let mut summary = result(&chunks[0]);
    summary.tiers[1].coverage[0].source_digest = "changed".into();
    assert!(summary.validate(&chunks[0]).is_err());
    let mut changed = chunks[0].clone();
    changed.sources[0].parts = vec![MessagePart::Text {
        text: "changed".into(),
    }];
    assert!(changed.validate().is_err());
    let mut reversed = source.clone();
    reversed.reverse();
    assert!(select_chunks(
        &reversed,
        ChunkLimits {
            max_messages: 2,
            max_bytes: 10000
        }
    )
    .is_err());
}
#[test]
fn restart_lease_cooldown_fencing_and_completion() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jobs.json");
    let chunk = select_chunks(
        &sources(),
        ChunkLimits {
            max_messages: 3,
            max_bytes: 10000,
        },
    )
    .unwrap()
    .remove(0);
    let mut historian = Historian::new(scope()).unwrap();
    let id = historian
        .enqueue("session", Cursor::default(), 7, chunk.clone(), 0)
        .unwrap();
    assert_eq!(
        historian
            .enqueue("session", Cursor::default(), 7, chunk.clone(), 0)
            .unwrap(),
        id
    );
    let first = historian.claim("worker", 0, 10).unwrap().unwrap();
    historian.heartbeat(&first, 5, 20).unwrap();
    historian.checkpoint(&path).unwrap();
    let mut restarted = Historian::restore(&path, &scope()).unwrap();
    assert!(restarted.claim("other", 24, 10).unwrap().is_none());
    assert_eq!(restarted.expire(25, 5).unwrap(), 1);
    assert!(restarted
        .complete(&first, 25, &chunk, 7, result(&chunk))
        .is_err());
    assert!(restarted.claim("other", 29, 10).unwrap().is_none());
    let second = restarted.claim("other", 30, 10).unwrap().unwrap();
    assert_eq!(second.attempt, 2);
    assert!(restarted
        .complete(&first, 31, &chunk, 7, result(&chunk))
        .is_err());
    assert!(restarted
        .complete(&second, 31, &chunk, 8, result(&chunk))
        .is_err());
    restarted
        .complete(&second, 31, &chunk, 7, result(&chunk))
        .unwrap();
    restarted.checkpoint(&path).unwrap();
    let restored = Historian::restore(&path, &scope()).unwrap();
    assert!(restored.result(&id).is_some());
    let mut wrong = scope();
    wrong.project_id = "elsewhere".into();
    assert!(Historian::restore(&path, &wrong).is_err());
}
#[test]
fn failure_cancel_and_late_publication() {
    let chunk = select_chunks(
        &sources(),
        ChunkLimits {
            max_messages: 3,
            max_bytes: 10000,
        },
    )
    .unwrap()
    .remove(0);
    let mut historian = Historian::new(scope()).unwrap();
    let id = historian
        .enqueue("session", Cursor::default(), 1, chunk.clone(), 0)
        .unwrap();
    let claim = historian.claim("worker", 0, 10).unwrap().unwrap();
    historian.fail(&claim, 1, 10).unwrap();
    assert!(historian.claim("worker", 10, 10).unwrap().is_none());
    let claim = historian.claim("worker", 11, 10).unwrap().unwrap();
    historian.cancel(&id).unwrap();
    assert!(historian
        .complete(&claim, 12, &chunk, 1, result(&chunk))
        .is_err());
    assert!(historian.result(&id).is_none());
}

#[test]
fn host_byte_spans_are_bound_and_bounded() {
    let messages = sources();
    let spans: Vec<_> = messages
        .iter()
        .map(|message| SourceSpan {
            source_id: message.id.clone(),
            source_digest: message.source_digest.clone(),
            byte_start: 0,
            byte_end: 700,
        })
        .collect();
    let chunks = select_chunks_with_spans(
        &messages,
        &spans,
        ChunkLimits {
            max_messages: 3,
            max_bytes: 1000,
        },
    )
    .unwrap();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].coverage().unwrap(), spans[..1]);
    let mut altered = chunks[0].clone();
    altered.spans[0].byte_end += 1;
    assert!(altered.validate().is_err());
    assert!(select_chunks_with_spans(
        &messages,
        &spans,
        ChunkLimits {
            max_messages: 3,
            max_bytes: 699
        }
    )
    .is_err());
}
