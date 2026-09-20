//! Bounded center-membership geometry for authoring selection, not physics.

use super::document::Point;

/// Hard bound shared by lasso sampling and its closed visual outline.
pub(crate) const MAX_SELECTION_POINTS: usize = 256;
const MAX_COORDINATE: f64 = 1_000_000.0;
const MIN_EPSILON: f64 = 1e-10;

/// Transient rectangle or closed even-odd lasso in editor world coordinates.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SelectionGesture {
    Rectangle { start: Point, end: Point },
    Lasso { points: Vec<Point> },
}

impl SelectionGesture {
    /// Starts a rectangle. Invalid origins remain inert rather than becoming
    /// an unrelated selection around the coordinate origin.
    pub(crate) fn rectangle(start: Point) -> Self {
        Self::Rectangle { start, end: start }
    }

    /// Starts a sampled lasso. An invalid origin produces an inert empty path.
    pub(crate) fn lasso(start: Point) -> Self {
        Self::Lasso {
            points: if valid(start) {
                vec![start]
            } else {
                Vec::new()
            },
        }
    }

    /// Updates a rectangle endpoint or samples a lasso point after sufficient
    /// world-space movement. Zero spacing includes the exact release endpoint.
    /// Invalid input is ignored. At capacity, deterministic every-other-point
    /// decimation preserves the first and previous final sample before append.
    pub(crate) fn update(&mut self, point: Point, min_spacing_world: f64) {
        if !valid(point) || !min_spacing_world.is_finite() || min_spacing_world < 0.0 {
            return;
        }
        match self {
            Self::Rectangle { start, end } => {
                if valid(*start) {
                    *end = point;
                }
            }
            Self::Lasso { points } => {
                if !valid_path(points) {
                    return;
                }
                let Some(&previous) = points.last() else {
                    return;
                };
                if previous == point || distance(previous, point) < min_spacing_world {
                    return;
                }
                if points.len() == MAX_SELECTION_POINTS {
                    let last = points.len() - 1;
                    let mut index = 0;
                    points.retain(|_| {
                        let keep = index % 2 == 0 || index == last;
                        index += 1;
                        keep
                    });
                }
                points.push(point);
            }
        }
    }

    /// Tests a body's center, including geometric boundaries. Degenerate,
    /// invalid or over-budget gestures select nothing. Shape intersection and
    /// physical collision are intentionally not part of this UI operation.
    pub(crate) fn contains(&self, center: Point) -> bool {
        if !valid(center) {
            return false;
        }
        match self {
            Self::Rectangle { start, end } => {
                if !valid(*start) || !valid(*end) {
                    return false;
                }
                let epsilon = tolerance(&[*start, *end, center]);
                if (start.x - end.x).abs() <= epsilon || (start.y - end.y).abs() <= epsilon {
                    return false;
                }
                center.x >= start.x.min(end.x) - epsilon
                    && center.x <= start.x.max(end.x) + epsilon
                    && center.y >= start.y.min(end.y) - epsilon
                    && center.y <= start.y.max(end.y) + epsilon
            }
            Self::Lasso { points } => {
                if !valid_path(points) || points.len() < 3 {
                    return false;
                }
                let epsilon = tolerance(points).max(tolerance(&[center]));
                if !has_area(points, epsilon) {
                    return false;
                }
                let mut inside = false;
                for index in 0..points.len() {
                    let a = points[index];
                    let b = points[(index + 1) % points.len()];
                    if on_segment(center, a, b, epsilon) {
                        return true;
                    }
                    if (a.y > center.y) != (b.y > center.y) {
                        let fraction = (center.y - a.y) / (b.y - a.y);
                        let crossing = a.x + fraction * (b.x - a.x);
                        if center.x < crossing {
                            inside = !inside;
                        }
                    }
                }
                inside
            }
        }
    }

    /// Returns at most 256 finite world-space segments. Lasso preview closes
    /// after three points; a two-point unfinished path still displays one line.
    pub(crate) fn outline(&self) -> Vec<(Point, Point)> {
        match self {
            Self::Rectangle { start, end } => {
                if !valid(*start) || !valid(*end) {
                    return Vec::new();
                }
                let epsilon = tolerance(&[*start, *end]);
                if (start.x - end.x).abs() <= epsilon || (start.y - end.y).abs() <= epsilon {
                    return Vec::new();
                }
                let top = Point::new(end.x, start.y);
                let bottom = Point::new(start.x, end.y);
                vec![(*start, top), (top, *end), (*end, bottom), (bottom, *start)]
            }
            Self::Lasso { points } => {
                if !valid_path(points) || points.len() < 2 {
                    return Vec::new();
                }
                let mut lines = Vec::with_capacity(points.len());
                lines.extend(points.windows(2).map(|pair| (pair[0], pair[1])));
                if points.len() >= 3 && points[0] != points[points.len() - 1] {
                    lines.push((points[points.len() - 1], points[0]));
                }
                lines
            }
        }
    }
}

