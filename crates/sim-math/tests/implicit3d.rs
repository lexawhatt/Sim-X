use sim_math::{
    Expression, MathError,
    implicit3d::{Implicit3d, wireframe},
    relation::Relation,
    statement::{Statement, definition},
};
use std::{cell::Cell, collections::BTreeMap};

fn relation(source: &str) -> Implicit3d {
    let Statement::Implicit3d(relation) = Statement::parse(source, &BTreeMap::new()).unwrap()
    else {
        panic!("expected a spatial relation: {source}");
    };
    relation
}

#[test]
fn spatial_coordinates_are_explicit_and_not_silently_flattened() {
    let bindings = BTreeMap::from([("a".into(), 5.0), ("z".into(), 99.0)]);
    let expression = Expression::with_coordinates("x+2*y+3*z+a", &bindings).unwrap();
    assert_eq!(expression.evaluate_xyz(1.0, 2.0, 3.0), Ok(19.0));
    assert_eq!(expression.evaluate_at(1.0, 2.0), Err(MathError::Syntax));
    assert_eq!(expression.evaluate(1.0, 2.0), Err(MathError::Syntax));
    assert_eq!(
        expression.screen_box([-1.0, 1.0], [-1.0, 1.0]),
        Err(MathError::Syntax)
    );
    assert_eq!(
        expression.screen_interval(-1.0, 1.0, 2.0),
        Err(MathError::Syntax)
    );
    assert!(Expression::with_variables("z", &bindings).is_err());
    assert!(Expression::constant_with("z", &bindings).is_err());
    assert_eq!(
        expression.screen_volume([-1.0, 1.0], [-2.0, 2.0], [-3.0, 3.0]),
        Ok(())
    );
    assert_eq!(
        expression.screen_volume([-1.0, 1.0], [-2.0, 2.0], [3.0, -3.0]),
        Err(MathError::NumericRange)
    );
    for nonfinite in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            expression.evaluate_xyz(1.0, 2.0, nonfinite),
            Err(MathError::NumericRange)
        );
        assert_eq!(
            expression.screen_volume([-1.0, 1.0], [-1.0, 1.0], [0.0, nonfinite]),
            Err(MathError::NumericRange)
        );
    }
}

#[test]
fn classification_distinguishes_xyz_relations_height_fields_and_xy_curves() {
    let empty = BTreeMap::new();
    for source in ["y=z", "x=z", "z=z^2", "x+y+z=0", "0=z"] {
        assert_eq!(relation(source).relation, Relation::Equal);
    }
    for source in ["z=x^2+y^2", "z=2", "z=y"] {
        assert!(matches!(
            Statement::parse(source, &empty),
            Ok(Statement::Surface(_))
        ));
    }
    for source in ["y=x", "x^2", "y=3"] {
        assert!(matches!(
            Statement::parse(source, &empty),
            Ok(Statement::Curve(_))
        ));
    }
    assert!(matches!(
        Statement::parse("x=y^2", &empty),
        Ok(Statement::InverseCurve(_))
    ));
    assert!(matches!(
        Statement::parse("x^2+y^2=9", &empty),
        Ok(Statement::Implicit(_))
    ));
    assert!(matches!(
        Statement::parse("3", &empty),
        Ok(Statement::Scalar(3.0))
    ));
    assert!(Statement::parse("z", &empty).is_err());
    assert!(Statement::parse("x+y+z", &empty).is_err());
    assert_eq!(
        Statement::parse("x^2+y^2<=1", &empty).unwrap_err(),
        MathError::UnsupportedPlanarInequality
    );
    for source in ["a>=1", "a<=1", "a=1<2", "a=1=2", "z=2"] {
        assert!(definition(source).is_none());
    }
    assert_eq!(definition("a=2"), Some(("a".into(), "2")));
    for source in ["x<y<z", "z==1", "z!=0", "z<", "<z", "sin(z<1)=0"] {
        assert!(Statement::parse(source, &empty).is_err(), "{source}");
    }
}

