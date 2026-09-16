#![forbid(unsafe_code)]

use hm_docs::diff::{ChangedRegion, changed_regions};
use hm_docs::identity::{OccurrenceCounter, chunk_content_hash, chunk_identity, document_identity};

const BASE: &str = "Alpha one.\n\nBeta two.\n\nGamma three.\n\nDelta four.\n\nEpsilon five.\n";
const SINGLE_EDIT: &str =
    "Alpha one.\n\nBeta two.\n\nGamma three and a half.\n\nDelta four.\n\nEpsilon five.\n";
const TWO_EDITS: &str = "Alpha uno.\n\nBeta two.\n\nGamma three.\n\nDelta four.\n\nEpsilon six.\n";

fn repetitive(paragraphs: usize, changed: Option<usize>) -> String {
    let mut text = String::new();
    for index in 0..paragraphs {
        if changed == Some(index) {
            text.push_str("Same paragraph, edited once.\n\n");
        } else {
            text.push_str("Same paragraph.\n\n");
        }
    }
    text
}

fn assert_outside_is_identical(old_text: &str, new_text: &str, regions: &[ChangedRegion]) {
    let mut old_cursor = 0;
    let mut new_cursor = 0;
    for region in regions {
        assert!(
            region.old.start >= old_cursor,
            "old regions must not overlap"
        );
        assert!(
            region.new.start >= new_cursor,
            "new regions must not overlap"
        );
        assert!(region.old.end >= region.old.start);
        assert!(region.new.end >= region.new.start);
        assert_eq!(
            &old_text[old_cursor..region.old.start],
            &new_text[new_cursor..region.new.start],
            "text before a region must be byte-identical"
        );
        old_cursor = region.old.end;
        new_cursor = region.new.end;
    }
    assert_eq!(
        &old_text[old_cursor..],
        &new_text[new_cursor..],
        "text after the last region must be byte-identical"
    );
}

#[test]
fn chunk_ids_are_content_derived_and_occurrence_scoped() {
    let document = document_identity("notes.txt", BASE.as_bytes());
    let content = chunk_content_hash("Gamma three.\n\n");
    assert_eq!(content, chunk_content_hash("Gamma three.\n\n"));

    let altered = chunk_content_hash("Gamma threa.\n\n");
    assert_ne!(content, altered);

    let identity = chunk_identity(&document, &content, 0);
    assert_eq!(identity, chunk_identity(&document, &content, 0));
    assert_ne!(identity, chunk_identity(&document, &content, 1));
    assert_ne!(identity, chunk_identity(&document, &altered, 0));

    let other_document = document_identity("other.txt", BASE.as_bytes());
    assert_ne!(identity, chunk_identity(&other_document, &content, 0));
}

#[test]
fn document_identity_separates_name_from_content() {
    let first = document_identity("notes.txt", BASE.as_bytes());
    assert_eq!(first, document_identity("notes.txt", BASE.as_bytes()));
    assert_ne!(first, document_identity("notes.md", BASE.as_bytes()));
    assert_ne!(
        first,
        document_identity("notes.txt", SINGLE_EDIT.as_bytes())
    );
}

#[test]
fn duplicate_paragraphs_get_distinct_ids() {
    let document = document_identity("repeats.txt", b"Same paragraph.\n\nSame paragraph.\n");
    let content = chunk_content_hash("Same paragraph.\n\n");
    let mut counter = OccurrenceCounter::new();

    let first = counter.next(&content);
    let second = counter.next(&content);
    assert_eq!(first, 0);
    assert_eq!(second, 1);
    assert_ne!(
        chunk_identity(&document, &content, first),
        chunk_identity(&document, &content, second)
    );

    let other = chunk_content_hash("Another paragraph.\n\n");
    assert_eq!(counter.next(&other), 0);
}

#[test]
fn reserved_occurrences_are_skipped() {
    let content = chunk_content_hash("Same paragraph.\n\n");
    let mut counter = OccurrenceCounter::new();
    counter.reserve(&content, 0);
    assert_eq!(counter.next(&content), 1);

    let mut skipping = OccurrenceCounter::new();
    skipping.reserve(&content, 0);
    skipping.reserve(&content, 2);
    let handed = [
        skipping.next(&content),
        skipping.next(&content),
        skipping.next(&content),
    ];
    assert_eq!(handed, [1, 3, 4]);
}

#[test]
fn unchanged_text_yields_no_regions() {
    assert!(changed_regions(BASE, BASE).is_empty());
    assert!(changed_regions("", "").is_empty());
}

#[test]
fn single_edit_yields_one_minimal_region() {
    let regions = changed_regions(BASE, SINGLE_EDIT);
    assert_eq!(regions.len(), 1);
    let region = &regions[0];

    assert_eq!(&BASE[..region.old.start], &SINGLE_EDIT[..region.new.start]);
    assert_eq!(&BASE[region.old.end..], &SINGLE_EDIT[region.new.end..]);
    assert!(SINGLE_EDIT[region.new.clone()].contains("and a half"));
    assert_outside_is_identical(BASE, SINGLE_EDIT, &regions);
}

#[test]
fn separated_edits_yield_disjoint_regions() {
    let regions = changed_regions(BASE, TWO_EDITS);
    assert_eq!(regions.len(), 2);

    assert!(regions[0].old.start < regions[1].old.start);
    assert!(regions[0].new.start < regions[1].new.start);
    assert!(regions[0].old.end <= regions[1].old.start);
    assert!(regions[0].new.end <= regions[1].new.start);
    assert_eq!(
        &BASE[regions[0].old.end..regions[1].old.start],
        &TWO_EDITS[regions[0].new.end..regions[1].new.start]
    );
    assert!(BASE[regions[0].old.end..regions[1].old.start].contains("Gamma three."));
    assert_outside_is_identical(BASE, TWO_EDITS, &regions);
}

#[test]
fn repetitive_document_still_satisfies_the_region_contract() {
    let old_text = repetitive(10, None);
    let new_text = repetitive(10, Some(4));
    let regions = changed_regions(&old_text, &new_text);
    assert!(!regions.is_empty());
    assert_outside_is_identical(&old_text, &new_text, &regions);

    let shortened = repetitive(9, None);
    let removals = changed_regions(&old_text, &shortened);
    assert!(!removals.is_empty());
    assert_outside_is_identical(&old_text, &shortened, &removals);
}
