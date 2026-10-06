use hm_context::notes::*;
use hm_context::{ContextError, Scope};
use std::collections::{BTreeMap, BTreeSet};
fn state() -> NotesProjection {
    NotesProjection::new(Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    })
    .unwrap()
}
fn note(id: &str, kind: NoteKind) -> Note {
    Note {
        id: id.into(),
        kind,
        revision: 1,
        text: "Evidence".into(),
        parents: BTreeSet::new(),
        contradictions: BTreeSet::new(),
        expires_at_ns: None,
        predicate: Predicate::True,
        tombstoned: false,
    }
}
fn apply(s: &mut NotesProjection, p: &str, c: NotesCommand) {
    let e = s.plan(p, c).unwrap();
    s.replay(e).unwrap();
}
#[test]
fn immutable_revisioned_retention_and_ledger_restore() {
    let mut s = state();
    apply(
        &mut s,
        "owner",
        NotesCommand::Create(note("anchor", NoteKind::Anchor)),
    );
    let mut a = note("anchor", NoteKind::Anchor);
    a.revision = 2;
    assert!(s
        .plan(
            "owner",
            NotesCommand::Revise {
                note: a,
                expected_revision: 1
            }
        )
        .is_err());
    let mut n = note("primer", NoteKind::Primer);
    n.parents.insert("anchor".into());
    n.expires_at_ns = Some(30);
    apply(&mut s, "owner", NotesCommand::Create(n.clone()));
    n.revision = 2;
    n.text = "Updated".into();
    apply(
        &mut s,
        "owner",
        NotesCommand::Revise {
            note: n.clone(),
            expected_revision: 1,
        },
    );
    assert!(matches!(
        s.plan(
            "owner",
            NotesCommand::Revise {
                note: n,
                expected_revision: 1
            }
        ),
        Err(ContextError::Stale)
    ));
    assert!(s
        .read("owner", "primer", 30, &BTreeMap::new())
        .unwrap()
        .is_none());
    apply(
        &mut s,
        "owner",
        NotesCommand::Tombstone {
            id: "anchor".into(),
            expected_revision: 1,
        },
    );
    assert!(s
        .read("owner", "anchor", 0, &BTreeMap::new())
        .unwrap()
        .is_none());
    let encoded = serde_json::to_vec(
        &s.events("owner")
            .unwrap()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let restored = NotesProjection::restore(
        Scope {
            owner_id: "owner".into(),
            project_id: "project".into(),
            workspace_id: None,
        },
        serde_json::from_slice::<Vec<NotesEvent>>(&encoded).unwrap(),
    )
    .unwrap();
    assert_eq!(s, restored);
}
#[test]
fn exact_permissions_revocation_and_replay_fences() {
    let mut s = state();
    apply(
        &mut s,
        "owner",
        NotesCommand::Create(note("a", NoteKind::Note)),
    );
    apply(
        &mut s,
        "owner",
        NotesCommand::Create(note("b", NoteKind::Note)),
    );
    assert!(s.read("guest", "a", 0, &BTreeMap::new()).is_err());
    apply(
        &mut s,
        "owner",
        NotesCommand::SetGrant(Grant {
            principal: "guest".into(),
            note_id: "a".into(),
            read: true,
            write: true,
        }),
    );
    assert!(s.read("guest", "a", 0, &BTreeMap::new()).unwrap().is_some());
    assert!(s.read("guest", "b", 0, &BTreeMap::new()).is_err());
    let mut a = note("a", NoteKind::Note);
    a.revision = 2;
    apply(
        &mut s,
        "guest",
        NotesCommand::Revise {
            note: a,
            expected_revision: 1,
        },
    );
    apply(
        &mut s,
        "owner",
        NotesCommand::RevokeGrant {
            principal: "guest".into(),
            note_id: "a".into(),
        },
    );
    assert!(s.read("guest", "a", 0, &BTreeMap::new()).is_err());
    let event = s.events("owner").unwrap()[0].clone();
    s.replay(event.clone()).unwrap();
    let mut conflict = event;
    conflict.principal = "guest".into();
    assert!(s.replay(conflict).is_err());
}
#[test]
fn lineage_and_contradictions_are_preserved_and_cycles_rejected() {
    let mut s = state();
    apply(
        &mut s,
        "owner",
        NotesCommand::Create(note("a", NoteKind::Note)),
    );
    let mut b = note("b", NoteKind::Note);
    b.parents.insert("a".into());
    b.contradictions.insert("a".into());
    apply(&mut s, "owner", NotesCommand::Create(b));
    let mut a = note("a", NoteKind::Note);
    a.revision = 2;
    a.parents.insert("b".into());
    assert!(s
        .plan(
            "owner",
            NotesCommand::Revise {
                note: a,
                expected_revision: 1
            }
        )
        .is_err());
    assert!(s
        .read("owner", "b", 0, &BTreeMap::new())
        .unwrap()
        .unwrap()
        .contradictions
        .contains("a"));
}
#[test]
fn conditional_predicates_fail_closed_and_attributes_require_acceptance() {
    let mut s = state();
    let mut n = note("n", NoteKind::Note);
    n.predicate = Predicate::Equals {
        key: "mode".into(),
        value: "work".into(),
    };
    apply(&mut s, "owner", NotesCommand::Create(n));
    assert!(s.read("owner", "n", 0, &BTreeMap::new()).unwrap().is_none());
    let facts = BTreeMap::from([("mode".into(), "work".into())]);
    assert!(s.read("owner", "n", 0, &facts).unwrap().is_some());
    assert!(Predicate::All(vec![Predicate::True; 129])
        .evaluate(&facts)
        .is_err());
    let mut deep = Predicate::True;
    for _ in 0..18 {
        deep = Predicate::Not(Box::new(deep));
    }
    assert!(deep.evaluate(&facts).is_err());
    apply(
        &mut s,
        "owner",
        NotesCommand::SetAttributeGrant {
            principal: "agent".into(),
            key: "preference".into(),
            write: true,
        },
    );
    apply(
        &mut s,
        "agent",
        NotesCommand::ProposeAttribute(AttributeProposal {
            id: "p".into(),
            key: "preference".into(),
            value: "blue".into(),
            base_revision: 0,
            proposer: "agent".into(),
        }),
    );
    assert!(s.attribute("owner", "preference").unwrap().is_none());
    assert!(s
        .plan(
            "agent",
            NotesCommand::AcceptAttribute {
                proposal_id: "p".into()
            }
        )
        .is_err());
    apply(
        &mut s,
        "owner",
        NotesCommand::AcceptAttribute {
            proposal_id: "p".into(),
        },
    );
    assert_eq!(
        s.attribute("owner", "preference").unwrap(),
        Some(&(1, "blue".into()))
    );
}
