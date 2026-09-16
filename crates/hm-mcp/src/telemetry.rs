use hm_core::telemetry::{Attribute, SpanKind, SpanOutcome};
use hm_llm::{LlmError, LlmProvider, ModelTier, StructuredRequest, StructuredResponse};
use std::sync::Arc;

pub struct ObservedProvider {
    inner: Arc<dyn LlmProvider>,
}

impl ObservedProvider {
    #[must_use]
    pub const fn new(inner: Arc<dyn LlmProvider>) -> Self {
        Self { inner }
    }
}

impl LlmProvider for ObservedProvider {
    fn model_id(&self) -> &str {
        self.inner.model_id()
    }

    fn tier(&self) -> ModelTier {
        self.inner.tier()
    }

    fn generate_structured(
        &self,
        request: &StructuredRequest,
    ) -> Result<StructuredResponse, LlmError> {
        let Some(mut span) =
            hm_core::telemetry::start_span(SpanKind::Provider, "hypermind.provider")
        else {
            return self.inner.generate_structured(request);
        };
        span.attribute(Attribute::Text(
            "hypermind.provider.tier",
            tier_label(self.inner.tier()),
        ));
        let response = self.inner.generate_structured(request);
        match response.as_ref() {
            Ok(structured) => {
                span.attribute(Attribute::Integer(
                    "hypermind.provider.input_tokens",
                    counter(structured.usage.input_tokens),
                ));
                span.attribute(Attribute::Integer(
                    "hypermind.provider.output_tokens",
                    counter(structured.usage.output_tokens),
                ));
                span.attribute(Attribute::Integer(
                    "hypermind.provider.microusd",
                    counter(structured.usage.cost_microusd),
                ));
                span.finish(SpanOutcome::Ok);
            }
            Err(_) => span.finish(SpanOutcome::Error),
        }
        response
    }
}

#[must_use]
pub fn observed(provider: Arc<dyn LlmProvider>) -> Arc<dyn LlmProvider> {
    Arc::new(ObservedProvider::new(provider))
}

#[must_use]
pub const fn tier_label(tier: ModelTier) -> &'static str {
    match tier {
        ModelTier::Economy => "economy",
        ModelTier::Standard => "standard",
        ModelTier::Capable => "capable",
    }
}

fn counter(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
