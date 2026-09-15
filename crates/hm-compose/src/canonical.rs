#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationBundle, Tier};
use hm_core::{Error, ErrorCode, LSN};

pub const CANONICAL_MAGIC: &[u8; 4] = b"HMA1";

pub fn canonical_bytes(bundle: &ActivationBundle) -> Result<Vec<u8>, Error> {
    if bundle.spent_tokens > bundle.budget_tokens
        || bundle.sections.len() != Tier::ALL.len()
        || bundle
            .sections
            .iter()
            .zip(Tier::ALL)
            .any(|(section, tier)| section.tier != tier || section.required != tier.required())
    {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let actual_spent = bundle.sections.iter().try_fold(0_usize, |total, section| {
        total
            .checked_add(section.tokens)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
    })?;
    if actual_spent != bundle.spent_tokens {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }

    let mut output = CANONICAL_MAGIC.to_vec();
    append_u64(&mut output, bundle.snapshot_epoch);
    append_usize(&mut output, bundle.budget_tokens)?;
    append_usize(&mut output, bundle.spent_tokens)?;
    append_usize(&mut output, bundle.sections.len())?;
    for section in &bundle.sections {
        output.push(section.tier as u8);
        output.push(u8::from(section.required));
        append_usize(&mut output, section.tokens)?;
        append_usize(&mut output, section.trimmed_items)?;
        append_usize(&mut output, section.coarsened_items)?;
        append_usize(&mut output, section.items.len())?;
        for item in &section.items {
            if item.tier != section.tier || item.provenance.is_empty() {
                return Err(Error::new(ErrorCode::InvariantViolation));
            }
            output.push(item.tier as u8);
            output.push(u8::from(item.coarsened));
            output.push(item.why as u8);
            output.push(item.authority as u8);
            append_u32(&mut output, item.vector_rank);
            append_u32(&mut output, item.lexical_rank);
            append_usize(&mut output, item.tokens)?;
            append_bytes(&mut output, item.uri.as_bytes())?;
            append_lsns(&mut output, &item.provenance)?;
            append_bytes(&mut output, &item.content)?;
        }
    }

    output.extend_from_slice(&bundle.manifest.manifest_id);
    output.extend_from_slice(&bundle.manifest.query_digest);
    append_u64(&mut output, bundle.manifest.snapshot_epoch);
    append_bytes(&mut output, bundle.manifest.encoder.as_bytes())?;
    append_u64(&mut output, bundle.manifest.index_generation);
    append_usize(&mut output, bundle.manifest.candidate_lanes.len())?;
    output.extend(
        bundle
            .manifest
            .candidate_lanes
            .iter()
            .map(|lane| *lane as u8),
    );
    append_lsns(&mut output, &bundle.manifest.candidates)?;
    append_lsns(&mut output, &bundle.manifest.selected)?;
    append_lsns(&mut output, &bundle.manifest.included)?;
    append_lsns(&mut output, &bundle.manifest.used)?;

    append_usize(&mut output, bundle.gaps.len())?;
    for gap in &bundle.gaps {
        output.push(gap.kind as u8);
        output.push(gap.tier.map_or(u8::MAX, |tier| tier as u8));
        output.push(gap.lane.map_or(u8::MAX, |lane| lane as u8));
        append_bytes(&mut output, gap.detail.as_bytes())?;
    }
    output.push(bundle.health.encoder as u8);
    output.push(bundle.health.backlog as u8);
    output.push(bundle.health.projection as u8);
    output.push(bundle.health.inclusion as u8);
    Ok(output)
}

pub fn bundle_hash(bundle: &ActivationBundle) -> Result<[u8; 32], Error> {
    Ok(*blake3::hash(&canonical_bytes(bundle)?).as_bytes())
}

pub fn verify_bundle_hash(bundle: &ActivationBundle) -> Result<(), Error> {
    if bundle.bundle_hash == bundle_hash(bundle)? {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::InvariantViolation))
    }
}

fn append_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn append_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn append_usize(output: &mut Vec<u8>, value: usize) -> Result<(), Error> {
    append_u64(
        output,
        u64::try_from(value).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
    );
    Ok(())
}

fn append_bytes(output: &mut Vec<u8>, value: &[u8]) -> Result<(), Error> {
    append_usize(output, value.len())?;
    output.extend_from_slice(value);
    Ok(())
}

fn append_lsns(output: &mut Vec<u8>, values: &[LSN]) -> Result<(), Error> {
    append_usize(output, values.len())?;
    for value in values {
        append_u64(output, value.get());
    }
    Ok(())
}
