//! Pixel-error polyline simplification. Scientific samples/results are unchanged;
//! only redundant display segments are merged, independently for every curve.
pub(super) fn simplify(points: &[[f32; 2]], error: f64) -> Vec<[f32; 2]> {
    simplify_indices(points, error)
        .into_iter()
        .map(|i| points[i])
        .collect()
}

pub(super) fn simplify_indices(points: &[[f32; 2]], error: f64) -> Vec<usize> {
    if points.len() < 3 {
        return (0..points.len()).collect();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut pending = vec![(0, points.len() - 1)];
    while let Some((first, last)) = pending.pop() {
        let a = points[first].map(f64::from);
        let b = points[last].map(f64::from);
        let d = [b[0] - a[0], b[1] - a[1]];
        let length = d[0] * d[0] + d[1] * d[1];
        let mut worst = error * error;
        let mut split = None;
        for (i, p) in points.iter().enumerate().take(last).skip(first + 1) {
            let p = p.map(f64::from);
            let t = if length > 0.0 {
                ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / length
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let distance = (p[0] - a[0] - t * d[0]).powi(2) + (p[1] - a[1] - t * d[1]).powi(2);
            if distance > worst {
                worst = distance;
                split = Some(i);
            }
        }
        if let Some(i) = split {
            keep[i] = true;
            pending.push((first, i));
            pending.push((i, last));
        }
    }
    keep.into_iter()
        .enumerate()
        .filter_map(|(i, keep)| keep.then_some(i))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merges_straight_samples_without_erasing_a_peak() {
        let straight: Vec<_> = (0..1000).map(|i| [i as f32, i as f32 * 2.0]).collect();
        assert_eq!(simplify(&straight, 0.25), vec![straight[0], straight[999]]);
        assert_eq!(
            simplify(&[[0.0, 0.0], [1.0, 3.0], [2.0, 0.0]], 0.25).len(),
            3
        );
    }
}