fn valid(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= MAX_COORDINATE
        && point.y.abs() <= MAX_COORDINATE
}

fn valid_path(points: &[Point]) -> bool {
    points.len() <= MAX_SELECTION_POINTS && points.iter().copied().all(valid)
}

fn tolerance(points: &[Point]) -> f64 {
    let scale = points.iter().fold(1.0_f64, |scale, point| {
        scale.max(point.x.abs()).max(point.y.abs())
    });
    (scale * 64.0 * f64::EPSILON).max(MIN_EPSILON)
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn on_segment(point: Point, a: Point, b: Point, epsilon: f64) -> bool {
    let length = distance(a, b);
    if length <= epsilon {
        return distance(point, a) <= epsilon;
    }
    let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
    cross.abs() <= epsilon * length
        && point.x >= a.x.min(b.x) - epsilon
        && point.x <= a.x.max(b.x) + epsilon
        && point.y >= a.y.min(b.y) - epsilon
        && point.y <= a.y.max(b.y) + epsilon
}

fn has_area(points: &[Point], epsilon: f64) -> bool {
    let origin = points[0];
    let Some(&other) = points
        .iter()
        .find(|&&point| distance(point, origin) > epsilon)
    else {
        return false;
    };
    let baseline = distance(origin, other);
    // Noncollinearity, rather than signed polygon area, also accepts a lasso
    // whose self-intersecting lobes cancel algebraically under opposite winding.
    points.iter().any(|point| {
        let cross = (other.x - origin.x) * (point.y - origin.y)
            - (other.y - origin.y) * (point.x - origin.x);
        cross.abs() > epsilon * baseline
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }
    fn polygon(points: &[Point]) -> SelectionGesture {
        let mut gesture = SelectionGesture::lasso(points[0]);
        for &point in &points[1..] {
            gesture.update(point, 0.0);
        }
        gesture
    }

    #[test]
    fn rectangle_is_reversible_boundary_inclusive_and_closed() {
        for (start, end) in [(p(-2.0, -1.0), p(3.0, 4.0)), (p(3.0, 4.0), p(-2.0, -1.0))] {
            let mut gesture = SelectionGesture::rectangle(start);
            gesture.update(end, 0.0);
            for point in [p(0.0, 0.0), start, end, p(-2.0, 2.0), p(1.0, 4.0)] {
                assert!(gesture.contains(point));
            }
            for point in [p(-2.1, 0.0), p(3.1, 0.0), p(0.0, 4.1)] {
                assert!(!gesture.contains(point));
            }
            let lines = gesture.outline();
            assert_eq!(lines.len(), 4);
            for index in 0..4 {
                assert_eq!(lines[index].1, lines[(index + 1) % 4].0);
            }
        }
    }

    #[test]
    fn concave_lasso_works_in_both_windings_and_includes_edges() {
        let mut points = vec![
            p(0.0, 0.0),
            p(4.0, 0.0),
            p(4.0, 1.0),
            p(1.0, 1.0),
            p(1.0, 4.0),
            p(0.0, 4.0),
        ];
        for _ in 0..2 {
            let gesture = polygon(&points);
            for point in [
                p(0.5, 3.0),
                p(3.0, 0.5),
                p(1.0, 2.0),
                p(1.0, 1.0),
                p(0.0, 0.0),
            ] {
                assert!(gesture.contains(point));
            }
            for point in [p(2.0, 2.0), p(-0.5, 0.0), p(5.0, 0.5)] {
                assert!(!gesture.contains(point));
            }
            assert_eq!(gesture.outline().len(), points.len());
            points.reverse();
        }
    }

    #[test]
    fn degenerate_click_lines_and_collinear_paths_select_nothing() {
        let mut rectangle = SelectionGesture::rectangle(p(0.0, 0.0));
        assert!(!rectangle.contains(p(0.0, 0.0)));
        rectangle.update(p(3.0, 0.0), 0.0);
        assert!(!rectangle.contains(p(1.0, 0.0)));
        for points in [
            vec![p(0.0, 0.0)],
            vec![p(0.0, 0.0), p(1.0, 1.0)],
            vec![p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0), p(0.0, 0.0)],
            vec![p(0.0, 0.0), p(1e-12, 0.0), p(0.0, 1e-12)],
        ] {
            let gesture = polygon(&points);
            assert!(!gesture.contains(p(0.0, 0.0)));
            assert!(!gesture.contains(p(1.0, 1.0)));
        }
    }

    #[test]
    fn sampling_respects_spacing_and_exact_release_endpoint() {
        let mut gesture = SelectionGesture::lasso(p(0.0, 0.0));
        gesture.update(p(0.5, 0.0), 1.0);
        assert!(gesture.outline().is_empty());
        gesture.update(p(1.0, 0.0), 1.0);
        assert_eq!(gesture.outline(), vec![(p(0.0, 0.0), p(1.0, 0.0))]);
        gesture.update(p(1.0, 0.2), 0.0);
        let SelectionGesture::Lasso { points } = gesture else {
            panic!("lasso expected")
        };
        assert_eq!(points, vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 0.2)]);
    }

    #[test]
    fn decimation_is_deterministic_bounded_and_keeps_endpoints() {
        let mut first = SelectionGesture::lasso(p(0.0, 0.0));
        let mut second = first.clone();
        for i in 1..20000 {
            let point = p(i as f64, (i as f64 * 0.03).sin());
            first.update(point, 0.0);
            second.update(point, 0.0);
            assert_eq!(first, second);
            let SelectionGesture::Lasso { points } = &first else {
                panic!("lasso expected")
            };
            assert!(points.len() <= MAX_SELECTION_POINTS);
            assert_eq!(points[0], p(0.0, 0.0));
            assert_eq!(points.last(), Some(&point));
            assert!(first.outline().len() <= MAX_SELECTION_POINTS);
        }
    }

    #[test]
    fn exact_capacity_preserves_previous_last_when_decimating() {
        let mut gesture = SelectionGesture::lasso(p(0.0, 0.0));
        for i in 1..MAX_SELECTION_POINTS {
            gesture.update(p(i as f64, 0.0), 0.0);
        }
        gesture.update(p(256.0, 1.0), 0.0);
        let SelectionGesture::Lasso { points } = gesture else {
            panic!("lasso expected")
        };
        assert_eq!(points.len(), 130);
        assert_eq!(points[0], p(0.0, 0.0));
        assert_eq!(points[128], p(255.0, 0.0));
        assert_eq!(points[129], p(256.0, 1.0));
    }

    #[test]
    fn invalid_and_out_of_range_inputs_are_inert_without_over_budget_work() {
        let mut gesture = polygon(&[p(0.0, 0.0), p(2.0, 0.0), p(0.0, 2.0)]);
        let before = gesture.clone();
        for point in [
            p(f64::NAN, 0.0),
            p(0.0, f64::INFINITY),
            p(f64::MAX, 0.0),
            p(1e6 + 1.0, 0.0),
        ] {
            gesture.update(point, 0.0);
            assert_eq!(gesture, before);
            assert!(!gesture.contains(point));
            let mut invalid = SelectionGesture::rectangle(point);
            invalid.update(p(1.0, 1.0), 0.0);
            assert!(!invalid.contains(p(0.0, 0.0)));
            assert!(invalid.outline().is_empty());
            let mut invalid = SelectionGesture::lasso(point);
            invalid.update(p(1.0, 1.0), 0.0);
            assert!(invalid.outline().is_empty());
        }
        for spacing in [f64::NAN, f64::INFINITY, -1.0] {
            gesture.update(p(1.0, 1.0), spacing);
            assert_eq!(gesture, before);
        }
        let mut oversized = SelectionGesture::Lasso {
            points: vec![p(0.0, 0.0); MAX_SELECTION_POINTS + 1],
        };
        let before = oversized.clone();
        oversized.update(p(1.0, 1.0), 0.0);
        assert_eq!(oversized, before);
        assert!(!oversized.contains(p(0.0, 0.0)));
        assert!(oversized.outline().is_empty());
    }

    #[test]
    fn large_world_coordinates_and_self_crossing_lobes_remain_usable() {
        let origin = 999_990.0;
        let gesture = polygon(&[
            p(origin, origin),
            p(origin + 5.0, origin),
            p(origin + 5.0, origin + 5.0),
            p(origin, origin + 5.0),
        ]);
        assert!(gesture.contains(p(origin + 2.5, origin + 2.5)));
        assert!(gesture.contains(p(origin, origin + 1.0)));
        assert!(!gesture.contains(p(origin - 0.001, origin + 1.0)));
        let crossed = polygon(&[p(0.0, 0.0), p(2.0, 2.0), p(0.0, 2.0), p(2.0, 0.0)]);
        assert!(crossed.contains(p(1.0, 0.25)));
        assert!(crossed.contains(p(1.0, 1.75)));
        assert!(!crossed.contains(p(0.1, 1.0)));
    }
}