#[test]
fn inequality_metadata_preserves_strictness_and_side() {
    for (operator, expected) in [
        ("=", Relation::Equal),
        ("<", Relation::Less),
        ("<=", Relation::LessOrEqual),
        (">", Relation::Greater),
        (">=", Relation::GreaterOrEqual),
        ("\u{2264}", Relation::LessOrEqual),
        ("\u{2265}", Relation::GreaterOrEqual),
    ] {
        let parsed = relation(&format!("x^2+y^2+z^2{operator}9"));
        assert_eq!(parsed.relation, expected);
        let center = parsed.expression.evaluate_xyz(0.0, 0.0, 0.0).unwrap();
        assert_eq!(
            expected.accepts(center),
            matches!(expected, Relation::Less | Relation::LessOrEqual)
        );
        assert_eq!(expected.accepts(0.0), expected.includes_boundary());
        assert!(!expected.accepts(f64::NAN));
        assert!(!expected.accepts(f64::INFINITY));
        assert!(!expected.accepts(f64::NEG_INFINITY));
        assert!(matches!(expected.symbol(), "=" | "<" | "<=" | ">" | ">="));
    }
}

#[test]
fn sphere_is_sampled_in_all_three_coordinate_sections() {
    let sphere = relation("x^2+y^2+z^2=9");
    let segments = wireframe(&sphere, [-4.0; 3], [4.0; 3], [8; 3], 8, || true).unwrap();
    assert!(!segments.is_empty());
    let mut extrema = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for segment in segments {
        assert_ne!(segment[0], segment[1]);
        for point in segment {
            let radius = point.iter().map(|v| v * v).sum::<f64>().sqrt();
            assert!((radius - 3.0).abs() < 0.004, "{point:?}: {radius}");
            for axis in 0..3 {
                extrema[axis][0] = extrema[axis][0].min(point[axis]);
                extrema[axis][1] = extrema[axis][1].max(point[axis]);
            }
        }
    }
    for [low, high] in extrema {
        assert!((low + 3.0).abs() < 1e-12);
        assert!((high - 3.0).abs() < 1e-12);
    }
}

#[test]
fn max_abs_inequality_has_cube_boundaries_even_at_exact_view_bounds() {
    let cube = relation("max(abs(x),abs(y),abs(z))<=1");
    for extent in [1.0, 2.0] {
        let segments = wireframe(&cube, [-extent; 3], [extent; 3], [4; 3], 4, || true).unwrap();
        assert!(!segments.is_empty());
        let mut faces = [[false; 2]; 3];
        for segment in segments {
            assert_ne!(segment[0], segment[1]);
            for point in segment {
                let norm = point.into_iter().map(f64::abs).fold(0.0, f64::max);
                assert!((norm - 1.0).abs() < 1e-12, "{point:?}");
                for axis in 0..3 {
                    faces[axis][0] |= point[axis] == -1.0;
                    faces[axis][1] |= point[axis] == 1.0;
                }
            }
        }
        assert!(faces.into_iter().flatten().all(|present| present));
    }
    let strict = relation("max(abs(x),abs(y),abs(z))<1");
    assert!(!strict.relation.includes_boundary());
    let inclusive = wireframe(&cube, [-2.0; 3], [2.0; 3], [4; 3], 2, || true).unwrap();
    let open = wireframe(&strict, [-2.0; 3], [2.0; 3], [4; 3], 2, || true).unwrap();
    assert_eq!(open, inclusive);
}

