#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_docs::formats::mail::{MAIL_EXTRACTION_VERSION, MailLoader};
use hm_docs::formats::pdf::{PDF_EXTRACTION_VERSION, PdfLoader};
use hm_docs::loader::{DocumentLoader, LoaderId, LoaderRegistry};

const TWO_PAGES: &[u8] = include_bytes!("fixtures/two-pages.pdf");
const DAMAGED_PAGE: &[u8] = include_bytes!("fixtures/damaged-page.pdf");
const PLAIN_MESSAGE: &[u8] = include_bytes!("fixtures/plain-message.eml");
const MESSAGE_WITH_ATTACHMENT: &[u8] = include_bytes!("fixtures/message-with-attachment.eml");

const UNREADABLE_SINGLE_PAGE_PDF: &str = concat!(
    "%PDF-1.4\n",
    "1 0 obj\n",
    "<< /Type /Catalog /Pages 2 0 R >>\n",
    "endobj\n",
    "2 0 obj\n",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n",
    "endobj\n",
    "3 0 obj\n",
    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\n",
    "endobj\n",
    "xref\n",
    "0 5\n",
    "0000000000 65535 f \n",
    "0000000009 00000 n \n",
    "0000000058 00000 n \n",
    "0000000115 00000 n \n",
    "0000000000 00001 f \n",
    "trailer\n",
    "<< /Size 5 /Root 1 0 R >>\n",
    "startxref\n",
    "202\n",
    "%%EOF\n",
);

#[test]
fn registry_selects_pdf_and_mail_loaders() {
    let registry = LoaderRegistry::standard();
    assert_eq!(
        registry.select("application/pdf", "report.pdf"),
        Ok(LoaderId::Pdf)
    );
    assert_eq!(
        registry.select("message/rfc822", "note.eml"),
        Ok(LoaderId::Mail)
    );
    assert_eq!(LoaderId::Pdf.as_str(), "pdf");
    assert_eq!(LoaderId::Mail.as_str(), "mail");
    assert_eq!(PdfLoader.loader_id(), LoaderId::Pdf);
    assert_eq!(PdfLoader.extraction_version(), PDF_EXTRACTION_VERSION);
    assert_eq!(MailLoader.loader_id(), LoaderId::Mail);
    assert_eq!(MailLoader.extraction_version(), MAIL_EXTRACTION_VERSION);
}

#[test]
fn pdf_extraction_records_one_page_span_per_page() {
    let registry = LoaderRegistry::standard();
    let extraction = registry
        .extract(LoaderId::Pdf, TWO_PAGES)
        .expect("two page extraction");
    assert_eq!(extraction.loader, LoaderId::Pdf);
    assert_eq!(extraction.extraction_version, PDF_EXTRACTION_VERSION);
    assert_eq!(extraction.partial, None);
    assert_eq!(extraction.table, None);
    assert_eq!(extraction.page_spans.len(), 2);
    assert_eq!(extraction.page_spans[0].page_number, 1);
    assert_eq!(extraction.page_spans[1].page_number, 2);
    assert_eq!(extraction.page_spans[0].byte_start, 0);
    assert!(extraction.page_spans[0].byte_end <= extraction.page_spans[1].byte_start);
    assert!(extraction.page_spans[1].byte_end <= extraction.text.len());
    assert!(
        extraction.text[extraction.page_spans[0].byte_start..extraction.page_spans[0].byte_end]
            .contains("Harbour"),
        "first page span carries the first page marker: {:?}",
        extraction.text
    );
    assert!(
        extraction.text[extraction.page_spans[1].byte_start..extraction.page_spans[1].byte_end]
            .contains("Appendix"),
        "second page span carries the second page marker: {:?}",
        extraction.text
    );
}

#[test]
fn pdf_partial_extraction_names_the_failed_page() {
    let extraction = PdfLoader
        .extract(DAMAGED_PAGE)
        .expect("a damaged page does not cost the whole document");
    let partial = extraction.partial.as_ref().expect("partial extraction");
    assert_eq!(partial.failed_units, vec![2]);
    assert!(!partial.reason.is_empty());
    assert_eq!(extraction.page_spans.len(), 1);
    assert_eq!(extraction.page_spans[0].page_number, 1);
    assert!(extraction.text.contains("Harbour"));
    assert!(!extraction.text.contains("Appendix"));
}

#[test]
fn pdf_total_failure_is_an_error() {
    let unreadable = PdfLoader
        .extract(UNREADABLE_SINGLE_PAGE_PDF.as_bytes())
        .expect_err("a document with no extractable page is refused");
    assert_eq!(unreadable.code, ErrorCode::SchemaInvalid);

    let not_a_pdf = PdfLoader
        .extract(b"this is not a pdf document at all\n")
        .expect_err("bytes that are not a pdf are refused");
    assert_eq!(not_a_pdf.code, ErrorCode::SchemaInvalid);
}

#[test]
fn mail_extraction_renders_a_deterministic_header_block() {
    let registry = LoaderRegistry::standard();
    let loader = registry
        .select("message/rfc822", "note.eml")
        .expect("mail loader");
    let extraction = registry
        .extract(loader, PLAIN_MESSAGE)
        .expect("plain message extraction");
    assert_eq!(extraction.loader, LoaderId::Mail);
    assert_eq!(extraction.extraction_version, MAIL_EXTRACTION_VERSION);
    assert_eq!(extraction.partial, None);
    assert_eq!(extraction.table, None);
    assert_eq!(extraction.page_spans.len(), 1);
    assert_eq!(extraction.page_spans[0].page_number, 1);
    assert_eq!(extraction.page_spans[0].byte_start, 0);
    assert_eq!(extraction.page_spans[0].byte_end, extraction.text.len());

    let mut lines = extraction.text.split('\n');
    assert_eq!(
        lines.next(),
        Some("From: Ada Marlow <ada.marlow@harbour.example>")
    );
    assert_eq!(
        lines.next(),
        Some("To: Ines Okafor <ines.okafor@harbour.example>")
    );
    assert_eq!(lines.next(), Some("Subject: Quarterly harbour manifest"));
    assert_eq!(lines.next(), Some("Date: 2024-03-05T09:15:00Z"));
    assert_eq!(lines.next(), Some(""));
    assert_eq!(
        lines.next(),
        Some("Berth seventeen is reserved for the survey vessel.")
    );
    assert!(
        extraction
            .text
            .contains("The manifest closes on the last working day of the quarter.")
    );
}

#[test]
fn mail_attachments_are_reported_as_partial() {
    let extraction = MailLoader
        .extract(MESSAGE_WITH_ATTACHMENT)
        .expect("multipart message extraction");
    let partial = extraction.partial.as_ref().expect("partial extraction");
    assert_eq!(partial.failed_units, vec![0]);
    assert!(
        partial.reason.contains("attachments"),
        "the reason names attachments: {:?}",
        partial.reason
    );
    assert!(
        partial.reason.contains("berths.csv"),
        "the reason names the attachment: {:?}",
        partial.reason
    );
    assert!(
        extraction
            .text
            .contains("The berth allocations are in the attached table."),
        "the text body is still extracted: {:?}",
        extraction.text
    );
    assert!(
        !extraction.text.contains("Tidewater Lark"),
        "attachment contents are not extracted: {:?}",
        extraction.text
    );
    assert_eq!(extraction.page_spans.len(), 1);
    assert_eq!(extraction.page_spans[0].byte_end, extraction.text.len());
}
