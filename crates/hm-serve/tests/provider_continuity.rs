use hm_context::{history::SourceHistory, provider::*, provider_continuity::*, *};
use hm_compose::tokens::{FallbackWeights, TokenCounter as NativeCounter};

#[test]
fn exact_profile_tool_occurrences_and_original_recovery() {
    let scope = Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: None };
    let mut history = SourceHistory::new(scope.clone(), "session").unwrap();
    let mut messages = Vec::new();
    for ordinal in 0..4 {
        let role = if ordinal % 2 == 0 { MessageRole::Assistant } else { MessageRole::Tool };
        let part = if ordinal % 2 == 0 { MessagePart::ToolCall { call_id: "repeated".into(), name: "read".into(), arguments: "{}".into() } } else { MessagePart::ToolResult { call_id: "repeated".into(), content: "observed".into(), failed: false } };
        let mut message = SourceMessage { id: format!("source{ordinal}"), ordinal, role, parts: vec![part], occurred_at_ns: None, recorded_at_ns: 1, authority: Authority::ToolObserved, source_digest: String::new() };
        message.source_digest = message.computed_digest().unwrap();
        history.ingest(message.clone(), vec![0xff, 0, ordinal as u8]).unwrap();
        messages.push(message);
    }
    let mut opaque = SourceMessage { id: "opaque".into(), ordinal: 4, role: MessageRole::User, parts: vec![MessagePart::Opaque {media_type: "application/octet-stream".into(), reference: "asset:one".into(), digest: digest_bytes(&[0xff,0])}], occurred_at_ns: None, recorded_at_ns: 1, authority: Authority::UserAsserted, source_digest: String::new() };
    opaque.source_digest = opaque.computed_digest().unwrap();
    history.ingest(opaque.clone(), vec![0xff,0]).unwrap(); messages.push(opaque);
    let spans = recovery_plan(&history,&scope,&messages).unwrap();
    let profile = ProviderProfile { model_id:"gpt-4o".into(), model_revision:"configured-1".into(), tokenizer_id:"tiktoken:o200k_base".into(), tokenizer_revision:"configured-1".into(), serializer_id:"context-json-v1".into(), serializer_revision:"1".into(), capabilities: CapabilityProfile::default() };
    let budget=TokenBudget {context_tokens:4096,reserved_output_tokens:8,required_tokens:0};
    let fence=fence_profile(&profile,budget,"source-1").unwrap();
    let native=NativeCounter::for_model("gpt-4o",None,FallbackWeights::default()).unwrap();
    assert!(matches!(native,NativeCounter::Tiktoken {..}));
    let counter=|bytes:&[u8]| native.count(bytes).map(|n|n as u64).map_err(|e|ContextError::Unavailable(e.to_string()));
    let required=messages.iter().map(|m|m.id.clone()).collect::<Vec<_>>();
    let request=||RenderRequest {scope:scope.clone(),session_id:"session".into(),cursor:history.cursor(),generation:1,messages:&messages,source_spans:&spans,blocks:&[],required_message_ids:&required,budget,profile:profile.capabilities};
    let rendered=normalize_tool_pairs(request(),&profile,&fence,"source-1",&counter,false).unwrap();
    assert_eq!(rendered.report.token_count,counter(&serde_json::to_vec(&rendered.messages).unwrap()).unwrap());
    let id=|i:usize|match &rendered.messages[i].parts[0] {MessagePart::ToolCall{call_id,..}|MessagePart::ToolResult{call_id,..}=>call_id.clone(),_=>panic!()};
    assert_eq!(id(0),id(1)); assert_eq!(id(2),id(3)); assert_ne!(id(0),id(2));
    for message in &rendered.messages {assert_eq!(history.recover(&scope,&message.provenance[0]).unwrap(),if message.id=="opaque" {vec![0xff,0]} else {vec![0xff,0,message.id.as_bytes()[6]-b'0']});}
    assert_eq!(rendered.messages[4].parts,messages[4].parts);
    let mut changed=profile.clone(); changed.model_id="gpt-4o-mini".into();
    assert!(matches!(normalize_tool_pairs(request(),&changed,&fence,"source-1",&counter,false),Err(ContinuityError::ProfileChanged)));
    changed=profile.clone(); changed.tokenizer_revision="changed".into(); assert_ne!(fence_profile(&changed,budget,"source-1").unwrap(),fence);
    assert_ne!(fence_profile(&profile,budget,"source-2").unwrap(),fence);
    let mut changed_budget=budget; changed_budget.reserved_output_tokens=9; assert_ne!(fence_profile(&profile,changed_budget,"source-1").unwrap(),fence);
    let mut broken=messages.clone(); broken.swap(1,2); let mut broken_request=request(); broken_request.messages=&broken; assert!(normalize_tool_pairs(broken_request,&profile,&fence,"source-1",&counter,false).is_err());
    changed=profile.clone(); changed.serializer_id="ollama-native-tools".into();
    assert!(matches!(normalize_tool_pairs(request(),&changed,&fence,"source-1",&counter,false),Err(ContinuityError::UnsupportedSerializer(_))));
    assert!(matches!(normalize_tool_pairs(request(),&profile,&fence,"source-1",&counter,true),Err(ContinuityError::Cancelled)));
    assert!(normalize_tool_pairs(request(),&profile,&fence,"source-1",&UnsupportedTokenizer,false).is_err());
    let mut tiny=request(); tiny.budget.context_tokens=9;
    let tiny_fence=fence_profile(&profile,tiny.budget,"source-1").unwrap();
    assert!(matches!(normalize_tool_pairs(tiny,&profile,&tiny_fence,"source-1",&counter,false),Err(ContinuityError::Render(RenderError::RequiredOverflow{..}))));
    assert!(validate_output_budget(8,budget).is_ok()); assert!(validate_output_budget(9,budget).is_err());
}

#[tokio::test]
async fn actual_local_text_generation_has_measured_output_ceiling() {
    let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(45)).build().unwrap();
    let response=client.post("http://127.0.0.1:11439/api/generate").json(&serde_json::json!({"model":"qwen2.5:3b","prompt":"Reply with one short word.","stream":false,"options":{"num_predict":8,"temperature":0}})).send().await.unwrap().error_for_status().unwrap();
    let observation:serde_json::Value=response.json().await.unwrap();
    assert_eq!(observation["done"],true);
    assert!(observation["prompt_eval_count"].as_u64().unwrap()>0);
    let measured=observation["eval_count"].as_u64().unwrap();
    validate_output_budget(measured,TokenBudget{context_tokens:4096,reserved_output_tokens:8,required_tokens:0}).unwrap();
    // Measured counters establish this text path, not an exact Qwen input counter
    // or a serializer for native tool identifiers and opaque parts.
}
