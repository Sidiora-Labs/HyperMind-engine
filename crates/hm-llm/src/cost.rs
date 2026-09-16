#![allow(clippy::cast_precision_loss)]

use crate::{LlmError, Pricing, Usage};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PriceEntry {
    pub model: &'static str,
    pub pricing: Pricing,
}

pub const PINNED_PRICE_TABLE_VERSION: &str = "2026-09-01";
pub const PINNED_PRICES: [PriceEntry; 6] = [
    price("claude-sonnet-4-5", 3_000_000, 15_000_000),
    price("claude-haiku-4-5", 1_000_000, 5_000_000),
    price("gpt-4.1", 2_000_000, 8_000_000),
    price("gpt-4.1-mini", 400_000, 1_600_000),
    price("gemini-2.5-pro", 1_250_000, 10_000_000),
    price("gemini-2.5-flash", 300_000, 2_500_000),
];

const fn price(model: &'static str, input: u64, output: u64) -> PriceEntry {
    PriceEntry {
        model,
        pricing: Pricing {
            input_microusd_per_million_tokens: input,
            output_microusd_per_million_tokens: output,
        },
    }
}

#[must_use]
pub fn pricing_for(model: &str) -> Option<Pricing> {
    PINNED_PRICES
        .iter()
        .find(|entry| entry.model == model)
        .map(|entry| entry.pricing)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RunCost {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_microusd: u64,
}

impl RunCost {
    pub fn record(&mut self, usage: Usage) -> Result<(), LlmError> {
        self.calls = self.calls.checked_add(1).ok_or(LlmError::Capacity)?;
        self.input_tokens = self
            .input_tokens
            .checked_add(usage.input_tokens)
            .ok_or(LlmError::Capacity)?;
        self.output_tokens = self
            .output_tokens
            .checked_add(usage.output_tokens)
            .ok_or(LlmError::Capacity)?;
        self.cache_read_tokens = self
            .cache_read_tokens
            .checked_add(usage.cache_read_tokens)
            .ok_or(LlmError::Capacity)?;
        self.cache_write_tokens = self
            .cache_write_tokens
            .checked_add(usage.cache_write_tokens)
            .ok_or(LlmError::Capacity)?;
        self.cost_microusd = self
            .cost_microusd
            .checked_add(usage.cost_microusd)
            .ok_or(LlmError::Capacity)?;
        Ok(())
    }

    #[must_use]
    pub fn tokens(self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }

    #[must_use]
    pub fn usd(self) -> f64 {
        self.cost_microusd as f64 / 1_000_000.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunBudget {
    pub max_calls: u64,
    pub max_tokens: u64,
    pub max_microusd: u64,
}

impl RunBudget {
    pub fn admit(self, accumulated: RunCost, next: Usage) -> Result<RunCost, LlmError> {
        let mut projected = accumulated;
        projected.record(next)?;
        if projected.calls > self.max_calls
            || projected.tokens() > self.max_tokens
            || projected.cost_microusd > self.max_microusd
        {
            Err(LlmError::Capacity)
        } else {
            Ok(projected)
        }
    }
}
