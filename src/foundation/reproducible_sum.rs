//! Exact, bounded reduction of finite IEEE-754 binary64 values.
//!
//! Physics subdomains share this implementation only because three concrete
//! slices require the identical numerical invariant: source identity and
//! insertion order cannot change a finite representable sum.

use std::cmp::Ordering;

/// Largest input accepted by the shared accumulator.
pub(crate) const MAX_REPRODUCIBLE_SUM_INPUTS: usize = 262_144;

// 262,144 finite maximum values require at most 2,116 magnitude bits. Any
// change to the public input bound must re-prove this 2,176-bit capacity.
const ACCUMULATOR_LIMBS: usize = 34;
const FRACTION_BITS: u32 = 52;
const COMMON_BINARY_EXPONENT: i32 = -1_074;
const SIGN_MASK: u64 = 1_u64 << 63;
const FRACTION_MASK: u64 = (1_u64 << FRACTION_BITS) - 1;

/// Structured failure from an exact bounded reduction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReproducibleSumError {
    /// The caller attempted work outside the proven accumulator bound.
    InputBudgetExceeded { submitted: usize, maximum: usize },
    /// A non-finite value crossed the numerical boundary.
    NonFiniteInput,
    /// The exact sum rounds outside the finite binary64 range.
    NonFiniteResult,
    /// The fixed accumulator capacity was exceeded unexpectedly.
    AccumulatorOverflow,
}

