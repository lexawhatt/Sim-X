//! Shared major/minor grid and collision-aware axis labels.
use crate::math_editor::{
    scene::{
        self,
        axes::{spatial as spatial_axes, ticks::Spacing},
    },
    state::MathState,
    view::{
        assets::Fonts,
        drawing::*,
        layout::{Layout, Rect},
    },
};
use sim_logic::prelude::Color;
use sim_math::geometry::Point;

pub(in crate::math_editor) struct Labels<'a> {
    occupied: Vec<Rect>,
    fonts: &'a Fonts,
    clip: Rect,
    spatial: bool,
}
impl<'a> Labels<'a> {
    pub fn new(layout: &Layout, fonts: &'a Fonts, spatial: bool) -> Self {
        let r = layout.canvas;
        let mut occupied: Vec<_> = layout
            .controls
            .iter()
            .filter_map(|(_, rect, _)| rect.intersection(r))
            .collect();
        occupied.push(Rect::new(r.x, r.y + r.h - 34.0, r.w, 34.0));
        occupied.push(Rect::new(
            r.x + 12.0,
            r.y + if spatial { 10.0 } else { 52.0 },
            if spatial { r.w - 24.0 } else { 270.0 },
            26.0,
        ));
        Self {
            occupied,
            fonts,
            clip: r,
            spatial,
        }
    }
    pub fn put(&mut self, out: &mut Scene, text: String, position: [f32; 2], color: Color) -> bool {
        let width = self.fonts.width(2, &text);
        let r = Rect::new(position[0] - 4.0, position[1] - 16.0, width + 8.0, 22.0);
        if r.x < self.clip.x + 2.0
            || r.x + r.w > self.clip.x + self.clip.w - 2.0
            || r.y < self.clip.y + 2.0
            || r.y + r.h > self.clip.y + self.clip.h - 2.0
            || self
                .occupied
                .iter()
                .any(|old| old.intersection(r).is_some())
        {
            return false;
        }
        self.occupied.push(r);
        if self.spatial {
            out.button(r, BG.with_alpha(color.alpha()), 4.0);
        }
        out.text(text, position, 2, color, self.clip);
        true
    }
}

pub(in crate::math_editor) fn spacing(
    scale: f64,
    low: f64,
    high: f64,
    fonts: &Fonts,
) -> Option<Spacing> {
    let initial = Spacing::new(scale, 0.0)?;
    let width = fonts
        .width(2, &ticks::label(low, initial.major))
        .max(fonts.width(2, &ticks::label(high, initial.major)));
    Spacing::new(scale, width)
}

pub(in crate::math_editor) fn projected_ticks(
    scale: f64,
    low: f64,
    high: f64,
    fonts: &Fonts,
) -> Option<f64> {
    let initial = Spacing::with_density(scale, 0.0, 56.0)?;
    let width = fonts
        .width(2, &ticks::label(low, initial.major))
        .max(fonts.width(2, &ticks::label(high, initial.major)));
    let mut step = Spacing::with_density(scale, width, 56.0)?.major;
    // The Z axis has a finite on-screen span, not a cube boundary. If rounding
    // up left only zero, use the preceding nice tick; actual label boxes still
    // have to pass overlap checks, so this never forces crowded labels through.
    if low <= 0.0 && high >= 0.0 && step > low.abs().max(high.abs()) {
        let decade = 10.0_f64.powf(step.log10().floor());
        let factor = step / decade;
        step = decade
            * if factor > 4.0 {
                2.0
            } else if factor > 1.5 {
                1.0
            } else {
                0.5
            };
    }
    Some(step)
}

pub(in crate::math_editor) fn plane(
    out: &mut Scene,
    state: &MathState,
    layout: &Layout,
    fonts: &Fonts,
) {
    let r = layout.canvas;
    let camera = state.camera;
    let Some(low) = camera.unproject([r.x, r.y + r.h], r) else {
        return;
    };
    let Some(high) = camera.unproject([r.x + r.w, r.y], r) else {
        return;
    };
    let Some(spacing) = spacing(camera.scale, low.x.min(low.y), high.x.max(high.y), fonts) else {
        return;
    };
    let mut labels = Labels::new(layout, fonts, false);
    for axis in 0..2 {
        let (lo, hi) = if axis == 0 {
            (low.x, high.x)
        } else {
            (low.y, high.y)
        };
        for (step, alpha) in [(spacing.minor, 0.36), (spacing.major, 0.85)] {
            for v in ticks::ticks(lo, hi, step) {
                if v == 0.0 {
                    continue;
                }
                let p = if axis == 0 {
                    Point { x: v, y: 0.0 }
                } else {
                    Point { x: 0.0, y: v }
                };
                if let Some(p) = camera.project(p, r) {
                    let (a, b) = if axis == 0 {
                        ([p[0], r.y], [p[0], r.y + r.h])
                    } else {
                        ([r.x, p[1]], [r.x + r.w, p[1]])
                    };
                    out.line(a, b, Color::rgb8(49, 62, 77).with_alpha(alpha), 1.0, r, 0.0);
                }
            }
        }
        for v in ticks::ticks(lo, hi, spacing.major) {
            if axis == 1 && v == 0.0 {
                continue;
            }
            let p = if axis == 0 {
                Point { x: v, y: 0.0 }
            } else {
                Point { x: 0.0, y: v }
            };
            let Some(p) = camera.project(p, r) else {
                continue;
            };
            let text = ticks::label(v, spacing.major);
            let width = fonts.width(2, &text);
            let position = if axis == 0 {
                [
                    p[0] + if v == 0.0 { 8.0 } else { -width * 0.5 },
                    (p[1] + 21.0).clamp(r.y + 22.0, r.y + r.h - 40.0),
                ]
            } else {
                [
                    (p[0] - width - 10.0).clamp(r.x + 8.0, r.x + r.w - width - 10.0),
                    p[1] + 5.0,
                ]
            };
            labels.put(out, text, position, MUTED);
        }
    }
    if let Some(origin) = camera.project(Point { x: 0.0, y: 0.0 }, r) {
        out.line(
            [r.x, origin[1]],
            [r.x + r.w, origin[1]],
            Color::rgb8(91, 105, 122),
            1.5,
            r,
            0.2,
        );
        out.line(
            [origin[0], r.y],
            [origin[0], r.y + r.h],
            Color::rgb8(91, 105, 122),
            1.5,
            r,
            0.2,
        );
    }
}

