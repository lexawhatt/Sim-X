//! Shared major/minor grid and collision-aware axis labels.
use super::{
    assets::Fonts,
    axis_ticks::{self, Spacing},
    drawing::*,
    layout::{Layout, Rect},
    state::MathState,
};
use sim_logic::prelude::Color;
use sim_math::geometry::Point;

pub(super) struct Labels<'a> {
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

pub(super) fn spacing(scale: f64, low: f64, high: f64, fonts: &Fonts) -> Option<Spacing> {
    let initial = Spacing::new(scale, 0.0)?;
    let width = fonts
        .width(2, &axis_ticks::label(low, initial.major))
        .max(fonts.width(2, &axis_ticks::label(high, initial.major)));
    Spacing::new(scale, width)
}

pub(super) fn projected_ticks(scale: f64, low: f64, high: f64, fonts: &Fonts) -> Option<f64> {
    let initial = Spacing::with_density(scale, 0.0, 56.0)?;
    let width = fonts
        .width(2, &axis_ticks::label(low, initial.major))
        .max(fonts.width(2, &axis_ticks::label(high, initial.major)));
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

pub(super) fn plane(out: &mut Scene, state: &MathState, layout: &Layout, fonts: &Fonts) {
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
            for v in axis_ticks::ticks(lo, hi, step) {
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
        for v in axis_ticks::ticks(lo, hi, spacing.major) {
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
            let text = axis_ticks::label(v, spacing.major);
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
pub(super) fn transition(
    out: &mut Scene,
    state: &MathState,
    layout: &Layout,
    projector: &super::spatial::Projector,
    fonts: &Fonts,
) {
    let blend = state.spatial.blend as f32;
    for (spatial, opacity) in [(false, 1.0 - blend), (true, blend)] {
        if opacity <= 0.0 {
            continue;
        }
        let mut layer = Scene::default();
        if spatial {
            super::axis_spatial::draw(&mut layer, state, layout, projector, fonts);
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
