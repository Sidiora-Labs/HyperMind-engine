#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

#[path = "../src/harness.rs"]
#[allow(dead_code)]
mod harness;

use harness::{HarnessResult, JourneyHarness};
use hm_core::ErrorCode;
use hm_schema::protocol::{CURRENT_PROTOCOL_VERSION, encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    AsOf, BeliefResult, Hello, Request, RequestPayload, ResponsePayload, ResponseStatus,
    WireEnvelope, WirePayload,
};
use hm_serve::protocol::{FrameParser, encode_frame};
use serde_json::{Value, json};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const CONVERSATION: &str = "slice5-belief-journey";

#[tokio::main]
async fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [mode, config] if mode == "mcp" => harness::run_mcp_daemon(Path::new(config)).await,
        [mode, config] if mode == "uds" => harness::run_uds_daemon(Path::new(config)).await,
        [] => journey().await,
        _ => Err("invalid slice5 journey helper invocation".into()),
    };
    if let Err(error) = result {
        eprintln!("slice5 journey failed: {error}");
        std::process::exit(1);
    }
}

async fn journey() -> HarnessResult<()> {
    let temporary = tempfile::tempdir()?;
    let harness = JourneyHarness::create(temporary.path(), std::env::current_exe()?)?;
    let writer = harness.start_mcp().await?;
    let evidence = writer
        .call(
            "remember",
            json!({
                "conversation": CONVERSATION,
                "content": "deployment state observed by the operator",
                "kind": "user"
            }),
        )
        .await?;
    let evidence_lsn = evidence["items"][0]["first_lsn"]
        .as_u64()
        .ok_or("remember omitted evidence LSN")?;
    let source = json!([{
        "first_lsn": evidence_lsn,
        "last_lsn": evidence_lsn,
        "byte_start": 0,
        "byte_end": 41
    }]);
    let first = believe_args(
        "deployment-europe",
        "fact",
        "deployment:active",
        "Europe",
        1,
        100,
        &source,
    );
    let second = believe_args(
        "deployment-america",
        "fact",
        "deployment:active",
        "America",
        101,
        0,
        &source,
    );
    require_lsn(&writer.call("believe", first).await?, 2)?;
    require_lsn(&writer.call("believe", second).await?, 3)?;

    let dispute = writer
        .call(
            "dispute",
            json!({
                "conversation": CONVERSATION,
                "existing": claim(
                    "europe-claim",
                    "deployment:region:europe",
                    "The deployment region is Europe.",
                    10,
                    &source,
                ),
                "incoming": claim(
                    "america-claim",
                    "deployment:region:america",
                    "The deployment region is America.",
                    20,
                    &source,
                )
            }),
        )
        .await?;
    if dispute["items"][0]["verdict"] != "genuine"
        || dispute["items"][0]["suggested_action"] != "review"
        || dispute["items"][0]["resulting_events"] != json!([])
    {
        return Err("dispute did not run the real tier-gated bidirectional NLI path".into());
    }

    let protected = believe_args(
        "run-identity",
        "identity",
        "self:name",
        "HyperMind",
        0,
        0,
        &source,
    );
    let protected = writer
        .call("believe", with_run_id(protected, "slice5-run"))
        .await
        .expect_err("protected write must be rejected");
    let protected = protected.to_string();
    if !protected.contains(ErrorCode::ProtectedTypeWrite.as_str())
        || !protected.contains("rejected")
    {
        return Err("protected run write lacked the typed rejected effect state".into());
    }
    writer.kill().await?;

    let reader = harness.start_mcp().await?;
    let recalled = reader
        .call(
            "recall",
            json!({"mode": "lexical", "query": "operator", "limit": 10}),
        )
        .await?;
    if !recalled["items"]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item["lsn"] == evidence_lsn))
    {
        return Err("restart recall omitted the belief evidence".into());
    }
    let activated = reader
        .call(
            "activate",
            json!({
                "conversation": CONVERSATION,
                "query": "identity proposal",
                "turn_text": "",
                "budget_tokens": 8192
            }),
        )
        .await?;
    if !activated["items"].as_array().is_some_and(|items| {
        items.iter().any(|item| {
            item["tier"] == "conflicts"
                && item["uri"]
                    .as_str()
                    .is_some_and(|uri| uri.contains("src=proposal"))
        })
    }) {
        return Err("protected proposal did not survive restart into conflicts".into());
    }
    reader.close().await?;

    let daemon = harness.start_uds().await?;
    let socket = temporary.path().join("hypermind.sock");
    let valid = as_of(&socket, 1, 50, 0).await?;
    let known = as_of(&socket, 2, 0, 3).await?;
    if valid.value.as_deref() != Some(b"Europe".as_slice())
        || valid.version != 1
        || known.value.as_deref() != Some(b"America".as_slice())
        || known.version != 2
        || known.supersedes_version != 1
    {
        return Err("valid_at and known_at did not return different scripted versions".into());
    }
    daemon.kill()?;

    let gate = hm_eval::slice5::run()
        .await
        .map_err(|error| error.to_string())?;
    if gate
        .judge_free
        .iter()
        .find(|metric| metric.name == "stale_fact_rate")
        .is_none_or(|metric| metric.value != 0.0)
    {
        return Err("slice5 stale-fact gate was not zero".into());
    }
    Ok(())
}

