use crate::math_editor::{
    scene::{
        axes,
        display::{self, contours},
        integrals, view as spatial_view,
    },
    state::MathState,
    view::{assets, drawing::*, layout::Layout},
};
use sim_math::geometry::{Construction, Shape};

pub(in crate::math_editor) fn draw(
    scene: &mut Scene,
    state: &MathState,
    layout: &Layout,
    fonts: &assets::Fonts,
) {
    if state.spatial.visible() {
        spatial_view::draw(scene, state, layout, fonts);
        return;
    }
    let rect = layout.canvas;
    let camera = state.camera;
    axes::plane(scene, state, layout, fonts);
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
                integrals::draw(
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
            for path in contours::paths(&projected) {
                style.draw_path(scene, &path, rect);
            }
            let mut run = vec![];
            for point in row.points.iter().chain(std::iter::once(&None)) {
                if let Some(p) = point.and_then(|p| camera.project(p, rect)) {
                    run.push(p);
                } else {
                    let reduced = display::simplify(&run, 0.35);
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
    let mut labels = labels::PointLabels::new(scene, layout, fonts);
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

pub(in crate::math_editor) mod labels {
    //! Screen-space labels prefer free space and retain a small readable backdrop.
    use crate::math_editor::view::{
        assets::Fonts,
        drawing::*,
        layout::{Layout, Rect},
    };

    pub(in crate::math_editor) struct PointLabels<'a> {
        fonts: &'a Fonts,
        clip: Rect,
        occupied: Vec<Rect>,
        lines: Vec<([f32; 2], [f32; 2])>,
    }

    impl<'a> PointLabels<'a> {
        pub(in crate::math_editor) fn new(
            scene: &Scene,
            layout: &Layout,
            fonts: &'a Fonts,
        ) -> Self {
            let clip = layout.canvas;
            let mut occupied: Vec<_> = layout
                .controls
                .iter()
                .filter_map(|(_, r, _)| r.intersection(clip))
                .collect();
            occupied.extend(scene.texts.iter().filter_map(|t| {
                Rect::new(
                    t.position[0] - 3.0,
                    t.position[1] - 16.0,
                    fonts.width(t.style, &t.text) + 6.0,
                    22.0,
                )
                .intersection(clip)
            }));
            // Reserve overlays rendered after the construction labels as well.
            occupied.push(Rect::new(clip.x, clip.y, clip.w, 96.0));
            occupied.push(Rect::new(clip.x, clip.y + clip.h - 35.0, clip.w, 35.0));
            let lines = scene
                .lines
                .iter()
                .filter(|line| {
                    line.depth >= 0.8
                        && line.color.alpha() > 0.1
                        && line.clip.intersection(clip).is_some()
                })
                .map(|line| (line.from, line.to))
                .collect();
            Self {
                fonts,
                clip,
                occupied,
                lines,
            }
        }

        pub(in crate::math_editor) fn draw(
            &mut self,
            scene: &mut Scene,
            index: usize,
            point: [f32; 2],
        ) {
            if !self.clip.contains(point) {
                return;
            }
            let label = format!("P{}", index + 1);
            let width = self.fonts.width(2, &label);
            let candidates = [
                [point[0] + 10.0, point[1] - 10.0],
                [point[0] + 10.0, point[1] + 24.0],
                [point[0] - width - 10.0, point[1] - 10.0],
                [point[0] - width - 10.0, point[1] + 24.0],
                [point[0] - width * 0.5, point[1] - 15.0],
                [point[0] - width * 0.5, point[1] + 31.0],
            ];
            let mut best = None;
            for (rank, position) in candidates.into_iter().enumerate() {
                let rect = Rect::new(position[0] - 3.0, position[1] - 16.0, width + 6.0, 21.0);
                if rect.x < self.clip.x
                    || rect.y < self.clip.y
                    || rect.x + rect.w > self.clip.x + self.clip.w
                    || rect.y + rect.h > self.clip.y + self.clip.h
                {
                    continue;
                }
                let blocked = self
                    .occupied
                    .iter()
                    .filter(|r| r.intersection(rect).is_some())
                    .count();
                let crossings = self
                    .lines
                    .iter()
                    .filter(|(a, b)| clip_line(*a, *b, rect).is_some())
                    .count();
                let score = blocked
                    .saturating_mul(10_000)
                    .saturating_add(crossings.saturating_mul(100))
                    .saturating_add(rank);
                if best.as_ref().is_none_or(|(old, _, _)| score < *old) {
                    best = Some((score, rect, position));
                }
            }
            if let Some((_, rect, position)) = best {
                self.occupied.push(rect);
                scene.panel(rect, BG.with_alpha(0.94), 4.0);
                scene.text(label, position, 2, INK, self.clip);
            }
        }
    }
}
