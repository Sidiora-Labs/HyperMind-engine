use hm_context::types::{Authority, ContextBlock, Scope};
use hm_fabric::roles::*;
use std::collections::BTreeSet;
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn pin() -> RoleVersion {
    RoleVersion::new(b"schema-v1", b"semantics-v1")
}
#[test]
fn tool_pins_and_withdrawal_are_stable() {
    let mut registry = ToolRegistry::default();
    registry
        .register(ToolContract {
            name: "lookup".into(),
            pin: pin(),
        })
        .unwrap();
    assert!(registry
        .resolve("lookup", &RoleVersion::new(b"changed", b"semantics-v1"))
        .is_err());
    let receipt = registry.withdraw("lookup", "maintenance".into()).unwrap();
    assert_eq!(
        receipt,
        registry.withdraw("lookup", "new reason".into()).unwrap()
    );
    assert_eq!(receipt, registry.resolve("lookup", &pin()).unwrap());
    assert!(registry
        .register(ToolContract {
            name: "lookup".into(),
            pin: pin()
        })
        .is_err());
}
#[test]
fn runner_has_one_terminal_and_deduplicates_delivery() {
    let mut runs = RunRegistry::default();
    runs.start("run-1".into(), scope(), pin()).unwrap();
    assert!(runs
        .deliver("run-1", &scope(), "delivery".into(), b"result")
        .is_err());
    let terminal = RunTerminal::Completed {
        result_digest: "digest".into(),
    };
    runs.finish("run-1", &scope(), terminal.clone()).unwrap();
    runs.finish("run-1", &scope(), terminal).unwrap();
    assert!(runs
        .finish("run-1", &scope(), RunTerminal::Cancelled)
        .is_err());
    assert!(runs
        .deliver("run-1", &scope(), "delivery".into(), b"result")
        .unwrap());
    assert!(!runs
        .deliver("run-1", &scope(), "delivery".into(), b"result")
        .unwrap());
    assert!(runs
        .deliver("run-1", &scope(), "delivery".into(), b"other")
        .is_err());
    let mut foreign = scope();
    foreign.project_id = "other".into();
    assert!(runs
        .deliver("run-1", &foreign, "new".into(), b"result")
        .is_err());
}
fn request(id: &str, start: u64, end: u64) -> CompactionRequest {
    CompactionRequest {
        id: id.into(),
        scope: scope(),
        generation: 3,
        deadline_ns: 100,
        start,
        end,
        source_digest: "source".into(),
        pin: pin(),
    }
}
#[test]
fn compaction_uses_half_open_ranges_and_request_fences() {
    let mut compaction = CompactionRegistry::default();
    let a = request("a", 0, 4);
    compaction.request(a.clone(), 0).unwrap();
    compaction.request(request("b", 4, 8), 0).unwrap();
    assert!(compaction.request(request("c", 3, 6), 0).is_err());
    assert!(compaction.publish(&a, 4, "source", 1, b"reduced").is_err());
    assert!(compaction.publish(&a, 3, "changed", 1, b"reduced").is_err());
    assert!(compaction
        .publish(&a, 3, "source", 100, b"reduced")
        .is_err());
    let mut changed = a.clone();
    changed.end = 3;
    assert!(compaction
        .publish(&changed, 3, "source", 1, b"reduced")
        .is_err());
    assert!(compaction.publish(&a, 3, "source", 1, b"reduced").unwrap());
    assert!(!compaction.publish(&a, 3, "source", 1, b"reduced").unwrap());
    assert!(compaction
        .publish(&a, 3, "source", 1, b"different")
        .is_err());
}
fn block(id: &str, tokens: u64, required: bool, authority: Authority) -> ContextBlock {
    ContextBlock {
        id: id.into(),
        text: id.into(),
        tokens,
        required,
        authority,
        provenance: vec![],
    }
}
#[test]
fn transform_filters_within_declared_bounds_preserving_order() {
    let mut transforms = TransformRegistry::default();
    transforms
        .register(TransformDeclaration {
            id: "select".into(),
            pin: pin(),
            authorities: vec![Authority::ToolObserved],
            max_input_tokens: 12,
            max_output_tokens: 5,
            hooks: BTreeSet::from(["before_compose".into()]),
            may_reorder: false,
        })
        .unwrap();
    let input = TransformInput {
        scope: scope(),
        hook: "before_compose".into(),
        blocks: vec![
            block("a", 2, false, Authority::ToolObserved),
            block("excluded", 1, false, Authority::UserAsserted),
            block("b", 3, true, Authority::ToolObserved),
        ],
    };
    let output = transforms.apply("select", &pin(), input.clone()).unwrap();
    assert_eq!(
        output
            .blocks
            .iter()
            .map(|b| b.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(output.blocks[1].authority, Authority::ToolObserved);
    let mut wrong_hook = input.clone();
    wrong_hook.hook = "after_compose".into();
    assert!(transforms.apply("select", &pin(), wrong_hook).is_err());
    let mut required = input.clone();
    required.blocks[1].required = true;
    assert!(transforms.apply("select", &pin(), required).is_err());
    let mut over = input;
    over.blocks[2].tokens = 10;
    assert!(transforms.apply("select", &pin(), over).is_err());
}

#[test]
fn output_contract_rejects_authority_elevation_and_reordering() {
    let mut registry = TransformRegistry::default();
    registry
        .register(TransformDeclaration {
            id: "narrow".into(),
            pin: pin(),
            authorities: vec![Authority::ToolObserved],
            max_input_tokens: 8,
            max_output_tokens: 8,
            hooks: BTreeSet::from(["compose".into()]),
            may_reorder: false,
        })
        .unwrap();
    let input = TransformInput {
        scope: scope(),
        hook: "compose".into(),
        blocks: vec![
            block("a", 2, true, Authority::ToolObserved),
            block("b", 2, false, Authority::ToolObserved),
        ],
    };
    let mut output = TransformOutput {
        blocks: input.blocks.clone(),
    };
    output.blocks[0].authority = Authority::DerivedInference;
    output.blocks[0].text = "reduced evidence".into();
    registry
        .validate_output("narrow", &pin(), &input, &output)
        .unwrap();
    output.blocks[0].authority = Authority::UserAsserted;
    assert!(registry
        .validate_output("narrow", &pin(), &input, &output)
        .is_err());
    output.blocks = input.blocks.iter().cloned().rev().collect();
    assert!(registry
        .validate_output("narrow", &pin(), &input, &output)
        .is_err());
    output.blocks = vec![input.blocks[1].clone()];
    assert!(registry
        .validate_output("narrow", &pin(), &input, &output)
        .is_err());
}
