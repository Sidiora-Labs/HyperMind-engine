#![allow(clippy::missing_errors_doc)]

use hm_llm::Usage;
use hm_schema::events::ConsolidationBudget;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BudgetUsage {
    pub llm_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub wall_ms: u64,
}

impl BudgetUsage {
    #[must_use]
    pub const fn tokens(self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetDimension {
    LlmCalls,
    Tokens,
    Cost,
    WallTime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetExceeded {
    pub dimension: BudgetDimension,
    pub limit: u64,
    pub attempted: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetTracker {
    limits: ConsolidationBudget,
    usage: BudgetUsage,
    started_at_ms: u64,
}

impl BudgetTracker {
    #[must_use]
    pub const fn new(limits: ConsolidationBudget, started_at_ms: u64) -> Self {
        Self {
            limits,
            usage: BudgetUsage {
                llm_calls: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost_microusd: 0,
                wall_ms: 0,
            },
            started_at_ms,
        }
    }

    #[must_use]
    pub const fn usage(&self) -> BudgetUsage {
        self.usage
    }

    pub fn admit_call(&mut self, call: Usage, now_ms: u64) -> Result<BudgetUsage, BudgetExceeded> {
        let candidate = BudgetUsage {
            llm_calls: self.usage.llm_calls.saturating_add(1),
            input_tokens: self.usage.input_tokens.saturating_add(call.input_tokens),
            output_tokens: self.usage.output_tokens.saturating_add(call.output_tokens),
            cost_microusd: self.usage.cost_microusd.saturating_add(call.cost_microusd),
            wall_ms: now_ms.saturating_sub(self.started_at_ms),
        };
        self.check(candidate)?;
        self.usage = candidate;
        Ok(candidate)
    }

    pub fn check_time(&mut self, now_ms: u64) -> Result<BudgetUsage, BudgetExceeded> {
        let candidate = BudgetUsage {
            wall_ms: now_ms.saturating_sub(self.started_at_ms),
            ..self.usage
        };
        self.check(candidate)?;
        self.usage = candidate;
        Ok(candidate)
    }

    fn check(&self, candidate: BudgetUsage) -> Result<(), BudgetExceeded> {
        check_limit(
            BudgetDimension::LlmCalls,
            self.limits.max_llm_calls,
            candidate.llm_calls,
        )?;
        check_limit(
            BudgetDimension::Tokens,
            self.limits.max_tokens,
            candidate.tokens(),
        )?;
        check_limit(
            BudgetDimension::Cost,
            self.limits.max_microusd,
            candidate.cost_microusd,
        )?;
        check_limit(
            BudgetDimension::WallTime,
            self.limits.max_wall_ms,
            candidate.wall_ms,
        )
    }
}

fn check_limit(
    dimension: BudgetDimension,
    limit: u64,
    attempted: u64,
) -> Result<(), BudgetExceeded> {
    if attempted > limit {
        Err(BudgetExceeded {
            dimension,
            limit,
            attempted,
        })
    } else {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallReservation {
    pub llm_calls: u64,
    pub output_tokens: u64,
}

#[must_use]
#[derive(Debug)]
pub struct ReservationTicket {
    reserved: CallReservation,
}

impl ReservationTicket {
    fn into_reservation(self) -> CallReservation {
        self.reserved
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunReservation {
    limits: ConsolidationBudget,
    settled: BudgetUsage,
    outstanding: CallReservation,
}

impl RunReservation {
    #[must_use]
    pub const fn new(limits: ConsolidationBudget) -> Self {
        Self {
            limits,
            settled: BudgetUsage {
                llm_calls: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost_microusd: 0,
                wall_ms: 0,
            },
            outstanding: CallReservation {
                llm_calls: 0,
                output_tokens: 0,
            },
        }
    }

    #[must_use]
    pub const fn unbounded() -> Self {
        Self::new(ConsolidationBudget {
            max_llm_calls: u64::MAX,
            max_tokens: u64::MAX,
            max_microusd: u64::MAX,
            max_wall_ms: u64::MAX,
        })
    }

    #[must_use]
    pub const fn settled(&self) -> BudgetUsage {
        self.settled
    }

    #[must_use]
    pub const fn outstanding(&self) -> CallReservation {
        self.outstanding
    }

    pub fn reserve(
        &mut self,
        request: CallReservation,
    ) -> Result<ReservationTicket, BudgetExceeded> {
        let calls = self
            .settled
            .llm_calls
            .saturating_add(self.outstanding.llm_calls)
            .saturating_add(request.llm_calls);
        check_limit(BudgetDimension::LlmCalls, self.limits.max_llm_calls, calls)?;
        let tokens = self
            .settled
            .tokens()
            .saturating_add(self.outstanding.output_tokens)
            .saturating_add(request.output_tokens);
        check_limit(BudgetDimension::Tokens, self.limits.max_tokens, tokens)?;
        self.outstanding.llm_calls = self.outstanding.llm_calls.saturating_add(request.llm_calls);
        self.outstanding.output_tokens = self
            .outstanding
            .output_tokens
            .saturating_add(request.output_tokens);
        Ok(ReservationTicket { reserved: request })
    }

    pub fn settle(
        &mut self,
        ticket: ReservationTicket,
        actual: Usage,
    ) -> Result<BudgetUsage, BudgetExceeded> {
        let reserved = ticket.into_reservation();
        self.discharge(reserved);
        self.settled = BudgetUsage {
            llm_calls: self.settled.llm_calls.saturating_add(reserved.llm_calls),
            input_tokens: self
                .settled
                .input_tokens
                .saturating_add(actual.input_tokens),
            output_tokens: self
                .settled
                .output_tokens
                .saturating_add(actual.output_tokens),
            cost_microusd: self
                .settled
                .cost_microusd
                .saturating_add(actual.cost_microusd),
            wall_ms: self.settled.wall_ms,
        };
        check_limit(
            BudgetDimension::LlmCalls,
            self.limits.max_llm_calls,
            self.settled.llm_calls,
        )?;
        check_limit(
            BudgetDimension::Tokens,
            self.limits.max_tokens,
            self.settled.tokens(),
        )?;
        check_limit(
            BudgetDimension::Cost,
            self.limits.max_microusd,
            self.settled.cost_microusd,
        )?;
        Ok(self.settled)
    }

    pub fn release(&mut self, ticket: ReservationTicket) {
        let reserved = ticket.into_reservation();
        self.discharge(reserved);
    }

    fn discharge(&mut self, reserved: CallReservation) {
        self.outstanding.llm_calls = self
            .outstanding
            .llm_calls
            .saturating_sub(reserved.llm_calls);
        self.outstanding.output_tokens = self
            .outstanding
            .output_tokens
            .saturating_sub(reserved.output_tokens);
    }
}
