//! Stitch sampled contour edges into display paths before simplification.
//! Subpixel endpoint snapping is presentation-only; mathematical samples remain
//! unchanged. Junctions terminate paths instead of choosing arbitrary branches.
use std::collections::BTreeMap;
type Key = [i64; 2];
fn key(p: [f32; 2]) -> Key {
    p.map(|v| (f64::from(v) * 1024.0).round() as i64)
}
pub(super) fn paths(segments: &[[[f32; 2]; 2]]) -> Vec<Vec<[f32; 2]>> {
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
        result.push(super::curve_display::simplify(&path, 0.35));
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
