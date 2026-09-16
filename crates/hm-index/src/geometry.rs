use crate::simd::scalar_dot;

pub const Q16_ONE: i64 = 1 << 16;
pub const NEUTRAL_SCORE_Q16: i64 = Q16_ONE / 2;
pub const MINIMUM_FACTOR_Q16: i64 = 3 * Q16_ONE / 4;
pub const MAXIMUM_FACTOR_Q16: i64 = 5 * Q16_ONE / 4;

fn norm_squared(vector: &[i8]) -> u64 {
    vector.iter().fold(0_u64, |total, value| {
        let magnitude = u64::from(value.unsigned_abs());
        total.saturating_add(magnitude * magnitude)
    })
}

#[must_use]
pub fn cosine_q16(left: &[i8], right: &[i8]) -> Option<i64> {
    if left.is_empty() {
        return None;
    }
    let dot = scalar_dot(left, right)?;
    let left_norm = norm_squared(left);
    let right_norm = norm_squared(right);
    if left_norm == 0 || right_norm == 0 {
        return Some(0);
    }
    let magnitude =
        i128::try_from((u128::from(left_norm) * u128::from(right_norm)).isqrt()).ok()?;
    let scaled = i128::from(dot) * i128::from(Q16_ONE);
    let cosine = (scaled / magnitude).clamp(-i128::from(Q16_ONE), i128::from(Q16_ONE));
    i64::try_from(cosine).ok()
}

#[must_use]
pub fn coordinates_q16(vector: &[i8], basis: &[&[i8]]) -> Vec<i64> {
    basis
        .iter()
        .map(|direction| cosine_q16(vector, direction).unwrap_or(0))
        .collect()
}

#[must_use]
pub fn alignment_score_q16(coordinates: &[i64], query_coordinates: &[i64]) -> i64 {
    if coordinates.is_empty() || query_coordinates.is_empty() {
        return NEUTRAL_SCORE_Q16;
    }
    let mut total_weight = 0_i128;
    let mut weighted = 0_i128;
    for (coordinate, weight) in coordinates.iter().zip(query_coordinates) {
        let weight = i128::from((*weight).clamp(0, Q16_ONE));
        if weight == 0 {
            continue;
        }
        let coordinate = i128::from((*coordinate).clamp(-Q16_ONE, Q16_ONE));
        total_weight += weight;
        weighted += weight * (coordinate + i128::from(Q16_ONE));
    }
    if total_weight == 0 {
        return NEUTRAL_SCORE_Q16;
    }
    let score = (weighted / (total_weight * 2)).clamp(0, i128::from(Q16_ONE));
    i64::try_from(score).unwrap_or(NEUTRAL_SCORE_Q16)
}

#[must_use]
pub fn alignment_factor_q16(coordinates: &[i64], query_coordinates: &[i64]) -> i64 {
    let score = alignment_score_q16(coordinates, query_coordinates);
    (Q16_ONE + (score - NEUTRAL_SCORE_Q16) / 2).clamp(MINIMUM_FACTOR_Q16, MAXIMUM_FACTOR_Q16)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn next_state(mut state: u64) -> u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    }

    #[test]
    fn cosine_is_exact_for_identical_orthogonal_and_opposed_vectors() {
        assert_eq!(cosine_q16(&[100, 0], &[100, 0]), Some(Q16_ONE));
        assert_eq!(cosine_q16(&[100, 0], &[0, 100]), Some(0));
        assert_eq!(cosine_q16(&[100, 0], &[-100, 0]), Some(-Q16_ONE));
        assert_eq!(cosine_q16(&[10, 10], &[20, 20]), Some(Q16_ONE));
        assert_eq!(cosine_q16(&[0, 0], &[1, 1]), Some(0));
        assert_eq!(cosine_q16(&[], &[]), None);
        assert_eq!(cosine_q16(&[1], &[1, 2]), None);
    }

    #[test]
    fn coordinates_have_one_entry_per_basis_direction() {
        let basis: [&[i8]; 3] = [&[1, 0], &[0, 1], &[0, 0]];
        let coordinates = coordinates_q16(&[1, 0], &basis);
        assert_eq!(coordinates.len(), 3);
        assert_eq!(coordinates, vec![Q16_ONE, 0, 0]);
        assert!(coordinates_q16(&[1, 0], &[]).is_empty());
    }

    #[test]
    fn alignment_is_neutral_when_nothing_is_supplied() {
        assert_eq!(alignment_score_q16(&[], &[]), NEUTRAL_SCORE_Q16);
        assert_eq!(alignment_score_q16(&[Q16_ONE], &[]), NEUTRAL_SCORE_Q16);
        assert_eq!(alignment_score_q16(&[], &[Q16_ONE]), NEUTRAL_SCORE_Q16);
        assert_eq!(alignment_score_q16(&[0, 0], &[0, 0]), NEUTRAL_SCORE_Q16);
        assert_eq!(alignment_factor_q16(&[], &[]), Q16_ONE);
        assert_eq!(alignment_factor_q16(&[Q16_ONE], &[]), Q16_ONE);
        assert_eq!(alignment_factor_q16(&[], &[Q16_ONE]), Q16_ONE);
        assert_eq!(alignment_factor_q16(&[0, 0], &[0, 0]), Q16_ONE);
    }

    #[test]
    fn alignment_factor_stays_in_band_and_is_monotone() {
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for _ in 0..1024 {
            let mut coordinates = Vec::with_capacity(4);
            let mut query_coordinates = Vec::with_capacity(4);
            for _ in 0..4 {
                state = next_state(state);
                coordinates
                    .push(i64::try_from(state % 131_073).expect("generated coordinate") - Q16_ONE);
                state = next_state(state);
                query_coordinates
                    .push(i64::try_from(state % 131_073).expect("generated weight") - Q16_ONE);
            }
            let factor = alignment_factor_q16(&coordinates, &query_coordinates);
            assert!(
                (MINIMUM_FACTOR_Q16..=MAXIMUM_FACTOR_Q16).contains(&factor),
                "factor {factor} outside band for {coordinates:?} and {query_coordinates:?}"
            );
        }
        let query_coordinates = [Q16_ONE, 0];
        let mut previous = MINIMUM_FACTOR_Q16;
        for coordinate in [0, Q16_ONE / 4, Q16_ONE / 2, Q16_ONE] {
            let factor = alignment_factor_q16(&[coordinate, 0], &query_coordinates);
            assert!(factor >= previous, "factor {factor} fell below {previous}");
            assert!((MINIMUM_FACTOR_Q16..=MAXIMUM_FACTOR_Q16).contains(&factor));
            previous = factor;
        }
        assert_eq!(alignment_factor_q16(&[0, 0], &query_coordinates), Q16_ONE);
        assert_eq!(
            alignment_factor_q16(&[Q16_ONE, 0], &query_coordinates),
            MAXIMUM_FACTOR_Q16
        );
        assert_eq!(
            alignment_factor_q16(&[-Q16_ONE, 0], &query_coordinates),
            MINIMUM_FACTOR_Q16
        );
    }
}
