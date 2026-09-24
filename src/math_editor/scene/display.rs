//! Pixel-error polyline simplification. Scientific samples/results are unchanged;
//! only redundant display segments are merged, independently for every curve.
pub(in crate::math_editor) fn simplify(points: &[[f32; 2]], error: f64) -> Vec<[f32; 2]> {
    simplify_indices(points, error)
        .into_iter()
        .map(|i| points[i])
        .collect()
}

pub(in crate::math_editor) fn simplify_indices(points: &[[f32; 2]], error: f64) -> Vec<usize> {
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

pub(in crate::math_editor) mod contours {
    //! Stitch sampled contour edges into display paths before simplification.
    //! Subpixel endpoint snapping is presentation-only; mathematical samples remain
    //! unchanged. Junctions terminate paths instead of choosing arbitrary branches.
    use crate::math_editor::scene::display;
    use std::collections::BTreeMap;
    type Key = [i64; 2];
    fn key(p: [f32; 2]) -> Key {
        p.map(|v| (f64::from(v) * 1024.0).round() as i64)
    }
    pub(in crate::math_editor) fn paths(segments: &[[[f32; 2]; 2]]) -> Vec<Vec<[f32; 2]>> {
        let mut edges = BTreeMap::<Key, Vec<usize>>::new();
        for (i, [a, b]) in segments.iter().enumerate() {
            edges.entry(key(*a)).or_default().push(i);
            edges.entry(key(*b)).or_default().push(i);
        }
        let mut visited = vec![false; segments.len()];
        let mut starts: Vec<_> = edges
            .iter()
            .filter(|(_, list)| list.len() != 2)
            .flat_map(|(k, list)| list.iter().map(move |i| (*i, *k)))
            .collect();
        // Remaining edges form closed loops.
        starts.extend(segments.iter().enumerate().map(|(i, [a, _])| (i, key(*a))));
        let mut result = vec![];
        for (mut i, mut start) in starts {
            if visited[i] {
                continue;
            }
            let mut path = vec![];
            loop {
                if visited[i] {
                    break;
                }
                visited[i] = true;
                let [a, b] = segments[i];
                let (from, to) = if key(a) == start { (a, b) } else { (b, a) };
                if path.is_empty() {
                    path.push(from);
                }
                path.push(to);
                start = key(to);
                let next = &edges[&start];
                if next.len() != 2 {
                    break;
                }
                let Some(id) = next.iter().copied().find(|j| !visited[*j]) else {
                    break;
                };
                i = id;
            }
            result.push(display::simplify(&path, 0.35));
        }
        result
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn chains_simplify_and_branches_are_preserved() {
            let edges: Vec<_> = (0..1000)
                .map(|i| [[i as f32, 0.0], [(i + 1) as f32, 0.0]])
                .collect();
            assert_eq!(paths(&edges), vec![vec![[0.0, 0.0], [1000.0, 0.0]]]);
            let fork = [
                [[0.0, 0.0], [1.0, 0.0]],
                [[1.0, 0.0], [2.0, 1.0]],
                [[1.0, 0.0], [2.0, -1.0]],
            ];
            assert_eq!(paths(&fork).len(), 3);
            let closed = [
                [[0.0, 0.0], [3.0, 0.0]],
                [[3.0, 0.0], [3.0, 3.0]],
                [[3.0, 3.0], [0.0, 0.0]],
            ];
            let p = paths(&closed);
            assert_eq!(p.len(), 1);
            assert_eq!(p[0].len(), 4);
        }
    }
}
