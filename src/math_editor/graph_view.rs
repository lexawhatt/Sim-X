use super::{drawing::*, layout::Layout, state::MathState};
use sim_math::geometry::{Construction, Shape};

pub(super) fn draw(
    scene: &mut Scene,
    state: &MathState,
    layout: &Layout,
    fonts: &super::assets::Fonts,
) {
    if state.spatial.visible() {
        super::spatial_view::draw(scene, state, layout, fonts);
        return;
    }
    let rect = layout.canvas;
    let camera = state.camera;
    super::axis_view::plane(scene, state, layout, fonts);
    if let Some(plot) = &state.plot {
        for (index, row) in plot.rows.iter().enumerate() {
            if !state.document.visible.get(index).copied().unwrap_or(false) {
                continue;
            }
            let mut style = state.document.styles[index];
            style.opacity *= state.motion.plot_opacity;
            if let Some(picture) = &row.integral_plot {
                let progress = (!state.stale && row.scalar.is_some() && !row.pending)
                    .then(|| state.area_progress.get(index).copied().unwrap_or(0.0));
                super::integral_view::draw(
                    scene,
                    picture,
                    style,
                    rect,
                    progress,
                    |p| camera.project(p, rect),
                    true,
                );
            }
            let projected: Vec<_> = row
                .segments
                .iter()
                .filter_map(|[a, b]| {
                    camera
                        .project(*a, rect)
                        .zip(camera.project(*b, rect))
                        .map(|(a, b)| [a, b])
                })
                .collect();
            for path in super::contour_display::paths(&projected) {
                style.draw_path(scene, &path, rect);
            }
            let mut run = vec![];
            for point in row.points.iter().chain(std::iter::once(&None)) {
                if let Some(p) = point.and_then(|p| camera.project(p, rect)) {
                    run.push(p);
                } else {
                    let reduced = super::curve_display::simplify(&run, 0.35);
                    style.draw_path(scene, &reduced, rect);
                    run.clear();
                }
            }
        }
    }
    let resolved = state.resolved();
    for shape in state.document.geometry.shapes() {
        let (Shape::Segment(a, b) | Shape::Circle(a, b)) = *shape;
        let (Some(Some(a)), Some(Some(b))) = (resolved.get(a.index()), resolved.get(b.index()))
        else {
            continue;
        };
        match shape {
            Shape::Segment(..) => {
                if let (Some(a), Some(b)) = (camera.project(*a, rect), camera.project(*b, rect)) {
                    scene.line(a, b, GEOMETRY, 1.8, rect, 2.0);
                }
            }
            Shape::Circle(..) => {
                let radius = (a.x - b.x).hypot(a.y - b.y) * camera.scale;
                let Some(center) = camera.project(*a, rect) else {
                    continue;
                };
                if !radius.is_finite() || radius <= 0.0 {
                    continue;
                }
                // Tessellation quality follows visible pixel resolution, never
                // the number of constructions allowed in the document.
                let segments = (std::f64::consts::TAU * radius.sqrt())
                    .ceil()
                    .max(16.0)
                    .min(f64::from((rect.w + rect.h) * 2.0))
                    as usize;
                let point = |i: usize| {
                    let angle = i as f64 / segments as f64 * std::f64::consts::TAU;
                    [
                        (f64::from(center[0]) + radius * angle.cos()) as f32,
                        (f64::from(center[1]) + radius * angle.sin()) as f32,
                    ]
                };
                for i in 0..segments {
                    scene.line(
                        point(i),
                        point(i + 1),
                        GEOMETRY.with_alpha(0.75),
                        1.5,
                        rect,
                        2.0,
                    );
                }
            }
        }
    }
    let mut labels = super::point_labels::PointLabels::new(scene, layout, fonts);
    for (index, point) in resolved.iter().enumerate() {
        let Some(position) = point.and_then(|point| camera.project(point, rect)) else {
            continue;
        };
        let derived = !matches!(
            state.document.geometry.points()[index],
            Construction::Free(_)
        );
        scene.dot(
            position,
            if derived { 4.0 } else { 5.5 },
            if derived { ACCENT } else { GEOMETRY },
            rect,
            3.0,
        );
        if rect.contains(position) {
            labels.draw(scene, index, position);
        }
        if state.link.is_some_and(|id| id.index() == index) {
            scene.dot([position[0], position[1] - 15.0], 2.5, INK, rect, 3.0);
        }
    }
    scene.text(
        if state.stale {
            "Updating - previous graph is dimmed"
        } else {
            "Euclidean plane   /   radians"
        },
        [rect.x + 18.0, rect.y + 72.0],
        2,
        MUTED,
        rect,
    );
}
