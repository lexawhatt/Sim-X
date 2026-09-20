//! Geometry-driven supplementary sections, without special-casing expressions.
use super::section;
use crate::{Expression, MathError};

pub(super) fn append(
    expression: &Expression,
    mut coordinates: [Vec<f64>; 3],
    alive: &impl Fn() -> bool,
    output: &mut Vec<[[f64; 3]; 2]>,
) -> Result<(), MathError> {
    if output.is_empty() {
        return Ok(());
    }
    let mut extrema = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for segment in output.iter() {
        if !alive() {
            return Err(MathError::Cancelled);
        }
        for point in segment {
            for (axis, value) in point.iter().enumerate() {
                extrema[axis][0] = extrema[axis][0].min(*value);
                extrema[axis][1] = extrema[axis][1].max(*value);
            }
        }
    }
    for (axis, values) in coordinates.iter_mut().enumerate() {
        // Exact deduplication preserves representable distinctions. No epsilon
        // broadens a flat zero plateau into nearby, nonzero space.
        values.extend(extrema[axis]);
        values.sort_unstable_by(f64::total_cmp);
        values.dedup();
    }
    for (fixed, [low, high]) in extrema.into_iter().enumerate() {
        let u = (fixed + 1) % 3;
        let v = (fixed + 2) % 3;
        for value in [low, high]
            .into_iter()
            .take(if low == high { 1 } else { 2 })
        {
            if !alive() {
                return Err(MathError::Cancelled);
            }
            section::sample(
                expression,
                section::Plane {
                    fixed_axis: fixed,
                    fixed_value: value,
                    u_axis: u,
                    v_axis: v,
                    u: &coordinates[u],
                    v: &coordinates[v],
                },
                alive,
                output,
            )?;
        }
    }
    Ok(())
}
