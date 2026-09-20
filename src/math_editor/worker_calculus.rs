//! Presentation adapter for the two resumable calculus result shapes.
use sim_math::{Integral, IntegrationJob, MathError, calculus::IntegralExpressionJob};

/// One mathematical request's completed scalars, independent of camera sampling.
/// This is not a history cache: source edits replace it; activation changes
/// invalidate only their own row, since these rows cannot define other values.
#[derive(Default)]
pub(super) struct CalculationCache {
    sources: Vec<Result<String, String>>,
    calculate: Vec<bool>,
    values: Vec<Option<(f64, Option<Integral>)>>,
}
impl CalculationCache {
    pub fn prepare(&mut self, request: &super::worker::Request) {
        if self.sources != request.sources {
            self.sources = request.sources.clone();
            self.values = vec![None; request.sources.len()];
        } else {
            for (index, value) in self.values.iter_mut().enumerate() {
                if self.calculate.get(index) != request.calculate.get(index) {
                    *value = None;
                }
            }
        }
        self.calculate = request.calculate.clone();
    }
    pub fn get(&self, index: usize) -> Option<(f64, Option<Integral>)> {
        self.values.get(index).copied().flatten()
    }
    pub fn record(&mut self, index: usize, value: (f64, Option<Integral>)) {
        if let Some(slot) = self.values.get_mut(index) {
            *slot = Some(value);
        }
    }
}

pub(super) enum Calculation {
    Integral(Box<IntegrationJob>),
    Scalar(Box<IntegralExpressionJob>),
}
impl Calculation {
    pub fn advance(&mut self, panels: usize) -> Result<Option<(f64, Option<Integral>)>, MathError> {
        match self {
            Self::Integral(job) => Ok(job
                .advance(panels)?
                .map(|result| (result.value, Some(result)))),
            Self::Scalar(job) => Ok(job.advance(panels)?.map(|value| (value, None))),
        }
    }
}
