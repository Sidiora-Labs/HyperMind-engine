use hm_schema::events::{Retention, Sensitivity};
use rmcp::schemars;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RememberKind {
    User,
    Assistant,
    Document,
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnchorFacet {
    Path,
    Symbol,
    Url,
    Entity,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberAnchor {
    pub facet: AnchorFacet,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetentionInput {
    CurrentState,
    Daily,
    Durable,
    DoNotStore,
}

impl From<RetentionInput> for Retention {
    fn from(value: RetentionInput) -> Self {
        match value {
            RetentionInput::CurrentState => Self::CurrentState,
            RetentionInput::Daily => Self::Daily,
            RetentionInput::Durable => Self::Durable,
            RetentionInput::DoNotStore => Self::DoNotStore,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityInput {
    Public,
    Personal,
    Secret,
}

impl From<SensitivityInput> for Sensitivity {
    fn from(value: SensitivityInput) -> Self {
        match value {
            SensitivityInput::Public => Self::Public,
            SensitivityInput::Personal => Self::Personal,
            SensitivityInput::Secret => Self::Secret,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberInput {
    pub conversation: String,
    pub content: String,
    pub kind: RememberKind,
    #[serde(default)]
    pub chunk_bytes: Option<usize>,
    #[serde(default)]
    pub anchor: Option<RememberAnchor>,
    #[serde(default)]
    pub retention: Option<RetentionInput>,
    #[serde(default)]
    pub sensitivity: Option<SensitivityInput>,
}
