#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_docs::formats::table::{TABLE_EXTRACTION_VERSION, TableLoader};
use hm_docs::formats::text::{TEXT_EXTRACTION_VERSION, TextLoader};
use hm_docs::loader::{
    DocumentLoader, Extraction, LoaderId, LoaderRegistry, MAXIMUM_DOCUMENT_BYTES, TableLayout,
};

fn layout(extraction: &Extraction) -> &TableLayout {
    extraction.table.as_ref().expect("tabular layout")
}

#[test]
fn registry_selects_loader_by_media_type_and_refuses_unknown_formats() {
    let registry = LoaderRegistry::standard();
    assert_eq!(
        registry.select("text/markdown", "notes.md"),
        Ok(LoaderId::Text)
    );
    assert_eq!(registry.select("text/plain", "a.txt"), Ok(LoaderId::Text));
    assert_eq!(registry.select("text/csv", "rows.csv"), Ok(LoaderId::Table));
    assert_eq!(
        registry.select("text/tab-separated-values", "rows.tsv"),
        Ok(LoaderId::Table)
    );
    let refusal = registry
        .select("application/zip", "a.zip")
        .expect_err("unsupported media type");
    assert_eq!(refusal.code, ErrorCode::OperationUnavailable);
    assert_eq!(LoaderId::Text.as_str(), "text");
    assert_eq!(LoaderId::Table.as_str(), "table");
}

#[test]
fn text_extraction_preserves_bytes_and_refuses_invalid_utf8() {
    let source = "# Title\n\nBody with punctuation, a tab\tand a rune \u{00fc}.\n";
    let extraction = TextLoader
        .extract(source.as_bytes())
        .expect("text extraction");
    assert_eq!(extraction.loader, LoaderId::Text);
    assert_eq!(extraction.extraction_version, TEXT_EXTRACTION_VERSION);
    assert_eq!(TextLoader.loader_id(), LoaderId::Text);
    assert_eq!(TextLoader.extraction_version(), TEXT_EXTRACTION_VERSION);
    assert_eq!(extraction.text.as_bytes(), source.as_bytes());
    assert_eq!(extraction.page_spans.len(), 1);
    assert_eq!(extraction.page_spans[0].page_number, 1);
    assert_eq!(extraction.page_spans[0].byte_start, 0);
    assert_eq!(extraction.page_spans[0].byte_end, extraction.text.len());
    assert_eq!(extraction.table, None);
    assert_eq!(extraction.partial, None);

    let registry = LoaderRegistry::standard();
    let through_registry = registry
        .extract(LoaderId::Text, source.as_bytes())
        .expect("text extraction through the registry");
    assert_eq!(through_registry, extraction);

    let malformed = [b'#', b' ', 0xFF, b'\n'];
    let rejected = TextLoader
        .extract(&malformed)
        .expect_err("invalid utf-8 is refused");
    assert_eq!(rejected.code, ErrorCode::SchemaInvalid);

    let oversize = vec![b' '; MAXIMUM_DOCUMENT_BYTES + 1];
    let refused = TextLoader
        .extract(&oversize)
        .expect_err("oversize input is refused");
    assert_eq!(refused.code, ErrorCode::CapacityExceeded);
}

