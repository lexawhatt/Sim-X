//! Projected grid density and XYZ tick labels in the open orthographic view.
use super::{
    assets::Fonts,
    axis_ticks,
    axis_view::{Labels, projected_ticks, spacing},
    drawing::Scene,
    layout::Layout,
    spatial::Projector,
    spatial_clip::Bounds,
    state::MathState,
};
use sim_logic::prelude::Color;

pub(super) fn draw(
    out: &mut Scene,
    state: &MathState,
    layout: &Layout,
    projector: &Projector,
    fonts: &Fonts,
) {
    let rect = layout.canvas;
    let camera = state.camera;
    let Some(low) = camera.unproject([rect.x, rect.y + rect.h], rect) else {
        return;
    };
    let Some(high) = camera.unproject([rect.x + rect.w, rect.y], rect) else {
        return;
    };
    let Some(bounds) = Bounds::for_view(camera, rect) else {
        return;
    };
    let ranges = bounds.ranges.map(|[a, b]| (a, b));
    let depth = ranges[2].1;
    let mut vectors = [[0.0_f64; 2]; 3];
    for (axis, vector) in vectors.iter_mut().enumerate() {
        let a = [camera.x, camera.y, 0.0];
        let mut b = a;
        b[axis] += depth;
        if let (Some(a), Some(b)) = (projector.point(a), projector.point(b)) {
            *vector = [
                f64::from(b[0] - a[0]) / depth,
                f64::from(b[1] - a[1]) / depth,
            ];
        }
    }
    // Axis ranges and the invisible surface volume agree. Orbit changes only
    // projection, not the displayed mathematical interval or domain sampling.
    let cross = (vectors[0][0] * vectors[1][1] - vectors[0][1] * vectors[1][0]).abs();
    let mut line = |a, b, color, width| {
        if let (Some(a), Some(b)) = (projector.point(a), projector.point(b)) {
            out.line(a, b, color, width, rect, 0.0);
        }
    };
    for axis in 0..2 {
        // Perpendicular separation between projected grid lines, not the axis
        // length: edge-on grids must thin out instead of turning into a solid fill.
        let other = vectors[1 - axis];
        let scale = cross / other[0].hypot(other[1]);
        let (lo, hi) = if axis == 0 {
            (low.x, high.x)
        } else {
            (low.y, high.y)
        };
        if let Some(s) = spacing(scale, lo, hi, fonts) {
            for (step, alpha) in [(s.minor, 0.36), (s.major, 0.85)] {
                for v in axis_ticks::ticks(lo, hi, step) {
                    if v == 0.0 {
                        continue;
                    }
                    let (a, b) = if axis == 0 {
                        ([v, low.y, 0.0], [v, high.y, 0.0])
                    } else {
                        ([low.x, v, 0.0], [high.x, v, 0.0])
                    };
                    line(a, b, Color::rgb8(49, 62, 77).with_alpha(alpha), 1.0);
                }
            }
        }
    }
    let colors = [
        Color::rgb8(186, 111, 133),
        Color::rgb8(126, 146, 115),
        Color::rgb8(119, 177, 241).with_alpha(state.spatial.blend as f32),
    ];
    for (axis, &(lo, hi)) in ranges.iter().enumerate() {
        let mut a = [0.0; 3];
        let mut b = a;
        a[axis] = lo;
        b[axis] = hi;
        line(a, b, colors[axis], 1.5);
    }
    let mut labels = Labels::new(layout, fonts, true);
    // Reserve axis names before values, using the same overlap policy.
    for (axis, name) in ["X", "Y", "Z"].iter().enumerate() {
        if axis == 2 && state.spatial.blend == 0.0 {
            continue;
        }
        let mut p = [0.0; 3];
        p[axis] = ranges[axis].1;
        if let Some(p) = projector.point(p) {
            let left = p[0] - fonts.width(2, name) - 10.0;
            // A vertical Z endpoint is often just below the top overlay. Try an
            // inward position before dropping the name, still reserving it
            // before numeric ticks and never forcing an overlapping label.
            for position in [
                [p[0] + 10.0, p[1] - 10.0],
                [p[0] + 10.0, p[1] + 20.0],
                [left, p[1] - 10.0],
                [left, p[1] + 20.0],
            ] {
                if labels.put(out, (*name).into(), position, colors[axis]) {
                    break;
                }
            }
        }
    }
    for (axis, &(lo, hi)) in ranges.iter().enumerate() {
        let v = vectors[axis];
        let scale = v[0].hypot(v[1]);
        let Some(step) = projected_ticks(scale, lo, hi, fonts) else {
            continue;
        };
        let normal = [(-v[1] / scale) as f32, (v[0] / scale) as f32];
        for value in axis_ticks::ticks(lo, hi, step) {
            if value == 0.0 && axis != 0 {
                continue;
            }
            let mut p = [0.0; 3];
            p[axis] = value;
            if let Some(p) = projector.point(p) {
                out.line(
                    [p[0] - normal[0] * 3.0, p[1] - normal[1] * 3.0],
                    [p[0] + normal[0] * 3.0, p[1] + normal[1] * 3.0],
                    colors[axis],
                    1.0,
                    rect,
                    0.3,
                );
                labels.put(
                    out,
                    axis_ticks::label(value, step),
                    [p[0] + normal[0] * 10.0 + 4.0, p[1] + normal[1] * 10.0 + 5.0],
                    colors[axis],
                );
            }
        }
    }
}
