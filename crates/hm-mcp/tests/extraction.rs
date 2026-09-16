use hm_mcp::extraction::{ExtractionLimits, ExtractionOutcomes, extract_bounded};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn bounded_extraction_records_every_item_and_restores_source_order() {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let closure_in_flight = Arc::clone(&in_flight);
    let closure_peak = Arc::clone(&peak);
    let outcomes = extract_bounded(
        (0..32usize).collect::<Vec<usize>>(),
        ExtractionLimits::new(4, false),
        move |index: usize| {
            let concurrent = closure_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            closure_peak.fetch_max(concurrent, Ordering::SeqCst);
            let outcome = if index % 5 == 3 {
                Err(index)
            } else {
                Ok(index * 2)
            };
            closure_in_flight.fetch_sub(1, Ordering::SeqCst);
            outcome
        },
    )
    .await;

    assert!(peak.load(Ordering::SeqCst) <= 4);
    assert_eq!(in_flight.load(Ordering::SeqCst), 0);
    assert_eq!(outcomes.extracted.len() + outcomes.failed.len(), 32);
    assert!(outcomes.skipped.is_empty());
    assert!(outcomes.aborted.is_empty());
    assert_eq!(outcomes.total(), 32);
    assert!(
        outcomes
            .extracted
            .windows(2)
            .all(|pair| pair[0].0 < pair[1].0)
    );
    assert!(outcomes.failed.windows(2).all(|pair| pair[0].0 < pair[1].0));
    for (index, value) in &outcomes.extracted {
        assert_ne!(index % 5, 3);
        assert_eq!(*value, index * 2);
    }
    for (index, error) in &outcomes.failed {
        assert_eq!(index % 5, 3);
        assert_eq!(error, index);
    }
    let seen = outcomes
        .extracted
        .iter()
        .map(|(index, _)| *index)
        .chain(outcomes.failed.iter().map(|(index, _)| *index))
        .collect::<BTreeSet<usize>>();
    assert_eq!(seen, (0..32usize).collect::<BTreeSet<usize>>());
}

#[tokio::test]
async fn stopping_dispatch_drains_running_work_and_skips_the_rest() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let closure_invocations = Arc::clone(&invocations);
    let outcomes = extract_bounded(
        (0..64usize).collect::<Vec<usize>>(),
        ExtractionLimits::new(2, true),
        move |index: usize| {
            closure_invocations.fetch_add(1, Ordering::SeqCst);
            if index == 0 { Err(index) } else { Ok(index) }
        },
    )
    .await;

    assert_eq!(outcomes.failed.first().map(|(index, _)| *index), Some(0));
    assert!(!outcomes.skipped.is_empty());
    assert_eq!(outcomes.total(), 64);
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        outcomes.extracted.len() + outcomes.failed.len()
    );
    assert!(outcomes.skipped.windows(2).all(|pair| pair[0] < pair[1]));
    let dispatched = outcomes
        .extracted
        .iter()
        .map(|(index, _)| *index)
        .chain(outcomes.failed.iter().map(|(index, _)| *index))
        .collect::<BTreeSet<usize>>();
    assert!(
        outcomes
            .skipped
            .iter()
            .all(|index| !dispatched.contains(index))
    );
    let mut every = dispatched;
    every.extend(outcomes.skipped.iter().copied());
    every.extend(outcomes.aborted.iter().copied());
    assert_eq!(every, (0..64usize).collect::<BTreeSet<usize>>());
}

#[tokio::test]
async fn dispatch_continues_past_a_failure_when_stopping_is_off() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let closure_invocations = Arc::clone(&invocations);
    let outcomes = extract_bounded(
        (0..16usize).collect::<Vec<usize>>(),
        ExtractionLimits::new(2, false),
        move |index: usize| {
            closure_invocations.fetch_add(1, Ordering::SeqCst);
            if index == 0 { Err(index) } else { Ok(index) }
        },
    )
    .await;

    assert_eq!(invocations.load(Ordering::SeqCst), 16);
    assert!(outcomes.skipped.is_empty());
    assert!(outcomes.aborted.is_empty());
    assert_eq!(outcomes.failed.len(), 1);
    assert_eq!(outcomes.extracted.len(), 15);
    assert_eq!(outcomes.extracted.last().map(|(index, _)| *index), Some(15));
}

#[tokio::test]
async fn a_zero_ceiling_serialises_rather_than_deadlocking() {
    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let closure_in_flight = Arc::clone(&in_flight);
    let closure_peak = Arc::clone(&peak);
    let outcomes = extract_bounded(
        (0..8usize).collect::<Vec<usize>>(),
        ExtractionLimits::new(0, false),
        move |index: usize| {
            let concurrent = closure_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            closure_peak.fetch_max(concurrent, Ordering::SeqCst);
            closure_in_flight.fetch_sub(1, Ordering::SeqCst);
            Ok::<usize, usize>(index)
        },
    )
    .await;

    assert_eq!(ExtractionLimits::new(0, false).permits(), 1);
    assert_eq!(peak.load(Ordering::SeqCst), 1);
    assert_eq!(outcomes.extracted.len(), 8);
    assert!(outcomes.failed.is_empty());
    assert!(outcomes.skipped.is_empty());
    assert!(outcomes.aborted.is_empty());
}

#[tokio::test]
async fn an_empty_candidate_list_produces_empty_outcomes() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let closure_invocations = Arc::clone(&invocations);
    let outcomes: ExtractionOutcomes<usize, usize> = extract_bounded(
        Vec::<usize>::new(),
        ExtractionLimits::new(4, true),
        move |index: usize| {
            closure_invocations.fetch_add(1, Ordering::SeqCst);
            Ok(index)
        },
    )
    .await;

    assert_eq!(invocations.load(Ordering::SeqCst), 0);
    assert!(outcomes.extracted.is_empty());
    assert!(outcomes.failed.is_empty());
    assert!(outcomes.skipped.is_empty());
    assert!(outcomes.aborted.is_empty());
    assert_eq!(outcomes.total(), 0);
}
