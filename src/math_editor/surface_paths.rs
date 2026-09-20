//! Join implicit boundary segments on the worker before display simplification.
//! Endpoints match within a billionth of the sampled view span. Junctions stop
//! paths, so crossing slice families never arbitrarily change direction.
use std::collections::{BTreeMap, BTreeSet};
type Key = [i64; 3];

pub(super) fn stitch(
    segments: &[[[f64; 3]; 2]],
    lower: [f64; 3],
    upper: [f64; 3],
    alive: impl Fn() -> bool,
) -> Result<Vec<Vec<Option<[f64; 3]>>>, sim_math::MathError> {
    let key = |point: [f64; 3]| -> Key {
        std::array::from_fn(|axis| {
            ((point[axis] - lower[axis]) / (upper[axis] - lower[axis]) * 1e9).round() as i64
        })
    };
    // Adjacent triangles/slices may report the same zero edge twice. Display
    // it once so exact grid-aligned faces neither brighten nor break every path.
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();
    for &[a, b] in segments {
        if !alive() {
            return Err(sim_math::MathError::Cancelled);
        }
        let (ka, kb) = (key(a), key(b));
        if ka != kb && seen.insert(if ka < kb { [ka, kb] } else { [kb, ka] }) {
            unique.push([a, b]);
        }
    }
    let segments = unique;
    let mut adjacency = BTreeMap::<Key, Vec<usize>>::new();
    for (i, segment) in segments.iter().enumerate() {
        if !alive() {
            return Err(sim_math::MathError::Cancelled);
        }
        for point in segment {
            adjacency.entry(key(*point)).or_default().push(i);
        }
    }
    let mut visited = vec![false; segments.len()];
    let mut starts: Vec<_> = adjacency
        .iter()
        .filter(|(_, list)| list.len() != 2)
        .flat_map(|(key, list)| list.iter().map(move |id| (*id, *key)))
        .collect();
    starts.extend(segments.iter().enumerate().map(|(i, [a, _])| (i, key(*a))));
    let mut paths = Vec::new();
    for (mut id, mut start) in starts {
        if visited[id] {
            continue;
        }
        let mut path = Vec::new();
        loop {
            if !alive() {
                return Err(sim_math::MathError::Cancelled);
            }
            if visited[id] {
                break;
            }
            visited[id] = true;
            let [a, b] = segments[id];
            let (from, to) = if key(a) == start { (a, b) } else { (b, a) };
            if path.is_empty() {
                path.push(Some(from));
            }
            path.push(Some(to));
            start = key(to);
            let next = &adjacency[&start];
            if next.len() != 2 {
                break;
            }
            let Some(next) = next.iter().copied().find(|id| !visited[*id]) else {
                break;
            };
            id = next;
        }
        paths.push(path);
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_edges_join_once_and_distant_branches_do_not_connect() {
        let a = [0.0; 3];
        let b = [1.0, 0.0, 0.0];
        let c = [2.0, 0.0, 0.0];
        let d = [0.0, 0.0, 2.0];
        let e = [1.0, 0.0, 2.0];
        let paths = stitch(
            &[[a, b], [b, a], [b, c], [d, e]],
            [-3.0; 3],
            [3.0; 3],
            || true,
        )
        .unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&vec![Some(a), Some(b), Some(c)]));
        assert!(paths.contains(&vec![Some(d), Some(e)]));
        assert!(stitch(&[[a, b]], [-3.0; 3], [3.0; 3], || false).is_err());
    }
}
