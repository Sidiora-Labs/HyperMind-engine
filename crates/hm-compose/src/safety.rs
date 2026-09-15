#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationBundle, ActivationItem, Tier};
use hm_core::{Error, ErrorCode, LSN};
use hm_schema::event::{self, Boundary};
use hm_schema::events::{Authority, EventPayload};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafeContent {
    pub bytes: Vec<u8>,
    pub authority: Authority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptItem {
    pub tier: Tier,
    pub role: &'static str,
    pub authority: Authority,
    pub provenance_uri: String,
    pub provenance: Vec<LSN>,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptSection {
    pub tier: Tier,
    pub items: Vec<PromptItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedPrompt {
    pub sections: Vec<PromptSection>,
    pub bundle_hash: [u8; 32],
}

#[must_use]
pub fn decode_semantic_content(
    encoded: &[u8],
    expected_kind: event::EventKind,
) -> Option<SafeContent> {
    let verified = event::verify_event(encoded, expected_kind, Boundary::Disk).ok()?;
    let bytes = match verified.envelope.payload {
        EventPayload::UserMsg(message) => message.content,
        EventPayload::DeliveredMsg(message) => message.content,
        _ => return None,
    };
    (!raw_wire_bytes(&bytes)).then_some(SafeContent {
        bytes,
        authority: verified.envelope.authority,
    })
}

#[must_use]
pub fn safe_item(item: &ActivationItem, same_turn_lsns: &BTreeSet<LSN>) -> bool {
    !item.uri.is_empty()
        && item.uri.starts_with("hm://")
        && !item.provenance.is_empty()
        && item.provenance.iter().all(|lsn| lsn.get() != 0)
        && item
            .provenance
            .iter()
            .all(|lsn| !same_turn_lsns.contains(lsn))
        && !raw_wire_bytes(&item.content)
}

pub fn render(
    bundle: &ActivationBundle,
    same_turn_lsns: impl IntoIterator<Item = LSN>,
) -> Result<RenderedPrompt, Error> {
    let same_turn_lsns = same_turn_lsns.into_iter().collect::<BTreeSet<_>>();
    let mut sections = Vec::with_capacity(bundle.sections.len());
    for section in &bundle.sections {
        let mut rendered = PromptSection {
            tier: section.tier,
            items: Vec::new(),
        };
        for item in &section.items {
            if !safe_item(item, &same_turn_lsns) {
                continue;
            }
            rendered.items.push(PromptItem {
                tier: item.tier,
                role: "user",
                authority: item.authority,
                provenance_uri: item.uri.clone(),
                provenance: item.provenance.clone(),
                content: String::from_utf8(item.content.clone())
                    .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?,
            });
        }
        sections.push(rendered);
    }
    Ok(RenderedPrompt {
        sections,
        bundle_hash: bundle.bundle_hash,
    })
}

#[must_use]
pub fn raw_wire_bytes(content: &[u8]) -> bool {
    content.get(4..8) == Some(b"NCEV")
        || content.starts_with(b"NCEV")
        || content.starts_with(b"PCCN")
        || content.starts_with(b"NCCP")
}
