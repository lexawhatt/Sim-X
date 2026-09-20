//! Presentation-only interpretation of a successfully classified curve row.
use super::{
    formula::{Atom, Formula},
    worker::RowPlot,
};

pub(super) fn xy_curve_hint(formula: &Formula, result: &RowPlot) -> Option<&'static str> {
    // `expression` is set only for Statement::Curve by the worker. Do not infer
    // a graph type from visible samples: an off-screen curve is still a curve,
    // and a failed surface or a scalar integral is not an XY function.
    if result.expression.is_none()
        || result.diagnostic.is_some()
        || result.scalar.is_some()
        || result.pending
        || result.integral_plot.is_some()
        || formula.is_integral()
    {
        return None;
    }
    let identity = matches!(
        formula.rows[0].as_slice(),
        [Atom::Symbol('x')] | [Atom::Symbol('y'), Atom::Symbol('='), Atom::Symbol('x')]
    );
    Some(if identity {
        "y = x  |  XY curve (z = 0)"
    } else {
        "y = f(x)  |  XY curve (z = 0)"
    })
}
