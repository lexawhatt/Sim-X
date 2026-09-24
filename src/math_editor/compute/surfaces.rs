//! View-dependent 3D sampling policy; all work and stitching runs off the UI.
use crate::math_editor::{
    compute::{Request, RowPlot, surfaces::paths as surface_paths},
    scene::clip,
};
use sim_math::{MathError, implicit3d::Implicit3d, relation::Relation};

pub(in crate::math_editor) fn implicit(
    row: &mut RowPlot,
    relation: &Implicit3d,
    request: &Request,
    alive: impl Fn() -> bool,
) -> Result<(), MathError> {
    row.boundary = Some(relation.relation);
    if !request.spatial {
        row.diagnostic = Some("Switch to 3D to view this boundary".into());
        return Ok(());
    }
    // Same invisible Z range as the axes and surface clipper. The view defines
    // sampling coverage, never the allowed mathematical/document domain.
    let bounds =
        clip::Bounds::from_xy(request.lower, request.upper).ok_or(MathError::NumericRange)?;
    let lower = bounds.ranges.map(|range| range[0]);
    let upper = bounds.ranges.map(|range| range[1]);
    let count = (request.pixels[0].min(request.pixels[1]) * 0.56 / 40.0)
        .ceil()
        .max(2.0) as usize;
    let cells = [count; 3];
    let segments = sim_math::implicit3d::wireframe(relation, lower, upper, cells, 8, &alive)?;
    row.surface = surface_paths::stitch(&segments, lower, upper, alive)?;
    row.diagnostic = Some(
        if row.surface.is_empty() {
            "No resolved boundary in this view"
        } else if relation.relation == Relation::Equal {
            "Sampled 3D boundary"
        } else if relation.relation.includes_boundary() {
            "Closed boundary only; no volume fill"
        } else {
            "Open boundary only; no volume fill"
        }
        .into(),
    );
    Ok(())
}

pub(in crate::math_editor) mod paths {
    //! Join implicit boundary segments on the worker before display simplification.
    //! Endpoints match within a billionth of the sampled view span. Junctions stop
    //! paths, so crossing slice families never arbitrarily change direction.
    use std::collections::{BTreeMap, BTreeSet};
    type Key = [i64; 3];

    pub(in crate::math_editor) fn stitch(
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
}
