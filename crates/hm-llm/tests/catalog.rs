use hm_llm::catalog::{CatalogError, ExactPrice, ModelCatalog};

#[test]
fn actual_external_format_file_preserves_additions_and_exact_tiers() {
    let bytes = br#"{
      "data":[{
        "id":"provider/model","name":"Model","context_length":0,
        "architecture":{"input_modalities":["text","image"],"output_modalities":["text"],"tokenizer":"external","future_architecture":{"channels":3}},
        "top_provider":{"context_length":200000,"max_completion_tokens":32768},
        "supported_parameters":["tools","reasoning"],"modality_limits":{"image":{"max_pixels":16000000}},
        "pricing":{"prompt":"0.0000000001","completion":0,"input_cache_read":0.0000000002,"new_price_metadata":{"currency":"USD"},"tiers":[
          {"max_context_tokens":128000,"pricing":{"prompt":"0.000003","completion":"0.000015"}},
          {"max_context_tokens":null,"pricing":{"prompt":"0.000006","completion":"0.0000225"},"future_tier":true}
        ]},"future_model":{"routing":"opaque"}
      }],"future_root":{"release":"catalog-v2"}}
    "#;
    let path = std::env::temp_dir().join(format!("hypermind-catalog-{}.json",std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    let source = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let catalog = ModelCatalog::parse(&source).unwrap();
    assert_eq!(catalog.original_bytes, source);
    assert_eq!(catalog.raw["future_root"]["release"], "catalog-v2");
    let model = &catalog.models[0];
    assert_eq!(model.context_tokens, Some(0));
    assert_eq!(model.max_output_tokens, Some(32768));
    assert_eq!(model.input_modalities.as_ref().unwrap(), &["text","image"]);
    assert_eq!(model.capabilities.as_ref().unwrap(), &["tools","reasoning"]);
    let prompt = model.prices["prompt"].as_ref().unwrap();
    assert_eq!((prompt.nanodollar_numerator,prompt.denominator),(1,10));
    let cache = model.prices["input_cache_read"].as_ref().unwrap();
    assert_eq!((cache.nanodollar_numerator,cache.denominator),(1,5));
    assert_eq!(model.prices["completion"].as_ref().unwrap().nanodollar_numerator,0);
    assert_eq!(model.prices["request"],None);
    assert_eq!(model.tiers.len(),2);
    assert_eq!(model.tiers[1].prices["completion"].as_ref().unwrap().nanodollar_numerator,22500);
    assert_eq!(model.tiers[1].raw["future_tier"],true);
    assert_eq!(model.raw["future_model"]["routing"],"opaque");
    assert_eq!(model.modality_limits.as_ref().unwrap()["image"]["max_pixels"],16000000);
}
#[test]
fn missing_fields_are_not_zero_and_invalid_prices_refuse() {
    let model = ModelCatalog::parse(br#"{"data":[{"id":"model"}]}"#).unwrap().models.remove(0);
    assert_eq!(model.context_tokens,None);assert_eq!(model.capabilities,None);assert_eq!(model.prices["prompt"],None);
    for value in ["-0","-0.01","NaN","Infinity","1.2.3","", "+1"] {
        assert!(ExactPrice::parse_usd(value,"token").is_err(),"{value}");
    }
    for source in [br#"{"data":[{"id":"m","pricing":{"prompt":false}}]}"#.as_slice(), br#"{"data":[{"id":"m","context_length":-1}]}"#.as_slice(), br#"{"data":[{"id":"m"},{"id":"m"}]}"#.as_slice()] {
        assert!(ModelCatalog::parse(source).is_err());
    }
    assert!(matches!(ExactPrice::parse_usd("9999999999999999999999999999","token"),Err(CatalogError::Capacity)));
    let exact = ExactPrice::parse_usd("0.123456789123456789","token").unwrap();
    let wire = serde_json::to_value(&exact).unwrap();
    assert_eq!(wire["nanodollar_numerator"],"123456789123456789");
    assert_eq!(serde_json::from_value::<ExactPrice>(wire).unwrap(),exact);
    assert_eq!(ExactPrice::parse_usd("1e-10","token").unwrap(),ExactPrice::parse_usd("0.0000000001","token").unwrap());
}
#[test]
fn tier_order_is_admitted_without_silent_sorting() {
    assert!(ModelCatalog::parse(br#"{"data":[{"id":"m","pricing":{"tiers":[{"max_context_tokens":200},{"max_context_tokens":100}]}}]}"#).is_err());
    assert!(ModelCatalog::parse(br#"{"data":[{"id":"m","pricing":{"tiers":[{"max_context_tokens":null},{"max_context_tokens":100}]}}]}"#).is_err());
}
