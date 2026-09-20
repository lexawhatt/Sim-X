use crate::{Expression, MathError, finite};
use gauss_quad::GaussLegendre;
use std::num::NonZeroUsize;

/// Integral and geometric area are different mathematical operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegralKind {
    /// Oriented integral of f; reversing bounds reverses the sign.
    Signed,
    /// Integral of |f| over ascending bounds.
    Area,
}

/// Approximate result; the error is an estimator, not a rigorous enclosure.
#[derive(Debug, Clone, Copy)]
pub struct Integral {
    /// Approximate value.
    pub value: f64,
    /// Sum of local quadrature differences.
    pub estimated_error: f64,
    /// Function evaluations performed.
    pub evaluations: usize,
}

/// Resumable adaptive Gaussian quadrature, with no total-work/depth ceiling.
/// Drop the job to cancel it. Callers schedule small batches off the UI thread.
/// The third-party crate owns Gaussian nodes, weights and weighted integration;
/// this adapter owns refinement, domain policy, resumability and error reporting.
pub struct IntegrationJob {
    expression: Expression,
    parameter: f64,
    kind: IntegralKind,
    sign: f64,
    pending: Vec<(f64, f64, f64)>,
    coarse: GaussLegendre,
    fine: GaussLegendre,
    result: Integral,
    correction: f64,
    failed: Option<MathError>,
}

impl IntegrationJob {
    /// Creates a job with an explicit positive absolute accuracy target.
    /// Improper or unverified intervals are rejected rather than assigned an
    /// unrequested principal value. f64 overflow/resolution remain real limits.
    pub fn new(
        expression: Expression,
        from: f64,
        to: f64,
        parameter: f64,
        kind: IntegralKind,
        tolerance: f64,
    ) -> Result<Self, MathError> {
        finite(tolerance)?;
        if tolerance <= 0.0 {
            return Err(MathError::NumericRange);
        }
        expression.screen_interval(from, to, parameter)?;
        let mut pending = Vec::new();
        let (low, high) = (from.min(to), from.max(to));
        finite(high - low)?;
        if low != high {
            for i in 0..16 {
                pending.push((
                    low + (high - low) * f64::from(i) / 16.0,
                    low + (high - low) * f64::from(i + 1) / 16.0,
                    tolerance / 16.0,
                ));
            }
        }
        Ok(Self {
            expression,
            parameter,
            kind,
            sign: if from > to && kind == IntegralKind::Signed {
                -1.0
            } else {
                1.0
            },
            pending,
            coarse: GaussLegendre::new(NonZeroUsize::new(16).ok_or(MathError::NumericRange)?),
            fine: GaussLegendre::new(NonZeroUsize::new(32).ok_or(MathError::NumericRange)?),
            result: Integral {
                value: 0.0,
                estimated_error: 0.0,
                evaluations: 0,
            },
            correction: 0.0,
            failed: None,
        })
    }

    /// Evaluations so far, including pending refinement work.
    pub fn evaluations(&self) -> usize {
        self.result.evaluations
    }

    /// Does at most `panels` refinements, returning None while work remains.
    /// This is a scheduling quantum, never a limit on the user's calculation.
    /// An error is sticky; calling again cannot turn a failed job into success.
    pub fn advance(&mut self, panels: usize) -> Result<Option<Integral>, MathError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        let result = self.advance_inner(panels);
        if let Err(error) = result {
            self.failed = Some(error);
        }
        result
    }

    fn advance_inner(&mut self, panels: usize) -> Result<Option<Integral>, MathError> {
        for _ in 0..panels {
            let Some((l, r, tolerance)) = self.pending.pop() else {
                break;
            };
            if l >= r {
                return Err(MathError::PrecisionExhausted);
            }
            let mut failure = None;
            let mut sample = |x| {
                self.result.evaluations = self.result.evaluations.saturating_add(1);
                match self.expression.evaluate(x, self.parameter) {
                    Ok(v) => {
                        if self.kind == IntegralKind::Area {
                            v.abs()
                        } else {
                            v
                        }
                    }
                    Err(e) => {
                        failure = Some(e);
                        f64::NAN
                    }
                }
            };
            let coarse = self.coarse.integrate(l, r, &mut sample);
            let fine = self.fine.integrate(l, r, &mut sample);
            if let Some(error) = failure {
                return Err(error);
            }
            let error = finite(fine - coarse)?.abs();
            if error <= tolerance {
                let adjusted = fine - self.correction;
                let next = finite(self.result.value + adjusted)?;
                self.correction = (next - self.result.value) - adjusted;
                self.result.value = next;
                self.result.estimated_error = finite(self.result.estimated_error + error)?;
            } else {
                let m = l + (r - l) * 0.5;
                if !(l < m && m < r) || tolerance * 0.5 == 0.0 {
                    return Err(MathError::PrecisionExhausted);
                }
                self.pending.push((l, m, tolerance * 0.5));
                self.pending.push((m, r, tolerance * 0.5));
            }
        }
        Ok(self.pending.is_empty().then_some(Integral {
            value: self.result.value * self.sign,
            ..self.result
        }))
    }
}
