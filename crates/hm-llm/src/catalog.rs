use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExactPrice {
    #[serde(with = "exact_integer")]
    pub nanodollar_numerator: u64,
    #[serde(with = "exact_integer")]
    pub denominator: u64,
    pub unit: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogError { Invalid(String), Capacity, Json(String) }
impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { Self::Invalid(message) | Self::Json(message) => write!(f,"{message}"), Self::Capacity => write!(f,"catalog capacity exceeded") } }
}
impl std::error::Error for CatalogError {}
fn invalid(field: &str) -> CatalogError { CatalogError::Invalid(format!("invalid catalog {field}")) }
impl ExactPrice {
    pub fn parse_usd(value: &str, unit: &str) -> Result<Self, CatalogError> {
        if value.is_empty() || value.len() > 96 || value.starts_with('-') || value.starts_with('+') { return Err(invalid("price")); }
        let (mantissa, exponent) = match value.split_once(['e','E']) {
            Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().map_err(|_|invalid("price exponent"))?),
            None => (value, 0),
        };
        if !(-18..=18).contains(&exponent) { return Err(CatalogError::Capacity); }
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        if whole.is_empty() || !whole.bytes().all(|b|b.is_ascii_digit()) || !fraction.bytes().all(|b|b.is_ascii_digit()) || mantissa.ends_with('.') { return Err(invalid("price")); }
        if fraction.len() > 18 { return Err(CatalogError::Capacity); }
        let digits = format!("{whole}{fraction}");
        let mut numerator = digits.parse::<u128>().map_err(|_|CatalogError::Capacity)?;
        let scale = i32::try_from(fraction.len()).map_err(|_|CatalogError::Capacity)? - exponent - 9;
        let mut denominator = 1_u128;
        if scale > 0 { denominator = 10_u128.checked_pow(scale as u32).ok_or(CatalogError::Capacity)?; }
        else { numerator = numerator.checked_mul(10_u128.checked_pow((-scale) as u32).ok_or(CatalogError::Capacity)?).ok_or(CatalogError::Capacity)?; }
        fn gcd(mut a:u128,mut b:u128)->u128 { while b != 0 { let remainder=a%b;a=b;b=remainder; } a }
        let divisor = gcd(numerator,denominator); numerator /= divisor; denominator /= divisor;
        Ok(Self { nanodollar_numerator: u64::try_from(numerator).map_err(|_|CatalogError::Capacity)?, denominator: u64::try_from(denominator).map_err(|_|CatalogError::Capacity)?, unit: unit.into() })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PriceTier {
    pub max_context_tokens: Option<u64>,
    pub prices: BTreeMap<String, Option<ExactPrice>>,
    pub raw: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogModel {
    pub id: String,
    pub name: Option<String>,
    pub context_tokens: Option<u64>,
    pub provider_context_tokens: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub input_modalities: Option<Vec<String>>,
    pub output_modalities: Option<Vec<String>>,
    pub capabilities: Option<Vec<String>>,
    pub prices: BTreeMap<String, Option<ExactPrice>>,
    pub tiers: Vec<PriceTier>,
    pub modality_limits: Option<Value>,
    pub raw: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    pub models: Vec<CatalogModel>,
    pub raw: Value,
    pub original_bytes: Vec<u8>,
    pub fingerprint: String,
}
pub struct ModelCatalog;
fn unsigned(value:&Value, field:&str)->Result<Option<u64>,CatalogError> {
    match value.get(field) { None|Some(Value::Null)=>Ok(None),Some(v)=>v.as_u64().map(Some).ok_or_else(||invalid(field)) }
}
fn strings(value:&Value,field:&str)->Result<Option<Vec<String>>,CatalogError> {
    match value.get(field) {
        None|Some(Value::Null)=>Ok(None),Some(Value::Array(values))=>values.iter().map(|v|v.as_str().map(String::from).ok_or_else(||invalid(field))).collect::<Result<Vec<_>,_>>().map(Some),Some(_)=>Err(invalid(field)),
    }
}
fn prices(value:&Value)->Result<BTreeMap<String,Option<ExactPrice>>,CatalogError> {
    const FIELDS:&[(&str,&str)] = &[("prompt","token"),("completion","token"),("input_cache_read","token"),("input_cache_write","token"),("internal_reasoning","token"),("image","image"),("request","request"),("web_search","search"),("audio","audio_unit")];
    if !value.is_null() && !value.is_object() { return Err(invalid("pricing")); }
    let mut prices=BTreeMap::new();
    for (field,unit) in FIELDS {
        let price=match value.get(*field) {
            None|Some(Value::Null)=>None,
            Some(Value::String(s))=>Some(ExactPrice::parse_usd(s,unit)?),
            Some(Value::Number(n))=>Some(ExactPrice::parse_usd(&n.to_string(),unit)?),
            _=>return Err(invalid(field)),
        };
        prices.insert((*field).into(),price);
    }
    Ok(prices)
}
impl ModelCatalog {
    pub fn parse(bytes:&[u8])->Result<CatalogSnapshot,CatalogError> {
        if bytes.len()>64*1024*1024 { return Err(CatalogError::Capacity); }
        let raw:Value=serde_json::from_slice(bytes).map_err(|e|CatalogError::Json(e.to_string()))?;
        let rows=raw.get("data").and_then(Value::as_array).ok_or_else(||invalid("data"))?;
        if rows.len()>100_000 { return Err(CatalogError::Capacity); }
        let mut identities=BTreeSet::new();
        let mut models=Vec::with_capacity(rows.len());
        for row in rows {
            let id=row.get("id").and_then(Value::as_str).ok_or_else(||invalid("model id"))?;
            if id.is_empty()||id.len()>512||!id.bytes().all(|b|b.is_ascii_graphic())||!identities.insert(id.to_owned()) { return Err(invalid("model identity")); }
            let name=match row.get("name") {None|Some(Value::Null)=>None,Some(Value::String(name))=>Some(name.clone()),_=>return Err(invalid("name"))};
            for field in ["architecture","top_provider"] { if row.get(field).is_some_and(|v|!v.is_object()&&!v.is_null()) { return Err(invalid(field)); } }
            let architecture=&row["architecture"];
            let top=&row["top_provider"];
            let pricing=&row["pricing"];
            let mut tiers=Vec::new();
            if let Some(value)=pricing.get("tiers") {
                for tier in value.as_array().ok_or_else(||invalid("price tiers"))? {
                    if !tier.is_object() { return Err(invalid("price tier")); }
                    tiers.push(PriceTier { max_context_tokens: unsigned(tier,"max_context_tokens")?, prices:prices(tier.get("pricing").unwrap_or(tier))?, raw:tier.clone() });
                }
                let mut previous=None;
                for (index,tier) in tiers.iter().enumerate() {
                    match tier.max_context_tokens {
                        Some(limit) if limit>0 && previous.is_none_or(|p|limit>p)=>previous=Some(limit),
                        None if index+1==tiers.len()=>{},
                        _=>return Err(invalid("price tier bounds")),
                    }
                }
            }
            models.push(CatalogModel { id:id.into(), name, context_tokens:unsigned(row,"context_length")?, provider_context_tokens:unsigned(top,"context_length")?, max_output_tokens:unsigned(top,"max_completion_tokens")?, input_modalities:strings(architecture,"input_modalities")?, output_modalities:strings(architecture,"output_modalities")?, capabilities:strings(row,"supported_parameters")?, prices:prices(pricing)?, tiers, modality_limits:row.get("modality_limits").cloned(), raw:row.clone() });
        }
        Ok(CatalogSnapshot { models, raw, original_bytes:bytes.to_vec(), fingerprint:blake3::hash(bytes).to_hex().to_string() })
    }
}

mod exact_integer {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}
