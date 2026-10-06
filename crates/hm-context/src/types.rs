use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONTRACT_VERSION: u32 = 1;
pub const MAX_IDENTIFIER_BYTES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub owner_id: String,
    pub project_id: String,
    pub workspace_id: Option<String>,
}

impl Scope {
    pub fn validate(&self) -> Result<(), ContextError> {
        validate_id(&self.owner_id)?;
        validate_id(&self.project_id)?;
        if let Some(workspace) = &self.workspace_id {
            validate_id(workspace)?;
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, ContextError> {
        self.validate()?;
        Ok(digest_bytes(&serde_json::to_vec(self)?))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cursor {
    pub epoch: u64,
    pub sequence: u64,
}

impl Default for Cursor {
    fn default() -> Self { Self { epoch: 1, sequence: 0 } }
}

impl Cursor {
    pub fn validate(self) -> Result<Self, ContextError> {
        const MAX_SAFE: u64 = (1_u64 << 53) - 1;
        if self.epoch == 0 || self.epoch > MAX_SAFE || self.sequence > MAX_SAFE {
            return Err(ContextError::Invalid("invalid cursor".into()));
        }
        Ok(self)
    }

    pub fn advance(self) -> Result<Self, ContextError> {
        self.validate()?;
        Self { epoch: self.epoch, sequence: self.sequence.checked_add(1).ok_or(ContextError::Capacity)? }.validate()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    UserAsserted,
    ExternalObserved,
    ToolObserved,
    RuntimeFact,
    AssistantGenerated,
    DerivedInference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole { User, Assistant, Tool }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MessagePart {
    Text { text: String },
    ToolCall { call_id: String, name: String, arguments: String },
    ToolResult { call_id: String, content: String, failed: bool },
    Opaque { media_type: String, reference: String, digest: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMessage {
    pub id: String,
    pub ordinal: u64,
    pub role: MessageRole,
    pub parts: Vec<MessagePart>,
    #[serde(with = "optional_timestamp_wire")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "timestamp_wire")]
    pub recorded_at_ns: i64,
    pub authority: Authority,
    pub source_digest: String,
}

impl SourceMessage {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut canonical = self.clone();
        canonical.source_digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&canonical)?))
    }

    pub fn validate(&self) -> Result<(), ContextError> {
        validate_id(&self.id)?;
        if self.parts.is_empty() || self.parts.len() > 4096 || self.source_digest != self.computed_digest()? {
            return Err(ContextError::Invalid("invalid source message".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpan {
    pub source_id: String,
    pub source_digest: String,
    pub byte_start: u64,
    pub byte_end: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenBudget {
    pub context_tokens: u64,
    pub reserved_output_tokens: u64,
    pub required_tokens: u64,
}

impl TokenBudget {
    pub fn available(self) -> Result<u64, ContextError> {
        self.context_tokens.checked_sub(self.reserved_output_tokens)
            .and_then(|tokens| tokens.checked_sub(self.required_tokens))
            .filter(|_| self.context_tokens > 0 && self.reserved_output_tokens > 0)
            .ok_or(ContextError::Capacity)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextBlock {
    pub id: String,
    pub text: String,
    pub authority: Authority,
    pub provenance: Vec<SourceSpan>,
    pub tokens: u64,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Omission { pub id: String, pub reason: String }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextReport {
    pub version: u32,
    pub scope: Scope,
    pub session_id: String,
    pub cursor: Cursor,
    pub generation: u64,
    pub blocks: Vec<ContextBlock>,
    pub included: Vec<String>,
    pub omitted: Vec<Omission>,
    pub gaps: Vec<String>,
    pub token_count: u64,
    pub digest: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("invalid input: {0}")] Invalid(String),
    #[error("scope mismatch")] ScopeMismatch,
    #[error("stale state")] Stale,
    #[error("conflicting state")] Conflict,
    #[error("capacity exceeded")] Capacity,
    #[error("unavailable: {0}")] Unavailable(String),
    #[error("storage failure: {0}")] Io(#[from] std::io::Error),
    #[error("serialization failure: {0}")] Json(#[from] serde_json::Error),
}

pub fn validate_id(value: &str) -> Result<(), ContextError> {
    if value.is_empty() || value.len() > MAX_IDENTIFIER_BYTES || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ContextError::Invalid("invalid identifier".into()));
    }
    Ok(())
}

pub fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}


#[derive(Deserialize)]
#[serde(untagged)]
enum WireTimestamp { Decimal(String), SafeInteger(i64) }

impl WireTimestamp {
    fn value<E: serde::de::Error>(self) -> Result<i64, E> {
        match self {
            Self::Decimal(text) => {
                let value = text.parse::<i64>().map_err(E::custom)?;
                if value.to_string() != text { return Err(E::custom("noncanonical timestamp")); }
                Ok(value)
            }
            Self::SafeInteger(value) if value.unsigned_abs() <= (1_u64 << 53) - 1 => Ok(value),
            Self::SafeInteger(_) => Err(E::custom("unsafe numeric timestamp")),
        }
    }
}

pub mod timestamp_wire {
    use super::*;
    pub fn serialize<S: serde::Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&value.to_string())
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
        WireTimestamp::deserialize(deserializer)?.value()
    }
}

pub mod optional_timestamp_wire {
    use super::*;
    pub fn serialize<S: serde::Serializer>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error> {
        match value { Some(value) => serializer.serialize_some(&value.to_string()), None => serializer.serialize_none() }
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<i64>, D::Error> {
        Option::<WireTimestamp>::deserialize(deserializer)?.map(WireTimestamp::value).transpose()
    }
}

#[cfg(test)]
mod timestamp_contract_tests {
    use super::*;

    fn source() -> SourceMessage {
        SourceMessage {
            id: "m1".into(), ordinal: 0, role: MessageRole::User,
            parts: vec![MessagePart::Text { text: "hello λ".into() }],
            occurred_at_ns: Some(1_791_287_999_123_456_789),
            recorded_at_ns: 1_791_288_000_123_456_789,
            authority: Authority::UserAsserted, source_digest: String::new(),
        }
    }

    #[test]
    fn current_scale_timestamps_have_cross_language_digest() {
        let mut source = source();
        source.source_digest = source.computed_digest().unwrap();
        assert_eq!(source.source_digest, "78af0610090d418ca2335976c873cfcc7647f9ce431f98696040d8fa1660500c");
        let wire = serde_json::to_value(&source).unwrap();
        assert_eq!(wire["occurred_at_ns"], "1791287999123456789");
        assert_eq!(wire["recorded_at_ns"], "1791288000123456789");
        let decoded: SourceMessage = serde_json::from_value(wire).unwrap();
        assert_eq!(source, decoded);
        decoded.validate().unwrap();
    }

    #[test]
    fn timestamp_wire_rejects_lossy_and_noncanonical_values() {
        let base = serde_json::to_value(source()).unwrap();
        for bad in [serde_json::json!(1791288000123456789_i64), serde_json::json!("01"), serde_json::json!("-0"), serde_json::json!("9223372036854775808")] {
            let mut value = base.clone(); value["recorded_at_ns"] = bad;
            assert!(serde_json::from_value::<SourceMessage>(value).is_err());
        }
        let mut legacy = base;
        legacy["occurred_at_ns"] = serde_json::Value::Null;
        legacy["recorded_at_ns"] = serde_json::json!(42);
        let decoded: SourceMessage = serde_json::from_value(legacy).unwrap();
        assert_eq!(decoded.recorded_at_ns, 42);
        assert_eq!(serde_json::to_value(decoded).unwrap()["recorded_at_ns"], "42");
    }
}
