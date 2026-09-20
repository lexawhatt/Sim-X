//! Sampled explicit height fields, without rendering or camera dependencies.
use crate::{Expression, MathError, geometry::Point};

/// Samples connected grid rows and columns of z=f(x,y), with gaps across
/// undefined or conservatively unresolved intervals. Resolution is selected by
/// the caller for its viewport, not a limit on the mathematical document.
/// `subdivisions` samples each visible grid interval more finely without adding
/// more grid lines. It must be nonzero; unresolved precision is reported.
/// Cancellation is checked at every vertex and interval.
pub fn wireframe(
    expression: &Expression,
    lower: Point,
    upper: Point,
    cells: [usize; 2],
    subdivisions: usize,
    alive: impl Fn() -> bool,
) -> Result<Vec<Vec<Option<[f64; 3]>>>, MathError> {
    let [nx, ny] = cells;
    if nx == 0 || ny == 0 || subdivisions == 0 || upper.x <= lower.x || upper.y <= lower.y {
        return Err(MathError::NumericRange);
    }
    let dx = (upper.x - lower.x) / nx as f64;
    let dy = (upper.y - lower.y) / ny as f64;
    if !dx.is_finite()
        || !dy.is_finite()
        || lower.x + dx / subdivisions as f64 == lower.x
        || lower.y + dy / subdivisions as f64 == lower.y
    {
        return Err(MathError::PrecisionExhausted);
    }
    let mut rows = Vec::new();
    for axis in 0..2 {
        let (across, along) = if axis == 0 { (ny, nx) } else { (nx, ny) };
        let samples = along
            .checked_mul(subdivisions)
            .ok_or(MathError::NumericRange)?;
        for i in 0..=across {
            let mut path = Vec::new();
            let mut previous: Option<[f64; 2]> = None;
            for j in 0..=samples {
                if !alive() {
                    return Err(MathError::Cancelled);
                }
                let j = j as f64 / subdivisions as f64;
                let (ix, iy) = if axis == 0 {
                    (j, i as f64)
                } else {
                    (i as f64, j)
                };
                let x = lower.x + dx * ix;
                let y = lower.y + dy * iy;
                if let Some([a, b]) = previous
                    && expression
                        .screen_box([a.min(x), a.max(x)], [b.min(y), b.max(y)])
                        .is_err()
                {
                    path.push(None);
                }
                path.push(expression.evaluate_at(x, y).ok().map(|z| [x, y, z]));
                previous = Some([x, y]);
            }
            rows.push(path);
        }
    }
    Ok(rows)
}
