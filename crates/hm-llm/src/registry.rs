use crate::LlmError;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Prompt {
    pub id: String,
    pub version: u32,
    pub content: String,
    pub digest: [u8; 32],
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PromptRegistry {
    prompts: BTreeMap<(String, u32), Prompt>,
}

impl PromptRegistry {
    pub fn load(directory: &Path) -> Result<Self, LlmError> {
        let mut paths = fs::read_dir(directory)
            .map_err(|error| LlmError::Wire(error.to_string()))?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| LlmError::Wire(error.to_string()))?;
        paths.sort();
        let mut registry = Self::default();
        for path in paths {
            if path.extension().and_then(|value| value.to_str()) != Some("md") {
                continue;
            }
            let (id, version) = parse_name(&path)?;
            let content =
                fs::read_to_string(&path).map_err(|error| LlmError::Wire(error.to_string()))?;
            if content.is_empty() || !content.ends_with('\n') {
                return Err(LlmError::Schema(format!(
                    "prompt {} must be non-empty and newline terminated",
                    path.display()
                )));
            }
            let prompt = Prompt {
                id: id.clone(),
                version,
                digest: *blake3::hash(content.as_bytes()).as_bytes(),
                content,
            };
            if registry.prompts.insert((id, version), prompt).is_some() {
                return Err(LlmError::Schema("duplicate prompt version".to_owned()));
            }
        }
        if registry.prompts.is_empty() {
            return Err(LlmError::Schema("prompt registry is empty".to_owned()));
        }
        Ok(registry)
    }

    #[must_use]
    pub fn get(&self, id: &str, version: u32) -> Option<&Prompt> {
        self.prompts.get(&(id.to_owned(), version))
    }

    #[must_use]
    pub fn latest(&self, id: &str) -> Option<&Prompt> {
        self.prompts
            .range((id.to_owned(), 0)..=(id.to_owned(), u32::MAX))
            .next_back()
            .map(|(_, prompt)| prompt)
    }

    pub fn verify_snapshot(
        &self,
        id: &str,
        version: u32,
        expected_digest: [u8; 32],
    ) -> Result<(), LlmError> {
        let prompt = self
            .get(id, version)
            .ok_or_else(|| LlmError::Schema(format!("missing prompt {id}@{version}")))?;
        if prompt.digest == expected_digest {
            Ok(())
        } else {
            Err(LlmError::Schema(format!(
                "prompt {id}@{version} changed without a version bump"
            )))
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.prompts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }
}

fn parse_name(path: &Path) -> Result<(String, u32), LlmError> {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| LlmError::Schema("prompt filename is not UTF-8".to_owned()))?;
    let (id, version) = stem
        .rsplit_once('@')
        .ok_or_else(|| LlmError::Schema(format!("unversioned prompt {stem}")))?;
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(LlmError::Schema(format!("invalid prompt id {id}")));
    }
    let version = version
        .parse::<u32>()
        .ok()
        .filter(|version| *version != 0)
        .ok_or_else(|| LlmError::Schema(format!("invalid prompt version {version}")))?;
    Ok((id.to_owned(), version))
}
