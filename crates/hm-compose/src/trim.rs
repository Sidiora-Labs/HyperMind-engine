#![allow(clippy::missing_errors_doc)]

use crate::budget::BudgetProfile;
use crate::bundle::{ActivationBundle, Gap, GapKind, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{Error, ErrorCode};

pub fn trim_to_budget(bundle: &mut ActivationBundle, counter: &TokenCounter) -> Result<(), Error> {
    trim_to_budget_with_profile(bundle, counter, BudgetProfile::default())
}

pub fn trim_to_budget_with_profile(
    bundle: &mut ActivationBundle,
    counter: &TokenCounter,
    profile: BudgetProfile,
) -> Result<(), Error> {
    profile.allocate(bundle.budget_tokens)?;
    let mut total = total_tokens(bundle)?;
    for tier in [Tier::Temporal, Tier::Fused, Tier::Conflicts, Tier::Entity] {
        drop_section_tail(bundle, tier, &mut total);
        if total <= bundle.budget_tokens {
            break;
        }
    }
    if total > bundle.budget_tokens {
        coarsen_section(bundle, Tier::Conversation, counter, &mut total, false)?;
    }
    if total > bundle.budget_tokens {
        drop_section_tail(bundle, Tier::Conversation, &mut total);
    }
    if total > bundle.budget_tokens {
        drop_section_tail(bundle, Tier::Prospective, &mut total);
    }
    if total > bundle.budget_tokens {
        bundle.gaps.push(Gap {
            kind: GapKind::NarrowedSubtask,
            tier: Some(Tier::Intent),
            lane: None,
            detail: "required context narrowed to the newest active loop".to_owned(),
        });
        narrow_intent(bundle, &mut total);
        for tier in [
            Tier::Intent,
            Tier::Bindings,
            Tier::WorkLedger,
            Tier::Resident,
        ] {
            coarsen_section(bundle, tier, counter, &mut total, true)?;
        }
    }
    if total > bundle.budget_tokens {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    bundle.spent_tokens = total;
    Ok(())
}

fn narrow_intent(bundle: &mut ActivationBundle, total: &mut usize) {
    let section = &mut bundle.sections[Tier::Intent as usize];
    if section.items.len() <= 1 {
        return;
    }
    let active = section.items.pop().expect("non-empty intent section");
    let removed = section.tokens - active.tokens;
    section.items.clear();
    section.items.push(active);
    section.tokens -= removed;
    section.coarsened_items += 1;
    *total -= removed;
}

fn total_tokens(bundle: &ActivationBundle) -> Result<usize, Error> {
    bundle.sections.iter().try_fold(0_usize, |total, section| {
        total
            .checked_add(section.tokens)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
    })
}

fn drop_section_tail(bundle: &mut ActivationBundle, tier: Tier, total: &mut usize) {
    debug_assert!(!tier.required());
    let section = &mut bundle.sections[tier as usize];
    while *total > bundle.budget_tokens {
        let Some(item) = section.items.pop() else {
            break;
        };
        section.tokens -= item.tokens;
        *total -= item.tokens;
        section.trimmed_items += 1;
        if item.why == WhyCode::Evidence
            && section
                .items
                .last()
                .is_some_and(|summary| summary.why == WhyCode::Fused)
        {
            let summary = section.items.pop().expect("paired summary");
            section.tokens -= summary.tokens;
            *total -= summary.tokens;
            section.trimmed_items += 1;
        }
    }
    if section.trimmed_items > 0
        && !bundle
            .gaps
            .iter()
            .any(|gap| gap.kind == GapKind::DroppedTier && gap.tier == Some(tier))
    {
        bundle.gaps.push(Gap {
            kind: GapKind::DroppedTier,
            tier: Some(tier),
            lane: None,
            detail: "items removed to satisfy the token budget".to_owned(),
        });
    }
}

fn coarsen_section(
    bundle: &mut ActivationBundle,
    tier: Tier,
    counter: &TokenCounter,
    total: &mut usize,
    provenance_only: bool,
) -> Result<(), Error> {
    let section = &mut bundle.sections[tier as usize];
    for item in &mut section.items {
        if *total <= bundle.budget_tokens {
            break;
        }
        let mut compact = item.provenance[0].get().to_string().into_bytes();
        if !provenance_only {
            compact.push(b' ');
            compact.extend_from_slice(
                item.content
                    .split(|byte| *byte == b'\n')
                    .next()
                    .unwrap_or_default(),
            );
        }
        let tokens = counter.count(&compact)?;
        if tokens < item.tokens {
            let reduction = item.tokens - tokens;
            item.content = compact;
            item.tokens = tokens;
            item.coarsened = true;
            section.tokens -= reduction;
            *total -= reduction;
            section.coarsened_items += 1;
        }
    }
    Ok(())
}
