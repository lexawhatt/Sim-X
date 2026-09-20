//! Sampled boundary wireframes for real three-coordinate relations.
//!
//! Cross-sections perpendicular to each coordinate axis use marching triangles.
//! This is a renderer-independent visualization, not a filled volume, an exact
//! mesh, or a proof that every connected component has been found.

mod outline;
mod section;

use crate::{Expression, MathError, relation::Relation};

/// A relation whose residual is its left side minus its right side.
#[derive(Clone, Debug)]
pub struct Implicit3d {
    /// Real expression F(x,y,z); its zero locus is the displayed boundary.
    pub expression: Expression,
    /// Comparison applied to F and zero; strictness is retained for the caller.
    pub relation: Relation,
}

/// Sample F(x,y,z)=0 inside a finite, ascending coordinate box.
///
/// `cells` controls cross-section spacing independently in each coordinate.
/// `subdivisions` refines the in-plane contour grid, without adding more section
/// planes. Both are caller-selected visual resolution, not document/work quotas.
/// A second pass samples the discovered coordinate extrema and adds those values
/// as grid knots. This can reveal flat-face outlines missed by the regular slices;
/// it never snaps a residual to zero or assumes a particular geometric shape.
/// For inequalities only the zero boundary is returned; the relation still
/// distinguishes strict/open boundaries from included ones. No volume is filled.
///
/// Cancellation is checked before work and at each vertex/cell. Invalid boxes,
/// zero resolution and count overflow fail; unrepresentable subdivisions return
/// [`MathError::PrecisionExhausted`]. Cells whose real continuity cannot be
/// established are omitted, not joined across poles. Small components, tangencies
/// and identically-zero regions can be missed; this is not an exact locus solver.
pub fn wireframe(
    relation: &Implicit3d,
    lower: [f64; 3],
    upper: [f64; 3],
    cells: [usize; 3],
    subdivisions: usize,
    alive: impl Fn() -> bool,
) -> Result<Vec<[[f64; 3]; 2]>, MathError> {
    if !alive() {
        return Err(MathError::Cancelled);
    }
    if subdivisions == 0 {
        return Err(MathError::NumericRange);
    }
    let axes = [
        Axis::new(lower[0], upper[0], cells[0])?,
        Axis::new(lower[1], upper[1], cells[1])?,
        Axis::new(lower[2], upper[2], cells[2])?,
    ];
    let refined = [
        axes[0].refined(subdivisions)?,
        axes[1].refined(subdivisions)?,
        axes[2].refined(subdivisions)?,
    ];
    let coordinates = [
        refined[0].coordinates(&alive)?,
        refined[1].coordinates(&alive)?,
        refined[2].coordinates(&alive)?,
    ];
    let mut output = Vec::new();
    for (fixed, axis) in axes.iter().enumerate() {
        let u = (fixed + 1) % 3;
        let v = (fixed + 2) % 3;
        for slice in 0..=axis.cells {
            if !alive() {
                return Err(MathError::Cancelled);
            }
            section::sample(
                &relation.expression,
                section::Plane {
                    fixed_axis: fixed,
                    fixed_value: axis.position(slice),
                    u_axis: u,
                    v_axis: v,
                    u: &coordinates[u],
                    v: &coordinates[v],
                },
                &alive,
                &mut output,
            )?;
        }
    }
    outline::append(&relation.expression, coordinates, &alive, &mut output)?;
    Ok(output)
}

#[derive(Clone, Copy)]
struct Axis {
    lower: f64,
    upper: f64,
    span: f64,
    cells: usize,
}
impl Axis {
    fn new(lower: f64, upper: f64, cells: usize) -> Result<Self, MathError> {
        if !lower.is_finite() || !upper.is_finite() || lower >= upper || cells == 0 {
            return Err(MathError::NumericRange);
        }
        cells.checked_add(1).ok_or(MathError::NumericRange)?;
        let span = upper - lower;
        let step = span / cells as f64;
        if !span.is_finite() {
            return Err(MathError::NumericRange);
        }
        if step <= 0.0 || lower + step == lower || upper - step == upper {
            return Err(MathError::PrecisionExhausted);
        }
        Ok(Self {
            lower,
            upper,
            span,
            cells,
        })
    }
    fn refined(self, subdivisions: usize) -> Result<Self, MathError> {
        let count = self
            .cells
            .checked_mul(subdivisions)
            .ok_or(MathError::NumericRange)?;
        Self::new(self.lower, self.upper, count)
    }
    fn position(self, index: usize) -> f64 {
        if index == self.cells {
            self.upper
        } else {
            self.lower + self.span * (index as f64 / self.cells as f64)
        }
    }
    fn coordinates(self, alive: &impl Fn() -> bool) -> Result<Vec<f64>, MathError> {
        (0..=self.cells)
            .map(|index| {
                if !alive() {
                    return Err(MathError::Cancelled);
                }
                Ok(self.position(index))
            })
            .collect()
    }
}
