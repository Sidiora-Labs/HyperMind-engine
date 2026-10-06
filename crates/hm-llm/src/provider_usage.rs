use crate::catalog::{CatalogError, CatalogModel, ExactPrice};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum ObservationFormat { OpenAi, Anthropic, Ollama, AccountQuota }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationMetadata {
    pub provider_id: String,
    pub source: String,
    pub evidence_id: String,
    #[serde(with="nanos")]
    pub observed_at_ns: i64,
    #[serde(with="optional_nanos")]
    pub expires_at_ns: Option<i64>,
    pub format: ObservationFormat,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum InputSemantics { IncludesCache, ExcludesCache, Unknown }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenCategories {
    #[serde(with="optional_count")]
    pub input: Option<u64>,
    #[serde(with="optional_count")]
    pub output: Option<u64>,
    #[serde(with="optional_count")]
    pub cache_read: Option<u64>,
    #[serde(with="optional_count")]
    pub cache_write: Option<u64>,
    #[serde(with="optional_count")]
    pub reasoning: Option<u64>,
    pub input_semantics: InputSemantics,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountScope { pub account_id: Option<String>, pub organization_id: Option<String>, pub project_id: Option<String>, pub label: Option<String> }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MonetaryAmount { pub negative: bool, pub magnitude: ExactPrice }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FundingObservation { pub credits: Option<MonetaryAmount>, pub spent: Option<MonetaryAmount>, pub provenance: Option<Value>, pub raw: Value }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag="kind",content="value",rename_all="snake_case")]
