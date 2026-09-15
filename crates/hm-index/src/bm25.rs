#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};

pub const Q32_ONE: u64 = 1_u64 << 32;
const LN_2_Q32: u64 = 2_977_044_471;

pub fn fixed_ln_ratio(numerator: u64, denominator: u64) -> Result<u64, Error> {
    if numerator < denominator || denominator == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let scaled_numerator = u128::from(numerator);
    let mut scaled_denominator = u128::from(denominator);
    let mut exponent = 0_u32;
    while scaled_numerator >= scaled_denominator * 2 {
        scaled_denominator *= 2;
        exponent += 1;
    }
    let y = u64::try_from(
        ((scaled_numerator - scaled_denominator) << 32) / (scaled_numerator + scaled_denominator),
    )
    .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let y_squared = u64::try_from((u128::from(y) * u128::from(y)) >> 32)
        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut power = y;
    let mut series = y;
    for divisor in (3_u64..=15).step_by(2) {
        power = u64::try_from((u128::from(power) * u128::from(y_squared)) >> 32)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        series = series
            .checked_add(power / divisor)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
    }
    u64::from(exponent)
        .checked_mul(LN_2_Q32)
        .and_then(|integral| integral.checked_add((series * 2).min(Q32_ONE * 2)))
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
}

pub fn score_q32(
    document_count: u64,
    document_frequency: u64,
    total_terms: u64,
    document_length: u32,
    term_frequency: u32,
) -> Result<u64, Error> {
    if document_count == 0
        || document_frequency == 0
        || document_frequency > document_count
        || total_terms == 0
        || document_length == 0
        || term_frequency == 0
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let idf = fixed_ln_ratio(
        document_count
            .checked_mul(2)
            .and_then(|value| value.checked_add(2))
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?,
        document_frequency
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?,
    )?;
    let tf = u128::from(term_frequency);
    let terms = u128::from(total_terms);
    let documents = u128::from(document_count);
    let numerator = u128::from(idf) * 44 * tf * terms;
    let denominator = 20 * tf * terms + 6 * terms + 18 * u128::from(document_length) * documents;
    u64::try_from(numerator / denominator).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}