/// Returns the correctly rounded exact sum of a bounded finite slice.
///
/// Every input is decoded as a signed integer multiple of `2^-1074`. Positive
/// and negative magnitudes accumulate separately, are subtracted exactly, and
/// are rounded once using IEEE round-to-nearest, ties-to-even. Exact zero is
/// canonicalized to positive zero.
pub(crate) fn reproducible_sum(values: &[f64]) -> Result<f64, ReproducibleSumError> {
    if values.len() > MAX_REPRODUCIBLE_SUM_INPUTS {
        return Err(ReproducibleSumError::InputBudgetExceeded {
            submitted: values.len(),
            maximum: MAX_REPRODUCIBLE_SUM_INPUTS,
        });
    }

    let mut positive = Magnitude::default();
    let mut negative = Magnitude::default();
    for &value in values {
        if !value.is_finite() {
            return Err(ReproducibleSumError::NonFiniteInput);
        }
        let bits = value.to_bits();
        let exponent_bits = ((bits >> FRACTION_BITS) & 0x7ff) as usize;
        let fraction = bits & FRACTION_MASK;
        let (significand, shift) = if exponent_bits == 0 {
            (fraction, 0)
        } else {
            ((1_u64 << FRACTION_BITS) | fraction, exponent_bits - 1)
        };
        if significand == 0 {
            continue;
        }
        let accumulator = if bits & SIGN_MASK == 0 {
            &mut positive
        } else {
            &mut negative
        };
        accumulator.add_shifted(significand, shift)?;
    }

    match positive.compare(&negative) {
        Ordering::Equal => Ok(0.0),
        Ordering::Greater => positive.subtract(&negative).to_f64(false),
        Ordering::Less => negative.subtract(&positive).to_f64(true),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Magnitude {
    limbs: [u64; ACCUMULATOR_LIMBS],
}

impl Default for Magnitude {
    fn default() -> Self {
        Self {
            limbs: [0; ACCUMULATOR_LIMBS],
        }
    }
}

impl Magnitude {
    fn add_shifted(&mut self, significand: u64, shift: usize) -> Result<(), ReproducibleSumError> {
        let word = shift / 64;
        let bit = shift % 64;
        let low = significand << bit;
        self.add_word(word, low)?;
        if bit != 0 {
            let high = significand >> (64 - bit);
            self.add_word(word + 1, high)?;
        }
        Ok(())
    }

    fn add_word(&mut self, mut index: usize, mut value: u64) -> Result<(), ReproducibleSumError> {
        while value != 0 {
            let limb = self
                .limbs
                .get_mut(index)
                .ok_or(ReproducibleSumError::AccumulatorOverflow)?;
            let (next, carried) = limb.overflowing_add(value);
            *limb = next;
            value = u64::from(carried);
            index += 1;
        }
        Ok(())
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.limbs.iter().rev().cmp(other.limbs.iter().rev())
    }

    fn subtract(&self, other: &Self) -> Self {
        debug_assert!(self.compare(other) != Ordering::Less);
        let mut result = Self::default();
        let mut borrowed = false;
        for index in 0..ACCUMULATOR_LIMBS {
            let (without_other, first_borrow) =
                self.limbs[index].overflowing_sub(other.limbs[index]);
            let (difference, second_borrow) = without_other.overflowing_sub(u64::from(borrowed));
            result.limbs[index] = difference;
            borrowed = first_borrow || second_borrow;
        }
        debug_assert!(!borrowed);
        result
    }

    fn to_f64(&self, negative: bool) -> Result<f64, ReproducibleSumError> {
        let Some(mut highest_bit) = self.highest_bit() else {
            return Ok(0.0);
        };
        let sign = if negative { SIGN_MASK } else { 0 };

        if highest_bit < FRACTION_BITS as usize {
            return Ok(f64::from_bits(sign | self.limbs[0]));
        }

        let mut shift = highest_bit - FRACTION_BITS as usize;
        let mut significand = self.shifted_low_u64(shift);
        if shift != 0 {
            let halfway = self.bit(shift - 1);
            let below_halfway = self.any_bit_below(shift - 1);
            if halfway && (below_halfway || significand & 1 == 1) {
                significand += 1;
            }
        }

        if significand == 1_u64 << (FRACTION_BITS + 1) {
            significand >>= 1;
            shift += 1;
            highest_bit += 1;
        }
        debug_assert_eq!(shift, highest_bit - FRACTION_BITS as usize);

        let unbiased_exponent = highest_bit as i32 + COMMON_BINARY_EXPONENT;
        if unbiased_exponent > 1_023 {
            return Err(ReproducibleSumError::NonFiniteResult);
        }
        let exponent_bits = (unbiased_exponent + 1_023) as u64;
        let fraction = significand & FRACTION_MASK;
        Ok(f64::from_bits(
            sign | (exponent_bits << FRACTION_BITS) | fraction,
        ))
    }

    fn highest_bit(&self) -> Option<usize> {
        self.limbs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, limb)| **limb != 0)
            .map(|(index, limb)| index * 64 + (63 - limb.leading_zeros() as usize))
    }

    fn shifted_low_u64(&self, shift: usize) -> u64 {
        let word = shift / 64;
        let bit = shift % 64;
        let mut value = self.limbs[word] >> bit;
        if bit != 0 && word + 1 < ACCUMULATOR_LIMBS {
            value |= self.limbs[word + 1] << (64 - bit);
        }
        value
    }

    fn bit(&self, index: usize) -> bool {
        self.limbs[index / 64] & (1_u64 << (index % 64)) != 0
    }

    fn any_bit_below(&self, count: usize) -> bool {
        let complete_words = count / 64;
        if self.limbs[..complete_words].iter().any(|&limb| limb != 0) {
            return true;
        }
        let remaining = count % 64;
        remaining != 0 && self.limbs[complete_words] & ((1_u64 << remaining) - 1) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_REPRODUCIBLE_SUM_INPUTS, ReproducibleSumError, reproducible_sum};

    #[test]
    fn exact_cancellation_never_overflows_an_intermediate() {
        assert_eq!(
            reproducible_sum(&[f64::MAX, f64::MAX, -f64::MAX, -f64::MAX]),
            Ok(0.0)
        );
        assert_eq!(
            reproducible_sum(&[f64::MAX, f64::MAX, -f64::MAX]),
            Ok(f64::MAX)
        );
    }

    #[test]
    fn tiny_residual_survives_complete_large_cancellation() {
        let smallest = f64::from_bits(1);
        assert_eq!(
            reproducible_sum(&[f64::MAX, smallest, -f64::MAX]),
            Ok(smallest)
        );
        assert_eq!(reproducible_sum(&[1.0e16, 1.0, -1.0e16]), Ok(1.0));
    }

    #[test]
    fn rounds_once_to_nearest_with_ties_to_even() {
        assert_eq!(reproducible_sum(&[1.0, 2_f64.powi(-53)]), Ok(1.0));
        assert_eq!(
            reproducible_sum(&[1.0, 2_f64.powi(-53), f64::from_bits(1)]),
            Ok(f64::from_bits(1.0_f64.to_bits() + 1))
        );
    }

    #[test]
    fn reports_only_a_non_finite_final_sum() {
        assert_eq!(
            reproducible_sum(&[f64::MAX, f64::MAX]),
            Err(ReproducibleSumError::NonFiniteResult)
        );
        assert_eq!(
            reproducible_sum(&[f64::INFINITY]),
            Err(ReproducibleSumError::NonFiniteInput)
        );
    }

    #[test]
    fn overflow_boundary_uses_ties_to_even() {
        assert_eq!(reproducible_sum(&[f64::MAX, 2_f64.powi(969)]), Ok(f64::MAX));
        assert_eq!(
            reproducible_sum(&[f64::MAX, 2_f64.powi(970)]),
            Err(ReproducibleSumError::NonFiniteResult)
        );

        let odd = f64::from_bits(1.0_f64.to_bits() + 1);
        assert_eq!(
            reproducible_sum(&[odd, 2_f64.powi(-53)]),
            Ok(f64::from_bits(1.0_f64.to_bits() + 2))
        );
    }

    #[test]
    fn full_budget_fits_the_accumulator_and_excess_is_rejected_first() {
        let maximums = vec![f64::MAX; MAX_REPRODUCIBLE_SUM_INPUTS];
        assert_eq!(
            reproducible_sum(&maximums),
            Err(ReproducibleSumError::NonFiniteResult)
        );
        let excess = vec![0.0; MAX_REPRODUCIBLE_SUM_INPUTS + 1];
        assert_eq!(
            reproducible_sum(&excess),
            Err(ReproducibleSumError::InputBudgetExceeded {
                submitted: MAX_REPRODUCIBLE_SUM_INPUTS + 1,
                maximum: MAX_REPRODUCIBLE_SUM_INPUTS,
            })
        );
    }

    #[test]
    fn signed_residual_and_permutations_are_canonical() {
        let smallest = f64::from_bits(1);
        let first = reproducible_sum(&[f64::MAX, -f64::MAX, -smallest]);
        let second = reproducible_sum(&[-smallest, -f64::MAX, f64::MAX]);
        assert_eq!(first, Ok(-smallest));
        assert_eq!(first, second);
        assert_eq!(reproducible_sum(&[-0.0, 0.0]), Ok(0.0));
        assert_eq!(reproducible_sum(&[-0.0, 0.0]).unwrap().to_bits(), 0);
    }
}