/// Crossfade only the bounded axis layers. Scientific curves use one continuous
/// projector and are never duplicated or resampled by the transition.
pub(in crate::math_editor) fn transition(
    out: &mut Scene,
    state: &MathState,
    layout: &Layout,
    projector: &scene::Projector,
    fonts: &Fonts,
) {
    let blend = state.spatial.blend as f32;
    for (spatial, opacity) in [(false, 1.0 - blend), (true, blend)] {
        if opacity <= 0.0 {
            continue;
        }
        let mut layer = Scene::default();
        if spatial {
            spatial_axes::draw(&mut layer, state, layout, projector, fonts);
        } else {
            plane(&mut layer, state, layout, fonts);
        }
        for mut line in layer.lines {
            line.color = line.color.with_alpha(line.color.alpha() * opacity);
            out.lines.push(line);
        }
        for mut text in layer.texts {
            text.color = text.color.with_alpha(text.color.alpha() * opacity);
            out.texts.push(text);
        }
        for mut panel in layer.panels {
            panel.color = panel.color.with_alpha(panel.color.alpha() * opacity);
            out.panels.push(panel);
        }
    }
}

pub(in crate::math_editor) mod ticks {
    //! View-dependent tick density, always anchored at zero rather than the first
    //! visible grid line. This is display LOD, never a limit on world coordinates.
    #[derive(Clone, Copy)]
    pub(in crate::math_editor) struct Spacing {
        pub major: f64,
        pub minor: f64,
    }
    impl Spacing {
        pub fn new(pixels_per_unit: f64, label_width: f32) -> Option<Self> {
            Self::with_density(pixels_per_unit, label_width, 88.0)
        }
        pub fn with_density(pixels_per_unit: f64, label_width: f32, minimum: f32) -> Option<Self> {
            if !pixels_per_unit.is_finite() || pixels_per_unit <= 0.0 {
                return None;
            }
            let desired = f64::from((label_width + 28.0).max(minimum)) / pixels_per_unit;
            let decade = 10.0_f64.powf(desired.log10().floor());
            let factor = desired / decade;
            let major = decade
                * if factor <= 1.0 {
                    1.0
                } else if factor <= 2.0 {
                    2.0
                } else if factor <= 5.0 {
                    5.0
                } else {
                    10.0
                };
            (major.is_finite() && major > 0.0 && major / 5.0 > 0.0).then_some(Self {
                major,
                minor: major / 5.0,
            })
        }
    }
    pub(in crate::math_editor) fn ticks(low: f64, high: f64, step: f64) -> Vec<f64> {
        if !low.is_finite() || !high.is_finite() || !step.is_finite() || step <= 0.0 || high < low {
            return vec![];
        }
        let first = (low / step).ceil();
        let last = (high / step).floor();
        if !first.is_finite() || !last.is_finite() || first + 1.0 == first || last + 1.0 == last {
            return vec![];
        }
        if last < first {
            return vec![];
        }
        (0..=(last - first) as usize)
            .filter_map(|i| {
                let v = (first + i as f64) * step;
                v.is_finite()
                    .then_some(if v.abs() < step * 1e-8 { 0.0 } else { v })
            })
            .collect()
    }
    pub(in crate::math_editor) fn label(value: f64, step: f64) -> String {
        if value.abs() < step * 1e-8 {
            return "0".into();
        }
        if value.abs() >= 1e6 || value.abs() < 1e-4 {
            let digits =
                (value.abs().log10().floor() - step.log10().floor()).clamp(0.0, 15.0) as usize;
            format!("{value:.digits$e}")
        } else {
            let digits = (-step.log10().floor()).clamp(0.0, 15.0) as usize;
            let text = format!("{value:.digits$}");
            if text.contains('.') {
                text.trim_end_matches('0').trim_end_matches('.').into()
            } else {
                text
            }
        }
    }
}

pub(in crate::math_editor) mod spatial {
    //! Projected grid density and XYZ tick labels in the open orthographic view.
    use crate::math_editor::{
        scene::{
            Projector,
            axes::{Labels, projected_ticks, spacing, ticks},
            clip::Bounds,
        },
        state::MathState,
        view::{assets::Fonts, drawing::Scene, layout::Layout},
    };
    use sim_logic::prelude::Color;

    pub(in crate::math_editor) fn draw(
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
                    for v in ticks::ticks(lo, hi, step) {
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
            for value in ticks::ticks(lo, hi, step) {
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
                        ticks::label(value, step),
                        [p[0] + normal[0] * 10.0 + 4.0, p[1] + normal[1] * 10.0 + 5.0],
                        colors[axis],
                    );
                }
            }
        }
    }
}