#[test]
fn table_extraction_records_header_and_byte_exact_spans() {
    let source = "name,age,city\nAlice,30,NYC\n\"Lee, B\",41,\"Rio\"\n";
    let extraction = TableLoader
        .extract(source.as_bytes())
        .expect("table extraction");
    assert_eq!(extraction.loader, LoaderId::Table);
    assert_eq!(extraction.extraction_version, TABLE_EXTRACTION_VERSION);
    assert_eq!(TableLoader.loader_id(), LoaderId::Table);
    assert_eq!(TableLoader.extraction_version(), TABLE_EXTRACTION_VERSION);
    assert_eq!(extraction.text, source);
    assert_eq!(extraction.text.as_bytes(), source.as_bytes());
    assert_eq!(extraction.partial, None);

    let table = layout(&extraction);
    assert_eq!(table.header, vec!["name", "age", "city"]);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].row_index, 1);
    assert_eq!(table.rows[1].row_index, 2);
    let first = &table.rows[0];
    assert_eq!(
        &extraction.text[first.byte_start..first.byte_end],
        "Alice,30,NYC"
    );
    assert_eq!(
        first
            .fields
            .iter()
            .map(|field| &extraction.text[field.byte_start..field.byte_end])
            .collect::<Vec<_>>(),
        vec!["Alice", "30", "NYC"]
    );
    let second = &table.rows[1];
    assert_eq!(
        &extraction.text[second.byte_start..second.byte_end],
        "\"Lee, B\",41,\"Rio\""
    );
    assert_eq!(second.fields.len(), 3);
    assert_eq!(second.fields[0].column_index, 0);
    assert_eq!(
        &extraction.text[second.fields[0].byte_start..second.fields[0].byte_end],
        "\"Lee, B\""
    );
    assert_eq!(second.fields[2].column_index, 2);
    assert_eq!(
        &extraction.text[second.fields[2].byte_start..second.fields[2].byte_end],
        "\"Rio\""
    );

    let quoted = "\"a\"\"b\",c\n\"x\"\"y\",\"line one\nline two\"\n";
    let extraction = TableLoader
        .extract(quoted.as_bytes())
        .expect("quoted table extraction");
    assert_eq!(extraction.text, quoted);
    let table = layout(&extraction);
    assert_eq!(table.header, vec!["a\"b", "c"]);
    assert_eq!(table.rows.len(), 1);
    let row = &table.rows[0];
    assert_eq!(row.row_index, 1);
    assert_eq!(
        &extraction.text[row.byte_start..row.byte_end],
        "\"x\"\"y\",\"line one\nline two\""
    );
    assert_eq!(row.fields.len(), 2);
    assert_eq!(
        &extraction.text[row.fields[0].byte_start..row.fields[0].byte_end],
        "\"x\"\"y\""
    );
    assert_eq!(
        &extraction.text[row.fields[1].byte_start..row.fields[1].byte_end],
        "\"line one\nline two\""
    );
}

#[test]
fn table_extraction_handles_crlf_and_tab_delimited_records() {
    let source = "name\tage\tcity\r\nAlice\t30\tNYC\r\n\"Lee\tB\"\t41\t\"Rio\"\r\n";
    let registry = LoaderRegistry::standard();
    let loader = registry
        .select("text/tab-separated-values", "rows.tsv")
        .expect("tabular loader");
    let extraction = registry
        .extract(loader, source.as_bytes())
        .expect("table extraction");
    assert_eq!(extraction.text, source);
    assert_eq!(extraction.text.as_bytes(), source.as_bytes());
    assert_eq!(extraction.partial, None);

    let table = layout(&extraction);
    assert_eq!(table.header, vec!["name", "age", "city"]);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].row_index, 1);
    assert_eq!(
        &extraction.text[table.rows[0].byte_start..table.rows[0].byte_end],
        "Alice\t30\tNYC"
    );
    let second = &table.rows[1];
    assert_eq!(second.row_index, 2);
    assert_eq!(
        &extraction.text[second.byte_start..second.byte_end],
        "\"Lee\tB\"\t41\t\"Rio\""
    );
    assert_eq!(
        &extraction.text[second.fields[0].byte_start..second.fields[0].byte_end],
        "\"Lee\tB\""
    );
    assert_eq!(
        &extraction.text[second.fields[1].byte_start..second.fields[1].byte_end],
        "41"
    );

    let trailing = "a,b\r\n1,2";
    let extraction = TableLoader
        .extract(trailing.as_bytes())
        .expect("table extraction without a trailing terminator");
    let table = layout(&extraction);
    assert_eq!(table.header, vec!["a", "b"]);
    assert_eq!(table.rows.len(), 1);
    assert_eq!(
        &extraction.text[table.rows[0].byte_start..table.rows[0].byte_end],
        "1,2"
    );
}
