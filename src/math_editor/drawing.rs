//! Transient presentation commands, clipped before GPU tessellation.
use super::layout::Rect;
use sim_logic::prelude::*;
pub(super) const BG: Color = Color::rgb(0.003347, 0.005182, 0.008568);
pub(super) const PANEL: Color = Color::rgb(0.005605, 0.009134, 0.015209);
pub(super) const INK: Color = Color::rgb(0.745404, 0.791298, 0.854993);
pub(super) const MUTED: Color = Color::rgb(0.283149, 0.341914, 0.439657);
pub(super) const ACCENT: Color = Color::rgb(0.450786, 0.381326, 0.930111);
pub(super) const GEOMETRY: Color = Color::rgb(0.165132, 0.610496, 0.485150);

pub(super) struct Line {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub color: Color,
    pub width: f32,
    pub clip: Rect,
    pub depth: f32,
}
pub(super) struct Text {
    pub text: String,
    pub position: [f32; 2],
    pub style: usize,
    pub color: Color,
    pub clip: Rect,
    pub depth: f32,
}
pub(super) struct Panel {
    pub rect: Rect,
    pub color: Color,
    pub depth: f32,
    pub radius: f32,
}
pub(super) struct Dot {
    pub center: [f32; 2],
    pub radius: f32,
    pub color: Color,
    pub clip: Rect,
    pub depth: f32,
}
#[derive(Default)]
pub(super) struct Scene {
    pub lines: Vec<Line>,
    pub texts: Vec<Text>,
    pub panels: Vec<Panel>,
    pub dots: Vec<Dot>,
}
impl Scene {
    pub fn line(
        &mut self,
        from: [f32; 2],
        to: [f32; 2],
        color: Color,
        width: f32,
        clip: Rect,
        depth: f32,
    ) {
        if let Some((from, to)) = clip_line(from, to, clip) {
            self.lines.push(Line {
                from,
                to,
                color,
                width,
                clip,
                depth,
            });
        }
    }
    pub fn text(
        &mut self,
        text: impl Into<String>,
        position: [f32; 2],
        style: usize,
        color: Color,
        clip: Rect,
    ) {
        if position[0] < clip.x + clip.w
            && position[1] + 10.0 > clip.y
            && position[1] - 32.0 < clip.y + clip.h
        {
            self.texts.push(Text {
                text: text.into(),
                position,
                style,
                color,
                clip,
                depth: 6.0,
            });
        }
    }
    pub fn panel(&mut self, rect: Rect, color: Color, depth: f32) {
        self.panels.push(Panel {
            rect,
            color,
            depth,
            radius: 0.0,
        });
    }
    pub fn button(&mut self, rect: Rect, color: Color, depth: f32) {
        self.panels.push(Panel {
            rect,
            color,
            depth,
            radius: if rect.w >= 20.0 && rect.h >= 20.0 {
                6.0
            } else {
                0.0
            },
        });
    }
    pub fn dot(&mut self, center: [f32; 2], radius: f32, color: Color, clip: Rect, depth: f32) {
        if center[0] + radius >= clip.x
            && center[0] - radius <= clip.x + clip.w
            && center[1] + radius >= clip.y
            && center[1] - radius <= clip.y + clip.h
        {
            self.dots.push(Dot {
                center,
                radius,
                color,
                clip,
                depth,
            });
        }
    }
}
/// Liang-Barsky in f64 avoids overflow while clipping large finite coordinates.
pub(super) fn clip_line(a: [f32; 2], b: [f32; 2], r: Rect) -> Option<([f32; 2], [f32; 2])> {
    if !a.iter().chain(&b).all(|v| v.is_finite()) {
        return None;
    }
    let (x, y) = (f64::from(a[0]), f64::from(a[1]));
    let (dx, dy) = (f64::from(b[0]) - x, f64::from(b[1]) - y);
    let (mut lower, mut upper) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, x - f64::from(r.x)),
        (dx, f64::from(r.x + r.w) - x),
        (-dy, y - f64::from(r.y)),
        (dy, f64::from(r.y + r.h) - y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                lower = lower.max(t);
            } else {
                upper = upper.min(t);
            }
        }
    }
    if lower > upper {
        return None;
    }
    let a = [(x + lower * dx) as f32, (y + lower * dy) as f32];
    let b = [(x + upper * dx) as f32, (y + upper * dy) as f32];
    ((a[0] - b[0]).hypot(a[1] - b[1]) >= 0.25).then_some((a, b))
}
