use hm_context::{temporal::*, types::Scope};
fn scope(workspace: &str) -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: Some(workspace.into()),
    }
}
fn identity(id: &str, ordinal: u64) -> SourceIdentity {
    SourceIdentity {
        id: id.into(),
        digest: "a".repeat(64),
        ordinal,
    }
}
fn source(id: &str, ordinal: u64) -> TemporalSource {
    TemporalSource {
        source: identity(id, ordinal),
        times: TemporalTimes {
            occurred_at_ns: None,
            recorded_at_ns: 99,
            valid_from_ns: Some(-3),
            valid_until_ns: Some(30),
            verified_at_ns: Some(101),
        },
    }
}
#[test]
fn precision_unknowns_coverage_and_stable_gaps() {
    let context = TemporalContext::build(
        scope("main"),
        "session".into(),
        19800,
        vec![source("a", 1)],
        vec![identity("b", 2), identity("a", 1)],
    )
    .unwrap();
    assert_eq!(
        context.sources[0].times.local_occurrence_ns(19800).unwrap(),
        None
    );
    assert_eq!(context.sources[0].times.recorded_at_ns, 99);
    assert_eq!(context.sources[0].times.verified_at_ns, Some(101));
    assert_eq!(context.coverage, vec![identity("a", 1)]);
    let reordered = TemporalContext::build(
        scope("main"),
        "session".into(),
        -3600,
        vec![source("a", 1)],
        vec![identity("a", 1), identity("b", 2)],
    )
    .unwrap();
    assert_eq!(context.gaps, reordered.gaps);
    let mut times = source("a", 1).times;
    times.occurred_at_ns = Some(123);
    assert_eq!(times.local_occurrence_ns(1).unwrap(), Some(1_000_000_123));
    times.occurred_at_ns = Some(i64::MAX);
    assert!(times.local_occurrence_ns(1).is_err());
    times.valid_until_ns = Some(-4);
    assert!(times.validate().is_err());
    assert!(TemporalContext::build(scope("main"), "s".into(), 86401, vec![], vec![]).is_err());
    assert!(TemporalContext::build(
        scope("main"),
        "s".into(),
        0,
        vec![source("other", 1)],
        vec![identity("a", 1)]
    )
    .is_err());
}
#[test]
fn explicit_actor_aliases_collisions_and_restart() {
    let alias = |id: &str| ExternalIdentity {
        namespace: "host".into(),
        external_id: id.into(),
    };
    let mut registry = IdentityRegistry::new();
    registry
        .bind(scope("main"), 65535, vec![alias("old")])
        .unwrap();
    registry
        .bind(scope("main"), 65535, vec![alias("new")])
        .unwrap();
    assert_eq!(
        registry.resolve("owner", "host", "new").unwrap().actor,
        65535
    );
    assert_eq!(
        registry.resolve("owner", "host", "old").unwrap().scope,
        scope("main")
    );
    assert!(registry.bind(scope("tree"), 65535, vec![]).is_err());
    assert!(registry.bind(scope("tree"), 2, vec![alias("old")]).is_err());
    assert!(registry.bind(scope("main"), 2, vec![]).is_err());
    registry
        .bind(scope("tree"), 2, vec![alias("tree")])
        .unwrap();
    assert!(registry.resolve("other-owner", "host", "old").is_none());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("identities.json");
    registry.save(&path).unwrap();
    assert_eq!(IdentityRegistry::load(&path).unwrap(), registry);
    std::fs::write(&path, br#"{"bindings":[{"scope":{"owner_id":"owner","project_id":"project","workspace_id":"main"},"actor":1,"aliases":[]},{"scope":{"owner_id":"owner","project_id":"project","workspace_id":"tree"},"actor":1,"aliases":[]}]}"#).unwrap();
    assert!(IdentityRegistry::load(&path).is_err());
}
#[test]
fn canonical_aliases_keep_distinct_worktrees() {
    let temp = tempfile::tempdir().unwrap();
    let main = temp.path().join("main");
    let tree = temp.path().join("tree");
    std::fs::create_dir(&main).unwrap();
    std::fs::create_dir(&tree).unwrap();
    assert_eq!(
        canonical_workspace(&main.join(".")).unwrap(),
        canonical_workspace(&main).unwrap()
    );
    assert_ne!(
        canonical_workspace(&main).unwrap(),
        canonical_workspace(&tree).unwrap()
    );
    #[cfg(unix)]
    {
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&main, &alias).unwrap();
        assert_eq!(
            canonical_workspace(&alias).unwrap(),
            canonical_workspace(&main).unwrap()
        );
    }
}
