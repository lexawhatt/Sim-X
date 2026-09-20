//! Arithmetic on finite definite integrals, scheduled without blocking a frame.
//! This is numerical composition, not symbolic or nested integration.
use crate::{Expression, IntegralKind, IntegrationJob, MathError};
use std::collections::{BTreeMap, VecDeque};
mod visualization;
pub use visualization::{IntegralOperand, IntegralVisualization};

#[derive(Clone, Debug)]
struct Term {
    name: String,
    expression: Expression,
    lower: f64,
    upper: f64,
}

/// A scalar expression containing one or more non-nested definite integrals.
/// Ordinary scalar functions and arithmetic may surround the integrals.
#[derive(Clone, Debug)]
pub struct IntegralExpression {
    outer: Expression,
    terms: VecDeque<Term>,
    parameter: f64,
}
impl IntegralExpression {
    /// Extract balanced `integral(lower,upper,body)` calls from a scalar.
    /// Returns None if there are no integral calls. Bounds must be scalar;
    /// integrands use x. Free coordinates outside integrals, nested calculus,
    /// invalid names and malformed syntax are rejected.
    pub fn parse(
        source: &str,
        variables: &BTreeMap<String, f64>,
    ) -> Result<Option<Self>, MathError> {
        if !source.contains("integral") {
            return Ok(None);
        }
        let mut prefix = "__quadrature_".to_owned();
        while source.contains(&prefix) || variables.keys().any(|k| k.starts_with(&prefix)) {
            prefix.push('_');
        }
        let mut bindings = variables.clone();
        let mut terms = VecDeque::new();
        let mut outer = String::new();
        let mut cursor = 0;
        while cursor < source.len() {
            let rest = &source[cursor..];
            let name_len = rest
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            if name_len == 0 {
                let ch = rest.chars().next().ok_or(MathError::Syntax)?;
                outer.push(ch);
                cursor += ch.len_utf8();
                continue;
            }
            let name = &rest[..name_len];
            let after = rest[name_len..].trim_start();
            if name != "integral" || !after.starts_with('(') {
                outer.push_str(name);
                cursor += name_len;
                continue;
            }
            let close = closing_paren(after).ok_or(MathError::Syntax)?;
            let args = crate::statement::split_arguments(&after[1..close]);
            if args.len() != 3 {
                return Err(MathError::Syntax);
            }
            let expression = Expression::with_variables(args[2], variables)?;
            if expression.uses("y") {
                return Err(MathError::Syntax);
            }
            let term_name = format!("{prefix}{}", terms.len());
            terms.push_back(Term {
                name: term_name.clone(),
                expression,
                lower: Expression::constant_with(args[0], variables)?,
                upper: Expression::constant_with(args[1], variables)?,
            });
            bindings.insert(term_name.clone(), 0.0);
            outer.push_str(&term_name);
            cursor = source.len() - after.len() + close + 1;
        }
        if terms.is_empty() {
            return Ok(None);
        }
        let outer = Expression::with_variables(&outer, &bindings)?;
        if outer.uses("x") || outer.uses("y") {
            return Err(MathError::Syntax);
        }
        Ok(Some(Self {
            outer,
            terms,
            parameter: variables.get("a").copied().unwrap_or(0.0),
        }))
    }

    /// Start resumable evaluation. Tolerance applies to each integral, not to
    /// the final composed scalar (nonlinear error propagation is not certified).
    /// No partial scalar is exposed; dropping this job cancels its remaining work.
    pub fn into_job(self, tolerance: f64) -> Result<IntegralExpressionJob, MathError> {
        crate::finite(tolerance)?;
        if tolerance <= 0.0 {
            return Err(MathError::NumericRange);
        }
        Ok(IntegralExpressionJob {
            plan: self,
            tolerance,
            current: None,
            result: None,
            failed: None,
        })
    }
}

/// One scalar calculation with bounded scheduling quanta and no total-work quota.
pub struct IntegralExpressionJob {
    plan: IntegralExpression,
    tolerance: f64,
    current: Option<(String, IntegrationJob)>,
    result: Option<f64>,
    failed: Option<MathError>,
}
impl IntegralExpressionJob {
    /// Perform at most `panels` quadrature refinements across all terms.
    /// Returns a scalar only when every term and the outer expression succeed.
    /// Completed results and failures are sticky across subsequent calls.
    pub fn advance(&mut self, panels: usize) -> Result<Option<f64>, MathError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if let Some(result) = self.result {
            return Ok(Some(result));
        }
        let result = self.advance_inner(panels);
        match result {
            Ok(value) => self.result = value,
            Err(error) => self.failed = Some(error),
        }
        result
    }
    fn advance_inner(&mut self, panels: usize) -> Result<Option<f64>, MathError> {
        for _ in 0..panels {
            if self.current.is_none() {
                let Some(term) = self.plan.terms.pop_front() else {
                    break;
                };
                self.current = Some((
                    term.name,
                    IntegrationJob::new(
                        term.expression,
                        term.lower,
                        term.upper,
                        self.plan.parameter,
                        IntegralKind::Signed,
                        self.tolerance,
                    )?,
                ));
            }
            if let Some((name, job)) = &mut self.current
                && let Some(result) = job.advance(1)?
            {
                self.plan.outer.bind_scalar(name, result.value)?;
                self.current = None;
            }
        }
        if self.current.is_none() && self.plan.terms.is_empty() {
            Ok(Some(self.plan.outer.evaluate_at(0.0, 0.0)?))
        } else {
            Ok(None)
        }
    }
}

/// Index of the matching first closing delimiter, not the last ')' in a row.
pub(crate) fn closing_paren(source: &str) -> Option<usize> {
    if !source.starts_with('(') {
        return None;
    }
    let mut depth = 0_usize;
    for (i, c) in source.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
