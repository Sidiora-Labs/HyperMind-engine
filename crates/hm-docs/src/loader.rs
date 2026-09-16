#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};

use crate::formats::mail::MailLoader;
use crate::formats::pdf::PdfLoader;
use crate::formats::table::TableLoader;
use crate::formats::text::TextLoader;

pub const MAXIMUM_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_EXTRACTED_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LoaderId {
    Text,
    Table,
    Pdf,
    Mail,
}

impl LoaderId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Table => "table",
            Self::Pdf => "pdf",
            Self::Mail => "mail",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageSpan {
    pub page_number: u32,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldSpan {
    pub column_index: u32,
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowSpan {
    pub row_index: u32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub fields: Vec<FieldSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableLayout {
    pub header: Vec<String>,
    pub rows: Vec<RowSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialExtraction {
    pub reason: String,
    pub failed_units: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Extraction {
    pub loader: LoaderId,
    pub extraction_version: u16,
    pub text: String,
    pub page_spans: Vec<PageSpan>,
    pub table: Option<TableLayout>,
    pub partial: Option<PartialExtraction>,
}

pub trait DocumentLoader {
    fn loader_id(&self) -> LoaderId;

    fn extraction_version(&self) -> u16;

    fn extract(&self, bytes: &[u8]) -> Result<Extraction, Error>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LoaderRegistry {
    text: TextLoader,
    table: TableLoader,
    pdf: PdfLoader,
    mail: MailLoader,
}

impl LoaderRegistry {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            text: TextLoader,
            table: TableLoader,
            pdf: PdfLoader,
            mail: MailLoader,
        }
    }

    pub fn select(&self, media_type: &str, name: &str) -> Result<LoaderId, Error> {
        let media = essential_media_type(media_type);
        let extension = extension_of(name);
        if is_table_source(&media, &extension) {
            return Ok(self.table.loader_id());
        }
        if is_pdf_source(&media, &extension) {
            return Ok(self.pdf.loader_id());
        }
        if is_mail_source(&media, &extension) {
            return Ok(self.mail.loader_id());
        }
        if is_text_source(&media, &extension) {
            return Ok(self.text.loader_id());
        }
        Err(Error::new(ErrorCode::OperationUnavailable))
    }

    pub fn extract(&self, loader: LoaderId, bytes: &[u8]) -> Result<Extraction, Error> {
        match loader {
            LoaderId::Text => self.text.extract(bytes),
            LoaderId::Table => self.table.extract(bytes),
            LoaderId::Pdf => self.pdf.extract(bytes),
            LoaderId::Mail => self.mail.extract(bytes),
        }
    }
}

fn essential_media_type(media_type: &str) -> String {
    media_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn extension_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => {
            extension.to_ascii_lowercase()
        }
        _ => String::new(),
    }
}

fn is_table_source(media: &str, extension: &str) -> bool {
    if matches!(media, "text/csv" | "text/tab-separated-values") {
        return true;
    }
    inherits_from_name(media) && matches!(extension, "csv" | "tsv" | "tab")
}

fn is_pdf_source(media: &str, extension: &str) -> bool {
    if media == "application/pdf" {
        return true;
    }
    inherits_from_name(media) && extension == "pdf"
}

fn is_mail_source(media: &str, extension: &str) -> bool {
    if media == "message/rfc822" {
        return true;
    }
    inherits_from_name(media) && extension == "eml"
}

fn is_text_source(media: &str, extension: &str) -> bool {
    if media.starts_with("text/") {
        return true;
    }
    inherits_from_name(media) && matches!(extension, "txt" | "text" | "md" | "markdown")
}

fn inherits_from_name(media: &str) -> bool {
    media.is_empty() || media == "application/octet-stream"
}

pub(crate) fn guard_input_size(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > MAXIMUM_DOCUMENT_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(())
}

pub(crate) fn guard_extracted_size(text: &str) -> Result<(), Error> {
    if text.len() > MAXIMUM_EXTRACTED_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(())
}

pub(crate) fn decode_text(bytes: &[u8]) -> Result<String, Error> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_owned()),
        Err(_) => Err(Error::new(ErrorCode::SchemaInvalid)),
    }
}

pub(crate) fn unit_index(index: usize) -> Result<u32, Error> {
    u32::try_from(index).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}
