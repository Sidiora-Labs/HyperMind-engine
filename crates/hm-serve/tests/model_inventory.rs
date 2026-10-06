use hm_serve::model_inventory::{ModelInventory,ModelReadiness,ReadinessObservation,describe_model_inventory};
#[test]
fn actual_catalog_file_inventory_generation_and_observation_fences() {
    let directory=tempfile::tempdir().unwrap();
    let path=directory.path().join("models.json");
    std::fs::write(&path,br#"{"data":[{"id":"local/model","context_length":4096,"pricing":{"prompt":"0"}}],"future_catalog":{"format":"external"}}"#).unwrap();
    let bytes=std::fs::read(path).unwrap();
    let mut inventory=ModelInventory::default();
    let empty=describe_model_inventory(&inventory);
    assert_eq!(empty["catalog_available"],false);
    let generation=inventory.install(&bytes,1791288000123456789,0).unwrap();
    let description=describe_model_inventory(&inventory);
    assert_eq!(description["generation"],1);
    assert_eq!(description["catalog_metadata"]["future_catalog"]["format"],"external");
    assert_eq!(description["catalog_observed_at_ns"],"1791288000123456789");
    assert_eq!(description["models"][0]["readiness_state"],"unknown");
    assert_eq!(description["models"][0]["catalog"]["context_tokens"],4096);
    let observation=ReadinessObservation {state:ModelReadiness::Unavailable,observed_at_ns:1791288000123456790,reason:"no_provider_configured".into()};
    inventory.observe("local/model",generation,observation.clone()).unwrap();
    assert_eq!(describe_model_inventory(&inventory)["models"][0]["readiness_state"],"unavailable");
    assert!(inventory.observe("unknown",generation,observation.clone()).is_err());
    assert!(inventory.install(&bytes,1791288000123456791,0).is_err());
    assert_eq!(inventory.install(&bytes,1791288000123456791,generation).unwrap(),2);
    assert_eq!(describe_model_inventory(&inventory)["models"][0]["readiness_state"],"unknown");
    assert!(inventory.observe("local/model",generation,observation).is_err());
    assert!(inventory.install(b"invalid",1791288000123456792,2).is_err());
    assert_eq!(describe_model_inventory(&inventory)["generation"],2);
}
