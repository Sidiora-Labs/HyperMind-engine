#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use mail_parser::{Addr, Address, DateTime, Message, MessageParser, MimeHeaders};

use crate::loader::{
    DocumentLoader, Extraction, LoaderId, PageSpan, PartialExtraction, guard_extracted_size,
    guard_input_size, unit_index,
};

pub const MAIL_EXTRACTION_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MailLoader;

impl DocumentLoader for MailLoader {
    fn loader_id(&self) -> LoaderId {
        LoaderId::Mail
    }

    fn extraction_version(&self) -> u16 {
        MAIL_EXTRACTION_VERSION
    }

    fn extract(&self, bytes: &[u8]) -> Result<Extraction, Error> {
        guard_input_size(bytes)?;
        let message = MessageParser::default()
            .parse(bytes)
            .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))?;
        let mut text = header_block(&message);
        text.push('\n');
        if let Some(body) = message.body_text(0) {
            text.push_str(body.as_ref());
        }
        guard_extracted_size(&text)?;
        let page_spans = vec![PageSpan {
            page_number: 1,
            byte_start: 0,
            byte_end: text.len(),
        }];
        let partial = unextracted_attachments(&message)?;
        Ok(Extraction {
            loader: LoaderId::Mail,
            extraction_version: MAIL_EXTRACTION_VERSION,
            text,
            page_spans,
            table: None,
            partial,
        })
    }
}

fn header_block(message: &Message<'_>) -> String {
    let from = message.from().map(render_address).unwrap_or_default();
    let to = message.to().map(render_address).unwrap_or_default();
    let subject = message.subject().unwrap_or_default();
    let date = message.date().map(DateTime::to_rfc3339).unwrap_or_default();
    format!("From: {from}\nTo: {to}\nSubject: {subject}\nDate: {date}\n")
}

fn render_address(address: &Address<'_>) -> String {
    address
        .iter()
        .map(render_mailbox)
        .filter(|rendered| !rendered.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_mailbox(mailbox: &Addr<'_>) -> String {
    match (mailbox.name.as_deref(), mailbox.address.as_deref()) {
        (Some(name), Some(address)) => format!("{name} <{address}>"),
        (Some(name), None) => name.to_owned(),
        (None, Some(address)) => address.to_owned(),
        (None, None) => String::new(),
    }
}

fn unextracted_attachments(message: &Message<'_>) -> Result<Option<PartialExtraction>, Error> {
    let mut failed_units = Vec::new();
    let mut names = Vec::new();
    for (index, attachment) in message.attachments().enumerate() {
        failed_units.push(unit_index(index)?);
        names.push(
            attachment
                .attachment_name()
                .map_or_else(|| format!("part {index}"), str::to_owned),
        );
    }
    if failed_units.is_empty() {
        return Ok(None);
    }
    let names = names.join(", ");
    Ok(Some(PartialExtraction {
        reason: format!("attachments are not extracted: {names}"),
        failed_units,
    }))
}