fn believe_args(
    belief_id: &str,
    belief_type: &str,
    canonical_identity: &str,
    value: &str,
    valid_from_ns: i64,
    valid_to_ns: i64,
    provenance: &Value,
) -> Value {
    json!({
        "conversation": CONVERSATION,
        "belief_id": belief_id,
        "belief_type": belief_type,
        "canonical_identity": canonical_identity,
        "value": value,
        "valid_from_ns": valid_from_ns,
        "valid_to_ns": valid_to_ns,
        "provenance": provenance,
        "conflict_domain": "deployment:region",
        "claim": "affirmative"
    })
}

fn claim(
    belief_id: &str,
    canonical_identity: &str,
    value: &str,
    valid_from_ns: i64,
    provenance: &Value,
) -> Value {
    json!({
        "belief_id": belief_id,
        "belief_type": "fact",
        "canonical_identity": canonical_identity,
        "value": value,
        "valid_from_ns": valid_from_ns,
        "valid_to_ns": 0,
        "provenance": provenance,
        "conflict_domain": "deployment:region",
        "claim": "affirmative"
    })
}

fn with_run_id(mut input: Value, run_id: &str) -> Value {
    input["run_id"] = Value::String(run_id.to_owned());
    input["conflict_domain"] = Value::String("protected-self".to_owned());
    input
}

fn require_lsn(envelope: &Value, expected: u64) -> HarnessResult<()> {
    if envelope["items"][0]["lsn"] == expected {
        Ok(())
    } else {
        Err(format!("believe returned the wrong LSN: {envelope}").into())
    }
}

async fn as_of(
    socket: &Path,
    request_id: u64,
    valid_time_ns: i64,
    known_lsn: u64,
) -> HarnessResult<BeliefResult> {
    let mut stream = UnixStream::connect(socket).await?;
    let welcome = exchange(
        &mut stream,
        WirePayload::Hello(Box::new(Hello {
            proto_version: CURRENT_PROTOCOL_VERSION,
            connection_id: vec![u8::try_from(request_id)?; 16],
            capability_token: vec![0x44; 32],
        })),
    )
    .await?;
    if !matches!(welcome.payload, WirePayload::Welcome(_)) {
        return Err("as-of handshake failed".into());
    }
    let response = exchange(
        &mut stream,
        WirePayload::Request(Box::new(Request {
            request_id,
            payload: RequestPayload::AsOf(Box::new(AsOf {
                belief_type: 0,
                canonical_identity: "deployment:active".to_owned(),
                valid_time_ns,
                transaction_lsn: 0,
                known_lsn,
            })),
        })),
    )
    .await?;
    let WirePayload::Response(response) = response.payload else {
        return Err("as-of returned a non-response".into());
    };
    if response.status != ResponseStatus::Ok {
        return Err("as-of returned an error".into());
    }
    let Some(ResponsePayload::BeliefResult(result)) = response.payload else {
        return Err("as-of omitted its belief result".into());
    };
    Ok(*result)
}

async fn exchange(stream: &mut UnixStream, payload: WirePayload) -> HarnessResult<WireEnvelope> {
    let encoded = encode_wire_envelope(&WireEnvelope {
        proto_version: CURRENT_PROTOCOL_VERSION,
        payload,
    });
    stream.write_all(&encode_frame(&encoded)?).await?;
    let mut header = [0; 8];
    stream.read_exact(&mut header).await?;
    let length = u32::from_le_bytes(header[..4].try_into()?) as usize;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await?;
    let mut parser = FrameParser::default();
    let frames = parser.push(&[header.as_slice(), payload.as_slice()].concat())?;
    Ok(verify_wire_envelope(
        frames.first().ok_or("daemon returned no frame")?,
    )?)
}
