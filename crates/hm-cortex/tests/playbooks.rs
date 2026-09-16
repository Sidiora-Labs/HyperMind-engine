#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_cortex::playbook::{MAXIMUM_DOCUMENT_BYTES, parse, procedure_id};
use hm_schema::event::{Boundary, EventKind, encode_event_envelope, verify_event};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProcedureImported, Retention, Sensitivity,
};

const SOURCE_URI: &str = "file:///playbooks/restart-ingest";

const CANONICAL: &str = r"%%
name: restart the ingest worker
strategy: drain the queue, then restart the worker
outcomes: queue depth returns to zero; alerts clear
preconditions: the worker is unresponsive; the lease is free
tools: shell; ledger
version: 3
%%
1. drain the queue
2. restart the worker
";

const EMPTY_BODY: &str = r"%%
name: restart the ingest worker
strategy: drain the queue, then restart the worker
outcomes: queue depth returns to zero
%%
";

#[test]
fn playbook_documents_parse_into_admissible_imported_procedures() {
    let parsed = parse(SOURCE_URI, CANONICAL.as_bytes()).unwrap();
    assert_eq!(
        parsed,
        ProcedureImported {
            procedure_id: procedure_id(SOURCE_URI, "restart the ingest worker"),
            name: "restart the ingest worker".to_owned(),
            strategy: "drain the queue, then restart the worker".to_owned(),
            expected_outcomes: vec![
                "queue depth returns to zero".to_owned(),
                "alerts clear".to_owned(),
            ],
            preconditions: vec![
                "the worker is unresponsive".to_owned(),
                "the lease is free".to_owned(),
            ],
            instructions: b"1. drain the queue\n2. restart the worker\n".to_vec(),
            declared_tools: vec!["shell".to_owned(), "ledger".to_owned()],
            source_uri: SOURCE_URI.to_owned(),
            source_digest: blake3::hash(CANONICAL.as_bytes()).as_bytes().to_vec(),
            playbook_version: 3,
        }
    );
    assert_eq!(parsed.procedure_id.len(), 32);
    assert_eq!(parsed.source_digest.len(), 32);

    let again = parse(SOURCE_URI, CANONICAL.as_bytes()).unwrap();
    assert_eq!(again.procedure_id, parsed.procedure_id);
    assert_eq!(again.source_digest, parsed.source_digest);

    let elsewhere = parse(
        "https://example.invalid/restart-ingest",
        CANONICAL.as_bytes(),
    )
    .unwrap();
    assert_ne!(elsewhere.procedure_id, parsed.procedure_id);
    assert_eq!(elsewhere.source_digest, parsed.source_digest);

    let non_utf8 = {
        let mut bytes = CANONICAL.as_bytes().to_vec();
        bytes.push(0xff);
        bytes
    };
    for malformed in [
        CANONICAL.replace("tools:", "toolset:").into_bytes(),
        CANONICAL
            .replace("version: 3", "name: restarted")
            .into_bytes(),
        CANONICAL
            .replace("name: restart the ingest worker\n", "")
            .into_bytes(),
        CANONICAL
            .replace("strategy: drain the queue, then restart the worker\n", "")
            .into_bytes(),
        CANONICAL
            .replace("outcomes: queue depth returns to zero; alerts clear\n", "")
            .into_bytes(),
        CANONICAL
            .replace("tools: shell; ledger", "tools: shell;;ledger")
            .into_bytes(),
        EMPTY_BODY.as_bytes().to_vec(),
        non_utf8,
    ] {
        assert_eq!(
            parse(SOURCE_URI, &malformed).unwrap_err().code,
            ErrorCode::InvalidArgument
        );
    }

    let mut bounded = CANONICAL.as_bytes().to_vec();
    bounded.resize(MAXIMUM_DOCUMENT_BYTES, b'x');
    parse(SOURCE_URI, &bounded).unwrap();
    let mut oversize = CANONICAL.as_bytes().to_vec();
    oversize.resize(MAXIMUM_DOCUMENT_BYTES + 1, b'x');
    assert_eq!(
        parse(SOURCE_URI, &oversize).unwrap_err().code,
        ErrorCode::CapacityExceeded
    );

    let verified = verify_event(
        &encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload: EventPayload::ProcedureImported(Box::new(parsed)),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 7,
            run_id: None,
            model_provenance: None,
            authority: Authority::ExternalObserved,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
        EventKind::ProcedureImported,
        Boundary::Socket,
    )
    .unwrap();
    assert_eq!(verified.kind, EventKind::ProcedureImported);
}
