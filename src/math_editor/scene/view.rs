//! Open-axis wireframe view. Projection belongs to Logic/Engine; mathematical
//! samples stay in sim-math. Filled/depth-tested surfaces await the mesh API.
use crate::math_editor::{
    scene::{Projector, axes, clip, display, integrals, style::GraphStyle, wireframe},
    state::MathState,
    view::{
        assets::Fonts,
        drawing::*,
        graph::labels,
        layout::{self, Layout},
    },
};
use sim_math::geometry::{Construction, Shape};

pub(in crate::math_editor) fn draw(
    out: &mut Scene,
    state: &MathState,
    layout: &Layout,
    fonts: &Fonts,
) {
    let rect = layout.canvas;
    let Ok(projector) = state.spatial.projector(state.camera, rect) else {
        out.text(
            "3D projection is outside the finite numeric range. Use Home.",
            [rect.x + 18.0, rect.y + 36.0],
            2,
            MUTED,
            rect,
        );
        return;
    };
    axes::transition(out, state, layout, &projector, fonts);
    let wireframe = clip::Bounds::for_view(state.camera, rect)
        .map(|bounds| wireframe::Wireframe::new(&projector, bounds, rect));
    if let Some(plot) = &state.plot {
        for (index, row) in plot.rows.iter().enumerate() {
            if !state.document.visible.get(index).copied().unwrap_or(false) {
                continue;
            }
            let mut style = state.document.styles[index];
            style.opacity *= state.motion.plot_opacity;
            path(
                out,
                &projector,
                row.points.iter().map(|p| p.map(|p| [p.x, p.y, 0.0])),
                style,
                layout,
            );
            for [a, b] in &row.segments {
                path(
                    out,
                    &projector,
                    [Some([a.x, a.y, 0.0]), Some([b.x, b.y, 0.0])],
                    style,
                    layout,
                );
            }
            if let Some(picture) = &row.integral_plot {
                let planar = (1.0 - state.spatial.blend as f32 / 0.2).clamp(0.0, 1.0);
                if planar > 0.0 && !state.stale && row.scalar.is_some() && !row.pending {
                    let mut fill = style;
                    fill.opacity *= planar * planar * (3.0 - 2.0 * planar);
                    integrals::fill_plane(
                        out,
                        picture,
                        fill,
                        rect,
                        state.area_progress.get(index).copied().unwrap_or(0.0),
                        |p| state.camera.project(p, rect),
                    );
                }
                integrals::draw(
                    out,
                    picture,
                    style,
                    rect,
                    None,
                    |p| projector.point([p.x, p.y, 0.0]),
                    false,
                );
            }
            style.opacity *= state.spatial.blend as f32;
            if row
                .boundary
                .is_some_and(|relation| !relation.includes_boundary())
            {
                // Strict inequalities exclude the displayed boundary itself.
                style.pattern = 1;
            }
            if let Some(wireframe) = &wireframe {
                for strip in &row.surface {
                    wireframe.strip(out, strip.iter().copied(), style);
                }
            }
        }
    }
    let points = state.resolved();
    let style = GraphStyle {
        color: 1,
        width: 1.8,
        opacity: 1.0,
        pattern: 0,
    };
    for shape in state.document.geometry.shapes() {
        let (Shape::Segment(a, b) | Shape::Circle(a, b)) = *shape;
        let (Some(Some(a)), Some(Some(b))) = (points.get(a.index()), points.get(b.index())) else {
            continue;
        };
        match shape {
            Shape::Segment(..) => path(
                out,
                &projector,
                [Some([a.x, a.y, 0.0]), Some([b.x, b.y, 0.0])],
                style,
                layout,
            ),
            Shape::Circle(..) => {
                let radius = (a.x - b.x).hypot(a.y - b.y);
                let count = (std::f64::consts::TAU * (radius * state.camera.scale).sqrt())
                    .ceil()
                    .max(16.0)
                    .min(f64::from((rect.w + rect.h) * 2.0)) as usize;
                path(
                    out,
                    &projector,
                    (0..=count).map(|i| {
                        let angle = i as f64 / count as f64 * std::f64::consts::TAU;
                        Some([a.x + radius * angle.cos(), a.y + radius * angle.sin(), 0.0])
                    }),
                    GraphStyle {
                        width: 1.5,
                        opacity: 0.75,
                        ..style
                    },
                    layout,
                );
            }
        }
    }
    let mut labels = labels::PointLabels::new(out, layout, fonts);
    for (index, point) in points.iter().enumerate() {
        if let Some(p) = point.and_then(|p| projector.point([p.x, p.y, 0.0])) {
            let derived = !matches!(
                state.document.geometry.points()[index],
                Construction::Free(_)
            );
            out.dot(
                p,
                if derived { 4.0 } else { 5.5 },
                if derived { ACCENT } else { GEOMETRY },
                rect,
                3.0,
            );
            labels.draw(out, index, p);
            if state.link.is_some_and(|id| id.index() == index) {
                out.dot([p[0], p[1] - 15.0], 2.5, INK, rect, 3.0);
            }
        }
    }
    let hint = "Orthographic 3D / curves: XY plane (z=0)";
    if let Some(backing) = layout::Rect::new(
        rect.x + 10.0,
        rect.y + 12.0,
        fonts.width(2, hint) + 16.0,
        28.0,
    )
    .intersection(rect)
    {
        out.button(backing, BG.with_alpha(state.spatial.blend as f32), 5.0);
    }
    out.text(
        hint,
        [rect.x + 18.0, rect.y + 32.0],
        2,
        MUTED.with_alpha(state.spatial.blend as f32),
        rect,
    );
    out.text(
        if state.stale {
            "Updating - previous graph is dimmed"
        } else {
            "Euclidean plane   /   radians"
        },
        [rect.x + 18.0, rect.y + 72.0],
        2,
        MUTED.with_alpha(1.0 - state.spatial.blend as f32),
        rect,
    );
}

fn path(
    out: &mut Scene,
    projector: &Projector,
    points: impl IntoIterator<Item = Option<[f64; 3]>>,
    style: GraphStyle,
    layout: &Layout,
) {
    let mut run = Vec::new();
    for p in points.into_iter().chain(std::iter::once(None)) {
        if let Some(p) = p.and_then(|p| projector.point(p)) {
            run.push(p);
        } else {
            let reduced = display::simplify(&run, 0.35);
            style.draw_path(out, &reduced, layout.canvas);
            run.clear();
        }
    }
}
