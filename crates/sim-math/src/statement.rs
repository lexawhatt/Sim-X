//! Classification of an expression row. Scalar definitions are distinct from
//! equations; x/y/z remain coordinates, never mutable document constants.
use crate::{Expression, MathError};
use std::collections::BTreeMap;

/// A mathematical row, independent of its sidebar or graph representation.
#[derive(Clone, Debug)]
pub enum Statement {
    /// y=f(x), or a bare expression depending on x.
    Curve(Expression),
    /// x=f(y).
    InverseCurve(Expression),
    /// A general equation represented by left minus right equals zero.
    Implicit(Expression),
    /// Explicit height field z=f(x,y). Sampling is independent of the view.
    Surface(Expression),
    /// A spatial equation or inequality, visualized as a sampled boundary only.
    Implicit3d(crate::implicit3d::Implicit3d),
    /// Constant expression, with no implicit horizontal graph.
    Scalar(f64),
    /// Explicit integral; calculation is scheduled by the caller.
    Integral {
        /// Integrand in x.
        body: Expression,
        /// Lower endpoint.
        lower: f64,
        /// Upper endpoint.
        upper: f64,
        /// Whether the row explicitly says y=integral(...).
        graph_result: bool,
    },
    /// Scalar arithmetic/functions around non-nested definite integrals.
    IntegralExpression {
        /// Compiled, resumable scalar calculation.
        expression: crate::calculus::IntegralExpression,
        /// Whether the row explicitly assigns the scalar to y.
        graph_result: bool,
    },
    /// Numerical derivative of a function of x.
    Derivative(Expression),
}
/// Extract a single-letter scalar definition. x, y and z are coordinates.
pub fn definition(source: &str) -> Option<(String, &str)> {
    let (left, right) = source.split_once('=')?;
    (left.len() == 1
        && left.as_bytes()[0].is_ascii_alphabetic()
        && !matches!(left, "x" | "y" | "z" | "e")
        && !right.contains(['=', '<', '>', '\u{2264}', '\u{2265}']))
    .then(|| (left.to_owned(), right))
}
impl Statement {
    /// Parse one equation, scalar, graph, derivative or explicit integral.
    /// Nested calculus operators and vector/list objects are not yet supported.
    pub fn parse(source: &str, variables: &BTreeMap<String, f64>) -> Result<Self, MathError> {
        let relation = crate::relation::split(source)?;
        if let Some((left, comparison, right)) = relation
            && comparison != crate::relation::Relation::Equal
        {
            let expression =
                Expression::with_coordinates(&format!("({left})-({right})"), variables)?;
            if !expression.uses("z") {
                return Err(MathError::UnsupportedPlanarInequality);
            }
            return Ok(Self::Implicit3d(crate::implicit3d::Implicit3d {
                expression,
                relation: comparison,
            }));
        }
        let (left, body) = relation.map_or((None, source.trim()), |(left, _, right)| {
            (Some(left), right)
        });
        if let Some(arguments) = call(body, "integral") {
            if left.is_some_and(|l| l != "y") {
                return Err(MathError::Syntax);
            }
            let args = split_arguments(arguments);
            if args.len() != 3 {
                return Err(MathError::Syntax);
            }
            let body = Expression::with_variables(args[2], variables)?;
            if body.uses("y") {
                return Err(MathError::Syntax);
            }
            return Ok(Self::Integral {
                body,
                lower: Expression::constant_with(args[0], variables)?,
                upper: Expression::constant_with(args[1], variables)?,
                graph_result: left.is_some(),
            });
        }
        if let Some(argument) = call(body, "derivative") {
            if left.is_some_and(|l| l != "y") {
                return Err(MathError::Syntax);
            }
            let expression = Expression::with_variables(argument, variables)?;
            if expression.uses("y") {
                return Err(MathError::Syntax);
            }
            return Ok(Self::Derivative(expression));
        }
        if let Some(expression) = crate::calculus::IntegralExpression::parse(body, variables)? {
            if left.is_some_and(|l| l != "y") {
                return Err(MathError::Syntax);
            }
            return Ok(Self::IntegralExpression {
                expression,
                graph_result: left.is_some(),
            });
        }
        if let Some(left) = left {
            let expression = Expression::with_coordinates(body, variables)?;
            if left == "z" && !expression.uses("z") {
                return Ok(Self::Surface(expression));
            }
            if left == "y" && !expression.uses("y") && !expression.uses("z") {
                return Ok(Self::Curve(expression));
            }
            if left == "x" && !expression.uses("x") && !expression.uses("z") {
                return Ok(Self::InverseCurve(expression));
            }
            let expression =
                Expression::with_coordinates(&format!("({left})-({body})"), variables)?;
            return Ok(if expression.uses("z") {
                Self::Implicit3d(crate::implicit3d::Implicit3d {
                    expression,
                    relation: crate::relation::Relation::Equal,
                })
            } else {
                Self::Implicit(expression)
            });
        }
        let expression = Expression::with_variables(body, variables)?;
        if expression.uses("y") {
            return Err(MathError::Syntax);
        }
        if !expression.uses("x") {
            Ok(Self::Scalar(expression.evaluate_at(0.0, 0.0)?))
        } else {
            Ok(Self::Curve(expression))
        }
    }
}
fn call<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let arguments = source.strip_prefix(name)?.trim_start();
    let close = crate::calculus::closing_paren(arguments)?;
    (close + 1 == arguments.len()).then_some(&arguments[1..close])
}
pub(crate) fn split_arguments(source: &str) -> Vec<&str> {
    let mut depth = 0;
    let mut start = 0;
    let mut out = vec![];
    for (i, c) in source.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&source[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&source[start..]);
    out
}
