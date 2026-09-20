//! View-dependent 3D sampling policy; all work and stitching runs off the UI.
use super::worker::{Request, RowPlot};
use sim_math::{MathError, implicit3d::Implicit3d, relation::Relation};

pub(super) fn implicit(
    row: &mut RowPlot,
    relation: &Implicit3d,
    request: &Request,
    alive: impl Fn() -> bool,
) -> Result<(), MathError> {
    row.boundary = Some(relation.relation);
    if !request.spatial {
        row.diagnostic = Some("Switch to 3D to view this boundary".into());
        return Ok(());
    }
    // Same invisible Z range as the axes and surface clipper. The view defines
    // sampling coverage, never the allowed mathematical/document domain.
    let bounds = super::spatial_clip::Bounds::from_xy(request.lower, request.upper)
        .ok_or(MathError::NumericRange)?;
    let lower = bounds.ranges.map(|range| range[0]);
    let upper = bounds.ranges.map(|range| range[1]);
    let count = (request.pixels[0].min(request.pixels[1]) * 0.56 / 40.0)
        .ceil()
        .max(2.0) as usize;
    let cells = [count; 3];
    let segments = sim_math::implicit3d::wireframe(relation, lower, upper, cells, 8, &alive)?;
    row.surface = super::surface_paths::stitch(&segments, lower, upper, alive)?;
    row.diagnostic = Some(
        if row.surface.is_empty() {
            "No resolved boundary in this view"
        } else if relation.relation == Relation::Equal {
            "Sampled 3D boundary"
        } else if relation.relation.includes_boundary() {
            "Closed boundary only; no volume fill"
        } else {
            "Open boundary only; no volume fill"
        }
        .into(),
    );
    Ok(())
}