#[test]
fn arbitrary_relations_and_planes_are_not_hardcoded_shapes() {
    for source in ["x+y+z=0", "y=z", "x=2*z"] {
        let plane = relation(source);
        let segments = wireframe(&plane, [-2.0; 3], [2.0; 3], [4; 3], 4, || true).unwrap();
        assert!(!segments.is_empty());
        for [x, y, z] in segments.into_iter().flatten() {
            assert!(plane.expression.evaluate_xyz(x, y, z).unwrap().abs() < 1e-12);
        }
    }
    let saddle = relation("x*y=z^2");
    let segments = wireframe(&saddle, [-2.0; 3], [2.0; 3], [4; 3], 8, || true).unwrap();
    assert!(!segments.is_empty());
    for [x, y, z] in segments.into_iter().flatten() {
        assert!(saddle.expression.evaluate_xyz(x, y, z).unwrap().abs() < 0.02);
    }
}

#[test]
fn poles_do_not_masquerade_as_zero_surfaces_and_real_domain_gaps_remain() {
    for source in ["1/z=0", "1/(z-0.13)=0", "1/(x+y+z)=0"] {
        let segments =
            wireframe(&relation(source), [-1.0; 3], [1.0; 3], [4; 3], 4, || true).unwrap();
        assert!(segments.is_empty(), "false zero boundary for {source}");
    }
    let curved = relation("sqrt(z)=x+y");
    let segments = wireframe(&curved, [-1.0; 3], [1.0; 3], [4; 3], 8, || true).unwrap();
    assert!(!segments.is_empty());
    for [x, y, z] in segments.into_iter().flatten() {
        assert!(z >= 0.0);
        assert!(x + y >= -1e-12);
        assert!(curved.expression.evaluate_xyz(x, y, z).unwrap().abs() < 0.07);
    }
}

#[test]
fn cancellation_checks_vertices_and_cells_without_partial_success() {
    let plane = relation("x+y+z=0");
    assert_eq!(
        wireframe(&plane, [-1.0; 3], [1.0; 3], [4; 3], 4, || false),
        Err(MathError::Cancelled)
    );
    let calls = Cell::new(0);
    let result = wireframe(&plane, [-1.0; 3], [1.0; 3], [4; 3], 4, || {
        calls.set(calls.get() + 1);
        calls.get() < 40
    });
    assert_eq!(result, Err(MathError::Cancelled));
    assert_eq!(calls.get(), 40);
}

#[test]
fn invalid_and_unrepresentable_resolution_fails_without_work_caps() {
    let plane = relation("x+y+z=0");
    for (lower, upper, cells, refinement, error) in [
        ([-1.0; 3], [1.0; 3], [0, 4, 4], 4, MathError::NumericRange),
        ([-1.0; 3], [1.0; 3], [4; 3], 0, MathError::NumericRange),
        (
            [-1.0; 3],
            [1.0; 3],
            [usize::MAX; 3],
            4,
            MathError::NumericRange,
        ),
        (
            [-1.0; 3],
            [1.0; 3],
            [4; 3],
            usize::MAX,
            MathError::NumericRange,
        ),
        ([1.0; 3], [-1.0; 3], [4; 3], 4, MathError::NumericRange),
        (
            [0.0; 3],
            [0.0, 1.0, 1.0],
            [4; 3],
            4,
            MathError::NumericRange,
        ),
        (
            [f64::NEG_INFINITY; 3],
            [1.0; 3],
            [4; 3],
            4,
            MathError::NumericRange,
        ),
        (
            [-f64::MAX; 3],
            [f64::MAX; 3],
            [4; 3],
            4,
            MathError::NumericRange,
        ),
        (
            [1e16; 3],
            [1e16 + 2.0; 3],
            [4; 3],
            4,
            MathError::PrecisionExhausted,
        ),
    ] {
        assert_eq!(
            wireframe(&plane, lower, upper, cells, refinement, || true),
            Err(error)
        );
    }
}

