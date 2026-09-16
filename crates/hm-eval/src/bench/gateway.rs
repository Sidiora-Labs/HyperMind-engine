#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc
)]

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type DynError = Box<dyn std::error::Error + Send + Sync>;
pub const MAXIMUM_BUDGET_MICROUSD: u64 = 49_000_000;
pub const READER_MODEL: &str = "openrouter/openai/gpt-5.6-luna";
pub const READER_CANONICAL_MODEL: &str = "openai/gpt-5.6-luna";
pub const READER_CANONICAL_SNAPSHOT: &str = "";
pub const JUDGE_MODEL: &str = READER_MODEL;
pub const MODEL_IDENTITY_KIND: &str = "catalog_alias_not_immutable_snapshot";
pub const MODEL_CATALOG_SOURCE: &str = "https://gateway.centra.ag/v1/models";
pub const MODEL_CATALOG_OBSERVED_AT: &str = "2026-09-16";
pub const PRICE_TABLE_VERSION: &str = "openai-official-2026-09-16";
pub const PRICE_SOURCES: [&str; 1] = ["https://developers.openai.com/api/docs/models/gpt-5.6-luna"];
const MAXIMUM_REQUEST_BYTES: usize = 512_000;
const MAXIMUM_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub const READER_REQUEST_POLICY_ID: &str = "benchmark-reader-luna-medium4096@1";

#[derive(Clone, Copy, Debug, Serialize)]
pub struct CompletionSettings {
    pub max_output_tokens: u32,
    pub reasoning_effort: &'static str,
}

