mod affine;
mod domain;
mod function_ranges;
mod root_ranges;
use crate::{MathError, finite, functions};
use meval::{ContextProvider, FuncEvalError, tokenizer::Token};
use std::collections::BTreeMap;

/// Parsed real expression, using meval for precedence and evaluation.
/// Bindings are explicit document inputs; no UI or system state is consulted.
#[derive(Debug, Clone)]
pub struct Expression {
    compiled: meval::Expr,
    bindings: BTreeMap<String, f64>,
}
impl Expression {
    /// Parse a legacy function of x and a, with explicit multiplication.
    pub fn parse(source: &str) -> Result<Self, MathError> {
        Self::compile(source, &BTreeMap::from([("a".into(), 0.0)]), 1)
    }
    /// Parse a plane expression with x/y and explicitly supplied scalar names.
    pub fn with_variables(
        source: &str,
        bindings: &BTreeMap<String, f64>,
    ) -> Result<Self, MathError> {
        Self::compile(source, bindings, 2)
    }
    /// Parse a spatial expression with x/y/z and explicit scalar bindings.
    /// Coordinate names are never replaced by scalar document bindings.
    pub fn with_coordinates(
        source: &str,
        bindings: &BTreeMap<String, f64>,
    ) -> Result<Self, MathError> {
        Self::compile(source, bindings, 3)
    }
    fn compile(
        source: &str,
        bindings: &BTreeMap<String, f64>,
        dimensions: u8,
    ) -> Result<Self, MathError> {
        for value in bindings.values() {
            finite(*value)?;
        }
        let compiled: meval::Expr = source.parse().map_err(|_| MathError::Syntax)?;
        for token in compiled.iter() {
            match token {
                Token::Var(name)
                    if !(matches!(name.as_str(), "x" | "pi" | "e" | "tau")
                        || (dimensions >= 2 && name == "y")
                        || (dimensions >= 3 && name == "z")
                        || (name != "z" && bindings.contains_key(name))) =>
                {
                    return Err(MathError::Syntax);
                }
                Token::Func(name, Some(count))
                    if !functions::find(name).is_some_and(|f| f.accepts(*count)) =>
                {
                    return Err(MathError::Syntax);
                }
                Token::Func(_, None) => return Err(MathError::Syntax),
                Token::Binary(meval::tokenizer::Operation::Rem) => return Err(MathError::Syntax),
                Token::Number(n) => {
                    finite(*n)?;
                }
                _ => {}
            }
        }
        Ok(Self {
            compiled,
            bindings: bindings.clone(),
        })
    }
    /// Constant-only evaluation; no implicit zero substitution of free variables.
    pub fn constant(source: &str) -> Result<f64, MathError> {
        Self::constant_with(source, &BTreeMap::new())
    }
    /// Evaluate a scalar using explicit bindings; x/y are not scalar constants.
    pub fn constant_with(source: &str, bindings: &BTreeMap<String, f64>) -> Result<f64, MathError> {
        let expression = Self::with_variables(source, bindings)?;
        if expression.uses("x") || expression.uses("y") {
            return Err(MathError::Syntax);
        }
        expression.evaluate_at(0.0, 0.0)
    }
    /// Whether the expression refers to a particular variable.
    pub fn uses(&self, name: &str) -> bool {
        self.compiled
            .iter()
            .any(|t| matches!(t,Token::Var(n) if n==name))
    }
    /// Evaluate a legacy x/a expression. Radians and real finite values only.
    pub fn evaluate(&self, x: f64, parameter: f64) -> Result<f64, MathError> {
        if self.uses("z") {
            return Err(MathError::Syntax);
        }
        self.eval([x, 0.0, 0.0], Some(parameter))
    }
    /// Evaluate an x/y expression with its document bindings.
    pub fn evaluate_at(&self, x: f64, y: f64) -> Result<f64, MathError> {
        if self.uses("z") {
            return Err(MathError::Syntax);
        }
        self.eval([x, y, 0.0], None)
    }
    /// Evaluate all three real coordinates with document bindings.
    /// Nonfinite coordinates/results and undefined real operations are errors.
    pub fn evaluate_xyz(&self, x: f64, y: f64, z: f64) -> Result<f64, MathError> {
        self.eval([x, y, z], None)
    }
    pub(crate) fn bind_scalar(&mut self, name: &str, value: f64) -> Result<(), MathError> {
        let binding = self.bindings.get_mut(name).ok_or(MathError::Syntax)?;
        *binding = finite(value)?;
        Ok(())
    }
    fn eval(&self, [x, y, z]: [f64; 3], parameter: Option<f64>) -> Result<f64, MathError> {
        finite(x)?;
        finite(y)?;
        finite(z)?;
        if let Some(a) = parameter {
            finite(a)?;
        }
        let value = self
            .compiled
            .eval_with_context(Values {
                x,
                y,
                z,
                parameter,
                bindings: &self.bindings,
            })
            .map_err(|_| MathError::Undefined)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(MathError::Undefined)
        }
    }
    /// Conservative domain screening, not certified interval arithmetic.
    pub fn screen_interval(&self, left: f64, right: f64, parameter: f64) -> Result<(), MathError> {
        if self.uses("z") {
            return Err(MathError::Syntax);
        }
        finite(parameter)?;
        let mut bindings = self.bindings.clone();
        bindings.insert("a".into(), parameter);
        self.screen(
            &bindings,
            [left.min(right), left.max(right)],
            [0.0, 0.0],
            [0.0, 0.0],
        )
    }
    /// Screen continuity over a rectangle, conservatively refusing unknown cases.
    pub fn screen_box(&self, x: [f64; 2], y: [f64; 2]) -> Result<(), MathError> {
        if self.uses("z") {
            return Err(MathError::Syntax);
        }
        self.screen(&self.bindings, x, y, [0.0, 0.0])
    }
    /// Screen continuity over a finite closed XYZ box, including planar slices.
    /// Each range must be ascending; rejection can be conservative/incomplete.
    pub fn screen_volume(&self, x: [f64; 2], y: [f64; 2], z: [f64; 2]) -> Result<(), MathError> {
        self.screen(&self.bindings, x, y, z)
    }
    fn screen(
        &self,
        bindings: &BTreeMap<String, f64>,
        x: [f64; 2],
        y: [f64; 2],
        z: [f64; 2],
    ) -> Result<(), MathError> {
        for v in x.into_iter().chain(y).chain(z) {
            finite(v)?;
        }
        if [x, y, z].iter().any(|r| r[0] > r[1]) {
            return Err(MathError::NumericRange);
        }
        domain::screen(&self.compiled, x, y, z, bindings)
    }
}
struct Values<'a> {
    x: f64,
    y: f64,
    z: f64,
    parameter: Option<f64>,
    bindings: &'a BTreeMap<String, f64>,
}
impl ContextProvider for Values<'_> {
    fn get_var(&self, name: &str) -> Option<f64> {
        match name {
            "x" => Some(self.x),
            "y" => Some(self.y),
            "z" => Some(self.z),
            "pi" => Some(std::f64::consts::PI),
            "tau" => Some(std::f64::consts::TAU),
            "e" => Some(std::f64::consts::E),
            "a" => self.parameter.or_else(|| self.bindings.get(name).copied()),
            _ => self.bindings.get(name).copied(),
        }
    }
    fn eval_func(&self, name: &str, args: &[f64]) -> Result<f64, FuncEvalError> {
        let f = functions::find(name).ok_or(FuncEvalError::UnknownFunction)?;
        Ok(f.evaluate(args))
    }
}
