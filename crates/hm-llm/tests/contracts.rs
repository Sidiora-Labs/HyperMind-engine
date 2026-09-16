#![forbid(unsafe_code)]

use hm_llm::Usage;
use hm_llm::contract::{
    ContractViolation, EXTRACTION_CONTRACT_VERSION, ExtractionContract, FieldRule, FieldShape,
    MAXIMUM_IDENTIFIER_CHARACTERS, MAXIMUM_LIST_ITEMS, MAXIMUM_PROSE_CHARACTERS,
};
use hm_llm::outcome::{RESPONSE_CONTRACT_VERSION, ResponseOutcome};
use serde_json::{Value, json};

static FIELDS: [FieldRule; 7] = [
    FieldRule {
        name: "label",
        shape: FieldShape::Identifier,
    },
    FieldRule {
        name: "target",
        shape: FieldShape::NullableIdentifier,
    },
    FieldRule {
        name: "definition",
        shape: FieldShape::Prose,
    },
    FieldRule {
        name: "tags",
        shape: FieldShape::IdentifierList,
    },
    FieldRule {
        name: "salience_micros",
        shape: FieldShape::Count,
    },
    FieldRule {
        name: "durable",
        shape: FieldShape::Flag,
    },
    FieldRule {
        name: "citations",
        shape: FieldShape::Opaque,
    },
];

const fn contract() -> ExtractionContract {
    ExtractionContract {
        contract_id: "fixture-extraction@1",
        version: EXTRACTION_CONTRACT_VERSION,
        fields: &FIELDS,
    }
}

fn accepted() -> Value {
    json!({
        "label": "berlin office",
        "target": null,
        "definition": "The team moved to the Berlin office in March.",
        "tags": ["office", "relocation"],
        "salience_micros": 640_000,
        "durable": true,
        "citations": [{"source": "note-1", "offset": 12}]
    })
}

fn with(field: &str, value: Value) -> Value {
    let mut object = accepted();
    object
        .as_object_mut()
        .expect("fixture object")
        .insert(field.to_owned(), value);
    object
}

fn without(field: &str) -> Value {
    let mut object = accepted();
    object
        .as_object_mut()
        .expect("fixture object")
        .remove(field);
    object
}

fn violation(value: &Value) -> ContractViolation {
    contract()
        .validate(value)
        .expect_err("contract should refuse this value")
}

#[test]
fn contract_rejects_unknown_missing_and_wrongly_shaped_fields() {
    assert!(contract().validate(&accepted()).is_ok());

    for value in [
        json!("a bare string"),
        json!(7),
        json!(["a", "b"]),
        json!(null),
    ] {
        assert_eq!(violation(&value), ContractViolation::NotAnObject);
        assert_eq!(violation(&value).field(), None);
    }

    assert_eq!(
        violation(&without("definition")),
        ContractViolation::MissingField("definition".to_owned())
    );
    assert_eq!(
        violation(&with("authority", json!("tool_observed"))),
        ContractViolation::UnknownField("authority".to_owned())
    );
    assert_eq!(
        violation(&with("label", json!(7))),
        ContractViolation::WrongShape("label".to_owned())
    );
    assert_eq!(
        violation(&with("salience_micros", json!(true))),
        ContractViolation::WrongShape("salience_micros".to_owned())
    );
    assert_eq!(
        violation(&with("durable", json!("yes"))),
        ContractViolation::WrongShape("durable".to_owned())
    );
    assert_eq!(
        violation(&with("label", json!(null))),
        ContractViolation::WrongShape("label".to_owned())
    );
    assert_eq!(
        violation(&with("label", json!("   "))),
        ContractViolation::EmptyIdentifier("label".to_owned())
    );
    assert_eq!(
        violation(&with("definition", json!(" "))),
        ContractViolation::WrongShape("definition".to_owned())
    );
    assert_eq!(
        violation(&with("salience_micros", json!(-4))),
        ContractViolation::WrongShape("salience_micros".to_owned())
    );

    assert!(
        contract()
            .validate(&with("target", json!("berlin")))
            .is_ok()
    );
    assert!(
        contract()
            .validate(&with("salience_micros", json!(0)))
            .is_ok()
    );
    assert!(contract().validate(&with("durable", json!(false))).is_ok());
    assert_eq!(violation(&with("label", json!(7))).field(), Some("label"));
}