fn assert_box_edges(segments: &[[[f64; 3]; 2]], bounds: [[f64; 2]; 3]) {
    for direction in 0..3 {
        let u = (direction + 1) % 3;
        let v = (direction + 2) % 3;
        for fixed_u in bounds[u] {
            for fixed_v in bounds[v] {
                let mut intervals: Vec<_> = segments
                    .iter()
                    .filter(|segment| {
                        segment.iter().all(|point| {
                            (point[u] - fixed_u).abs() < 1e-12 && (point[v] - fixed_v).abs() < 1e-12
                        })
                    })
                    .map(|[a, b]| {
                        [
                            a[direction].min(b[direction]),
                            a[direction].max(b[direction]),
                        ]
                    })
                    .collect();
                intervals.sort_unstable_by(|a, b| a[0].total_cmp(&b[0]));
                let mut end = bounds[direction][0];
                assert!(
                    !intervals.is_empty(),
                    "missing edge along {direction} at {fixed_u}, {fixed_v}"
                );
                for [low, high] in intervals {
                    assert!(
                        low <= end + 1e-12,
                        "gap on edge along {direction}: {end}..{low}"
                    );
                    end = end.max(high);
                }
                assert!((end - bounds[direction][1]).abs() < 1e-12);
            }
        }
    }
}

#[test]
fn discovered_extremal_sections_reveal_all_box_edges_on_misaligned_grids() {
    let cube = relation("max(abs(x),abs(y),abs(z))<=1");
    for (lower, upper, cells) in [
        ([-7.0, -4.5, -7.0], [7.0, 4.5, 7.0], [13; 3]),
        ([-6.73, -4.11, -6.73], [7.89, 4.52, 6.73], [11; 3]),
        ([-10.0; 3], [10.0; 3], [18; 3]),
    ] {
        let segments = wireframe(&cube, lower, upper, cells, 8, || true).unwrap();
        let mut extents = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for p in segments.iter().flatten() {
            for (axis, value) in p.iter().enumerate() {
                extents[axis][0] = extents[axis][0].min(*value);
                extents[axis][1] = extents[axis][1].max(*value);
            }
        }
        assert!(!segments.is_empty());
        assert_eq!(extents, [[-1.0, 1.0]; 3]);
        assert_box_edges(&segments, [[-1.0, 1.0]; 3]);
    }
}

#[test]
fn discovered_outline_is_independent_of_relation_direction_or_origin() {
    for source in [
        "max(abs(x),abs(y),abs(z))=1",
        "1>=max(abs(x),abs(y),abs(z))",
        "max(abs(x),abs(y),abs(z))>1",
    ] {
        let segments = wireframe(
            &relation(source),
            [-6.73, -4.11, -6.73],
            [7.89, 4.52, 6.73],
            [11; 3],
            8,
            || true,
        )
        .unwrap();
        assert_box_edges(&segments, [[-1.0, 1.0]; 3]);
    }
    let translated = relation("max(abs((x-0.5)/2),abs((y+0.25)/0.5),abs((z-1)/1.5))<=1");
    let segments = wireframe(
        &translated,
        [-6.73, -4.11, -6.73],
        [7.89, 4.52, 6.73],
        [11; 3],
        8,
        || true,
    )
    .unwrap();
    assert_box_edges(&segments, [[-1.5, 2.5], [-0.75, 0.25], [-0.5, 2.5]]);
}

#[test]
fn cancellation_during_the_supplementary_pass_discards_the_whole_result() {
    let cube = relation("max(abs(x),abs(y),abs(z))<=1");
    let calls = Cell::new(0);
    wireframe(&cube, [-2.3; 3], [2.7; 3], [4; 3], 4, || {
        calls.set(calls.get() + 1);
        true
    })
    .unwrap();
    let cancel_at = calls.get() - 1;
    calls.set(0);
    let result = wireframe(&cube, [-2.3; 3], [2.7; 3], [4; 3], 4, || {
        calls.set(calls.get() + 1);
        calls.get() < cancel_at
    });
    assert_eq!(result, Err(MathError::Cancelled));
    assert_eq!(calls.get(), cancel_at);
}
