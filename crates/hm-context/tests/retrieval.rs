use hm_context::retrieval::*;
use hm_context::{Authority, ContextError, Scope, SourceSpan};

fn scope(owner: &str) -> Scope {
    Scope {
        owner_id: owner.into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn vector(digest: &str, values: Vec<f32>) -> EmbeddedVector {
    EmbeddedVector {
        fingerprint: VectorFingerprint {
            model: "encoder".into(),
            revision: "v1".into(),
            dimensions: 2,
        },
        content_digest: digest.into(),
        values,
    }
}
fn candidate(id: &str, kind: SourceKind) -> EvidenceCandidate {
    EvidenceCandidate {
        scope: scope("owner"),
        kind,
        id: id.into(),
        text: format!("evidence {id}"),
        authority: Authority::ExternalObserved,
        provenance: vec![SourceSpan {
            source_id: id.into(),
            source_digest: format!("digest-{id}"),
            byte_start: 0,
            byte_end: 10,
        }],
        tokens: 3,
        content_digest: format!("digest-{id}"),
        source_revision: 1,
        current_revision: 1,
        occurred_at_ns: Some(20),
        recorded_at_ns: 25,
        valid_from_ns: None,
        expires_at_ns: None,
        tombstoned: false,
        vector: None,
    }
}
fn request() -> RetrievalRequest {
    RetrievalRequest {
        scope: scope("owner"),
        now_ns: 30,
        grants: vec![],
        visible: vec![],
        time_filter: None,
        query_vector: None,
        max_candidates: 100,
        max_results: 20,
        max_tokens: 100,
    }
}
fn batch(kind: SourceKind, candidates: Vec<EvidenceCandidate>) -> IndexBatch {
    IndexBatch {
        kind,
        index_revision: 1,
        candidates,
    }
}

#[test]
fn fuses_all_sources_deterministically_with_exact_provenance() {
    let kinds = [
        SourceKind::Memory,
        SourceKind::Conversation,
        SourceKind::File,
        SourceKind::Commit,
        SourceKind::Document,
        SourceKind::Entity,
        SourceKind::Relationship,
    ];
    let mut batches: Vec<_> = kinds
        .into_iter()
        .enumerate()
        .map(|(n, kind)| batch(kind, vec![candidate(&format!("source-{n}"), kind)]))
        .collect();
    let report = fuse(&request(), &batches).unwrap();
    batches.reverse();
    assert_eq!(report, fuse(&request(), &batches).unwrap());
    assert_eq!(report.results.len(), 7);
    assert_eq!(report.token_count, 21);
    assert!(matches!(
        report.semantic,
        SemanticStatus::Unavailable { .. }
    ));
    for result in report.results {
        assert_eq!(
            result.context_block().provenance,
            result.candidate.provenance
        );
    }
}

#[test]
fn computes_actual_cosine_and_marks_incompatible_vectors() {
    let mut req = request();
    req.query_vector = Some(vector("query-content", vec![1.0, 0.0]));
    let mut a = candidate("a", SourceKind::Memory);
    a.vector = Some(vector(&a.content_digest, vec![0.0, 1.0]));
    let mut b = candidate("b", SourceKind::Memory);
    b.vector = Some(vector(&b.content_digest, vec![1.0, 0.0]));
    let mut c = candidate("c", SourceKind::Memory);
    c.vector = Some(vector("older-content", vec![1.0, 0.0]));
    let report = fuse(&req, &[batch(SourceKind::Memory, vec![a, b, c])]).unwrap();
    assert_eq!(
        report.semantic,
        SemanticStatus::Available {
            scored: 2,
            unavailable: 1
        }
    );
    assert_eq!(
        report
            .results
            .iter()
            .find(|r| r.candidate.id == "a")
            .unwrap()
            .semantic_score,
        Some(0.0)
    );
    assert_eq!(
        report
            .results
            .iter()
            .find(|r| r.candidate.id == "b")
            .unwrap()
            .semantic_score,
        Some(1.0)
    );
    assert_eq!(
        report
            .results
            .iter()
            .find(|r| r.candidate.id == "c")
            .unwrap()
            .semantic_score,
        None
    );
    req.query_vector.as_mut().unwrap().values[0] = f32::NAN;
    assert!(matches!(fuse(&req, &[]), Err(ContextError::Invalid(_))));
}

#[test]
fn filters_access_time_expiry_revisions_and_visible_union() {
    let mut req = request();
    req.time_filter = Some(SourceTimeFilter {
        field: TimeField::Occurred,
        from_ns: Some(10),
        through_ns: Some(25),
    });
    let mut candidates: Vec<_> = [
        "private", "expired", "deleted", "stale", "unknown", "visible", "valid",
    ]
    .into_iter()
    .map(|id| candidate(id, SourceKind::Memory))
    .collect();
    candidates[0].scope = scope("other");
    candidates[1].expires_at_ns = Some(30);
    candidates[2].tombstoned = true;
    candidates[3].current_revision = 2;
    candidates[4].occurred_at_ns = None;
    req.visible = vec![
        SourceSpan {
            source_id: "visible".into(),
            source_digest: "digest-visible".into(),
            byte_start: 0,
            byte_end: 4,
        },
        SourceSpan {
            source_id: "visible".into(),
            source_digest: "digest-visible".into(),
            byte_start: 4,
            byte_end: 10,
        },
    ];
    let report = fuse(&req, &[batch(SourceKind::Memory, candidates)]).unwrap();
    assert_eq!(report.results[0].candidate.id, "valid");
    assert_eq!(report.results.len(), 1);
    assert_eq!(report.unauthorized_count, 1);
    assert_eq!(report.omitted.len(), 5);
}

#[test]
fn exact_revision_grants_and_bounds_are_fail_closed() {
    let mut req = request();
    let mut shared = candidate("shared", SourceKind::Document);
    shared.scope = scope("other");
    req.grants.push(EvidenceGrant {
        source_scope: shared.scope.clone(),
        recipient_scope: req.scope.clone(),
        kind: shared.kind,
        source_id: shared.id.clone(),
        source_digest: shared.content_digest.clone(),
        source_revision: 1,
        expires_at_ns: 31,
    });
    let batches = vec![batch(SourceKind::Document, vec![shared])];
    assert_eq!(fuse(&req, &batches).unwrap().results.len(), 1);
    req.grants[0].source_revision = 2;
    assert_eq!(fuse(&req, &batches).unwrap().unauthorized_count, 1);
    req.max_candidates = 0;
    assert!(matches!(fuse(&req, &batches), Err(ContextError::Capacity)));
    req = request();
    req.max_tokens = 2;
    let report = fuse(
        &req,
        &[batch(
            SourceKind::File,
            vec![candidate("large", SourceKind::File)],
        )],
    )
    .unwrap();
    assert!(report.results.is_empty());
    assert_eq!(report.omitted[0].reason, "retrieval_budget");
}

#[test]
fn duplicate_index_entries_do_not_gain_votes_and_conflicts_fail() {
    let req = request();
    let original = batch(SourceKind::Memory, vec![candidate("a", SourceKind::Memory)]);
    assert_eq!(
        fuse(&req, &[original.clone()]).unwrap(),
        fuse(&req, &[original.clone(), original.clone()]).unwrap()
    );
    let mut conflict = original.clone();
    conflict.candidates[0].text = "different bytes".into();
    assert!(matches!(
        fuse(&req, &[original, conflict]),
        Err(ContextError::Conflict)
    ));
}

#[test]
fn timestamp_transport_preserves_extremes_and_rejects_unsafe_numbers() {
    let mut req = request();
    req.now_ns = i64::MAX;
    req.time_filter = Some(SourceTimeFilter {
        field: TimeField::Occurred,
        from_ns: Some(i64::MIN),
        through_ns: Some(i64::MAX),
    });
    let value = serde_json::to_value(&req).unwrap();
    assert_eq!(value["now_ns"], i64::MAX.to_string());
    assert_eq!(
        serde_json::from_value::<RetrievalRequest>(value.clone()).unwrap(),
        req
    );
    let mut unsafe_value = value;
    unsafe_value["now_ns"] = serde_json::json!(9007199254740992u64);
    assert!(serde_json::from_value::<RetrievalRequest>(unsafe_value).is_err());
}
