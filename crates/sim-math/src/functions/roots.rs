//! Real indexed-root convention shared by scalar evaluation and domain checks.

pub(crate) fn odd_integer(index: f64) -> bool {
    // Above 2^53 there are no representable odd f64 integers. Never infer
    // integer parity by rounding an index or saturating a float-to-int cast.
    index.abs() < 9_007_199_254_740_992.0 && index.fract() == 0.0 && index % 2.0 != 0.0
}

pub(crate) fn evaluate(index: f64, radicand: f64) -> f64 {
    if !index.is_finite() || !radicand.is_finite() || index == 0.0 {
        return f64::NAN;
    }
    if radicand == 0.0 {
        return if index > 0.0 { 0.0 } else { f64::NAN };
    }
    if radicand < 0.0 && !odd_integer(index) {
        return f64::NAN;
    }
    let magnitude = radicand.abs();
    let value = match index.abs() {
        1.0 => magnitude,
        2.0 => magnitude.sqrt(),
        3.0 => magnitude.cbrt(),
        _ => {
            // Use the signed index directly: a reciprocal of an overflowing
            // intermediate power could erase a representable subnormal result.
            let exponent = index.recip();
            let root = if magnitude == 1.0 {
                1.0
            } else if exponent.is_finite() {
                magnitude.powf(exponent)
            } else {
                (magnitude.ln() / index).exp()
            };
            return root.copysign(radicand);
        }
    };
    let value = if index < 0.0 { value.recip() } else { value };
    value.copysign(radicand)
}
