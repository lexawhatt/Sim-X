//! Continuity envelopes for real indexed roots, not complex branch choices.
use super::domain::Range;
use crate::{MathError, functions::roots};

pub(super) fn screen(index: Range, radicand: Range) -> Result<Range, MathError> {
    if index.0 <= 0.0 && index.1 >= 0.0 {
        return Err(MathError::UnresolvedDomain);
    }
    if radicand.0 < 0.0 && (index.0 != index.1 || !roots::odd_integer(index.0)) {
        return Err(MathError::UnresolvedDomain);
    }
    if index.1 < 0.0 && radicand.0 <= 0.0 && radicand.1 >= 0.0 {
        return Err(MathError::UnresolvedDomain);
    }
    // With positive radicands, log(root(n,x))=log(x)/n. On a rectangle
    // excluding n=0 its extrema are at corners, including when x crosses 1.
    // Fixed odd-index roots are monotone on their continuous real branches.
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for n in [index.0, index.1] {
        for x in [radicand.0, radicand.1] {
            let value = roots::evaluate(n, x);
            if !value.is_finite() {
                return Err(MathError::UnresolvedDomain);
            }
            low = low.min(value);
            high = high.max(value);
        }
    }
    // Preserve the known nonnegative boundary for nested even roots. Widening
    // an exact zero below zero would incorrectly reject root(2,root(4,x)).
    Ok((
        if radicand.0 >= 0.0 && low == 0.0 {
            0.0
        } else {
            low.next_down()
        },
        if radicand.1 <= 0.0 && high == 0.0 {
            0.0
        } else {
            high.next_up()
        },
    ))
}