#[test]
fn identifier_bound_is_counted_in_characters_and_stays_under_the_ledger_ceiling() {
    assert_eq!(EXTRACTION_CONTRACT_VERSION, 1);
    assert_eq!(MAXIMUM_IDENTIFIER_CHARACTERS, 512);
    assert_eq!(MAXIMUM_PROSE_CHARACTERS, 65_536);
    assert_eq!(MAXIMUM_LIST_ITEMS, 64);

    let ascii = "a".repeat(512);
    let wide = "\u{1F600}".repeat(512);
    assert_eq!(wide.chars().count(), 512);
    assert_eq!(wide.len(), 2048);
    assert!(wide.len() <= hm_schema::event::MAXIMUM_IDENTIFIER_BYTES);

    assert!(contract().validate(&with("label", json!(ascii))).is_ok());
    assert!(contract().validate(&with("label", json!(wide))).is_ok());
    assert!(
        contract()
            .validate(&with("target", json!("\u{1F600}".repeat(512))))
            .is_ok()
    );

    for long in ["a".repeat(513), "\u{1F600}".repeat(513)] {
        assert_eq!(
            violation(&with("label", json!(long))),
            ContractViolation::IdentifierTooLong {
                field: "label".to_owned(),
                characters: 513,
                limit: 512,
            }
        );
        assert_eq!(
            violation(&with("target", json!(long))),
            ContractViolation::IdentifierTooLong {
                field: "target".to_owned(),
                characters: 513,
                limit: 512,
            }
        );
    }

    let definition = "d".repeat(1000);
    assert!(
        contract()
            .validate(&with("definition", json!(definition)))
            .is_ok()
    );
    assert_eq!(
        violation(&with("label", json!(definition))),
        ContractViolation::IdentifierTooLong {
            field: "label".to_owned(),
            characters: 1000,
            limit: 512,
        }
    );

    let prose = "p".repeat(MAXIMUM_PROSE_CHARACTERS + 1);
    assert_eq!(
        violation(&with("definition", json!(prose))),
        ContractViolation::ProseTooLong {
            field: "definition".to_owned(),
            characters: MAXIMUM_PROSE_CHARACTERS + 1,
            limit: MAXIMUM_PROSE_CHARACTERS,
        }
    );
}

#[test]
fn violation_detail_names_the_field_and_length_but_never_the_value() {
    let value = "zq".repeat(300);
    assert_eq!(value.chars().count(), 600);

    let failure = violation(&with("label", json!(value)));
    assert_eq!(
        failure,
        ContractViolation::IdentifierTooLong {
            field: "label".to_owned(),
            characters: 600,
            limit: 512,
        }
    );

    let detail = failure.detail();
    assert!(detail.contains("label"), "{detail}");
    assert!(detail.contains("600"), "{detail}");
    assert!(detail.contains("512"), "{detail}");
    assert!(!detail.contains(&value), "{detail}");
    assert!(!detail.contains("zqzq"), "{detail}");

    let usage = Usage {
        input_tokens: 40,
        output_tokens: 12,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 9,
    };
    let fault = contract().fault(&failure, "fixture-model", 256, usage);
    assert_eq!(fault.outcome, ResponseOutcome::Incomplete);
    assert_eq!(fault.version, RESPONSE_CONTRACT_VERSION);
    assert_eq!(fault.version, 1);
    assert_eq!(fault.detail, failure.detail());
    assert_eq!(fault.model_id, "fixture-model");
    assert_eq!(fault.requested_output_tokens, 256);
    assert_eq!(fault.usage, usage);
    assert!(!fault.detail.contains(&value));
}

#[test]
fn list_and_opaque_shapes_bound_entries_without_reading_them() {
    let sixty_four: Vec<String> = (0..64).map(|index| format!("tag-{index}")).collect();
    assert!(
        contract()
            .validate(&with("tags", json!(sixty_four)))
            .is_ok()
    );

    let sixty_five: Vec<String> = (0..65).map(|index| format!("tag-{index}")).collect();
    assert_eq!(
        violation(&with("tags", json!(sixty_five))),
        ContractViolation::ListTooLong {
            field: "tags".to_owned(),
            items: 65,
            limit: 64,
        }
    );

    assert_eq!(
        violation(&with("tags", json!([]))),
        ContractViolation::WrongShape("tags".to_owned())
    );
    assert_eq!(
        violation(&with("tags", json!("office"))),
        ContractViolation::WrongShape("tags".to_owned())
    );
    assert_eq!(
        violation(&with("tags", json!(["office", 4]))),
        ContractViolation::WrongShape("tags".to_owned())
    );
    assert_eq!(
        violation(&with("tags", json!(["office", "a".repeat(513)]))),
        ContractViolation::IdentifierTooLong {
            field: "tags".to_owned(),
            characters: 513,
            limit: 512,
        }
    );

    for opaque in [
        json!([{"source": "note-1", "offset": 3}, {"source": "note-2", "offset": 9}]),
        json!({"source": "note-1"}),
        json!("note-1"),
        json!(null),
        json!(41),
    ] {
        assert!(contract().validate(&with("citations", opaque)).is_ok());
    }
}
