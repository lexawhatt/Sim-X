//! World-clipped, depth-faded surface isolines. This is a transparent wireframe,
//! not hidden-surface removal or a substitute for the future filled mesh path.
use super::{
    drawing::Scene, graph_style::GraphStyle, layout::Rect, spatial::Projector, spatial_clip::Bounds,
};

pub(super) struct Wireframe<'a> {
    projector: &'a Projector,
    bounds: Bounds,
    depths: [f64; 2],
    rect: Rect,
}
impl<'a> Wireframe<'a> {
    pub fn new(projector: &'a Projector, bounds: Bounds, rect: Rect) -> Self {
        let depths = bounds
            .corners()
            .filter_map(|p| projector.depth(p))
            .fold([f64::INFINITY, f64::NEG_INFINITY], |[a, b], d| {
                [a.min(d), b.max(d)]
            });
        Self {
            projector,
            bounds,
            depths,
            rect,
        }
    }

    pub fn strip(
        &self,
        out: &mut Scene,
        points: impl IntoIterator<Item = Option<[f64; 3]>>,
        style: GraphStyle,
    ) {
        if style.opacity <= 0.0 {
            return;
        }
        let start = out.lines.len();
        let mut previous = None;
        let mut run = Vec::new();
        for point in points.into_iter().chain(std::iter::once(None)) {
            let segment = previous
                .zip(point)
                .and_then(|(a, b)| self.bounds.segment([a, b]))
                .and_then(|s| self.projector.segment(s));
            if let Some(segment) = segment {
                let [a, b] = segment.points;
                let [da, db] = segment.depths;
                if run.last().is_some_and(|&(xy, _)| xy != a) {
                    self.flush(out, &run, style);
                    run.clear();
                }
                if run.is_empty() {
                    run.push((a, da));
                }
                run.push((b, db));
            } else {
                self.flush(out, &run, style);
                run.clear();
            }
            previous = point;
        }
        // Plane curves stay legible above the transparent surface. This is an
        // explanatory layer, not a claim that the curve is physically in front.
        for line in &mut out.lines[start..] {
            line.depth = 0.8;
        }
    }

    fn flush(&self, out: &mut Scene, run: &[([f32; 2], f64)], style: GraphStyle) {
        let xy: Vec<_> = run.iter().map(|p| p.0).collect();
        let indices = super::curve_display::simplify_indices(&xy, 0.35);
        let mut phase = 0.0;
        for pair in indices.windows(2) {
            let (a, da) = run[pair[0]];
            let (b, db) = run[pair[1]];
            // Straight isolines still need a continuous depth cue. Split by
            // screen distance, not by a limit on formula or surface complexity.
            let count = ((b[0] - a[0]).hypot(b[1] - a[1]) / 48.0).ceil().max(1.0) as usize;
            let at = |t: f32| [a[0] * (1.0 - t) + b[0] * t, a[1] * (1.0 - t) + b[1] * t];
            for i in 0..count {
                let t0 = i as f32 / count as f32;
                let t1 = (i + 1) as f32 / count as f32;
                let t = f64::from((t0 + t1) * 0.5);
                let depth = da * (1.0 - t) + db * t;
                let distance =
                    ((depth - self.depths[0]) / (self.depths[1] - self.depths[0])).clamp(0.0, 1.0);
                let mut faded = style;
                faded.opacity *= (0.78 - 0.5 * distance) as f32;
                faded.draw(out, at(t0), at(t1), self.rect, &mut phase);
            }
        }
    }
}
