//! Exact algebraic eligibility for integral illustrations, separate from
//! quadrature and from the caller's finite display sampling.
use super::IntegralExpression;
use crate::Expression;

/// One integrand and its explicit bounds, with an optional linear coefficient.
#[derive(Clone, Debug)]
pub struct IntegralOperand {
    /// Real function of x with captured document bindings.
    pub expression: Expression,
    /// Ordered integration endpoints, including reversed and equal bounds.
    pub bounds: [f64; 2],
    /// Signed outer multiplier; None means the surrounding expression is nonlinear.
    pub coefficient: Option<f64>,
}
impl IntegralOperand {
    /// Multiplier for the displayed curve. The contribution's sign is separate,
    /// so reversed limits do not reflect the integrand across the x axis.
    pub fn display_scale(&self) -> f64 {
        self.coefficient.unwrap_or(1.0).abs()
    }
    /// Orientation of the contribution over ascending bounds. None forbids
    /// presenting the integrand's area as the surrounding expression's value.
    pub fn direction(&self) -> Option<f64> {
        self.coefficient.map(|c| {
            if c == 0.0 {
                return 0.0;
            }
            c.signum()
                * if self.bounds[1] > self.bounds[0] {
                    1.0
                } else if self.bounds[1] < self.bounds[0] {
                    -1.0
                } else {
                    0.0
                }
        })
    }
}

/// Mathematically justified illustration for a bare integral expression.
#[derive(Clone, Debug)]
pub enum IntegralVisualization {
    /// Difference of two oppositely signed contributions on the same interval.
    /// Display the scaled curves and signed vertical separation over ascending x.
    Between {
        /// Positive oriented contribution, not necessarily the upper curve.
        positive: IntegralOperand,
        /// Negative oriented contribution, not necessarily the lower curve.
        negative: IntegralOperand,
    },
    /// Separate contributions, each on its own interval, or reference-only
    /// curves when the outer expression is nonlinear. No invented common area.
    Terms {
        /// All operands, in source order.
        operands: Vec<IntegralOperand>,
        /// Affine scalar offset. None means no linear area interpretation exists.
        offset: Option<f64>,
    },
}
impl IntegralVisualization {
    /// Illustration of a single definite integral, without algebraic composition.
    pub fn single(expression: Expression, bounds: [f64; 2]) -> Self {
        Self::Terms {
            operands: vec![IntegralOperand {
                expression,
                bounds,
                coefficient: Some(1.0),
            }],
            offset: Some(0.0),
        }
    }
}
impl IntegralExpression {
    /// Derive area semantics structurally, never from the integral's numeric value.
    /// Nonlinear combinations still expose reference curves, but not a false fill.
    pub fn visualization(&self) -> IntegralVisualization {
        let names = self.terms.iter().map(|t| t.name.as_str()).collect();
        let affine = self.outer.affine(&names);
        let operands: Vec<_> = self
            .terms
            .iter()
            .map(|term| IntegralOperand {
                expression: term.expression.clone(),
                bounds: [term.lower, term.upper],
                coefficient: affine
                    .as_ref()
                    .map(|a| a.weights.get(&term.name).copied().unwrap_or(0.0)),
            })
            .collect();
        if let [a, b] = operands.as_slice()
            && affine.as_ref().is_some_and(|a| a.offset == 0.0)
            && a.bounds[0].min(a.bounds[1]) == b.bounds[0].min(b.bounds[1])
            && a.bounds[0].max(a.bounds[1]) == b.bounds[0].max(b.bounds[1])
            && let (Some(da), Some(db)) = (a.direction(), b.direction())
            && da * db < 0.0
        {
            let (positive, negative) = if da > 0.0 { (a, b) } else { (b, a) };
            return IntegralVisualization::Between {
                positive: positive.clone(),
                negative: negative.clone(),
            };
        }
        IntegralVisualization::Terms {
            operands,
            offset: affine.map(|a| a.offset),
        }
    }
}
