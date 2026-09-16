#![forbid(unsafe_code)]

use hm_cortex::budget::{
    BudgetDimension, BudgetExceeded, BudgetUsage, CallReservation, RunReservation,
};
use hm_llm::Usage;
use hm_schema::events::ConsolidationBudget;

fn budget(max_llm_calls: u64, max_tokens: u64, max_microusd: u64) -> ConsolidationBudget {
    ConsolidationBudget {
        max_llm_calls,
        max_tokens,
        max_microusd,
        max_wall_ms: 60_000,
    }
}

fn usage(input_tokens: u64, output_tokens: u64, cost_microusd: u64) -> Usage {
    Usage {
        input_tokens,
        output_tokens,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd,
    }
}

#[test]
fn outstanding_reservations_count_against_the_declared_budget() {
    let mut reservation = RunReservation::new(budget(1, 4096, 1000));
    assert_eq!(reservation.settled(), BudgetUsage::default());
    assert_eq!(reservation.outstanding(), CallReservation::default());

    let request = CallReservation {
        llm_calls: 1,
        output_tokens: 1024,
    };
    let ticket = reservation
        .reserve(request)
        .expect("the first attempt fits the declared budget");
    assert_eq!(reservation.outstanding(), request);

    let refused = reservation
        .reserve(request)
        .expect_err("a second outstanding call exceeds a one call budget");
    assert_eq!(
        refused,
        BudgetExceeded {
            dimension: BudgetDimension::LlmCalls,
            limit: 1,
            attempted: 2,
        }
    );
    assert_eq!(reservation.settled(), BudgetUsage::default());
    assert_eq!(reservation.outstanding(), request);

    let settled = reservation
        .settle(ticket, usage(100, 200, 50))
        .expect("the first attempt settles inside the budget");
    assert_eq!(
        settled,
        BudgetUsage {
            llm_calls: 1,
            input_tokens: 100,
            output_tokens: 200,
            cost_microusd: 50,
            wall_ms: 0,
        }
    );
    assert_eq!(reservation.outstanding(), CallReservation::default());
}

#[test]
fn a_raised_output_ceiling_is_reserved_before_it_is_spent() {
    let mut reservation = RunReservation::new(budget(4, 2048, 10_000));
    let first = reservation
        .reserve(CallReservation {
            llm_calls: 1,
            output_tokens: 1024,
        })
        .expect("the first ceiling fits");
    let settled = reservation
        .settle(first, usage(500, 500, 10))
        .expect("the first attempt settles inside the budget");
    assert_eq!(settled.tokens(), 1000);
    assert_eq!(reservation.outstanding(), CallReservation::default());

    let refused = reservation
        .reserve(CallReservation {
            llm_calls: 1,
            output_tokens: 2048,
        })
        .expect_err("a raised ceiling that does not fit is refused before the call");
    assert_eq!(
        refused,
        BudgetExceeded {
            dimension: BudgetDimension::Tokens,
            limit: 2048,
            attempted: 3048,
        }
    );
    assert_eq!(reservation.settled(), settled);
    assert_eq!(reservation.outstanding(), CallReservation::default());

    let retry = CallReservation {
        llm_calls: 1,
        output_tokens: 1000,
    };
    let ticket = reservation
        .reserve(retry)
        .expect("a raised ceiling that still fits is admitted");
    assert_eq!(reservation.outstanding(), retry);
    reservation.release(ticket);
}

#[test]
fn a_failed_attempt_is_still_charged_and_release_only_frees_unspent_room() {
    let mut reservation = RunReservation::new(budget(4, 8192, 1000));
    let ticket = reservation
        .reserve(CallReservation {
            llm_calls: 1,
            output_tokens: 1024,
        })
        .expect("the attempt fits the declared budget");
    let exceeded = reservation
        .settle(ticket, usage(400, 100, 1500))
        .expect_err("the settled cost exceeds the declared budget");
    assert_eq!(
        exceeded,
        BudgetExceeded {
            dimension: BudgetDimension::Cost,
            limit: 1000,
            attempted: 1500,
        }
    );
    let charged = reservation.settled();
    assert_eq!(
        charged,
        BudgetUsage {
            llm_calls: 1,
            input_tokens: 400,
            output_tokens: 100,
            cost_microusd: 1500,
            wall_ms: 0,
        }
    );
    assert_eq!(reservation.outstanding(), CallReservation::default());

    let unspent = CallReservation {
        llm_calls: 1,
        output_tokens: 512,
    };
    let released = reservation
        .reserve(unspent)
        .expect("room remains for one more call");
    assert_eq!(reservation.outstanding(), unspent);
    reservation.release(released);
    assert_eq!(reservation.outstanding(), CallReservation::default());
    assert_eq!(reservation.settled(), charged);

    let again = reservation
        .reserve(unspent)
        .expect("the released room is reservable again");
    assert_eq!(reservation.outstanding(), unspent);
    reservation.release(again);
}

#[test]
fn an_unbounded_reservation_never_refuses() {
    let mut reservation = RunReservation::unbounded();
    for _ in 0..1000 {
        let ticket = reservation
            .reserve(CallReservation {
                llm_calls: 1,
                output_tokens: 4096,
            })
            .expect("an unbounded reservation admits every attempt");
        reservation
            .settle(ticket, usage(128, 256, 7))
            .expect("an unbounded reservation settles every attempt");
    }
    assert_eq!(
        reservation.settled(),
        BudgetUsage {
            llm_calls: 1000,
            input_tokens: 128_000,
            output_tokens: 256_000,
            cost_microusd: 7000,
            wall_ms: 0,
        }
    );
    assert_eq!(reservation.outstanding(), CallReservation::default());
}
