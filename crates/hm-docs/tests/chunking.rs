#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_docs::chunk::{
    ChunkCut, ChunkSpan, MAXIMUM_TOKEN_BUDGET, TokenEstimator, WordEstimator, chunk_paragraphs,
    chunk_table, paragraph_units, tiles,
};
use hm_docs::formats::table::TableLoader;
use hm_docs::loader::{DocumentLoader, PageSpan, TableLayout};

const PARAGRAPHS: &str = "Alpha one two.\n\nBeta three four.\n\nGamma five six. Delta seven eight nine ten eleven.\n\nEpsilon zw\u{00f6}lf dreizehn.\n";

const RECORDS: &str = "name,role,note\nada,engineer,short\ngrace,admiral,a very long note with many words here indeed\n";

fn rejoined(text: &str, spans: &[ChunkSpan]) -> String {
    let mut joined = String::new();
    for span in spans {
        joined.push_str(&text[span.byte_start..span.byte_end]);
    }
    joined
}

fn table_layout(text: &str) -> TableLayout {
    TableLoader
        .extract(text.as_bytes())
        .expect("table extraction")
        .table
        .expect("tabular layout")
}

#[test]
fn paragraph_spans_tile_the_text_and_preserve_bytes() {
    let units = paragraph_units(PARAGRAPHS);
    assert_eq!(units.len(), 4);
    assert_eq!(units[0].start, 0);
    assert_eq!(units[3].end, PARAGRAPHS.len());

    for budget in [1, 2, 3, 6, 64, MAXIMUM_TOKEN_BUDGET] {
        let spans = chunk_paragraphs(PARAGRAPHS, budget, &WordEstimator, &[])
            .expect("paragraph chunking at a valid budget");
        assert!(
            tiles(&spans, PARAGRAPHS.len()),
            "spans must tile the text at budget {budget}"
        );
        assert_eq!(spans[0].byte_start, 0);
        assert_eq!(spans[spans.len() - 1].byte_end, PARAGRAPHS.len());
        for pair in spans.windows(2) {
            assert_eq!(pair[1].byte_start, pair[0].byte_end);
        }
        for span in &spans {
            assert!(PARAGRAPHS.is_char_boundary(span.byte_start));
            assert!(PARAGRAPHS.is_char_boundary(span.byte_end));
            assert_eq!(span.row_index, 0);
            assert_eq!(span.column_start, 0);
            assert_eq!(span.column_end, 0);
        }
        assert_eq!(rejoined(PARAGRAPHS, &spans), PARAGRAPHS);
    }

    let whole = chunk_paragraphs(PARAGRAPHS, MAXIMUM_TOKEN_BUDGET, &WordEstimator, &[])
        .expect("paragraph chunking at the maximum budget");
    assert_eq!(whole.len(), 1);
    assert_eq!(whole[0].cut, ChunkCut::ParagraphEnd);
    assert_eq!(whole[0].token_estimate, WordEstimator.estimate(PARAGRAPHS));
}

#[test]
fn paragraphs_batch_until_the_budget_and_cut_at_sentences() {
    let spans = chunk_paragraphs(PARAGRAPHS, 6, &WordEstimator, &[]).expect("paragraph chunking");
    assert!(tiles(&spans, PARAGRAPHS.len()));
    assert_eq!(spans.len(), 4);

    let gamma = PARAGRAPHS.find("Gamma").expect("third paragraph");
    let epsilon = PARAGRAPHS.find("Epsilon").expect("fourth paragraph");

    assert_eq!(spans[0].byte_start, 0);
    assert_eq!(spans[0].byte_end, gamma);
    assert_eq!(spans[0].cut, ChunkCut::ParagraphEnd);
    assert_eq!(spans[0].token_estimate, 6);

    assert_eq!(spans[1].byte_start, gamma);
    assert_eq!(spans[1].cut, ChunkCut::ParagraphCut);
    assert_eq!(
        &PARAGRAPHS[spans[1].byte_start..spans[1].byte_end],
        "Gamma five six. "
    );

    assert_eq!(spans[2].byte_end, epsilon);
    assert_eq!(spans[2].cut, ChunkCut::ParagraphEnd);

    assert_eq!(spans[3].byte_start, epsilon);
    assert_eq!(spans[3].byte_end, PARAGRAPHS.len());
    assert_eq!(spans[3].cut, ChunkCut::ParagraphEnd);

    let cut_small = chunk_paragraphs(PARAGRAPHS, 2, &WordEstimator, &[])
        .expect("paragraph chunking at a tight budget");
    assert!(tiles(&cut_small, PARAGRAPHS.len()));
    assert_eq!(rejoined(PARAGRAPHS, &cut_small), PARAGRAPHS);
    assert!(
        cut_small
            .iter()
            .any(|span| span.cut == ChunkCut::ParagraphCut)
    );
    assert_eq!(cut_small[cut_small.len() - 1].cut, ChunkCut::ParagraphEnd);
}

