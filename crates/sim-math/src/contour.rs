//! Pixel-resolution-independent marching triangles for implicit plane curves.
//! This is sampled visualization, not a symbolic locus/feature completeness proof.
use crate::{Expression, MathError, geometry::Point};

/// Sample F(x,y)=0 in the given bounds; check cancellation once per grid row.
/// `cells` is caller-selected visual resolution, not a document/work ceiling.
/// Poles/unresolved domains are excluded through rectangle screening.
pub fn contours(
    expression: &Expression,
    lower: Point,
    upper: Point,
    cells: [usize; 2],
    alive: impl Fn() -> bool,
) -> Result<Vec<[Point; 2]>, MathError> {
    let [nx, ny] = cells;
    if nx == 0 || ny == 0 || upper.x <= lower.x || upper.y <= lower.y {
        return Err(MathError::NumericRange);
    }
    let dx = (upper.x - lower.x) / nx as f64;
    let dy = (upper.y - lower.y) / ny as f64;
    if !dx.is_finite() || !dy.is_finite() || lower.x + dx == lower.x || lower.y + dy == lower.y {
        return Err(MathError::PrecisionExhausted);
    }
    let point = |i: usize, j: usize| Point {
        x: lower.x + dx * i as f64,
        y: lower.y + dy * j as f64,
    };
    let mut bottom: Vec<_> = (0..=nx)
        .map(|i| {
            let p = point(i, 0);
            expression.evaluate_at(p.x, p.y).ok()
        })
        .collect();
    let mut out = vec![];
    for j in 0..ny {
        if !alive() {
            return Err(MathError::Cancelled);
        }
        let top: Vec<_> = (0..=nx)
            .map(|i| {
                let p = point(i, j + 1);
                expression.evaluate_at(p.x, p.y).ok()
            })
            .collect();
        for i in 0..nx {
            let p = [
                point(i, j),
                point(i + 1, j),
                point(i + 1, j + 1),
                point(i, j + 1),
            ];
            let values = [bottom[i], bottom[i + 1], top[i + 1], top[i]];
            if expression
                .screen_box([p[0].x, p[2].x], [p[0].y, p[2].y])
                .is_err()
            {
                continue;
            }
            for ids in [[0, 1, 2], [0, 2, 3]] {
                let mut crossings = vec![];
                for [a, b] in [[ids[0], ids[1]], [ids[1], ids[2]], [ids[2], ids[0]]] {
                    let (Some(va), Some(vb)) = (values[a], values[b]) else {
                        continue;
                    };
                    if va == 0.0 && vb == 0.0 {
                        continue;
                    }
                    if (va <= 0.0) == (vb <= 0.0) {
                        continue;
                    }
                    let scale = va.abs().max(vb.abs());
                    let t = (va / scale) / ((va / scale) - (vb / scale));
                    let q = Point {
                        x: p[a].x + (p[b].x - p[a].x) * t,
                        y: p[a].y + (p[b].y - p[a].y) * t,
                    };
                    if q.x.is_finite() && q.y.is_finite() {
                        crossings.push(q);
                    }
                }
                if let [a, b] = crossings.as_slice() {
                    out.push([*a, *b]);
                }
            }
        }
        bottom = top;
    }
    Ok(out)
}
