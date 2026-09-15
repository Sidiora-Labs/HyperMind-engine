use rmcp::schemars;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecallMode {
    Semantic,
    Lexical,
    Entity,
    Temporal,
    Near,
    Timeline,
}

#[derive(Clone, Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct RecallFilters {
    #[serde(default)]
    pub conversation: Option<String>,
    #[serde(default)]
    pub since_lsn: Option<u64>,
    #[serde(default)]
    pub until_lsn: Option<u64>,
    #[serde(default)]
    pub temporal_from_ns: Option<i64>,
    #[serde(default)]
    pub temporal_to_ns: Option<i64>,
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(default)]
    pub turn_text: String,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RecallInput {
    pub mode: RecallMode,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub conversation: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub since_lsn: u64,
    #[serde(default)]
    pub filters: RecallFilters,
}

const fn default_limit() -> usize {
    32
}