pub const READER_SETTINGS: CompletionSettings = CompletionSettings {
    max_output_tokens: 4_096,
    reasoning_effort: "medium",
};
pub const JUDGE_SETTINGS: CompletionSettings = CompletionSettings {
    max_output_tokens: 2_048,
    reasoning_effort: "medium",
};
pub const PROBE_SETTINGS: CompletionSettings = CompletionSettings {
    max_output_tokens: 10,
    reasoning_effort: "none",
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub reasoning_tokens: u64,
    pub cached_input_tokens: u64,
    pub cost_microusd: u64,
    pub cost_source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Completion {
    pub text: String,
    pub model: String,
    pub response_model: String,
    pub canonical_snapshot: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub canonical_model: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model_identity_kind: String,
    pub model_catalog_source: String,
    pub model_catalog_observed_at: String,
    pub request_hash: String,
    pub cached: bool,
    pub usage: Usage,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BudgetSummary {
    pub budget_microusd: u64,
    pub charged_microusd: u64,
    pub retained_reservations_microusd: u64,
    pub available_microusd: u64,
    pub dispatched_calls: usize,
    pub completed_calls: usize,
    pub unknown_calls: usize,
    #[serde(default)]
    pub upstream_usage_accounted_microusd: u64,
}

#[derive(Clone)]
pub struct Gateway {
    inner: Arc<GatewayState>,
}

struct GatewayState {
    root: PathBuf,
    endpoint: String,
    api_key: Option<String>,
    allow_live: bool,
    client: reqwest::Client,
    budget: Mutex<BudgetFile>,
    requests: Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>,
    _process_lock: File,
}

#[derive(Clone, Serialize, Deserialize)]
struct BudgetFile {
    version: u32,
    budget_microusd: u64,
    price_table: String,
    #[serde(default)]
    blocked_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    billing_basis: Option<String>,
    reservations: BTreeMap<String, Reservation>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Reservation {
    reserved_microusd: u64,
    charged_microusd: Option<u64>,
    completed: bool,
    usage: Option<Usage>,
}

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    version: u32,
    request_hash: String,
    endpoint: String,
    model: String,
    prompt_id: String,
    request: Value,
    http_status: u16,
    raw_response: String,
    completion: Option<Completion>,
}

#[derive(Serialize, Deserialize)]
struct CheckedCache {
    digest: String,
    entry: CacheEntry,
}

impl Gateway {
    pub fn open(
        cache_root: impl AsRef<Path>,
        allow_live: bool,
        budget_microusd: u64,
    ) -> Result<Self, DynError> {
        if budget_microusd == 0 || budget_microusd > MAXIMUM_BUDGET_MICROUSD {
            return Err("benchmark budget must be greater than zero and at most $49".into());
        }
        let root = cache_root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("responses"))?;
        let process_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("budget.lock"))?;
        process_lock
            .try_lock()
            .map_err(|_| "benchmark budget is in use by another process")?;
        let budget_path = root.join("budget.json");
        let budget = if budget_path.exists() {
            let prior: BudgetFile = serde_json::from_slice(&fs::read(&budget_path)?)?;
            if prior.version != 1
                || prior.budget_microusd != budget_microusd
                || prior.price_table != PRICE_TABLE_VERSION
            {
                return Err("existing benchmark budget configuration cannot be changed".into());
            }
            prior
        } else {
            let initial = BudgetFile {
                version: 1,
                budget_microusd,
                price_table: PRICE_TABLE_VERSION.into(),
                blocked_reason: None,
                billing_basis: None,
                reservations: BTreeMap::new(),
            };
            write_json(&budget_path, &initial)?;
            initial
        };
        let base = std::env::var("CENTRA_GATEWAY_URL")
            .unwrap_or_else(|_| "https://gateway.centra.ag/v1".into());
        let endpoint = centra_endpoint(&base)?;
        let api_key = if allow_live {
            Some(
                std::env::var("CENTRA_GATEWAY_API_KEY")
                    .map_err(|_| "CENTRA_GATEWAY_API_KEY is required for live Centra calls")?,
            )
        } else {
            None
        };
        if api_key.as_ref().is_some_and(|key| key.trim().is_empty()) {
            return Err("CENTRA_GATEWAY_API_KEY is empty".into());
        }
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(180))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            inner: Arc::new(GatewayState {
                root,
                endpoint,
                api_key,
                allow_live,
                client,
                budget: Mutex::new(budget),
                requests: Mutex::new(BTreeMap::new()),
                _process_lock: process_lock,
            }),
        })
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.inner.endpoint
    }

    pub fn budget(&self) -> Result<BudgetSummary, DynError> {
        let budget = self
            .inner
            .budget
            .lock()
            .map_err(|_| "benchmark budget lock poisoned")?;
        let mut summary = BudgetSummary {
            budget_microusd: budget.budget_microusd,
            dispatched_calls: budget.reservations.len(),
            ..BudgetSummary::default()
        };
        for reservation in budget.reservations.values() {
            if let Some(charged) = reservation.charged_microusd {
                summary.charged_microusd = summary.charged_microusd.saturating_add(charged);
                if reservation
                    .usage
                    .as_ref()
                    .is_some_and(|usage| usage.cost_source == UPSTREAM_BILLING_BASIS)
                {
                    summary.upstream_usage_accounted_microusd = summary
                        .upstream_usage_accounted_microusd
                        .saturating_add(charged);
                }
            } else {
                summary.retained_reservations_microusd = summary
                    .retained_reservations_microusd
                    .saturating_add(reservation.reserved_microusd);
            }
            summary.completed_calls += usize::from(reservation.completed);
            summary.unknown_calls += usize::from(!reservation.completed);
        }
        summary.available_microusd = summary.budget_microusd.saturating_sub(
            summary
                .charged_microusd
                .saturating_add(summary.retained_reservations_microusd),
        );
        Ok(summary)
    }

    pub fn confirm_upstream_rates(&self) -> Result<BudgetSummary, DynError> {
        let mut budget = self
            .inner
            .budget
            .lock()
            .map_err(|_| "benchmark budget lock poisoned")?;
        let mut reconciled = budget.clone();
        for (hash, reservation) in &mut reconciled.reservations {
            if !reservation.completed || reservation.charged_microusd.is_some() {
                continue;
            }
            let path = self
                .inner
                .root
                .join("responses")
                .join(format!("{hash}.json"));
            let unchecked: CheckedCache = serde_json::from_slice(&fs::read(&path)?)?;
            if unchecked.entry.model != READER_MODEL {
                continue;
            }
            let cache = read_cache(&path, hash, &unchecked.entry.request)?;
            if cache.completion.is_none() || !(200..300).contains(&cache.http_status) {
                continue;
            }
            let completion = parse_completion(
                READER_MODEL,
                hash,
                &cache.raw_response,
                reservation.reserved_microusd,
            )?;
            let usage = upstream_usage(&completion.usage)?;
            if usage.cost_microusd > reservation.reserved_microusd {
                return Err(
                    "confirmed upstream usage exceeds original reservation; reconciliation refused"
                        .into(),
                );
            }
            reservation.charged_microusd = Some(usage.cost_microusd);
            reservation.usage = Some(usage);
        }
        reconciled.billing_basis = Some(UPSTREAM_BILLING_BASIS.into());
        write_json(&self.inner.root.join("budget.json"), &reconciled)?;
        *budget = reconciled;
        drop(budget);
        self.budget()
    }

    pub async fn complete(
        &self,
        model: &str,
        prompt_id: &str,
        system: &str,
        user: &str,
        max_output_tokens: u32,
    ) -> Result<Completion, DynError> {
        self.complete_with_settings(
            model,
            prompt_id,
            system,
            user,
            CompletionSettings {
                max_output_tokens,
                reasoning_effort: "none",
            },
        )
        .await
    }

    #[allow(clippy::too_many_lines)]
    pub async fn complete_with_settings(
        &self,
        model: &str,
        prompt_id: &str,
        system: &str,
        user: &str,
        settings: CompletionSettings,
    ) -> Result<Completion, DynError> {
        let prices = prices(model)?;
        if prompt_id.is_empty()
            || user.is_empty()
            || !(1..=4_096).contains(&settings.max_output_tokens)
            || !matches!(
                settings.reasoning_effort,
                "none" | "low" | "medium" | "high" | "xhigh" | "max"
            )
        {
            return Err("invalid benchmark completion request".into());
        }
        let request = completion_request(model, system, user, settings);
        let bytes = serde_json::to_vec(&request)?;
        if bytes.len() > MAXIMUM_REQUEST_BYTES {
            return Err("benchmark request exceeds the fixed input limit".into());
        }
        let request_hash = request_hash(&self.inner.endpoint, prompt_id, &request)?;
        let lock = self
            .inner
            .requests
            .lock()
            .map_err(|_| "benchmark request lock poisoned")?
            .entry(request_hash.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone();
        let _request_guard = lock.lock().await;
        let path = self
            .inner
            .root
            .join("responses")
            .join(format!("{request_hash}.json"));
        if path.exists() {
            let cache = read_cache(&path, &request_hash, &request)?;
            let mut completion = cache
                .completion
                .ok_or("cached provider failure is not retried automatically")?;
            completion.usage = self.settle(&request_hash, &completion.usage, true)?;
            completion.cached = true;
            return Ok(completion);
        }
        if !self.inner.allow_live {
            return Err("offline benchmark cache miss; live calls are disabled".into());
        }
        let maximum_input_tokens = u64::try_from(bytes.len())?.saturating_add(1_024);
        let reserve = token_cost(
            maximum_input_tokens,
            u64::from(settings.max_output_tokens),
            prices,
        )?
        .checked_mul(2)
        .ok_or("benchmark reservation overflow")?
        .max(100);
        self.reserve(&request_hash, reserve)?;
        let mut response = self
            .inner
            .client
            .post(&self.inner.endpoint)
            .bearer_auth(
                self.inner
                    .api_key
                    .as_deref()
                    .ok_or("live gateway has no credential")?,
            )
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes)
            .send()
            .await
            .map_err(
                |_| "gateway transport failed; request reservation retained and no automatic retry",
            )?;
        let status = response.status().as_u16();
        let mut response_bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "gateway response interrupted; reservation retained")?
        {
            if response_bytes.len().saturating_add(chunk.len()) > MAXIMUM_RESPONSE_BYTES {
                return Err(
                    "gateway response exceeds bounded cache size; reservation retained".into(),
                );
            }
            response_bytes.extend_from_slice(&chunk);
        }
        let raw_response = String::from_utf8(response_bytes)
            .map_err(|_| "gateway response was not UTF-8; reservation retained")?;
        let raw_response = self.inner.api_key.as_ref().map_or_else(
            || raw_response.clone(),
            |key| raw_response.replace(key, "[REDACTED_API_KEY]"),
        );
        let parsed = if (200..300).contains(&status) {
            parse_completion(model, &request_hash, &raw_response, reserve)
        } else {
            Err("gateway returned unsuccessful HTTP status; reservation retained".into())
        };
        let cache = CacheEntry {
            version: 1,
            request_hash: request_hash.clone(),
            endpoint: self.inner.endpoint.clone(),
            model: model.into(),
            prompt_id: prompt_id.into(),
            request,
            http_status: status,
            raw_response,
            completion: parsed.as_ref().ok().cloned(),
        };
        write_cache(&path, cache)?;
        let mut completion = parsed?;
        completion.usage = self.settle(&request_hash, &completion.usage, true)?;
        Ok(completion)
    }

    fn reserve(&self, request_hash: &str, reserve: u64) -> Result<(), DynError> {
        let mut budget = self
            .inner
            .budget
            .lock()
            .map_err(|_| "benchmark budget lock poisoned")?;
        if budget.blocked_reason.is_some() {
            return Err(
                "benchmark spending is blocked pending gateway price reconciliation".into(),
            );
        }
        if budget.reservations.contains_key(request_hash) {
            return Err("request has a previous unresolved dispatch; explicit reconciliation required before any retry".into());
        }
        let spent = budget
            .reservations
            .values()
            .try_fold(0_u64, |sum, item| {
                sum.checked_add(item.charged_microusd.unwrap_or(item.reserved_microusd))
            })
            .ok_or("benchmark budget overflow")?;
        if spent
            .checked_add(reserve)
            .is_none_or(|projected| projected > budget.budget_microusd)
        {
            return Err("benchmark spending cap reached; no provider request dispatched".into());
        }
        budget.reservations.insert(
            request_hash.into(),
            Reservation {
                reserved_microusd: reserve,
                charged_microusd: None,
                completed: false,
                usage: None,
            },
        );
        write_json(&self.inner.root.join("budget.json"), &*budget)
    }

    fn settle(
        &self,
        request_hash: &str,
        usage: &Usage,
        completed: bool,
    ) -> Result<Usage, DynError> {
        let mut budget = self
            .inner
            .budget
            .lock()
            .map_err(|_| "benchmark budget lock poisoned")?;
        let usage = if budget.billing_basis.as_deref() == Some(UPSTREAM_BILLING_BASIS) {
            upstream_usage(usage)?
        } else {
            usage.clone()
        };
        let reservation = budget
            .reservations
            .get_mut(request_hash)
            .ok_or("response cache lacks its durable spending reservation")?;
        if usage.cost_source == "gateway_reported" || usage.cost_source == UPSTREAM_BILLING_BASIS {
            reservation.charged_microusd = Some(usage.cost_microusd);
        }
        reservation.completed = completed;
        reservation.usage = Some(usage.clone());
        let over_reservation = usage.cost_microusd > reservation.reserved_microusd;
        if over_reservation {
            budget.blocked_reason = Some("gateway charge exceeded conservative reservation".into());
        }
        write_json(&self.inner.root.join("budget.json"), &*budget)?;
        if over_reservation {
            return Err("gateway charge exceeded its conservative reservation; further dispatch requires price reconciliation".into());
        }
        Ok(usage)
    }
}

