//! Bounded exact products with one final binary64 rounding.
//!
//! Four factors contain at most 212 significand bits, so four 64-bit limbs
//! suffice. Powers of two are tracked separately: a tiny intermediate product
//! cannot disappear before a later large factor restores its representability.

use crate::{Error, NumericStage};

const MAX_FACTORS: usize = 4;
const LIMBS: usize = 4;
const FRACTION_MASK: u64 = (1_u64 << 52) - 1;

/// Correctly rounds the exact product of at most four finite binary64 values.
/// A nonzero exact product that rounds to zero is an explicit precision loss.
pub(crate) fn product(factors: &[f64], stage: NumericStage) -> Result<f64, Error> {
    if factors.len() > MAX_FACTORS || factors.iter().any(|factor| !factor.is_finite()) {
        return Err(Error::Numeric(stage));
    }
    if factors.contains(&0.0) {
        return Ok(0.0);
    }
    let mut magnitude = Magnitude([1, 0, 0, 0]);
    let mut exponent = 0_i32;
    let mut negative = false;
    for &factor in factors {
        let bits = factor.to_bits();
        let encoded_exponent = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & FRACTION_MASK;
        let (significand, factor_exponent) = if encoded_exponent == 0 {
            (fraction, -1074)
        } else {
            ((1_u64 << 52) | fraction, encoded_exponent - 1023 - 52)
        };
        magnitude.multiply(significand, stage)?;
        exponent += factor_exponent;
        negative ^= bits >> 63 != 0;
    }
    magnitude.round(exponent, negative, stage)
}

struct Magnitude([u64; LIMBS]);

impl Magnitude {
    fn multiply(&mut self, factor: u64, stage: NumericStage) -> Result<(), Error> {
        let mut carry = 0_u128;
        for limb in &mut self.0 {
            let next = u128::from(*limb) * u128::from(factor) + carry;
            *limb = next as u64;
            carry = next >> 64;
        }
        if carry != 0 {
            return Err(Error::Numeric(stage));
        }
        Ok(())
    }

    fn highest_bit(&self) -> i32 {
        self.0
            .iter()
            .enumerate()
            .rev()
            .find(|(_, limb)| **limb != 0)
            .map_or(0, |(index, limb)| {
                (index * 64 + 63 - limb.leading_zeros() as usize) as i32
            })
    }

    fn bit(&self, index: usize) -> bool {
        self.0
            .get(index / 64)
            .is_some_and(|limb| limb & (1_u64 << (index % 64)) != 0)
    }

    fn any_below(&self, count: usize) -> bool {
        let words = (count / 64).min(LIMBS);
        if self.0[..words].iter().any(|&limb| limb != 0) {
            return true;
        }
        let remaining = count % 64;
        words < LIMBS && remaining != 0 && self.0[words] & ((1_u64 << remaining) - 1) != 0
    }

    fn rounded_shift(&self, shift: i32) -> u64 {
        if shift <= 0 {
            // The caller chooses a maximum 53-bit output significand.
            return self.0[0] << (-shift as u32);
        }
        let shift = shift as usize;
        let word = shift / 64;
        let bit = shift % 64;
        let mut value = self.0.get(word).copied().unwrap_or(0) >> bit;
        if bit != 0 && word + 1 < LIMBS {
            value |= self.0[word + 1] << (64 - bit);
        }
        if self.bit(shift - 1) && (self.any_below(shift - 1) || value & 1 != 0) {
            value += 1;
        }
        value
    }

    fn round(&self, exponent: i32, negative: bool, stage: NumericStage) -> Result<f64, Error> {
        let highest = self.highest_bit();
        let mut value_exponent = exponent + highest;
        if value_exponent > 1023 {
            return Err(Error::Numeric(stage));
        }
        if value_exponent < -1075 {
            return Err(Error::PrecisionLoss(stage));
        }
        let sign = u64::from(negative) << 63;
        if value_exponent < -1022 {
            let significand = self.rounded_shift(-1074 - exponent);
            if significand == 0 {
                return Err(Error::PrecisionLoss(stage));
            }
            return Ok(f64::from_bits(sign | significand));
        }
        let mut significand = self.rounded_shift(highest - 52);
        if significand == 1_u64 << 53 {
            significand >>= 1;
            value_exponent += 1;
        }
        if value_exponent > 1023 {
            return Err(Error::Numeric(stage));
        }
        Ok(f64::from_bits(
            sign | (((value_exponent + 1023) as u64) << 52) | (significand & FRACTION_MASK),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evaluate(factors: &[f64]) -> Result<f64, Error> {
        product(factors, NumericStage::Telemetry)
    }

    #[test]
    fn rescales_tiny_and_large_intermediates_before_rounding() {
        let v = 1e-162;
        let result = evaluate(&[0.5, 1e12, v, v]).unwrap();
        assert!(result > 0.0);
        assert!((result / 5e-313 - 1.0).abs() < 1e-10);
        assert_eq!(result, evaluate(&[v, v, 1e12, 0.5]).unwrap());
        assert_eq!(evaluate(&[f64::MAX, 2.0, 0.5]), Ok(f64::MAX));
        assert_eq!(
            evaluate(&[f64::from_bits(1), 0.5, 2.0]),
            Ok(f64::from_bits(1))
        );
    }

    #[test]
    fn rounds_subnormal_ties_even_and_rejects_true_zero_loss() {
        let tiny = f64::from_bits(1);
        assert_eq!(evaluate(&[tiny, 1.5]), Ok(f64::from_bits(2)));
        assert_eq!(evaluate(&[tiny, 2.5]), Ok(f64::from_bits(2)));
        assert_eq!(
            evaluate(&[tiny, 0.5]),
            Err(Error::PrecisionLoss(NumericStage::Telemetry))
        );
        assert_eq!(
            evaluate(&[tiny, tiny]),
            Err(Error::PrecisionLoss(NumericStage::Telemetry))
        );
        assert_eq!(
            evaluate(&[f64::MAX, 2.0]),
            Err(Error::Numeric(NumericStage::Telemetry))
        );
    }

    #[test]
    fn signs_empty_zero_and_factor_budget_are_explicit() {
        assert_eq!(evaluate(&[]), Ok(1.0));
        assert_eq!(evaluate(&[-2.0, 3.0, 0.5]), Ok(-3.0));
        assert_eq!(evaluate(&[-0.0, -2.0]).unwrap().to_bits(), 0);
        assert_eq!(
            evaluate(&[1.0; 5]),
            Err(Error::Numeric(NumericStage::Telemetry))
        );
        assert_eq!(
            evaluate(&[0.0, f64::INFINITY]),
            Err(Error::Numeric(NumericStage::Telemetry))
        );
    }

    #[test]
    fn seeded_pairs_match_hardware_single_multiply_rounding() {
        let mut seed = 0xe7037ed1a0b428db_u64;
        for _ in 0..20000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let a = f64::from_bits(seed & 0xffef_ffff_ffff_ffff);
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let b = f64::from_bits(seed & 0xffef_ffff_ffff_ffff);
            let expected = a * b;
            if !expected.is_finite() {
                assert_eq!(
                    evaluate(&[a, b]),
                    Err(Error::Numeric(NumericStage::Telemetry))
                );
            } else if expected == 0.0 && a != 0.0 && b != 0.0 {
                assert_eq!(
                    evaluate(&[a, b]),
                    Err(Error::PrecisionLoss(NumericStage::Telemetry))
                );
            } else {
                assert_eq!(evaluate(&[a, b]).unwrap(), expected);
            }
        }
    }
}
