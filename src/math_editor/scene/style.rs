//! Graph appearance is document presentation metadata, not an equation edit.
use crate::math_editor::view::{
    drawing::{self, Scene},
    layout::Rect,
};
use sim_logic::prelude::Color;
#[derive(Clone, Copy, PartialEq)]
pub(in crate::math_editor) struct GraphStyle {
    pub color: usize,
    pub width: f32,
    pub opacity: f32,
    pub pattern: usize,
}
impl GraphStyle {
    pub fn new(index: usize) -> Self {
        Self {
            color: index % 6,
            width: 2.0,
            opacity: 1.0,
            pattern: 0,
        }
    }
    pub fn tint(self) -> Color {
        color(self.color).with_alpha(self.opacity)
    }
    pub fn draw_path(self, out: &mut Scene, points: &[[f32; 2]], clip: Rect) {
        let mut phase = 0.0;
        for pair in points.windows(2) {
            self.draw(out, pair[0], pair[1], clip, &mut phase);
        }
    }
    pub(in crate::math_editor) fn draw(
        self,
        out: &mut Scene,
        start: [f32; 2],
        end: [f32; 2],
        clip: Rect,
        phase: &mut f64,
    ) {
        let whole = (f64::from(end[0]) - f64::from(start[0]))
            .hypot(f64::from(end[1]) - f64::from(start[1]));
        let cycle = if self.pattern == 1 { 15.0 } else { 6.0 };
        let initial = *phase;
        *phase = (*phase + whole).rem_euclid(cycle);
        let Some((a, b)) = drawing::clip_line(start, end, clip) else {
            return;
        };
        if self.pattern == 0 {
            out.line(a, b, self.tint(), self.width, clip, 1.0);
            return;
        }
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        let (dash, gap) = if self.pattern == 1 {
            (9.0, 6.0)
        } else {
            (1.0, 5.0)
        };
        let skipped =
            (f64::from(a[0]) - f64::from(start[0])).hypot(f64::from(a[1]) - f64::from(start[1]));
        let mut at = -((initial + skipped).rem_euclid(cycle) as f32);
        while at < length {
            let p = |t: f32| {
                [
                    a[0] + (b[0] - a[0]) * t / length,
                    a[1] + (b[1] - a[1]) * t / length,
                ]
            };
            if at + dash > 0.0 {
                out.line(
                    p(at.max(0.0)),
                    p((at + dash).min(length)),
                    self.tint(),
                    self.width,
                    clip,
                    1.0,
                );
            }
            at += dash + gap;
        }
    }
}
pub(in crate::math_editor) fn color(index: usize) -> Color {
    match index % 6 {
        0 => drawing::ACCENT,
        1 => drawing::GEOMETRY,
        2 => Color::rgb8(231, 166, 112),
        3 => Color::rgb8(119, 177, 241),
        4 => Color::rgb8(231, 147, 191),
        _ => Color::rgb8(198, 208, 124),
    }
}
