#![forbid(unsafe_code)]

#[path = "../src/harness.rs"]
mod harness;

use base64::Engine as _;
use harness::{HarnessResult, JourneyHarness};
use serde_json::{Value, json};
use std::path::Path;

const QUERY: &str = "heliotrope";
const BUDGET_TOKENS: usize = 4_096;
const FACTS: [&str; 3] = [
    "The heliotrope release channel is amber.",
    "The heliotrope deployment region is eu-central-1.",
    "The heliotrope rollback owner is the runtime team.",
];

#[tokio::main]
async fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [mode, config] if mode == "mcp" => harness::run_mcp_daemon(Path::new(config)).await,
        [mode, config] if mode == "uds" => harness::run_uds_daemon(Path::new(config)).await,
        [] => slice1_journey().await,
        _ => Err("invalid slice1 journey helper invocation".into()),
    };
    if let Err(error) = result {
        eprintln!("slice1 journey failed: {error}");
        std::process::exit(1);
    }
}

async fn slice1_journey() -> HarnessResult<()> {
    let temporary = tempfile::tempdir()?;
    let harness = JourneyHarness::create(temporary.path(), std::env::current_exe()?)?;

    let writer = harness.start_mcp().await?;
    for (index, fact) in FACTS.iter().enumerate() {
        let remembered = writer
            .call(
                "remember",
                json!({
                    "conversation": format!("slice1-fact-{index}"),
                    "content": fact,
                    "kind": "user"
                }),
            )
            .await?;
        let provenance = array(&remembered, "provenance")?;
        if provenance.len() != 1 || !uri(provenance[0].as_str()) {
            return Err("remember did not return one provenance URI".into());
        }
    }
    writer.kill().await?;

    let reader = harness.start_mcp().await?;
    let recalled = reader
        .call(
            "recall",
            json!({"mode": "lexical", "query": QUERY, "limit": 10}),
        )
        .await?;
    assert_facts_and_provenance(&recalled)?;

    let activated = reader
        .call(
            "activate",
            json!({
                "conversation": "slice1-fact-0",
                "query": QUERY,
                "turn_text": "",
                "budget_tokens": BUDGET_TOKENS
            }),
        )
        .await?;
    assert_facts_and_provenance(&activated)?;
    let expected_hash = activated
        .pointer("/health/bundle_hash")
        .and_then(Value::as_str)
        .ok_or("activation omitted bundle hash")?;
    if expected_hash.len() != 64 || !expected_hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("activation returned an invalid bundle hash".into());
    }
    reader.close().await?;

    let daemon = harness.start_uds().await?;
    let canonical = harness
        .uds_activate("slice1-fact-0", QUERY, BUDGET_TOKENS)
        .await?;
    if !canonical.starts_with(hm_compose::canonical::CANONICAL_MAGIC) {
        return Err("daemon activation was not HMA1 canonical bytes".into());
    }
    let actual_hash = blake3::hash(&canonical).to_hex().to_string();
    if actual_hash != expected_hash {
        return Err("HMA1 bytes did not match the MCP bundle hash".into());
    }
    daemon.kill()?;

    hm_eval::slice1::gate(None)
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn assert_facts_and_provenance(envelope: &Value) -> HarnessResult<()> {
    let items = array(envelope, "items")?;
    let provenance = array(envelope, "provenance")?;
    if provenance.len() < FACTS.len() || !provenance.iter().all(|value| uri(value.as_str())) {
        return Err("result omitted full provenance URIs".into());
    }
    let mut contents = Vec::with_capacity(items.len());
    for item in items {
        if let Some(content) = item.get("content").and_then(Value::as_str) {
            contents.push(content.to_owned());
        } else if let Some(encoded) = item.get("content_base64").and_then(Value::as_str) {
            contents.push(String::from_utf8(
                base64::engine::general_purpose::STANDARD.decode(encoded)?,
            )?);
        }
        if !uri(item.get("uri").and_then(Value::as_str)) {
            return Err("result item omitted its provenance URI".into());
        }
    }
    for fact in FACTS {
        if !contents.iter().any(|content| content == fact) {
            return Err(format!("result omitted fact: {fact}").into());
        }
    }
    Ok(())
}

fn array<'value>(value: &'value Value, field: &str) -> HarnessResult<&'value Vec<Value>> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("envelope field {field} was not an array").into())
}

fn uri(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.starts_with("hm://7/") && value.contains("?at="))
        || value.is_some_and(|value| value.starts_with("hm://7/lsn/"))
}