const UPSTREAM_BILLING_BASIS: &str = "user_confirmed_upstream_rates_uncached_upper_bound";

fn upstream_usage(usage: &Usage) -> Result<Usage, DynError> {
    let mut accounted = usage.clone();
    if accounted.cost_source != "gateway_reported" {
        accounted.cost_microusd = token_cost(
            usage.input_tokens,
            usage.output_tokens,
            prices(READER_MODEL)?,
        )?;
        accounted.cost_source = UPSTREAM_BILLING_BASIS.into();
    }
    Ok(accounted)
}

fn centra_endpoint(base: &str) -> Result<String, DynError> {
    let endpoint = format!("{}/chat/completions", base.trim_end_matches('/'));
    let url = reqwest::Url::parse(&endpoint)?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "benchmark gateway requires an HTTPS endpoint without embedded credentials".into(),
        );
    }
    Ok(endpoint)
}

fn prices(model: &str) -> Result<(u64, u64), DynError> {
    match model {
        READER_MODEL => Ok((200_000, 1_200_000)),
        _ => Err("model is not the selected benchmark model".into()),
    }
}

fn completion_request(
    model: &str,
    system: &str,
    user: &str,
    settings: CompletionSettings,
) -> Value {
    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(json!({"role":"system","content":system}));
    }
    messages.push(json!({"role":"user","content":user}));
    json!({"model":model,"reasoning_effort":settings.reasoning_effort,"max_completion_tokens":settings.max_output_tokens,
        "stream":false,"messages":messages})
}

