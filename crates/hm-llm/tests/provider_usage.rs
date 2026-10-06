use hm_llm::{catalog::ModelCatalog,provider_usage::*};
fn metadata(format:ObservationFormat,bytes:&[u8])->ObservationMetadata {
    ObservationMetadata {provider_id:"provider".into(),source:"external_format_file".into(),evidence_id:blake3::hash(bytes).to_hex().to_string(),observed_at_ns:1791288000123456789,expires_at_ns:Some(1791288000123456889),format}
}
#[test]
fn actual_external_account_file_preserves_quota_zero_unknown_and_funding() {
    let bytes=br#"{"data":{"account_id":"account","organization_id":"organization","project_id":"project","label":"development","currency":"USD","limit":"10.25","limit_remaining":0,"usage_daily":0,"usage_weekly":null,"balance":"-0.0000000001","funding":{"total_credits":"20.5","total_usage":"10.25","sources":[{"kind":"credit_purchase","reference":"receipt"}]},"rate_limit":{"requests":100,"interval":"10s"},"quota_windows":[{"name":"tokens","unit":"tokens","limit":"9007199254740993","remaining":0,"starts_at_ns":"1791288000123456789","resets_at_ns":"1791288000123456889","refill":{"amount":10,"interval_ms":1000},"future_window":true}],"future_account":{"flag":true}},"error":{"code":429,"message":"quota unavailable"},"future_root":true}"#;
    let path=std::env::temp_dir().join(format!("hypermind-provider-usage-{}.json",std::process::id()));
    std::fs::write(&path,bytes).unwrap();let source=std::fs::read(&path).unwrap();std::fs::remove_file(path).unwrap();
    let snapshot=ProviderUsageSnapshot::parse(&source,metadata(ObservationFormat::AccountQuota,&source)).unwrap();
    assert_eq!(snapshot.original_bytes,source);assert_eq!(snapshot.tokens.input,None);
    assert_eq!(snapshot.account.as_ref().unwrap().project_id.as_deref(),Some("project"));
    assert_eq!(serde_json::to_value(snapshot.quota_windows[0].remaining.as_ref().unwrap()).unwrap()["value"]["magnitude"]["nanodollar_numerator"],"0");
    assert_eq!(snapshot.quota_windows[0].used,None);
    assert_eq!(snapshot.quota_windows[2].used,None);
    let tokens=snapshot.quota_windows.iter().find(|w|w.name=="tokens").unwrap();
    assert_eq!(tokens.limit,Some(QuotaAmount::Count(9007199254740993)));
    assert_eq!(tokens.interval_ms,Some(1000));assert_eq!(tokens.refill_amount,Some(QuotaAmount::Count(10)));
    assert_eq!(tokens.raw["future_window"],true);
    let balance=snapshot.balance.as_ref().unwrap();assert!(balance.negative);assert_eq!((balance.magnitude.nanodollar_numerator,balance.magnitude.denominator),(1,10));
    assert_eq!(snapshot.funding.as_ref().unwrap().provenance.as_ref().unwrap()[0]["reference"],"receipt");
    assert_eq!(snapshot.error.as_ref().unwrap().code.as_ref().unwrap(),429);
    assert_eq!(snapshot.raw["future_root"],true);
    assert_eq!(snapshot.freshness(1791288000123456789,100),Freshness::Fresh);
    assert_eq!(snapshot.freshness(1791288000123456889,100),Freshness::Stale);
    assert_eq!(snapshot.freshness(1791288000123456788,100),Freshness::FutureObservation);
}
#[test]
fn provider_categories_and_exact_catalog_charge_do_not_invent_missing_usage() {
    let bytes=br#"{"usage":{"input_tokens":8,"output_tokens":4,"cache_read_input_tokens":2,"cache_creation_input_tokens":0},"future_usage":"retained"}"#;
    let snapshot=ProviderUsageSnapshot::parse(bytes,metadata(ObservationFormat::Anthropic,bytes)).unwrap();
    assert_eq!(snapshot.tokens.reasoning,None);assert_eq!(snapshot.tokens.cache_write,Some(0));
    let model=ModelCatalog::parse(br#"{"data":[{"id":"model","pricing":{"prompt":"0.0000000001","completion":"0.0000000003","input_cache_read":"0.00000000005","input_cache_write":"0"}}]}"#).unwrap().models.remove(0);
    let charge=account_catalog_usage(&model,&snapshot.tokens,None).unwrap();
    let exact=charge.nanodollars.unwrap();assert_eq!((exact.nanodollar_numerator,exact.denominator),(21,10));
    let bytes=br#"{"prompt_eval_count":0,"eval_count":3,"done":true}"#;
    let observed=ProviderUsageSnapshot::parse(bytes,metadata(ObservationFormat::Ollama,bytes)).unwrap();
    assert_eq!(observed.tokens.input,Some(0));assert_eq!(observed.tokens.output,Some(3));assert_eq!(observed.tokens.cache_read,None);
    let unknown=account_catalog_usage(&model,&observed.tokens,None).unwrap();assert_eq!(unknown.nanodollars,None);assert!(!unknown.unknown.is_empty());
    let bytes=br#"{"usage":{"prompt_tokens":10,"completion_tokens":4,"prompt_tokens_details":{"cached_tokens":2,"cache_creation_tokens":0},"completion_tokens_details":{"reasoning_tokens":1}}}"#;
    let inclusive=ProviderUsageSnapshot::parse(bytes,metadata(ObservationFormat::OpenAi,bytes)).unwrap();
    assert_eq!(account_catalog_usage(&model,&inclusive.tokens,None).unwrap().nanodollars,Some(exact));
    assert_eq!(inclusive.tokens.reasoning,Some(1));
    let reasoning_model=ModelCatalog::parse(br#"{"data":[{"id":"reasoning","pricing":{"prompt":"0.0000000001","completion":"0.0000000003","input_cache_read":"0.00000000005","input_cache_write":"0","internal_reasoning":"0.0000000002"}}]}"#).unwrap().models.remove(0);
    let reasoning_charge=account_catalog_usage(&reasoning_model,&inclusive.tokens,None).unwrap().nanodollars.unwrap();
    assert_eq!((reasoning_charge.nanodollar_numerator,reasoning_charge.denominator),(2,1));
    let request_model=ModelCatalog::parse(br#"{"data":[{"id":"request-priced","pricing":{"prompt":"0","completion":"0","input_cache_read":"0","input_cache_write":"0","request":"0.001"}}]}"#).unwrap().models.remove(0);
    let unmeasured=account_catalog_usage(&request_model,&inclusive.tokens,None).unwrap();
    assert_eq!(unmeasured.nanodollars,None);assert!(unmeasured.unknown.contains(&"request_units".into()));
    let bytes=br#"{"usage":{"prompt_tokens":"9007199254740993"}}"#;
    let large=ProviderUsageSnapshot::parse(bytes,metadata(ObservationFormat::OpenAi,bytes)).unwrap();
    assert_eq!(serde_json::to_value(&large).unwrap()["tokens"]["input"],"9007199254740993");
}
#[test]
fn tier_context_and_in_band_error_counters_remain_unknown() {
    let bytes=br#"{"error":{"code":"rate_limited","message":"retry later"}}"#;
    let snapshot=ProviderUsageSnapshot::parse(bytes,metadata(ObservationFormat::OpenAi,bytes)).unwrap();
    assert_eq!(snapshot.tokens.input,None);assert_eq!(snapshot.reported_charge,None);assert!(snapshot.error.is_some());
    let model=ModelCatalog::parse(br#"{"data":[{"id":"tiered","pricing":{"tiers":[{"max_context_tokens":100,"pricing":{"prompt":"0","completion":"0"}},{"max_context_tokens":null,"pricing":{"prompt":"0.000001","completion":"0.000002"}}]}}]}"#).unwrap().models.remove(0);
    assert_eq!(account_catalog_usage(&model,&snapshot.tokens,None).unwrap().unknown,vec!["context_tokens_for_price_tier"]);
    let tokens=TokenCategories {input:Some(0),output:Some(0),cache_read:Some(0),cache_write:Some(0),reasoning:None,input_semantics:InputSemantics::IncludesCache};
    let free=account_catalog_usage(&model,&tokens,Some(50)).unwrap();assert_eq!(free.tier_index,Some(0));assert_eq!(free.nanodollars.unwrap().nanodollar_numerator,0);
}
#[test]
fn invalid_observed_shapes_and_counters_refuse() {
    for bytes in [br#"{"usage":{"prompt_tokens":-1}}"#.as_slice(),br#"{"usage":"invalid"}"#.as_slice(),br#"{"usage":{"prompt_tokens":1,"prompt_tokens_details":{"cached_tokens":2,"cache_creation_tokens":0}}}"#.as_slice()] {
        assert!(ProviderUsageSnapshot::parse(bytes,metadata(ObservationFormat::OpenAi,bytes)).is_err());
    }
    let bytes=br#"{"data":{"rate_limit":{"requests":10,"interval":"never"}}}"#;
    assert!(ProviderUsageSnapshot::parse(bytes,metadata(ObservationFormat::AccountQuota,bytes)).is_err());
}
