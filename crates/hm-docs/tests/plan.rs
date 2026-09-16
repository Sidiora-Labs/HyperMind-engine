#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_docs::chunk::{ChunkSpan, WordEstimator, chunk_paragraphs};
use hm_docs::identity::{chunk_content_hash, document_identity};
use hm_docs::plan::{
    ChangeKind, ChangePlan, PlannedChunk, StoredChunk, plan_initial, plan_revision, validate_plan,
};

const BUDGET: u32 = 4;

const BASE: &str = "Alpha one two.\n\nBeta three four.\n\nGamma five six.\n\nDelta seven eight.\n\nEpsilon nine ten.\n";
const EDITED: &str = "Alpha one two.\n\nBeta three four.\n\nGamma five sixty.\n\nDelta seven eight.\n\nEpsilon nine ten.\n";
const INSERTED: &str = "Alpha one two.\n\nBeta three four.\n\nGamma five six.\n\nDelta seven eight.\n\nOmega ten eleven.\n\nEpsilon nine ten.\n";

const REPEATED: &str = "Alpha one two.\n\nBeta three four.\n\nRepeat this line.\n\nGamma five six.\n\nRepeat this line.\n\nEcho six seven.\n\nOmega nine ten.\n";
const REPEATED_EDITED: &str = "Alpha one two.\n\nBeta three five.\n\nRepeat this line.\n\nGamma five seven.\n\nRepeat this line.\n\nEcho six eight.\n\nOmega nine ten.\n";

const DUPLICATED: &str =
    "Alpha one two.\n\nRepeat this line.\n\nRepeat this line.\n\nOmega nine ten.\n";

fn document() -> [u8; 32] {
    document_identity("notes.txt", BASE.as_bytes())
}

fn spans(text: &str) -> Vec<ChunkSpan> {
    chunk_paragraphs(text, BUDGET, &WordEstimator, &[]).expect("paragraph chunking")
}

fn initial(text: &str) -> ChangePlan {
    plan_initial(document(), text, &spans(text)).expect("initial plan")
}

fn stored_chunks(text: &str) -> Vec<StoredChunk> {
    initial(text)
        .chunks
        .iter()
        .map(|chunk| StoredChunk {
            chunk_id: chunk.chunk_id,
            content_hash: chunk.content_hash,
            occurrence: chunk.occurrence,
            ordinal: chunk.ordinal,
            byte_start: chunk.byte_start,
            byte_end: chunk.byte_end,
            cut: chunk.cut,
            page_number: chunk.page_number,
            row_index: chunk.row_index,
            column_start: chunk.column_start,
            column_end: chunk.column_end,
            token_estimate: chunk.token_estimate,
        })
        .collect()
}

fn revise(old_text: &str, stored: &[StoredChunk], new_text: &str) -> ChangePlan {
    plan_revision(document(), old_text, stored, new_text, &|text| {
        chunk_paragraphs(text, BUDGET, &WordEstimator, &[])
    })
    .expect("revision plan")
}

fn total(plan: &ChangePlan) -> u32 {
    plan.retained + plan.moved + plan.replaced + plan.added
}

fn text_of<'a>(document_text: &'a str, chunk: &PlannedChunk) -> &'a str {
    &document_text[chunk.byte_start..chunk.byte_end]
}

#[test]
fn initial_plan_marks_every_chunk_added() {
    let plan = initial(BASE);
    assert_eq!(plan.chunks.len(), 5);
    assert_eq!(plan.regions, 0);
    assert_eq!(plan.added, 5);
    assert_eq!(total(&plan), 5);

    for (position, chunk) in plan.chunks.iter().enumerate() {
        assert_eq!(chunk.change, ChangeKind::Added);
        assert_eq!(
            chunk.ordinal,
            u32::try_from(position).expect("ordinal fits")
        );
        assert_eq!(chunk.occurrence, 0);
        assert_eq!(chunk.content_hash, chunk_content_hash(text_of(BASE, chunk)));
    }
    validate_plan(&plan, &[], BASE).expect("an initial plan reconstructs the document");
}

#[test]
fn unchanged_document_retains_every_chunk() {
    let stored = stored_chunks(BASE);
    let plan = revise(BASE, &stored, BASE);

    assert_eq!(plan.regions, 0);
    assert_eq!(plan.retained, 5);
    assert_eq!(
        plan.retained,
        u32::try_from(plan.chunks.len()).expect("count fits")
    );
    for (position, chunk) in plan.chunks.iter().enumerate() {
        assert_eq!(chunk.change, ChangeKind::Retained);
        assert_eq!(chunk.chunk_id, stored[position].chunk_id);
        assert_eq!(chunk.ordinal, stored[position].ordinal);
    }
    validate_plan(&plan, &stored, BASE).expect("an unchanged document reconstructs");
}

#[test]
fn local_edit_replaces_only_the_overlapping_run() {
    let stored = stored_chunks(BASE);
    let plan = revise(BASE, &stored, EDITED);

    assert_eq!(plan.regions, 1);
    assert_eq!(plan.chunks.len(), 5);
    assert_eq!(total(&plan), 5);
    assert_eq!(plan.retained, 4);

    for position in [0, 1, 3, 4] {
        let chunk = &plan.chunks[position];
        assert_eq!(chunk.change, ChangeKind::Retained);
        assert_eq!(chunk.chunk_id, stored[position].chunk_id);
    }
    let rewritten = &plan.chunks[2];
    assert_ne!(rewritten.change, ChangeKind::Retained);
    assert_ne!(rewritten.chunk_id, stored[2].chunk_id);
    assert_eq!(text_of(EDITED, rewritten), "Gamma five sixty.\n\n");
    validate_plan(&plan, &stored, EDITED).expect("a local edit reconstructs the document");
}