#[test]
fn page_numbers_come_from_the_containing_page_span() {
    let gamma = PARAGRAPHS.find("Gamma").expect("third paragraph");
    let pages = [
        PageSpan {
            page_number: 1,
            byte_start: 0,
            byte_end: gamma,
        },
        PageSpan {
            page_number: 2,
            byte_start: gamma,
            byte_end: PARAGRAPHS.len(),
        },
    ];
    let spans =
        chunk_paragraphs(PARAGRAPHS, 6, &WordEstimator, &pages).expect("paragraph chunking");
    assert_eq!(spans[0].page_number, 1);
    for span in &spans[1..] {
        assert!(span.byte_start >= gamma);
        assert_eq!(span.page_number, 2);
    }

    let unpaged = chunk_paragraphs(PARAGRAPHS, 6, &WordEstimator, &[]).expect("paragraph chunking");
    for span in &unpaged {
        assert_eq!(span.page_number, 0);
    }
}

#[test]
fn table_rows_stay_whole_and_oversized_rows_cut_on_field_boundaries() {
    let layout = table_layout(RECORDS);
    assert_eq!(layout.header.len(), 3);
    assert_eq!(layout.rows.len(), 2);

    let spans = chunk_table(RECORDS, &layout, 6, &WordEstimator).expect("table chunking");
    assert!(tiles(&spans, RECORDS.len()));
    assert_eq!(rejoined(RECORDS, &spans), RECORDS);
    assert_eq!(spans.len(), 4);

    assert_eq!(spans[0].row_index, 0);
    assert_eq!(spans[0].column_start, 0);
    assert_eq!(spans[0].column_end, 3);
    assert_eq!(spans[0].cut, ChunkCut::RowEnd);
    assert_eq!(
        &RECORDS[spans[0].byte_start..spans[0].byte_end],
        "name,role,note\n"
    );

    assert_eq!(spans[1].row_index, 1);
    assert_eq!(spans[1].column_start, 0);
    assert_eq!(spans[1].column_end, 3);
    assert_eq!(spans[1].cut, ChunkCut::RowEnd);
    assert_eq!(
        &RECORDS[spans[1].byte_start..spans[1].byte_end],
        "ada,engineer,short\n"
    );

    assert_eq!(spans[2].row_index, 2);
    assert_eq!(spans[2].column_start, 0);
    assert_eq!(spans[2].column_end, 2);
    assert_eq!(spans[2].cut, ChunkCut::RowCut);
    assert_eq!(
        &RECORDS[spans[2].byte_start..spans[2].byte_end],
        "grace,admiral,"
    );

    assert_eq!(spans[3].row_index, 2);
    assert_eq!(spans[3].column_start, 2);
    assert_eq!(spans[3].column_end, 3);
    assert_eq!(spans[3].cut, ChunkCut::RowEnd);
    assert_eq!(
        &RECORDS[spans[3].byte_start..spans[3].byte_end],
        "a very long note with many words here indeed\n"
    );

    let whole = chunk_table(RECORDS, &layout, MAXIMUM_TOKEN_BUDGET, &WordEstimator)
        .expect("table chunking at the maximum budget");
    assert!(tiles(&whole, RECORDS.len()));
    assert_eq!(whole.len(), 3);
    assert_eq!(whole[2].row_index, 2);
    assert_eq!(whole[2].column_start, 0);
    assert_eq!(whole[2].column_end, 3);
    assert_eq!(whole[2].cut, ChunkCut::RowEnd);
}

#[test]
fn rejects_zero_and_oversized_budgets_and_accepts_empty_text() {
    let layout = table_layout(RECORDS);
    for budget in [0, MAXIMUM_TOKEN_BUDGET + 1] {
        let refused = chunk_paragraphs(PARAGRAPHS, budget, &WordEstimator, &[])
            .expect_err("budget outside the permitted range");
        assert_eq!(refused.code, ErrorCode::InvalidArgument);
        let refused_table = chunk_table(RECORDS, &layout, budget, &WordEstimator)
            .expect_err("budget outside the permitted range");
        assert_eq!(refused_table.code, ErrorCode::InvalidArgument);
    }

    assert!(paragraph_units("").is_empty());
    let empty = chunk_paragraphs("", 8, &WordEstimator, &[]).expect("empty text");
    assert!(empty.is_empty());
    assert!(tiles(&empty, 0));
    let empty_table = chunk_table("", &layout, 8, &WordEstimator).expect("empty text");
    assert!(empty_table.is_empty());
}
