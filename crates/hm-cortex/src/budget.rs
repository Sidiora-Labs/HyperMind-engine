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
