#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use lopdf::{Document, Object, ObjectId};

use crate::loader::{
    DocumentLoader, Extraction, LoaderId, PageSpan, PartialExtraction, guard_extracted_size,
    guard_input_size,
};

pub const PDF_EXTRACTION_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PdfLoader;

impl DocumentLoader for PdfLoader {
    fn loader_id(&self) -> LoaderId {
        LoaderId::Pdf
    }

    fn extraction_version(&self) -> u16 {
        PDF_EXTRACTION_VERSION
    }

    fn extract(&self, bytes: &[u8]) -> Result<Extraction, Error> {
        guard_input_size(bytes)?;
        let document =
            Document::load_mem(bytes).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
        let pages = document.get_pages();
        if pages.is_empty() {
            return Err(Error::new(ErrorCode::SchemaInvalid));
        }
        let mut text = String::new();
        let mut page_spans = Vec::with_capacity(pages.len());
        let mut failed_units = Vec::new();
        for (page_number, page_id) in pages {
            match page_text(&document, page_number, page_id) {
                Some(extracted) => {
                    if !page_spans.is_empty() {
                        text.push('\n');
                    }
                    let byte_start = text.len();
                    text.push_str(&extracted);
                    page_spans.push(PageSpan {
                        page_number,
                        byte_start,
                        byte_end: text.len(),
                    });
                }
                None => failed_units.push(page_number),
            }
        }
        if page_spans.is_empty() {
            return Err(Error::new(ErrorCode::SchemaInvalid));
        }
        guard_extracted_size(&text)?;
        let partial = (!failed_units.is_empty()).then(|| PartialExtraction {
            reason: failure_reason(&failed_units),
            failed_units,
        });
        Ok(Extraction {
            loader: LoaderId::Pdf,
            extraction_version: PDF_EXTRACTION_VERSION,
            text,
            page_spans,
            table: None,
            partial,
        })
    }
}

fn page_text(document: &Document, page_number: u32, page_id: ObjectId) -> Option<String> {
    if !content_streams_resolve(document, page_id) {
        return None;
    }
    let mut text = String::new();
    for chunk in document.extract_text_chunks(&[page_number]) {
        text.push_str(chunk.ok()?.as_str());
    }
    Some(text)
}

fn content_streams_resolve(document: &Document, page_id: ObjectId) -> bool {
    let streams = document.get_page_contents(page_id);
    streams.is_empty()
        || streams
            .iter()
            .any(|id| document.get_object(*id).and_then(Object::as_stream).is_ok())
}

fn failure_reason(failed_units: &[u32]) -> String {
    let pages = failed_units
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!("page content could not be extracted for pages {pages}")
}
