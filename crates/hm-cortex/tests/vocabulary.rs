#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::ErrorCode;
use hm_cortex::ingest::IngestResult;
use hm_cortex::vocabulary::{MAXIMUM_SOURCE_BYTES, VocabularySource, import_envelope, parse_terms};
use hm_schema::event::{Boundary, EventKind, encode_event_envelope, verify_event};
use hm_schema::events::{
    Authority, EventPayload, Retention, Sensitivity, VocabularyCategory, VocabularyTerm,
};
use std::fmt::Write as _;

const FIXTURE: &str = r#"# HyperMind vocabulary fixture
<http://hypermind.test/vocab#Component> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .
<http://hypermind.test/vocab#Component> <http://www.w3.org/2000/01/rdf-schema#label> "Component" .
<http://hypermind.test/vocab#Component> <http://purl.org/dc/terms/created> "2026-01-01T00:00:00Z"^^<http://www.w3.org/2001/XMLSchema#dateTime> .

<http://hypermind.test/vocab#dependsOn> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#ObjectProperty> .
<http://hypermind.test/vocab#dependsOn> <http://www.w3.org/2000/01/rdf-schema#label> "depends on" .
<http://hypermind.test/vocab#dependsOn> <http://www.w3.org/2004/02/skos/core#altLabel> "requires" .
<http://hypermind.test/vocab#dependsOn> <http://www.w3.org/2004/02/skos/core#altLabel> "requires" .
<http://hypermind.test/vocab#dependsOn> <http://www.w3.org/2002/07/owl#equivalentProperty> <http://other.test/schema#relies_on> .
<http://hypermind.test/vocab#dependsOn> <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> <http://hypermind.test/vocab#relatedTo> .
<http://hypermind.test/vocab#dependsOn> <http://www.w3.org/2000/01/rdf-schema#comment> "A \"hard\" dependency."@en .

<http://hypermind.test/vocab#gateway> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#NamedIndividual> .
"#;

fn expected_terms() -> Vec<VocabularyTerm> {
    vec![
        VocabularyTerm {
            term_id: "http://hypermind.test/vocab#Component".to_owned(),
            canonical_name: "Component".to_owned(),
            category: VocabularyCategory::EntityType,
            parent_term_id: None,
            aliases: Some(vec!["Component".to_owned()]),
        },
        VocabularyTerm {
            term_id: "http://hypermind.test/vocab#dependsOn".to_owned(),
            canonical_name: "dependsOn".to_owned(),
            category: VocabularyCategory::Relation,
            parent_term_id: Some("http://hypermind.test/vocab#relatedTo".to_owned()),
            aliases: Some(vec![
                "depends on".to_owned(),
                "relies_on".to_owned(),
                "requires".to_owned(),
            ]),
        },
        VocabularyTerm {
            term_id: "http://hypermind.test/vocab#gateway".to_owned(),
            canonical_name: "gateway".to_owned(),
            category: VocabularyCategory::EntityInstance,
            parent_term_id: None,
            aliases: None,
        },
    ]
}

fn source(document: &str) -> VocabularySource<'_> {
    VocabularySource {
        vocabulary_id: b"hypermind-vocabulary",
        version: 3,
        source_uri: "https://hypermind.test/vocab.nt",
        document,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
    }
}

#[test]
fn parses_recognised_triples_and_counts_the_rest() {
    let (terms, ignored_triples) = parse_terms(FIXTURE).unwrap();
    assert_eq!(terms, expected_terms());
    assert_eq!(ignored_triples, 2);

    let untyped = concat!(
        "<http://hypermind.test/vocab#Component> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .\n",
        "<http://hypermind.test/vocab#Ghost> <http://www.w3.org/2000/01/rdf-schema#label> \"Ghost\" .\n",
        "<http://hypermind.test/vocab#Ghost> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://hypermind.test/vocab#Component> .\n",
    );
    let (terms, ignored_triples) = parse_terms(untyped).unwrap();
    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].term_id, "http://hypermind.test/vocab#Component");
    assert_eq!(ignored_triples, 2);
}

#[test]
fn rejects_malformed_and_oversized_documents() {
    let prefix = "<http://hypermind.test/vocab#Component> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .\n";
    let malformed = [
        "<http://hypermind.test/vocab#A> <http://www.w3.org/2000/01/rdf-schema#label> \"A\"\n",
        "<http://hypermind.test/vocab#A <http://www.w3.org/2000/01/rdf-schema#label> \"A\" .\n",
        "<http://hypermind.test/vocab#A> <http://www.w3.org/2000/01/rdf-schema#label> A .\n",
        "<http://hypermind.test/vocab#A> <http://www.w3.org/2000/01/rdf-schema#label> .\n",
    ];
    for line in malformed {
        let document = format!("{prefix}{line}");
        assert_eq!(
            parse_terms(&document).unwrap_err().code,
            ErrorCode::SchemaInvalid,
            "line must be rejected: {line}"
        );
    }

    let padding = "# padding comment line\n";
    let oversized = padding.repeat(MAXIMUM_SOURCE_BYTES / padding.len() + 2);
    assert!(oversized.len() > MAXIMUM_SOURCE_BYTES);
    assert_eq!(
        parse_terms(&oversized).unwrap_err().code,
        ErrorCode::CapacityExceeded
    );

    let mut crowded = String::new();
    for index in 0..=hm_schema::event::MAXIMUM_VOCABULARY_TERMS {
        writeln!(
            crowded,
            "<hypermind:term:{index:05}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> ."
        )
        .expect("write a term declaration");
    }
    assert!(crowded.len() <= MAXIMUM_SOURCE_BYTES);
    assert_eq!(
        parse_terms(&crowded).unwrap_err().code,
        ErrorCode::CapacityExceeded
    );
}

#[test]
fn import_envelope_is_user_asserted_digest_bound_and_schema_valid() {
    let source = source(FIXTURE);
    let IngestResult::Append(envelope) = import_envelope(&source).unwrap() else {
        panic!("a durable vocabulary import must append");
    };
    assert_eq!(envelope.authority, Authority::UserAsserted);
    assert_eq!(envelope.schema_version, 2);
    assert_eq!(envelope.retention, Retention::Durable);
    assert_eq!(envelope.sensitivity, Sensitivity::Personal);
    let EventPayload::VocabularyImported(payload) = &envelope.payload else {
        panic!("import must build a VocabularyImported payload");
    };
    assert_eq!(payload.vocabulary_id, b"hypermind-vocabulary".to_vec());
    assert_eq!(payload.version, 3);
    assert_eq!(payload.source_uri, "https://hypermind.test/vocab.nt");
    assert_eq!(payload.source_media_type, "application/n-triples");
    assert_eq!(
        payload.source_digest,
        blake3::hash(FIXTURE.as_bytes()).as_bytes().to_vec()
    );
    assert_eq!(payload.ignored_triples, 2);
    assert_eq!(payload.terms, expected_terms());

    let encoded = encode_event_envelope(&envelope);
    let verified = verify_event(&encoded, EventKind::VocabularyImported, Boundary::Socket).unwrap();
    assert_eq!(verified.kind, EventKind::VocabularyImported);

    let IngestResult::Append(again) = import_envelope(&source).unwrap() else {
        panic!("a durable vocabulary import must append");
    };
    assert_eq!(encode_event_envelope(&again), encoded);
}
