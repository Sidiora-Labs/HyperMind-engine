use hm_context::TokenBudget;
use hm_serve::context_tokenizer::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

async fn inference(client: &reqwest::Client, endpoint: &str, bytes: Vec<u8>) -> Value {
    let response = client
        .post(format!("{endpoint}/api/chat"))
        .header("Content-Type", "application/json")
        .body(bytes)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    response.json().await.unwrap()
}

#[tokio::test]
async fn actual_tool_cycle_matches_pinned_server_framing_and_reserve() {
    let config: OwnedTokenizerConfig = serde_json::from_slice(
        &std::fs::read(std::env::var("HYPERMIND_CONTEXT_TOKENIZERS").unwrap()).unwrap(),
    )
    .unwrap();
    let installed = config
        .tokenizers
        .iter()
        .find(|t| t.model_id == "qwen2.5:3b")
        .unwrap()
        .clone();
    let endpoint = installed
        .ollama_text
        .as_ref()
        .unwrap()
        .endpoint
        .as_ref()
        .unwrap();
    let identity = identity(&installed.model_id).unwrap();
    let budget = TokenBudget {
        context_tokens: 4096,
        reserved_output_tokens: 128,
        required_tokens: 0,
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("facts.txt");
    std::fs::write(
        &file,
        "The observed label is quartz. Unicode observation: 你好.\n",
    )
    .unwrap();
    let tools = json!([{"type":"function","function":{"name":"read_file","description":"Read actual UTF-8 file contents at the specified absolute path.","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}}]);
    let mut payload = json!({"model":installed.model_id,"messages":[{"role":"system","content":"Use the read_file function to observe the requested file before answering. Call read_file exactly once, then summarize the actual observed label."},{"role":"user","content":format!("Call read_file with path {} and report the observed label.",file.display())}],"tools":tools,"stream":false,"truncate":false,"options":{"num_ctx":4096,"num_predict":128,"temperature":0}});
    let original = serde_json::to_vec_pretty(&payload).unwrap();
    let measured =
        count_ollama_chat_input(&installed.model_id, identity.generation, &original, budget)
            .await
            .unwrap();
    assert_eq!(
        measured.payload_sha256,
        format!("{:x}", Sha256::digest(&original))
    );
    assert!(measured.tool_occurrences.is_empty());
    let first = inference(&client, endpoint, original).await;
    assert_eq!(first["done"], true);
    assert_eq!(
        first["prompt_eval_count"].as_u64().unwrap(),
        measured.tokens
    );
    assert!(first["eval_count"].as_u64().unwrap() <= budget.reserved_output_tokens);
    let actual_calls = first["message"]["tool_calls"]
        .as_array()
        .expect("actual provider must propose a tool call");
    assert_eq!(actual_calls.len(), 1);
    let function = &actual_calls[0]["function"];
    assert_eq!(function["name"], "read_file");
    assert_eq!(
        function["arguments"]["path"].as_str().unwrap(),
        file.to_str().unwrap()
    );
    let call_id = actual_calls[0]["id"].as_str().unwrap_or("ollama-0");
    let result = std::fs::read_to_string(function["arguments"]["path"].as_str().unwrap()).unwrap();
    let messages = payload["messages"].as_array_mut().unwrap();
    messages.push(json!({"role":"assistant","content":first["message"]["content"],"tool_calls":[{"id":call_id,"type":"function","function":function}]}));
    messages
        .push(json!({"role":"tool","tool_call_id":call_id,"content":result,"_tool_status":true}));
    let followup = serde_json::to_vec_pretty(&payload).unwrap();
    let counted =
        count_ollama_chat_input(&installed.model_id, identity.generation, &followup, budget)
            .await
            .unwrap();
    assert_eq!(
        counted.tool_occurrences,
        vec![ToolOccurrence {
            message_index: 2,
            call_index: 0,
            call_id: call_id.into()
        }]
    );
    let final_response = inference(&client, endpoint, followup.clone()).await;
    assert_eq!(final_response["done"], true);
    assert_eq!(
        final_response["prompt_eval_count"].as_u64().unwrap(),
        counted.tokens
    );
    assert!(final_response["eval_count"].as_u64().unwrap() <= budget.reserved_output_tokens);

    let mut repeated = payload.clone();
    let cycle = payload["messages"].as_array().unwrap()[2..].to_vec();
    repeated["messages"].as_array_mut().unwrap().extend(cycle);
    let repeated_bytes = serde_json::to_vec(&repeated).unwrap();
    let repeated_count = count_ollama_chat_input(
        &installed.model_id,
        identity.generation,
        &repeated_bytes,
        budget,
    )
    .await
    .unwrap();
    assert_eq!(repeated_count.tool_occurrences.len(), 2);
    assert_eq!(
        repeated_count.tool_occurrences[0].call_id,
        repeated_count.tool_occurrences[1].call_id
    );
    assert_ne!(
        repeated_count.tool_occurrences[0].message_index,
        repeated_count.tool_occurrences[1].message_index
    );
    // Replay actual completed observations in the request; never fabricate a tool result.
    let replayed = inference(&client, endpoint, repeated_bytes).await;
    assert_eq!(
        replayed["prompt_eval_count"].as_u64().unwrap(),
        repeated_count.tokens
    );
    assert!(replayed["eval_count"].as_u64().unwrap() <= budget.reserved_output_tokens);

    let mut malformed = payload.clone();
    malformed["messages"][2]["tool_calls"][0]["function"]["arguments"] = Value::String("{}".into());
    assert!(
        count_ollama_chat_input(
            &installed.model_id,
            identity.generation,
            &serde_json::to_vec(&malformed).unwrap(),
            budget
        )
        .await
        .is_err()
    );
    let mut opaque = payload.clone();
    opaque["messages"][1]["images"] = json!(["/w=="]);
    assert!(
        count_ollama_chat_input(
            &installed.model_id,
            identity.generation,
            &serde_json::to_vec(&opaque).unwrap(),
            budget
        )
        .await
        .is_err()
    );
    let mut parts = payload.clone();
    parts["messages"][1]["content"] = json!([{"type":"text","text":"unsupported multipart"}]);
    assert!(
        count_ollama_chat_input(
            &installed.model_id,
            identity.generation,
            &serde_json::to_vec(&parts).unwrap(),
            budget
        )
        .await
        .is_err()
    );
    let mut injected = followup.clone();
    let closing = injected.iter().rposition(|b| *b == b'}').unwrap();
    injected.splice(
        closing..closing,
        b",\"model\":\"qwen2.5:3b\"".iter().copied(),
    );
    assert!(
        count_ollama_chat_input(&installed.model_id, identity.generation, &injected, budget)
            .await
            .is_err()
    );
    let mut debug = payload.clone();
    debug["_debug_render_only"] = json!(true);
    assert!(
        count_ollama_chat_input(
            &installed.model_id,
            identity.generation,
            &serde_json::to_vec(&debug).unwrap(),
            budget
        )
        .await
        .is_err()
    );
    let mut excess = payload.clone();
    excess["options"]["num_predict"] = json!(129);
    assert!(
        count_ollama_chat_input(
            &installed.model_id,
            identity.generation,
            &serde_json::to_vec(&excess).unwrap(),
            budget
        )
        .await
        .is_err()
    );
    let renewed = install_owned(installed.clone(), identity.generation).unwrap();
    assert!(renewed.generation > identity.generation);
    assert!(
        count_ollama_chat_input(&installed.model_id, identity.generation, &followup, budget)
            .await
            .is_err()
    );
    let mut changed = installed;
    changed.ollama_text.as_mut().unwrap().server_version = Some("unobserved-version".into());
    let stale = install_owned(changed, renewed.generation).unwrap();
    assert!(
        count_ollama_chat_input(&stale.model_id, stale.generation, &followup, budget)
            .await
            .is_err()
    );
}