fn token_cost(input: u64, output: u64, prices: (u64, u64)) -> Result<u64, DynError> {
    let cost = u128::from(input) * u128::from(prices.0) + u128::from(output) * u128::from(prices.1);
    u64::try_from(cost.div_ceil(1_000_000)).map_err(Into::into)
}

fn parse_completion(
    model: &str,
    request_hash: &str,
    raw: &str,
    reserve: u64,
) -> Result<Completion, DynError> {
    let value = response_value(raw)?;
    validate_finish(&value)?;
    let text = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or("gateway response omitted nonempty assistant content")?
        .to_owned();
    let usage = value
        .get("usage")
        .ok_or("gateway omitted actual token usage; reservation retained")?;
    let input_tokens = usage
        .get("prompt_tokens")
        .and_then(Value::as_u64)
        .ok_or("gateway omitted prompt usage")?;
    let output_tokens = usage
        .get("completion_tokens")
        .and_then(Value::as_u64)
        .ok_or("gateway omitted output usage")?;
    let reasoning_tokens = usage
        .pointer("/completion_tokens_details/reasoning_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let cached_input_tokens = usage
        .pointer("/prompt_tokens_details/cached_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let reported_cost = usage
        .get("cost")
        .and_then(Value::as_f64)
        .filter(|cost| cost.is_finite() && *cost >= 0.0);
    let (cost_microusd, cost_source) = if let Some(cost) = reported_cost {
        let micros = (cost * 1_000_000.0).ceil();
        if micros > u64::MAX as f64 {
            return Err("gateway cost exceeds representable budget".into());
        }
        (micros as u64, "gateway_reported")
    } else {
        (reserve, "retained_reservation_missing_gateway_cost")
    };
    let response_model = value
        .get("model")
        .and_then(Value::as_str)
        .ok_or("gateway omitted response model")?
        .to_owned();
    let canonical_model = confirm_model(model, &response_model)?;
    Ok(Completion {
        text,
        model: model.into(),
        response_model,
        canonical_snapshot: READER_CANONICAL_SNAPSHOT.into(),
        canonical_model: canonical_model.into(),
        model_identity_kind: MODEL_IDENTITY_KIND.into(),
        model_catalog_source: MODEL_CATALOG_SOURCE.into(),
        model_catalog_observed_at: MODEL_CATALOG_OBSERVED_AT.into(),
        request_hash: request_hash.into(),
        cached: false,
        usage: Usage {
            input_tokens,
            output_tokens,
            reasoning_tokens,
            cached_input_tokens,
            cost_microusd,
            cost_source: cost_source.into(),
        },
    })
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(value: &u64) -> bool {
    *value == 0
}

fn response_value(raw: &str) -> Result<Value, DynError> {
    let wire = raw.trim();
    let wire = wire
        .strip_suffix("data: [DONE]")
        .or_else(|| wire.strip_suffix("[DONE]"))
        .unwrap_or(wire)
        .trim_end();
    serde_json::from_str(wire).map_err(Into::into)
}

fn validate_finish(value: &Value) -> Result<(), DynError> {
    if value
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
        != Some("stop")
    {
        return Err("gateway completion did not finish normally; incomplete response cannot qualify and reservation is retained".into());
    }
    if value
        .pointer("/choices/0/native_finish_reason")
        .and_then(Value::as_str)
        .is_some_and(|reason| {
            matches!(
                reason.to_ascii_lowercase().as_str(),
                "length"
                    | "max_tokens"
                    | "max_completion_tokens"
                    | "max_output_tokens"
                    | "incomplete"
                    | "truncated"
            )
        })
    {
        return Err("gateway reported a truncated native completion; reservation retained and no automatic retry".into());
    }
    Ok(())
}

fn confirm_model(model: &str, response_model: &str) -> Result<&'static str, DynError> {
    let response_model = response_model
        .strip_prefix("openrouter/")
        .unwrap_or(response_model);
    let response_model = response_model
        .strip_prefix("openai/")
        .unwrap_or(response_model);
    match (model, response_model) {
        (READER_MODEL, "gpt-5.6-luna") => Ok(READER_CANONICAL_MODEL),
        _ => Err("gateway response did not match the exact selected Luna model identity".into()),
    }
}

fn request_hash(endpoint: &str, prompt_id: &str, request: &Value) -> Result<String, DynError> {
    let canonical = serde_json::to_vec(
        &json!({"version":2,"endpoint":endpoint,"prompt_id":prompt_id,"request":request,
        "canonical_model":READER_CANONICAL_MODEL,"model_identity_kind":MODEL_IDENTITY_KIND,
        "reader_canonical_snapshot":READER_CANONICAL_SNAPSHOT,"model_catalog_source":MODEL_CATALOG_SOURCE,
        "model_catalog_observed_at":MODEL_CATALOG_OBSERVED_AT}),
    )?;
    Ok(blake3::hash(&canonical).to_hex().to_string())
}

fn write_cache(path: &Path, entry: CacheEntry) -> Result<(), DynError> {
    let digest = blake3::hash(&serde_json::to_vec(&entry)?)
        .to_hex()
        .to_string();
    write_json(path, &CheckedCache { digest, entry })
}

fn read_cache(path: &Path, expected_hash: &str, request: &Value) -> Result<CacheEntry, DynError> {
    let cache: CheckedCache = serde_json::from_slice(&fs::read(path)?)?;
    if cache.entry.version != 1
        || cache.entry.request_hash != expected_hash
        || &cache.entry.request != request
        || cache.digest
            != blake3::hash(&serde_json::to_vec(&cache.entry)?)
                .to_hex()
                .as_str()
        || request_hash(
            &cache.entry.endpoint,
            &cache.entry.prompt_id,
            &cache.entry.request,
        )? != expected_hash
    {
        return Err("benchmark response cache digest or request identity mismatch".into());
    }
    if cache.entry.completion.is_some() {
        validate_finish(&response_value(&cache.entry.raw_response)?)?;
    }
    Ok(cache.entry)
}

pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), DynError> {
    let parent = path.parent().ok_or("output path has no parent")?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&serde_json::to_vec(value)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centra_configuration_preserves_endpoint_and_request_identity() -> Result<(), DynError> {
        let centra = centra_endpoint("https://gateway.centra.ag/v1/")?;
        assert_eq!(centra, "https://gateway.centra.ag/v1/chat/completions");
        let configured = centra_endpoint("https://configured.example/gateway/v1")?;
        assert_eq!(
            configured,
            "https://configured.example/gateway/v1/chat/completions"
        );
        assert!(centra_endpoint("http://configured.example/v1").is_err());
        assert!(centra_endpoint("https://credential@configured.example/v1").is_err());
        let request = json!({"model":READER_MODEL});
        assert_eq!(
            request_hash(&centra, "identity", &request)?,
            request_hash(
                "https://gateway.centra.ag/v1/chat/completions",
                "identity",
                &request
            )?
        );
        assert_ne!(
            request_hash(&centra, "identity", &request)?,
            request_hash(&configured, "identity", &request)?
        );
        Ok(())
    }

    #[test]
    fn durable_budget_reservations_survive_restart_and_enforce_cap() -> Result<(), DynError> {
        let directory = tempfile::tempdir()?;
        let gateway = Gateway::open(directory.path(), false, 1_000)?;
        gateway.reserve("unknown-dispatch", 600)?;
        assert!(Gateway::open(directory.path(), false, 1_000).is_err());
        assert!(gateway.reserve("unknown-dispatch", 1).is_err());
        assert!(gateway.reserve("over-cap", 401).is_err());
        assert_eq!(gateway.budget()?.available_microusd, 400);
        drop(gateway);
        let gateway = Gateway::open(directory.path(), false, 1_000)?;
        assert_eq!(gateway.budget()?.retained_reservations_microusd, 600);
        assert_eq!(gateway.budget()?.unknown_calls, 1);
        assert!(gateway.reserve("unknown-dispatch", 1).is_err());
        gateway.settle(
            "unknown-dispatch",
            &Usage {
                cost_microusd: 200,
                cost_source: "gateway_reported".into(),
                ..Usage::default()
            },
            true,
        )?;
        gateway.settle(
            "unknown-dispatch",
            &Usage {
                cost_microusd: 200,
                cost_source: "gateway_reported".into(),
                ..Usage::default()
            },
            true,
        )?;
        gateway.reserve("missing-cost", 800)?;
        gateway.settle(
            "missing-cost",
            &Usage {
                cost_microusd: 800,
                cost_source: "retained_reservation_missing_gateway_cost".into(),
                ..Usage::default()
            },
            true,
        )?;
        assert_eq!(gateway.budget()?.available_microusd, 0);
        assert_eq!(gateway.budget()?.charged_microusd, 200);
        assert!(gateway.reserve("one-more", 1).is_err());
        drop(gateway);
        assert!(Gateway::open(directory.path(), false, 1_001).is_err());
        let gateway = Gateway::open(directory.path(), false, 1_000)?;
        assert_eq!(gateway.budget()?.retained_reservations_microusd, 800);
        assert_eq!(gateway.budget()?.completed_calls, 2);
        assert_eq!(gateway.budget()?.available_microusd, 0);
        Ok(())
    }

    #[test]
    fn confirmed_upstream_accounting_preserves_unknown_costs_and_counts_reasoning_once()
    -> Result<(), DynError> {
        let directory = tempfile::tempdir()?;
        let gateway = Gateway::open(directory.path(), false, 10_000)?;
        gateway.reserve("unknown", 500)?;
        gateway.confirm_upstream_rates()?;
        gateway.reserve("measured", 9_000)?;
        let measured = Usage {
            input_tokens: 1_000,
            cached_input_tokens: 500,
            output_tokens: 200,
            reasoning_tokens: 150,
            cost_microusd: 9_000,
            cost_source: "retained_reservation_missing_gateway_cost".into(),
        };
        let accounted = gateway.settle("measured", &measured, true)?;
        assert_eq!(accounted.cost_microusd, 440);
        assert_eq!(accounted.cost_source, UPSTREAM_BILLING_BASIS);
        gateway.settle("measured", &measured, true)?;
        assert_eq!(gateway.budget()?.upstream_usage_accounted_microusd, 440);
        assert_eq!(gateway.budget()?.retained_reservations_microusd, 500);
        assert_eq!(gateway.budget()?.available_microusd, 9_060);
        drop(gateway);
        let gateway = Gateway::open(directory.path(), false, 10_000)?;
        gateway.reserve("reported", 100)?;
        gateway.settle(
            "reported",
            &Usage {
                cost_microusd: 19,
                cost_source: "gateway_reported".into(),
                ..measured
            },
            true,
        )?;
        assert_eq!(gateway.budget()?.charged_microusd, 459);
        assert_eq!(gateway.budget()?.unknown_calls, 1);
        assert_eq!(gateway.budget()?.upstream_usage_accounted_microusd, 440);
        assert!(gateway.reserve("over-cap", 9_042).is_err());
        Ok(())
    }

    #[test]
    fn budget_price_mismatch_blocks_future_dispatch_after_restart() -> Result<(), DynError> {
        let directory = tempfile::tempdir()?;
        let gateway = Gateway::open(directory.path(), false, 1_000)?;
        gateway.reserve("price-mismatch", 100)?;
        assert!(
            gateway
                .settle(
                    "price-mismatch",
                    &Usage {
                        cost_microusd: 101,
                        cost_source: "gateway_reported".into(),
                        ..Usage::default()
                    },
                    true
                )
                .is_err()
        );
        assert!(gateway.reserve("blocked", 1).is_err());
        drop(gateway);
        let gateway = Gateway::open(directory.path(), false, 1_000)?;
        assert!(gateway.reserve("still-blocked", 1).is_err());
        assert_eq!(gateway.budget()?.charged_microusd, 101);
        Ok(())
    }

    #[test]
    fn selected_luna_model_confirmation_rejects_old_models() {
        assert!(confirm_model(READER_MODEL, "gpt-5.6-luna").is_ok());
        assert!(confirm_model(READER_MODEL, "openai/gpt-5.6-luna").is_ok());
        assert!(confirm_model(JUDGE_MODEL, "openrouter/openai/gpt-5.6-luna").is_ok());
        assert!(confirm_model(READER_MODEL, "gpt-5.6-luna-wrong").is_err());
        assert!(confirm_model(READER_MODEL, "gpt-4.1-mini-2025-04-14").is_err());
        assert!(confirm_model(JUDGE_MODEL, "gpt-4o-2024-08-06").is_err());
        assert!(confirm_model("openrouter/openai/gpt-4.1-mini", "gpt-5.6-luna").is_err());
    }

    #[test]
    fn shared_luna_reader_and_judge_use_one_price_and_supported_request_shape()
    -> Result<(), DynError> {
        assert_eq!(READER_MODEL, JUDGE_MODEL);
        assert_eq!(prices(READER_MODEL)?, (200_000, 1_200_000));
        assert_eq!(prices(JUDGE_MODEL)?, prices(READER_MODEL)?);
        assert!(prices("openrouter/openai/gpt-4o-2024-08-06").is_err());
        let judge = completion_request(JUDGE_MODEL, "", "grade", JUDGE_SETTINGS);
        assert_eq!(judge["model"], "openrouter/openai/gpt-5.6-luna");
        assert_eq!(judge["reasoning_effort"], "medium");
        assert_eq!(judge["max_completion_tokens"], 2_048);
        assert!(judge.get("temperature").is_none());
        assert!(judge.get("max_tokens").is_none());
        assert_eq!(judge["messages"].as_array().unwrap().len(), 1);
        assert_eq!(judge["messages"][0]["role"], "user");
        let reader = completion_request(READER_MODEL, "memory reader", "question", READER_SETTINGS);
        assert_eq!(reader["max_completion_tokens"], 4_096);
        assert_eq!(reader["reasoning_effort"], "medium");
        assert_eq!(reader["messages"].as_array().unwrap().len(), 2);
        let old = json!({"model":"openrouter/openai/gpt-4o-2024-08-06","temperature":0,
            "max_tokens":10,"stream":false,"messages":[{"role":"user","content":"grade"}]});
        assert_ne!(
            request_hash(
                "https://gateway.centra.ag/v1/chat/completions",
                "judge",
                &judge
            )?,
            request_hash(
                "https://gateway.centra.ag/v1/chat/completions",
                "judge",
                &old
            )?
        );
        let probe = completion_request(READER_MODEL, "", "grade", PROBE_SETTINGS);
        assert_eq!(probe["max_completion_tokens"], 10);
        assert_eq!(probe["reasoning_effort"], "none");
        assert_ne!(
            request_hash(
                "https://gateway.centra.ag/v1/chat/completions",
                "same",
                &probe
            )?,
            request_hash(
                "https://gateway.centra.ag/v1/chat/completions",
                "same",
                &judge
            )?
        );
        let max_only = completion_request(
            READER_MODEL,
            "",
            "grade",
            CompletionSettings {
                max_output_tokens: JUDGE_SETTINGS.max_output_tokens,
                reasoning_effort: "none",
            },
        );
        assert_ne!(
            request_hash(
                "https://gateway.centra.ag/v1/chat/completions",
                "same",
                &max_only
            )?,
            request_hash(
                "https://gateway.centra.ag/v1/chat/completions",
                "same",
                &judge
            )?
        );
        assert!(
            token_cost(
                0,
                u64::from(READER_SETTINGS.max_output_tokens),
                prices(READER_MODEL)?
            )? > token_cost(
                0,
                u64::from(PROBE_SETTINGS.max_output_tokens),
                prices(READER_MODEL)?
            )?
        );
        Ok(())
    }

    #[test]
    fn incomplete_finish_reasons_do_not_qualify() {
        assert!(
            validate_finish(
                &json!({"choices":[{"finish_reason":"stop","native_finish_reason":"completed"}]})
            )
            .is_ok()
        );
        for reason in ["length", "truncated", "content_filter", "tool_calls"] {
            assert!(validate_finish(&json!({"choices":[{"finish_reason":reason}]})).is_err());
        }
        assert!(
            validate_finish(
                &json!({"choices":[{"finish_reason":"stop","native_finish_reason":"MAX_TOKENS"}]})
            )
            .is_err()
        );
        assert!(validate_finish(&json!({"choices":[{}]})).is_err());
    }

    #[test]
    fn zero_reasoning_usage_preserves_historical_serialization() -> Result<(), DynError> {
        let historical = json!({"input_tokens":100,"output_tokens":5,"cached_input_tokens":0,
            "cost_microusd":100,"cost_source":"retained_reservation_missing_gateway_cost"});
        let mut usage: Usage = serde_json::from_value(historical.clone())?;
        assert_eq!(usage.reasoning_tokens, 0);
        assert_eq!(serde_json::to_value(&usage)?, historical);
        usage.reasoning_tokens = 4;
        assert_eq!(serde_json::to_value(&usage)?["reasoning_tokens"], 4);
        assert_eq!(usage.output_tokens, 5);
        Ok(())
    }
}