#[test]
fn insertion_moves_trailing_chunks_without_changing_ids() {
    let stored = stored_chunks(BASE);
    let plan = revise(BASE, &stored, INSERTED);

    assert_eq!(plan.chunks.len(), 6);
    assert_eq!(total(&plan), 6);
    assert_eq!(plan.retained, 4);
    assert_eq!(plan.added, 1);
    assert_eq!(plan.moved, 1);

    for (chunk, kept) in plan.chunks.iter().zip(stored.iter()).take(4) {
        assert_eq!(chunk.change, ChangeKind::Retained);
        assert_eq!(chunk.chunk_id, kept.chunk_id);
    }
    assert_eq!(plan.chunks[4].change, ChangeKind::Added);
    assert_eq!(text_of(INSERTED, &plan.chunks[4]), "Omega ten eleven.\n\n");

    let trailing = &plan.chunks[5];
    assert_eq!(trailing.change, ChangeKind::Moved);
    assert_eq!(trailing.chunk_id, stored[4].chunk_id);
    assert!(trailing.ordinal > stored[4].ordinal);
    validate_plan(&plan, &stored, INSERTED).expect("an insertion reconstructs the document");
}

#[test]
fn identical_replacement_keeps_its_chunk_id() {
    let stored = stored_chunks(REPEATED);
    let plan = revise(REPEATED, &stored, REPEATED_EDITED);

    assert_eq!(plan.regions, 1);
    assert_eq!(plan.chunks.len(), 7);
    assert_eq!(total(&plan), 7);
    assert_eq!(plan.replaced, 2);

    let surviving: Vec<&PlannedChunk> = plan
        .chunks
        .iter()
        .filter(|chunk| text_of(REPEATED_EDITED, chunk) == "Repeat this line.\n\n")
        .collect();
    assert_eq!(surviving.len(), 2);
    assert_eq!(surviving[0].change, ChangeKind::Replaced);
    assert_eq!(surviving[1].change, ChangeKind::Replaced);
    assert_eq!(surviving[0].chunk_id, stored[2].chunk_id);
    assert_eq!(surviving[1].chunk_id, stored[4].chunk_id);
    assert_ne!(surviving[0].chunk_id, surviving[1].chunk_id);
    validate_plan(&plan, &stored, REPEATED_EDITED)
        .expect("a re-cut region reconstructs the document");
}

#[test]
fn duplicate_paragraphs_do_not_collide() {
    let plan = initial(DUPLICATED);
    assert_eq!(plan.chunks.len(), 4);

    let repeated: Vec<&PlannedChunk> = plan
        .chunks
        .iter()
        .filter(|chunk| text_of(DUPLICATED, chunk) == "Repeat this line.\n\n")
        .collect();
    assert_eq!(repeated.len(), 2);
    assert_eq!(repeated[0].content_hash, repeated[1].content_hash);
    assert_eq!(repeated[0].occurrence, 0);
    assert_eq!(repeated[1].occurrence, 1);
    assert_ne!(repeated[0].chunk_id, repeated[1].chunk_id);

    for chunk in &plan.chunks {
        let claims = plan
            .chunks
            .iter()
            .filter(|other| other.chunk_id == chunk.chunk_id)
            .count();
        assert_eq!(claims, 1, "every chunk id appears exactly once");
    }
    validate_plan(&plan, &[], DUPLICATED).expect("duplicated paragraphs reconstruct");
}

#[test]
fn corrupt_plans_fail_the_reconstruction_gate() {
    let plan = initial(BASE);
    validate_plan(&plan, &[], BASE).expect("the uncorrupted plan is accepted");

    let mut duplicated_ordinal = plan.clone();
    duplicated_ordinal.chunks[1].ordinal = 0;
    assert_eq!(
        validate_plan(&duplicated_ordinal, &[], BASE)
            .expect_err("a duplicated ordinal is refused")
            .code,
        ErrorCode::InvariantViolation
    );

    let mut missing_ordinal = plan.clone();
    missing_ordinal.chunks[3].ordinal = u32::try_from(plan.chunks.len()).expect("count fits");
    assert_eq!(
        validate_plan(&missing_ordinal, &[], BASE)
            .expect_err("a missing ordinal is refused")
            .code,
        ErrorCode::InvariantViolation
    );

    let mut holed_ranges = plan.clone();
    holed_ranges.chunks[2].byte_start += 1;
    assert_eq!(
        validate_plan(&holed_ranges, &[], BASE)
            .expect_err("a hole in the byte ranges is refused")
            .code,
        ErrorCode::InvariantViolation
    );

    let mut altered_text = plan;
    altered_text.chunks[2].content_hash = chunk_content_hash("Gamma five seven.\n\n");
    assert_eq!(
        validate_plan(&altered_text, &[], BASE)
            .expect_err("text that is not in the document is refused")
            .code,
        ErrorCode::InvariantViolation
    );
}

#[test]
fn non_tiling_stored_chunks_are_refused() {
    let mut stored = stored_chunks(BASE);
    stored[2].byte_start += 1;

    let refusal = plan_revision(document(), BASE, &stored, EDITED, &|text| {
        chunk_paragraphs(text, BUDGET, &WordEstimator, &[])
    })
    .expect_err("stored chunks that do not tile the document are refused");
    assert_eq!(refusal.code, ErrorCode::InvariantViolation);
}
