use hm_context::{maintenance::*, types::*};

fn scope() -> Scope { Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: None } }
fn config() -> SchedulerConfig { SchedulerConfig { max_concurrency: 2, budget: 100, lease_ms: 10, backoff_ms: 5, max_attempts: 3 } }
fn request(kind: JobKind, sequence: u64) -> JobRequest {
    let mut source = SourceMessage { id: format!("source-{sequence}"), ordinal: sequence, role: MessageRole::User, parts: vec![MessagePart::Text { text: format!("Observation {sequence}") }], occurred_at_ns: None, recorded_at_ns: 1, authority: Authority::UserAsserted, source_digest: String::new() };
    source.source_digest = source.computed_digest().unwrap();
    JobRequest { kind, sources: vec![source], cursor: Cursor { epoch: 1, sequence }, source_revision: 1, policy_revision: 1, reservation: 30 }
}

#[test]
fn durable_distinct_jobs_publication_watermarks_and_dedup() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("jobs.json");
    let mut scheduler = MaintenanceScheduler::open(&path, scope(), config()).unwrap();
    for kind in [JobKind::Historian, JobKind::Verification, JobKind::Curation, JobKind::Extraction, JobKind::Indexing, JobKind::Consolidation] {
        let input = request(kind, 1);
        let id = scheduler.enqueue(input.clone()).unwrap();
        assert_eq!(id, scheduler.enqueue(input).unwrap());
    }
    assert_eq!(scheduler.snapshot().jobs.len(), 6);
    let a = scheduler.claim(0).unwrap().unwrap();
    let b = scheduler.claim(0).unwrap().unwrap();
    assert!(scheduler.claim(0).unwrap().is_none());
    let mut stale = a.fence.clone(); stale.policy_revision += 1;
    assert!(matches!(scheduler.complete(&a, &stale, Usage::Known(2), &digest_bytes(b"output"), 1), Err(ContextError::Stale)));
    drop(scheduler);
    let mut scheduler = MaintenanceScheduler::open(&path, scope(), config()).unwrap();
    let receipt = scheduler.complete(&a, &a.fence, Usage::Known(2), &digest_bytes(b"output"), 1).unwrap();
    assert_eq!(receipt.authority, Authority::DerivedInference);
    assert_eq!(scheduler.snapshot().watermarks[&receipt.kind], Cursor { epoch: 1, sequence: 1 });
    assert!(scheduler.complete(&a, &a.fence, Usage::Known(2), &digest_bytes(b"output"), 1).is_err());
    scheduler.cancel(&b.job_id, 1).unwrap();
    assert!(scheduler.complete(&b, &b.fence, Usage::Known(2), &digest_bytes(b"output"), 1).is_err());
    assert_eq!(scheduler.snapshot().unknown_usage.len(), 1);
    scheduler.settle_unknown(&b.job_id, b.attempt, 4).unwrap();
    assert_eq!(scheduler.snapshot().spent, 6);
    drop(scheduler);
    let scheduler = MaintenanceScheduler::open(&path, scope(), config()).unwrap();
    assert_eq!(scheduler.snapshot().spent, 6);
    assert!(MaintenanceScheduler::open(&path, Scope { owner_id: "other".into(), ..scope() }, config()).is_err());
}

#[test]
fn lease_expiry_unknown_usage_budget_backoff_and_attempt_fences() {
    let directory = tempfile::tempdir().unwrap();
    let mut bounds = config(); bounds.budget = 30;
    let mut scheduler = MaintenanceScheduler::open(directory.path().join("jobs.json"), scope(), bounds).unwrap();
    let id = scheduler.enqueue(request(JobKind::Extraction, 1)).unwrap();
    let first = scheduler.claim(0).unwrap().unwrap();
    assert!(scheduler.claim(10).unwrap().is_none());
    assert_eq!(scheduler.snapshot().unknown_usage.len(), 1);
    assert!(scheduler.complete(&first, &first.fence, Usage::Known(0), &digest_bytes(b"result"), 10).is_err());
    scheduler.settle_unknown(&id, 1, 0).unwrap();
    assert!(scheduler.claim(14).unwrap().is_none());
    let second = scheduler.claim(15).unwrap().unwrap();
    assert_eq!(second.attempt, 2);
    assert!(scheduler.fail(&first, Usage::Known(0), 16).is_err());
    scheduler.fail(&second, Usage::Known(0), 16).unwrap();
    assert!(scheduler.claim(25).unwrap().is_none());
    let third = scheduler.claim(26).unwrap().unwrap();
    scheduler.fail(&third, Usage::Known(31), 27).unwrap();
    assert_eq!(scheduler.snapshot().jobs[&id].status, JobStatus::Exhausted);
    assert_eq!(scheduler.snapshot().spent, 31);
    assert!(scheduler.claim(100).unwrap().is_none());
}

#[test]
fn invalid_sources_never_become_observations() {
    let directory = tempfile::tempdir().unwrap();
    let mut scheduler = MaintenanceScheduler::open(directory.path().join("jobs.json"), scope(), config()).unwrap();
    let mut derived = request(JobKind::Consolidation, 1);
    derived.sources[0].authority = Authority::DerivedInference;
    derived.sources[0].source_digest = derived.sources[0].computed_digest().unwrap();
    assert!(scheduler.enqueue(derived).is_err());
    let mut invalid = request(JobKind::Historian, 1);
    invalid.sources[0].parts = vec![MessagePart::Text { text: "changed".into() }];
    assert!(scheduler.enqueue(invalid).is_err());
    let mut duplicate = request(JobKind::Verification, 1);
    duplicate.sources.push(duplicate.sources[0].clone());
    assert!(scheduler.enqueue(duplicate).is_err());
    assert!(scheduler.snapshot().jobs.is_empty());
}
