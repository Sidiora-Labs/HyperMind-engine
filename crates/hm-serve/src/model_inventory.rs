use hm_llm::catalog::{CatalogError, CatalogSnapshot, ModelCatalog};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ModelReadiness { Unknown, Available, Unavailable }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadinessObservation {
    pub state: ModelReadiness,
    pub observed_at_ns: i64,
    pub reason: String,
}
#[derive(Clone, Debug, Default)]
pub struct ModelInventory {
    generation: u64,
    snapshot: Option<CatalogSnapshot>,
    catalog_observed_at_ns: Option<i64>,
    readiness: BTreeMap<String,ReadinessObservation>,
}
impl ModelInventory {
    pub fn install(&mut self,bytes:&[u8],observed_at_ns:i64,expected_generation:u64)->Result<u64,CatalogError> {
        if expected_generation!=self.generation { return Err(CatalogError::Invalid("stale catalog generation".into())); }
        let snapshot=ModelCatalog::parse(bytes)?;
        let generation=self.generation.checked_add(1).ok_or(CatalogError::Capacity)?;
        self.snapshot=Some(snapshot);self.catalog_observed_at_ns=Some(observed_at_ns);self.generation=generation;self.readiness.clear();
        Ok(generation)
    }
    pub fn observe(&mut self,model_id:&str,generation:u64,observation:ReadinessObservation)->Result<(),CatalogError> {
        if generation!=self.generation || !self.snapshot.as_ref().is_some_and(|s|s.models.iter().any(|m|m.id==model_id)) { return Err(CatalogError::Invalid("stale or unknown model observation".into())); }
        if observation.reason.trim().is_empty() || observation.reason.len()>4096 || self.readiness.get(model_id).is_some_and(|old|old.observed_at_ns>observation.observed_at_ns) {return Err(CatalogError::Invalid("invalid readiness observation".into()));}
        self.readiness.insert(model_id.into(),observation);Ok(())
    }
    pub fn describe(&self)->Value {
        let models=self.snapshot.as_ref().map(|snapshot|snapshot.models.iter().map(|model|json!({"catalog":model,"readiness":self.readiness.get(&model.id),"readiness_state":self.readiness.get(&model.id).map(|o|&o.state).unwrap_or(&ModelReadiness::Unknown)})).collect::<Vec<_>>()).unwrap_or_default();
        let metadata = self.snapshot.as_ref().and_then(|s|s.raw.as_object()).map(|object|object.iter().filter(|(key,_)|key.as_str()!="data").map(|(key,value)|(key.clone(),value.clone())).collect::<serde_json::Map<String,Value>>());
        json!({"version":1,"catalog_metadata":metadata,"generation":self.generation,"catalog_available":self.snapshot.is_some(),"catalog_observed_at_ns":self.catalog_observed_at_ns.map(|ns|ns.to_string()),"fingerprint":self.snapshot.as_ref().map(|s|&s.fingerprint),"models":models,"provider_usage":"not_observed"})
    }
}
pub fn describe_model_inventory(inventory:&ModelInventory)->Value { inventory.describe() }
