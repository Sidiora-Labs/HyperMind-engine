#![allow(clippy::missing_errors_doc)]

use hm_core::Error;

use crate::loader::{
    DocumentLoader, Extraction, FieldSpan, LoaderId, PageSpan, RowSpan, TableLayout, decode_text,
    guard_extracted_size, guard_input_size, unit_index,
};

pub const TABLE_EXTRACTION_VERSION: u16 = 1;

const COMMA: u8 = b',';
const TAB: u8 = b'\t';
const QUOTE: u8 = b'"';

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TableLoader;

impl DocumentLoader for TableLoader {
    fn loader_id(&self) -> LoaderId {
        LoaderId::Table
    }

    fn extraction_version(&self) -> u16 {
        TABLE_EXTRACTION_VERSION
    }

    fn extract(&self, bytes: &[u8]) -> Result<Extraction, Error> {
        guard_input_size(bytes)?;
        let text = decode_text(bytes)?;
        guard_extracted_size(&text)?;
        let delimiter = record_delimiter(text.as_bytes());
        let records = scan_records(text.as_bytes(), delimiter);
        let header = match records.first() {
            Some(record) => record
                .fields
                .iter()
                .map(|field| field_text(&text, field.0, field.1))
                .collect(),
            None => Vec::new(),
        };
        let mut rows = Vec::with_capacity(records.len().saturating_sub(1));
        for (index, record) in records.iter().enumerate().skip(1) {
            let mut fields = Vec::with_capacity(record.fields.len());
            for (column, field) in record.fields.iter().enumerate() {
                fields.push(FieldSpan {
                    column_index: unit_index(column)?,
                    byte_start: field.0,
                    byte_end: field.1,
                });
            }
            rows.push(RowSpan {
                row_index: unit_index(index)?,
                byte_start: record.start,
                byte_end: record.end,
                fields,
            });
        }
        let page_spans = vec![PageSpan {
            page_number: 1,
            byte_start: 0,
            byte_end: text.len(),
        }];
        Ok(Extraction {
            loader: LoaderId::Table,
            extraction_version: TABLE_EXTRACTION_VERSION,
            text,
            page_spans,
            table: Some(TableLayout { header, rows }),
            partial: None,
        })
    }
}

struct RecordScan {
    start: usize,
    end: usize,
    fields: Vec<(usize, usize)>,
}

fn record_delimiter(bytes: &[u8]) -> u8 {
    let mut quoted = false;
    for byte in bytes {
        match *byte {
            QUOTE => quoted = !quoted,
            TAB if !quoted => return TAB,
            b'\n' if !quoted => break,
            _ => {}
        }
    }
    COMMA
}

fn scan_records(bytes: &[u8], delimiter: u8) -> Vec<RecordScan> {
    let mut records = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let start = index;
        let mut fields = Vec::new();
        loop {
            let field_start = index;
            if bytes.get(index) == Some(&QUOTE) {
                index = scan_quoted_field(bytes, index);
            }
            while index < bytes.len()
                && bytes[index] != delimiter
                && !starts_terminator(bytes, index)
            {
                index += 1;
            }
            fields.push((field_start, index));
            if bytes.get(index) == Some(&delimiter) {
                index += 1;
                continue;
            }
            break;
        }
        let end = index;
        index = skip_terminator(bytes, index);
        records.push(RecordScan { start, end, fields });
    }
    records
}

fn scan_quoted_field(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 1;
    while index < bytes.len() {
        if bytes[index] == QUOTE {
            if bytes.get(index + 1) == Some(&QUOTE) {
                index += 2;
                continue;
            }
            return index + 1;
        }
        index += 1;
    }
    index
}

fn starts_terminator(bytes: &[u8], index: usize) -> bool {
    match bytes.get(index) {
        Some(&b'\n') => true,
        Some(&b'\r') => bytes.get(index + 1) == Some(&b'\n'),
        _ => false,
    }
}

fn skip_terminator(bytes: &[u8], index: usize) -> usize {
    match bytes.get(index) {
        Some(&b'\n') => index + 1,
        Some(&b'\r') if bytes.get(index + 1) == Some(&b'\n') => index + 2,
        _ => index,
    }
}

fn field_text(text: &str, start: usize, end: usize) -> String {
    let raw = &text[start..end];
    match raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        Some(value) => value.replace("\"\"", "\""),
        None => raw.to_owned(),
    }
}
