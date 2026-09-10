//! Correctly rounded evaluation of bounded binary64 product quotients.
//!
//! Each finite input is decoded into an integer significand and a power of two.
//! Integer products and the final division remain exact; the result is rounded
//! once using IEEE round-to-nearest, ties-to-even. This is intentionally
//! crate-private: domains retain ownership of formula meaning and diagnostics.

use std::cmp::Ordering;

const MAX_FACTORS: usize = 9;
// Nine 53-bit factors need at most 477 product bits. Normalization for a
// 53-bit quotient and doubled rounding remainder raises the proven maximum to
// 531 bits, which fits these nine limbs.
const BIG_LIMBS: usize = 9;
const FRACTION_BITS: u32 = 52;
const FRACTION_MASK: u64 = (1_u64 << FRACTION_BITS) - 1;
const SIGN_MASK: u64 = 1_u64 << 63;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScaledProductError {
    FactorBudgetExceeded,
    NonFiniteInput,
    ZeroDenominator,
    InternalCapacityExceeded,
    NonFiniteResult,
    Underflow,
}

/// Evaluates `product(numerators) / product(denominators)` exactly, then rounds
/// once to binary64 using round-to-nearest, ties-to-even.
pub(crate) fn scaled_product_quotient(
    numerators: &[f64],
    denominators: &[f64],
) -> Result<f64, ScaledProductError> {
    if numerators.len() + denominators.len() > MAX_FACTORS {
        return Err(ScaledProductError::FactorBudgetExceeded);
    }
    if numerators
        .iter()
        .chain(denominators)
        .any(|value| !value.is_finite())
    {
        return Err(ScaledProductError::NonFiniteInput);
    }
    if denominators.contains(&0.0) {
        return Err(ScaledProductError::ZeroDenominator);
    }
    if numerators.contains(&0.0) {
        return Ok(0.0);
    }

    let (numerator, numerator_exponent, numerator_negative) = exact_product(numerators)?;
    let (denominator, denominator_exponent, denominator_negative) = exact_product(denominators)?;
    let binary_exponent = numerator_exponent - denominator_exponent;
    let ratio_exponent = floor_log2_ratio(&numerator, &denominator)?;
    let value_exponent = binary_exponent + ratio_exponent;
    let negative = numerator_negative ^ denominator_negative;

    if value_exponent > 1_023 {
        return Err(ScaledProductError::NonFiniteResult);
    }
    if value_exponent < -1_075 {
        return Err(ScaledProductError::Underflow);
    }

    if value_exponent >= -1_022 {
        let shift = binary_exponent - value_exponent + FRACTION_BITS as i32;
        let mut significand = rounded_quotient(&numerator, &denominator, shift)?;
        let mut rounded_exponent = value_exponent;
        if significand == 1_u64 << (FRACTION_BITS + 1) {
            significand >>= 1;
            rounded_exponent += 1;
        }
        if rounded_exponent > 1_023 {
            return Err(ScaledProductError::NonFiniteResult);
        }
        debug_assert!(
            (1_u64 << FRACTION_BITS..1_u64 << (FRACTION_BITS + 1)).contains(&significand)
        );
        let sign = u64::from(negative) << 63;
        let encoded_exponent = (rounded_exponent + 1_023) as u64;
        return Ok(f64::from_bits(
            sign | (encoded_exponent << FRACTION_BITS) | (significand & FRACTION_MASK),
        ));
    }

    let significand = rounded_quotient(&numerator, &denominator, binary_exponent + 1_074)?;
    if significand == 0 {
        return Err(ScaledProductError::Underflow);
    }
    debug_assert!(significand <= 1_u64 << FRACTION_BITS);
    Ok(f64::from_bits((u64::from(negative) << 63) | significand))
}

fn exact_product(values: &[f64]) -> Result<(BigMagnitude, i32, bool), ScaledProductError> {
    let mut product = BigMagnitude::one();
    let mut exponent = 0_i32;
    let mut negative = false;
    for &value in values {
        let bits = value.to_bits();
        let encoded_exponent = ((bits >> FRACTION_BITS) & 0x7ff) as i32;
        let fraction = bits & FRACTION_MASK;
        let (significand, factor_exponent) = if encoded_exponent == 0 {
            (fraction, -1_074)
        } else {
            (
                (1_u64 << FRACTION_BITS) | fraction,
                encoded_exponent - 1_023 - FRACTION_BITS as i32,
            )
        };
        product.multiply(significand)?;
        exponent += factor_exponent;
        negative ^= bits & SIGN_MASK != 0;
    }
    Ok((product, exponent, negative))
}

