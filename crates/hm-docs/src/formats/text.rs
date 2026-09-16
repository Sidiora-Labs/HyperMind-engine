#![allow(clippy::missing_errors_doc)]

use hm_core::Error;

use crate::loader::{
    DocumentLoader, Extraction, LoaderId, PageSpan, decode_text, guard_extracted_size,
    guard_input_size,
};

pub const TEXT_EXTRACTION_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextLoader;

impl DocumentLoader for TextLoader {
    fn loader_id(&self) -> LoaderId {
        LoaderId::Text
    }

    fn extraction_version(&self) -> u16 {
        TEXT_EXTRACTION_VERSION
    }

    fn extract(&self, bytes: &[u8]) -> Result<Extraction, Error> {
        guard_input_size(bytes)?;
        let text = decode_text(bytes)?;
        guard_extracted_size(&text)?;
        let page_spans = vec![PageSpan {
            page_number: 1,
            byte_start: 0,
            byte_end: text.len(),
        }];
        Ok(Extraction {
            loader: LoaderId::Text,
            extraction_version: TEXT_EXTRACTION_VERSION,
            text,
            page_spans,
            table: None,
            partial: None,
        })
    }
}