pub enum QuotaAmount { Count(#[serde(with="exact_count")] u64), Usd(MonetaryAmount) }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub name: String,
    pub unit: String,
    pub limit: Option<QuotaAmount>,
    pub remaining: Option<QuotaAmount>,
    pub used: Option<QuotaAmount>,
    pub starts_at_ns: Option<String>,
    pub resets_at_ns: Option<String>,
    pub refill_amount: Option<QuotaAmount>,
    #[serde(with="optional_count")]
    pub interval_ms: Option<u64>,
    pub raw: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderObservationError { pub code: Option<Value>, pub message: Option<String>, pub raw: Value }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderUsageSnapshot {
    pub version: u32,
    pub observation: ObservationMetadata,
    pub tokens: TokenCategories,
    pub account: Option<AccountScope>,
    pub quota_windows: Vec<QuotaWindow>,
    pub balance: Option<MonetaryAmount>,
    pub funding: Option<FundingObservation>,
    pub reported_charge: Option<MonetaryAmount>,
    pub error: Option<ProviderObservationError>,
    pub raw: Value,
    pub original_bytes: Vec<u8>,
    pub fingerprint: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum Freshness { Fresh, Stale, FutureObservation }
fn invalid(field:&str)->CatalogError { CatalogError::Invalid(format!("invalid provider observation {field}")) }
fn count(value:&Value,field:&str)->Result<Option<u64>,CatalogError> {
    match value.get(field) {None|Some(Value::Null)=>Ok(None),Some(Value::String(s))=>s.parse::<u64>().map(Some).map_err(|_|invalid(field)),Some(v)=>v.as_u64().map(Some).ok_or_else(||invalid(field))}
}
fn text(value:&Value,field:&str)->Result<Option<String>,CatalogError> {
    match value.get(field) {None|Some(Value::Null)=>Ok(None),Some(Value::String(s))=>Ok(Some(s.clone())),_=>Err(invalid(field))}
}
fn money(value:&Value,field:&str,unit:&str)->Result<Option<MonetaryAmount>,CatalogError> {
    let decimal=match value.get(field) {None|Some(Value::Null)=>return Ok(None),Some(Value::String(s))=>s.clone(),Some(Value::Number(n))=>n.to_string(),_=>return Err(invalid(field))};
    let negative=decimal.starts_with('-');
    let magnitude=ExactPrice::parse_usd(decimal.strip_prefix('-').unwrap_or(&decimal),unit)?;
    Ok(Some(MonetaryAmount {negative:negative&&magnitude.nanodollar_numerator!=0,magnitude}))
}
fn timestamp(value:&Value,field:&str)->Result<Option<String>,CatalogError> {
    match value.get(field) {None|Some(Value::Null)=>Ok(None),Some(Value::String(s))=>s.parse::<i64>().map(|n|Some(n.to_string())).map_err(|_|invalid(field)),Some(v)=>v.as_i64().map(|n|Some(n.to_string())).ok_or_else(||invalid(field))}
}
fn quota_amount(value:&Value,field:&str,unit:&str)->Result<Option<QuotaAmount>,CatalogError> {
    if unit=="usd" {return money(value,field,"usd").map(|amount|amount.map(QuotaAmount::Usd));}
    count(value,field).map(|amount|amount.map(QuotaAmount::Count))
}
fn validate_tokens(tokens:&TokenCategories)->Result<(),CatalogError> {
    if tokens.input_semantics==InputSemantics::IncludesCache {
        if matches!((tokens.input,tokens.cache_read),(Some(input),Some(read)) if read>input) || matches!((tokens.input,tokens.cache_write),(Some(input),Some(write)) if write>input) {return Err(invalid("cache counts exceed input"));}
        if let (Some(input),Some(read),Some(write))=(tokens.input,tokens.cache_read,tokens.cache_write) {
            if read.checked_add(write).ok_or(CatalogError::Capacity)?>input {return Err(invalid("cache counts exceed input"));}
        }
    }
    if matches!((tokens.reasoning,tokens.output),(Some(reasoning),Some(output)) if reasoning>output) {return Err(invalid("reasoning exceeds output"));}
    Ok(())
}
impl ProviderUsageSnapshot {
    pub fn parse(bytes:&[u8],observation:ObservationMetadata)->Result<Self,CatalogError> {
        if bytes.len()>8*1024*1024 {return Err(CatalogError::Capacity);}
        for value in [&observation.provider_id,&observation.source,&observation.evidence_id] {if value.trim().is_empty()||value.len()>4096 {return Err(invalid("observation identity"));}}
        if observation.expires_at_ns.is_some_and(|expiry|expiry<observation.observed_at_ns) {return Err(invalid("observation expiry"));}
        let raw:Value=serde_json::from_slice(bytes).map_err(|e|CatalogError::Json(e.to_string()))?;
        if !raw.is_object() {return Err(invalid("envelope"));}
        for field in ["usage","error"] {
            if field=="usage" && raw.get(field).is_some_and(|v|!v.is_null()&&!v.is_object()) {return Err(invalid("usage"));}
        }
        let usage=&raw["usage"];
        for field in ["prompt_tokens_details","completion_tokens_details"] {if usage.get(field).is_some_and(|v|!v.is_null()&&!v.is_object()) {return Err(invalid(field));}}
        let (input,output,cache_read,cache_write,reasoning,input_semantics)=match observation.format {
            ObservationFormat::OpenAi=>(count(usage,"prompt_tokens")?,count(usage,"completion_tokens")?,count(&usage["prompt_tokens_details"],"cached_tokens")?,count(&usage["prompt_tokens_details"],"cache_creation_tokens")?,count(&usage["completion_tokens_details"],"reasoning_tokens")?,InputSemantics::IncludesCache),
            ObservationFormat::Anthropic=>(count(usage,"input_tokens")?,count(usage,"output_tokens")?,count(usage,"cache_read_input_tokens")?,count(usage,"cache_creation_input_tokens")?,count(usage,"reasoning_tokens")?,InputSemantics::ExcludesCache),
            ObservationFormat::Ollama=>(count(&raw,"prompt_eval_count")?,count(&raw,"eval_count")?,None,None,None,InputSemantics::Unknown),
            ObservationFormat::AccountQuota=>(None,None,None,None,None,InputSemantics::Unknown),
        };
        let tokens=TokenCategories {input,output,cache_read,cache_write,reasoning,input_semantics};validate_tokens(&tokens)?;
        let data=if observation.format==ObservationFormat::AccountQuota {raw.get("data").unwrap_or(&raw)} else {&raw};
        if !data.is_object()&&!data.is_null() {return Err(invalid("account data"));}
        let account_fields=["account_id","organization_id","project_id","label"];
        let account=if account_fields.iter().any(|field|data.get(*field).is_some()) {Some(AccountScope {account_id:text(data,"account_id")?,organization_id:text(data,"organization_id")?,project_id:text(data,"project_id")?,label:text(data,"label")?})} else {None};
        let mut quota_windows=Vec::new();
        if observation.format==ObservationFormat::AccountQuota {
            if data.get("currency").is_some_and(|currency|currency.as_str()!=Some("USD")) {return Err(invalid("unsupported account currency"));}
            if ["limit","limit_remaining","usage"].iter().any(|field|data.get(*field).is_some()) {
                quota_windows.push(QuotaWindow {name:"account".into(),unit:"usd".into(),limit:quota_amount(data,"limit","usd")?,remaining:quota_amount(data,"limit_remaining","usd")?,used:quota_amount(data,"usage","usd")?,starts_at_ns:None,resets_at_ns:None,refill_amount:None,interval_ms:None,raw:data.clone()});
            }
            for (field,name) in [("usage_daily","daily"),("usage_weekly","weekly"),("usage_monthly","monthly")] {
                if data.get(field).is_some() {quota_windows.push(QuotaWindow {name:name.into(),unit:"usd".into(),limit:None,remaining:None,used:quota_amount(data,field,"usd")?,starts_at_ns:None,resets_at_ns:None,refill_amount:None,interval_ms:None,raw:data[field].clone()});}
            }
            if let Some(windows)=data.get("quota_windows") {
                for window in windows.as_array().ok_or_else(||invalid("quota windows"))? {
                    if !window.is_object() {return Err(invalid("quota window"));}
                    let name=text(window,"name")?.filter(|name|!name.is_empty()).ok_or_else(||invalid("window name"))?;
                    let unit=text(window,"unit")?.ok_or_else(||invalid("window unit"))?;
                    if !["usd","requests","tokens"].contains(&unit.as_str()) {return Err(invalid("window unit"));}
                    quota_windows.push(QuotaWindow {name,unit:unit.clone(),limit:quota_amount(window,"limit",&unit)?,remaining:quota_amount(window,"remaining",&unit)?,used:quota_amount(window,"used",&unit)?,starts_at_ns:timestamp(window,"starts_at_ns")?,resets_at_ns:timestamp(window,"resets_at_ns")?,refill_amount:quota_amount(&window["refill"],"amount",&unit)?,interval_ms:count(&window["refill"],"interval_ms")?,raw:window.clone()});
                }
            }
            if let Some(rate)=data.get("rate_limit").filter(|v|!v.is_null()) {
                if !rate.is_object() {return Err(invalid("rate limit"));}
                let interval=match text(rate,"interval")? {Some(s)=>parse_interval(&s)?,None=>count(rate,"interval_ms")?};
                quota_windows.push(QuotaWindow {name:"rate_limit".into(),unit:"requests".into(),limit:quota_amount(rate,"requests","requests")?,remaining:None,used:None,starts_at_ns:None,resets_at_ns:None,refill_amount:None,interval_ms:interval,raw:rate.clone()});
            }
        }
        let funding_value=data.get("funding").or_else(||if data.get("total_credits").is_some()||data.get("total_usage").is_some(){Some(data)}else{None});
        let funding=match funding_value {None|Some(Value::Null)=>None,Some(value) if value.is_object()=>Some(FundingObservation {credits:money(value,"total_credits","usd")?,spent:money(value,"total_usage","usd")?,provenance:value.get("sources").cloned(),raw:value.clone()}),_=>return Err(invalid("funding"))};
        let error=match raw.get("error") {None|Some(Value::Null)=>None,Some(Value::String(message))=>Some(ProviderObservationError {code:None,message:Some(message.clone()),raw:Value::String(message.clone())}),Some(value) if value.is_object()=>Some(ProviderObservationError {code:value.get("code").cloned(),message:text(value,"message")?,raw:value.clone()}),_=>return Err(invalid("error"))};
        Ok(Self {version:1,observation,tokens,account,quota_windows,balance:money(data,"balance","usd")?,funding,reported_charge:money(usage,"cost","usd")?,error,raw,original_bytes:bytes.to_vec(),fingerprint:blake3::hash(bytes).to_hex().to_string()})
    }
    pub fn freshness(&self,now_ns:i64,max_age_ns:u64)->Freshness {
        if now_ns<self.observation.observed_at_ns {return Freshness::FutureObservation;}
        let age=i128::from(now_ns)-i128::from(self.observation.observed_at_ns);
        if age>i128::from(max_age_ns)||self.observation.expires_at_ns.is_some_and(|expiry|now_ns>=expiry) {Freshness::Stale} else {Freshness::Fresh}
    }
}
fn parse_interval(value:&str)->Result<Option<u64>,CatalogError> {
    for (suffix,multiplier) in [("ms",1_u64),("s",1000),("m",60_000),("h",3_600_000)] {
        if let Some(count)=value.strip_suffix(suffix) {return count.parse::<u64>().map_err(|_|invalid("rate interval"))?.checked_mul(multiplier).map(Some).ok_or(CatalogError::Capacity);}
    }
    Err(invalid("rate interval"))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CatalogCharge {
    pub basis: String,
    pub model_id: String,
    pub tier_index: Option<usize>,
    pub nanodollars: Option<ExactPrice>,
    pub unknown: Vec<String>,
}
pub fn account_catalog_usage(model:&CatalogModel,tokens:&TokenCategories,context_tokens:Option<u64>)->Result<CatalogCharge,CatalogError> {
    validate_tokens(tokens)?;
    let mut charge=CatalogCharge {basis:"catalog_tariff_estimate".into(),model_id:model.id.clone(),tier_index:None,nanodollars:None,unknown:Vec::new()};
    let prices=if model.tiers.is_empty() {&model.prices} else if let Some(context)=context_tokens {
        let index=model.tiers.iter().position(|tier|tier.max_context_tokens.is_none_or(|limit|context<=limit)).ok_or_else(||invalid("price tier context"))?;
        charge.tier_index=Some(index);&model.tiers[index].prices
    } else {charge.unknown.push("context_tokens_for_price_tier".into());return Ok(charge);};
    let input=match (tokens.input,tokens.cache_read,tokens.cache_write,tokens.input_semantics) {
        (Some(input),Some(read),Some(write),InputSemantics::IncludesCache)=>Some(input.checked_sub(read).and_then(|n|n.checked_sub(write)).ok_or_else(||invalid("cache input accounting"))?),
        (Some(input),_,_,InputSemantics::ExcludesCache)=>Some(input),
        _=>None,
    };
    for field in ["request","image","web_search","audio"] {
        if prices.get(field).and_then(Option::as_ref).is_some_and(|p|p.nanodollar_numerator>0) {charge.unknown.push(format!("{field}_units"));}
    }
    let reasoning_priced=prices.get("internal_reasoning").and_then(Option::as_ref).is_some();
    let output=if reasoning_priced {match (tokens.output,tokens.reasoning) {(Some(output),Some(reasoning))=>Some(output.checked_sub(reasoning).ok_or_else(||invalid("reasoning accounting"))?),_=>None}} else {tokens.output};
    let mut categories=vec![("prompt",input),("completion",output),("input_cache_read",tokens.cache_read),("input_cache_write",tokens.cache_write)];
    if reasoning_priced {categories.push(("internal_reasoning",tokens.reasoning));}
    let mut numerator=0_u128;let mut denominator=1_u128;
    fn gcd(mut a:u128,mut b:u128)->u128 {while b!=0 {let rest=a%b;a=b;b=rest;}a}
    for (field,count) in categories {
        let Some(count)=count else {charge.unknown.push(format!("{field}_tokens"));continue;};
        if count==0 {continue;}
        let Some(price)=prices.get(field).and_then(Option::as_ref) else {charge.unknown.push(format!("{field}_price"));continue;};
        if price.denominator==0 || price.unit!="token" {return Err(invalid("price units"));}
        let next=u128::from(price.nanodollar_numerator).checked_mul(u128::from(count)).ok_or(CatalogError::Capacity)?;
        let next_denominator=u128::from(price.denominator);
        let common=gcd(denominator,next_denominator);
        let left=next_denominator/common;let right=denominator/common;
        numerator=numerator.checked_mul(left).and_then(|n|next.checked_mul(right).and_then(|r|n.checked_add(r))).ok_or(CatalogError::Capacity)?;
        denominator=denominator.checked_mul(left).ok_or(CatalogError::Capacity)?;
        let reduce=gcd(numerator,denominator);numerator/=reduce;denominator/=reduce;
    }
    if charge.unknown.is_empty() {charge.nanodollars=Some(ExactPrice {nanodollar_numerator:u64::try_from(numerator).map_err(|_|CatalogError::Capacity)?,denominator:u64::try_from(denominator).map_err(|_|CatalogError::Capacity)?,unit:"observed_tokens".into()});}
    Ok(charge)
}
mod nanos {
    use serde::{Deserialize,Deserializer,Serializer};
    pub fn serialize<S:Serializer>(value:&i64,serializer:S)->Result<S::Ok,S::Error>{serializer.serialize_str(&value.to_string())}
    pub fn deserialize<'de,D:Deserializer<'de>>(deserializer:D)->Result<i64,D::Error>{String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)}
}
mod optional_nanos {
    use serde::{Deserialize,Deserializer,Serializer};
    pub fn serialize<S:Serializer>(value:&Option<i64>,serializer:S)->Result<S::Ok,S::Error>{value.map(|n|n.to_string()).serialize(serializer)}
    pub fn deserialize<'de,D:Deserializer<'de>>(deserializer:D)->Result<Option<i64>,D::Error>{Option::<String>::deserialize(deserializer)?.map(|s|s.parse().map_err(serde::de::Error::custom)).transpose()}
    use serde::Serialize;
}

mod optional_count {
    use serde::{Deserialize,Deserializer,Serialize,Serializer};
    pub fn serialize<S:Serializer>(value:&Option<u64>,serializer:S)->Result<S::Ok,S::Error>{value.map(|n|n.to_string()).serialize(serializer)}
    pub fn deserialize<'de,D:Deserializer<'de>>(deserializer:D)->Result<Option<u64>,D::Error>{Option::<String>::deserialize(deserializer)?.map(|s|s.parse().map_err(serde::de::Error::custom)).transpose()}
}

mod exact_count {
    use serde::{Deserialize,Deserializer,Serializer};
    pub fn serialize<S:Serializer>(value:&u64,serializer:S)->Result<S::Ok,S::Error>{serializer.serialize_str(&value.to_string())}
    pub fn deserialize<'de,D:Deserializer<'de>>(deserializer:D)->Result<u64,D::Error>{String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)}
}