fn floor_log2_ratio(
    numerator: &BigMagnitude,
    denominator: &BigMagnitude,
) -> Result<i32, ScaledProductError> {
    let bit_difference = numerator.bit_length() as i32 - denominator.bit_length() as i32;
    let comparison = if bit_difference >= 0 {
        numerator.compare(&denominator.shifted(bit_difference as usize)?)
    } else {
        numerator
            .shifted((-bit_difference) as usize)?
            .compare(denominator)
    };
    if comparison == Ordering::Less {
        Ok(bit_difference - 1)
    } else {
        Ok(bit_difference)
    }
}

fn rounded_quotient(
    numerator: &BigMagnitude,
    denominator: &BigMagnitude,
    binary_shift: i32,
) -> Result<u64, ScaledProductError> {
    let (scaled_numerator, scaled_denominator) = if binary_shift >= 0 {
        (
            numerator.shifted(binary_shift as usize)?,
            denominator.clone(),
        )
    } else {
        (
            numerator.clone(),
            denominator.shifted((-binary_shift) as usize)?,
        )
    };
    let (floor, remainder) = scaled_numerator.divide_to_u64(&scaled_denominator)?;
    let twice_remainder = remainder.multiplied(2)?;
    let round_up = match twice_remainder.compare(&scaled_denominator) {
        Ordering::Greater => true,
        Ordering::Equal => floor & 1 == 1,
        Ordering::Less => false,
    };
    floor
        .checked_add(u64::from(round_up))
        .ok_or(ScaledProductError::InternalCapacityExceeded)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BigMagnitude {
    limbs: [u64; BIG_LIMBS],
}

impl BigMagnitude {
    fn one() -> Self {
        let mut value = Self {
            limbs: [0; BIG_LIMBS],
        };
        value.limbs[0] = 1;
        value
    }

    fn multiply(&mut self, factor: u64) -> Result<(), ScaledProductError> {
        let mut carry = 0_u128;
        for limb in &mut self.limbs {
            let product = u128::from(*limb) * u128::from(factor) + carry;
            *limb = product as u64;
            carry = product >> 64;
        }
        if carry != 0 {
            return Err(ScaledProductError::InternalCapacityExceeded);
        }
        Ok(())
    }

    fn multiplied(&self, factor: u64) -> Result<Self, ScaledProductError> {
        let mut result = self.clone();
        result.multiply(factor)?;
        Ok(result)
    }

    fn shifted(&self, shift: usize) -> Result<Self, ScaledProductError> {
        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        let mut result = Self {
            limbs: [0; BIG_LIMBS],
        };
        for (source_index, &limb) in self.limbs.iter().enumerate() {
            if limb == 0 {
                continue;
            }
            let target_index = source_index + word_shift;
            if target_index >= BIG_LIMBS {
                return Err(ScaledProductError::InternalCapacityExceeded);
            }
            result.limbs[target_index] |= limb << bit_shift;
            if bit_shift != 0 {
                let high = limb >> (64 - bit_shift);
                if high != 0 {
                    if target_index + 1 >= BIG_LIMBS {
                        return Err(ScaledProductError::InternalCapacityExceeded);
                    }
                    result.limbs[target_index + 1] |= high;
                }
            }
        }
        Ok(result)
    }

    fn compare(&self, other: &Self) -> Ordering {
        self.limbs.iter().rev().cmp(other.limbs.iter().rev())
    }

    fn bit_length(&self) -> usize {
        self.limbs
            .iter()
            .enumerate()
            .rev()
            .find(|(_, limb)| **limb != 0)
            .map_or(0, |(index, limb)| {
                index * 64 + (64 - limb.leading_zeros() as usize)
            })
    }

    fn leading_binary64_integer(&self) -> (u64, i32) {
        let shift = self.bit_length().saturating_sub(53);
        let word = shift / 64;
        let bit = shift % 64;
        let mut leading = self.limbs[word] >> bit;
        if bit != 0 && word + 1 < BIG_LIMBS {
            leading |= self.limbs[word + 1] << (64 - bit);
        }
        (leading, shift as i32)
    }

    fn subtract_assign(&mut self, other: &Self) {
        debug_assert!(self.compare(other) != Ordering::Less);
        let mut borrowed = false;
        for index in 0..BIG_LIMBS {
            let (without_other, first_borrow) =
                self.limbs[index].overflowing_sub(other.limbs[index]);
            let (difference, second_borrow) = without_other.overflowing_sub(u64::from(borrowed));
            self.limbs[index] = difference;
            borrowed = first_borrow || second_borrow;
        }
        debug_assert!(!borrowed);
    }

    fn divide_to_u64(&self, denominator: &Self) -> Result<(u64, Self), ScaledProductError> {
        let (numerator_leading, numerator_shift) = self.leading_binary64_integer();
        let (denominator_leading, denominator_shift) = denominator.leading_binary64_integer();
        let approximate = (numerator_leading as f64 / denominator_leading as f64)
            * 2.0_f64.powi(numerator_shift - denominator_shift);
        let mut quotient = approximate as u64;

        // Leading-bit truncation leaves this estimate only a few integer units
        // away. Exact multiplication corrects it; the bounded binary fallback
        // retains correctness if this implementation detail changes.
        for _ in 0..16 {
            let product = denominator.multiplied(quotient)?;
            match product.compare(self) {
                Ordering::Greater => {
                    quotient = quotient
                        .checked_sub(1)
                        .ok_or(ScaledProductError::InternalCapacityExceeded)?;
                }
                Ordering::Equal => return Ok((quotient, Self::zero())),
                Ordering::Less => {
                    let next = quotient
                        .checked_add(1)
                        .ok_or(ScaledProductError::InternalCapacityExceeded)?;
                    let next_product = denominator.multiplied(next)?;
                    if next_product.compare(self) == Ordering::Greater {
                        let mut remainder = self.clone();
                        remainder.subtract_assign(&product);
                        return Ok((quotient, remainder));
                    }
                    quotient = next;
                }
            }
        }

        self.divide_to_u64_binary(denominator)
    }

    fn divide_to_u64_binary(&self, denominator: &Self) -> Result<(u64, Self), ScaledProductError> {
        let mut remainder = self.clone();
        if remainder.compare(denominator) == Ordering::Less {
            return Ok((0, remainder));
        }
        let highest_shift = remainder.bit_length() - denominator.bit_length();
        if highest_shift >= 64 {
            return Err(ScaledProductError::InternalCapacityExceeded);
        }
        let mut quotient = 0_u64;
        for shift in (0..=highest_shift).rev() {
            let shifted_denominator = denominator.shifted(shift)?;
            if remainder.compare(&shifted_denominator) != Ordering::Less {
                remainder.subtract_assign(&shifted_denominator);
                quotient |= 1_u64 << shift;
            }
        }
        Ok((quotient, remainder))
    }

    fn zero() -> Self {
        Self {
            limbs: [0; BIG_LIMBS],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ScaledProductError, scaled_product_quotient};

    #[test]
    fn avoids_intermediate_overflow_and_underflow_associations() {
        assert_eq!(
            scaled_product_quotient(&[f64::MAX, 2.0], &[f64::MAX]),
            Ok(2.0)
        );
        assert_eq!(
            scaled_product_quotient(
                &[2.0_f64.powi(512), 2.0_f64.powi(512)],
                &[2.0_f64.powi(512), 2.0_f64.powi(512)]
            ),
            Ok(1.0)
        );
        assert_eq!(
            scaled_product_quotient(
                &[2.0_f64.powi(1_023)],
                &[2.0_f64.powi(511), 2.0_f64.powi(511)]
            ),
            Ok(2.0)
        );
    }

    #[test]
    fn rounds_once_at_the_finite_overflow_boundary() {
        assert_eq!(
            scaled_product_quotient(
                &[
                    1.269_511_161_302_448_5e308,
                    1.301_001_203_388_323_8,
                    1.301_001_203_388_323_8,
                ],
                &[1.093_297_059_508_779_9; 2]
            ),
            Ok(f64::MAX)
        );
        assert_eq!(
            scaled_product_quotient(
                &[
                    1.230_095_462_592_292e308,
                    1.692_033_924_774_457_1,
                    1.692_033_924_774_457_1,
                ],
                &[1.399_654_015_922_604; 2]
            ),
            Err(ScaledProductError::NonFiniteResult)
        );
    }

    #[test]
    fn rounds_once_at_the_minimum_subnormal_boundary() {
        assert_eq!(
            scaled_product_quotient(
                &[
                    9.477_621_891_619_394e300,
                    1.132_244_946_103e-312,
                    1.132_244_946_103e-312,
                ],
                &[2.217_750_825_257_271_5; 2]
            ),
            Ok(f64::from_bits(1))
        );
    }

    #[test]
    fn reports_only_true_final_range_failures() {
        assert_eq!(
            scaled_product_quotient(&[f64::MAX, 2.0], &[]),
            Err(ScaledProductError::NonFiniteResult)
        );
        assert_eq!(
            scaled_product_quotient(&[f64::from_bits(1), 0.5], &[]),
            Err(ScaledProductError::Underflow)
        );
    }

    #[test]
    fn factor_permutations_have_one_canonical_result() {
        let first = scaled_product_quotient(&[3.0, f64::MAX, 0.5], &[f64::MAX, 1.5]);
        let second = scaled_product_quotient(&[0.5, 3.0, f64::MAX], &[1.5, f64::MAX]);
        assert_eq!(first, Ok(1.0));
        assert_eq!(first, second);
    }
}
