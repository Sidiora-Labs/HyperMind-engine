use crate::Usage;
use crate::outcome::{ResponseFault, ResponseOutcome};
use serde_json::Value;

pub const EXTRACTION_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_IDENTIFIER_CHARACTERS: usize = 512;
pub const MAXIMUM_PROSE_CHARACTERS: usize = 65_536;
pub const MAXIMUM_LIST_ITEMS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldShape {
    Identifier,
    NullableIdentifier,
    Prose,
    IdentifierList,
    Count,
    Flag,
    Opaque,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldRule {
    pub name: &'static str,
    pub shape: FieldShape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtractionContract {
    pub contract_id: &'static str,
    pub version: u16,
    pub fields: &'static [FieldRule],
}

impl ExtractionContract {
    pub fn validate(&self, value: &Value) -> Result<(), ContractViolation> {
        let Some(object) = value.as_object() else {
            return Err(ContractViolation::NotAnObject);
        };
        for rule in self.fields {
            let Some(present) = object.get(rule.name) else {
                return Err(ContractViolation::MissingField(rule.name.to_owned()));
            };
            check_shape(rule.name, rule.shape, present)?;
        }
        for key in object.keys() {
            if !self.fields.iter().any(|rule| rule.name == key.as_str()) {
                return Err(ContractViolation::UnknownField(key.clone()));
            }
        }
        Ok(())
    }

    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn fault(
        &self,
        violation: &ContractViolation,
        model_id: &str,
        requested_output_tokens: u32,
        usage: Usage,
    ) -> ResponseFault {
        ResponseFault::new(
            ResponseOutcome::Incomplete,
            model_id,
            violation.detail(),
            requested_output_tokens,
            usage,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractViolation {
    NotAnObject,
    MissingField(String),
    UnknownField(String),
    WrongShape(String),
    EmptyIdentifier(String),
    IdentifierTooLong {
        field: String,
        characters: usize,
        limit: usize,
    },
    ProseTooLong {
        field: String,
        characters: usize,
        limit: usize,
    },
    ListTooLong {
        field: String,
        items: usize,
        limit: usize,
    },
}

impl ContractViolation {
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        match self {
            Self::NotAnObject => None,
            Self::MissingField(field)
            | Self::UnknownField(field)
            | Self::WrongShape(field)
            | Self::EmptyIdentifier(field)
            | Self::IdentifierTooLong { field, .. }
            | Self::ProseTooLong { field, .. }
            | Self::ListTooLong { field, .. } => Some(field.as_str()),
        }
    }

    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::NotAnObject => "structured response was not a JSON object".to_owned(),
            Self::MissingField(field) => {
                format!("field {field} is declared by the contract but absent")
            }
            Self::UnknownField(field) => {
                format!("field {field} is present but not declared by the contract")
            }
            Self::WrongShape(field) => format!("field {field} has the wrong shape"),
            Self::EmptyIdentifier(field) => format!("field {field} is an empty identifier"),
            Self::IdentifierTooLong {
                field,
                characters,
                limit,
            } => format!("field {field} is {characters} characters, identifier limit {limit}"),
            Self::ProseTooLong {
                field,
                characters,
                limit,
            } => format!("field {field} is {characters} characters, prose limit {limit}"),
            Self::ListTooLong {
                field,
                items,
                limit,
            } => format!("field {field} carries {items} entries, list limit {limit}"),
        }
    }
}

fn check_shape(name: &str, shape: FieldShape, value: &Value) -> Result<(), ContractViolation> {
    match shape {
        FieldShape::Identifier => check_identifier(name, value),
        FieldShape::NullableIdentifier => {
            if value.is_null() {
                Ok(())
            } else {
                check_identifier(name, value)
            }
        }
        FieldShape::Prose => check_prose(name, value),
        FieldShape::IdentifierList => check_identifier_list(name, value),
        FieldShape::Count => {
            if value.as_u64().is_some() {
                Ok(())
            } else {
                Err(ContractViolation::WrongShape(name.to_owned()))
            }
        }
        FieldShape::Flag => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err(ContractViolation::WrongShape(name.to_owned()))
            }
        }
        FieldShape::Opaque => Ok(()),
    }
}

fn check_identifier(name: &str, value: &Value) -> Result<(), ContractViolation> {
    let Some(text) = value.as_str() else {
        return Err(ContractViolation::WrongShape(name.to_owned()));
    };
    if text.trim().is_empty() {
        return Err(ContractViolation::EmptyIdentifier(name.to_owned()));
    }
    let characters = text.chars().count();
    if characters > MAXIMUM_IDENTIFIER_CHARACTERS {
        return Err(ContractViolation::IdentifierTooLong {
            field: name.to_owned(),
            characters,
            limit: MAXIMUM_IDENTIFIER_CHARACTERS,
        });
    }
    Ok(())
}

fn check_prose(name: &str, value: &Value) -> Result<(), ContractViolation> {
    let Some(text) = value.as_str() else {
        return Err(ContractViolation::WrongShape(name.to_owned()));
    };
    if text.trim().is_empty() {
        return Err(ContractViolation::WrongShape(name.to_owned()));
    }
    let characters = text.chars().count();
    if characters > MAXIMUM_PROSE_CHARACTERS {
        return Err(ContractViolation::ProseTooLong {
            field: name.to_owned(),
            characters,
            limit: MAXIMUM_PROSE_CHARACTERS,
        });
    }
    Ok(())
}

fn check_identifier_list(name: &str, value: &Value) -> Result<(), ContractViolation> {
    let Some(entries) = value.as_array() else {
        return Err(ContractViolation::WrongShape(name.to_owned()));
    };
    if entries.is_empty() {
        return Err(ContractViolation::WrongShape(name.to_owned()));
    }
    if entries.len() > MAXIMUM_LIST_ITEMS {
        return Err(ContractViolation::ListTooLong {
            field: name.to_owned(),
            items: entries.len(),
            limit: MAXIMUM_LIST_ITEMS,
        });
    }
    for entry in entries {
        check_identifier(name, entry)?;
    }
    Ok(())
}
