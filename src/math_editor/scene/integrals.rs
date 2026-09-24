//! Shared explanatory curves/bounds, with signed fill restricted to the 2D view.
//! The fill is a finite-resolution illustration, never a numerical area solver.
use crate::math_editor::{
    compute::integrals::{IntegralPlot, Section},
    scene::{
        display,
        style::{self, GraphStyle},
    },
    view::{drawing::Scene, layout::Rect},
};
use sim_logic::prelude::Color;
use sim_math::geometry::Point;

pub(in crate::math_editor) fn negative_color(style: GraphStyle) -> Color {
    style::color(style.color + 2)
}

pub(in crate::math_editor) fn draw(
    out: &mut Scene,
    picture: &IntegralPlot,
    style: GraphStyle,
    rect: Rect,
    progress: Option<f32>,
    project: impl Fn(Point) -> Option<[f32; 2]>,
    plane: bool,
) {
    if plane && let Some(progress) = progress {
        fill_plane(out, picture, style, rect, progress, &project);
    }
    let mut drawn_bounds = std::collections::BTreeSet::new();
    for band in &picture.bands {
        let guide = GraphStyle {
            opacity: style.opacity * 0.4,
            width: 1.0,
            pattern: 1,
            ..style
        };
        for bound in band.bounds {
            if plane {
                if !drawn_bounds.insert(if bound == 0.0 { 0 } else { bound.to_bits() }) {
                    continue;
                }
                if let Some(p) = project(Point { x: bound, y: 0.0 }) {
                    guide.draw_path(out, &[[p[0], rect.y], [p[0], rect.y + rect.h]], rect);
                }
            } else if let Some(section) = band.sections.iter().flatten().find(|p| p.x == bound)
                && let (Some(a), Some(b)) = (
                    project(Point {
                        x: bound,
                        y: section.first,
                    }),
                    project(Point {
                        x: bound,
                        y: section.second,
                    }),
                )
            {
                guide.draw_path(out, &[a, b], rect);
            }
        }
    }
    for (index, points) in picture.curves.iter().enumerate() {
        let curve = GraphStyle {
            color: style.color + index,
            ..style
        };
        let mut run = vec![];
        for p in points.iter().chain(std::iter::once(&None)) {
            if let Some(p) = p.and_then(&project) {
                run.push(p);
            } else {
                curve.draw_path(out, &display::simplify(&run, 0.35), rect);
                run.clear();
            }
        }
    }
}

/// Flat explanatory bands can fade near the 2D endpoint without duplicating
/// curves or inventing an area in the orbit view. This accepts a 2D projector.
pub(in crate::math_editor) fn fill_plane(
    out: &mut Scene,
    picture: &IntegralPlot,
    style: GraphStyle,
    rect: Rect,
    progress: f32,
    project: impl Fn(Point) -> Option<[f32; 2]>,
) {
    let t = f64::from(progress * progress * (3.0 - 2.0 * progress));
    for band in &picture.bands {
        let end = band.bounds[0] * (1.0 - t) + band.bounds[1] * t;
        for pair in band.sections.windows(2) {
            let [Some(a), Some(mut b)] = [pair[0], pair[1]] else {
                continue;
            };
            if b.x <= a.x || a.x >= end {
                continue;
            }
            if b.x > end {
                b = interpolate(a, b, (end - a.x) / (b.x - a.x));
            }
            // Split at crossings of the displayed piecewise-linear curves so
            // positive and negative contributions never share one strip color.
            let scale = a
                .first
                .abs()
                .max(a.second.abs())
                .max(b.first.abs())
                .max(b.second.abs());
            let da = a.first / scale - a.second / scale;
            let db = b.first / scale - b.second / scale;
            let strips = if da * db < 0.0 {
                let cross = interpolate(a, b, da / (da - db));
                [Some((a, cross)), Some((cross, b))]
            } else {
                [Some((a, b)), None]
            };
            for (a, b) in strips.into_iter().flatten() {
                let mid = interpolate(a, b, 0.5);
                let positive = (mid.first > mid.second) == (band.direction > 0.0);
                let tint = if positive {
                    style.tint()
                } else {
                    negative_color(style)
                };
                let tint = tint.with_alpha(style.opacity * 0.22);
                let (Some(top), Some(bottom), Some(left), Some(right)) = (
                    project(Point {
                        x: mid.x,
                        y: mid.first,
                    }),
                    project(Point {
                        x: mid.x,
                        y: mid.second,
                    }),
                    project(Point { x: a.x, y: 0.0 }),
                    project(Point { x: b.x, y: 0.0 }),
                ) else {
                    continue;
                };
                let bar = Rect::new(
                    left[0],
                    top[1].min(bottom[1]),
                    right[0] - left[0],
                    (top[1] - bottom[1]).abs(),
                );
                if let Some(bar) = bar.intersection(rect) {
                    out.panel(bar, tint, 0.5);
                }
            }
        }
    }
}
fn interpolate(a: Section, b: Section, t: f64) -> Section {
    Section {
        x: a.x * (1.0 - t) + b.x * t,
        first: a.first * (1.0 - t) + b.first * t,
        second: a.second * (1.0 - t) + b.second * t,
    }
}
