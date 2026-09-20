use sim_math::geometry::{Geometry, Point, Shape, triangle_area};
use sim_math::{Expression, IntegralKind, IntegrationJob, MathError};

fn value(text: &str, x: f64) -> f64 {
    Expression::parse(text).unwrap().evaluate(x, 1.0).unwrap()
}

#[test]
fn library_precedence_is_mathematical_and_division_is_real() {
    assert_eq!(value("-x^2", 3.0), -9.0);
    assert_eq!(value("2^3^2", 0.0), 512.0);
    assert_eq!(value("1/2", 0.0), 0.5);
    assert_eq!(value("2^-2", 0.0), 0.25);
    assert!((value("sin(pi/2)", 0.0) - 1.0).abs() < 1e-15);
}

#[test]
fn expressions_have_no_old_source_node_depth_or_coordinate_quotas() {
    let long = std::iter::repeat_n("x", 500).collect::<Vec<_>>().join("+");
    assert_eq!(value(&long, 2.0), 1000.0);
    let nested = format!("{}x{}", "(".repeat(200), ")".repeat(200));
    assert_eq!(value(&nested, 1e12), 1e12);
    assert_eq!(value("a+x", 1e12), 1e12 + 1.0);
}

#[test]
fn malformed_and_undefined_inputs_are_not_numeric_results() {
    for text in ["", "x+", "mystery(x)", "y", "sin(x,2)", "1e999"] {
        assert!(Expression::parse(text).is_err(), "{text}");
    }
    for text in ["1/0", "sqrt(-1)", "ln(0)"] {
        assert!(Expression::parse(text).unwrap().evaluate(0.0, 1.0).is_err());
    }
}

#[test]
fn hidden_singularities_and_cancellation_are_screened() {
    for text in [
        "tan(x)",
        "tg(x)",
        "1/(x-1.234567)",
        "0/(x-1.234567)",
        "sqrt(x-2)",
    ] {
        assert!(
            Expression::parse(text)
                .unwrap()
                .screen_interval(1.0, 6.0, 1.0)
                .is_err(),
            "{text}"
        );
    }
    assert!(
        Expression::parse("1/(x^2+1)")
            .unwrap()
            .screen_interval(-2.0, 2.0, 1.0)
            .is_ok()
    );
}

fn integral(text: &str, a: f64, b: f64, kind: IntegralKind, batch: usize) -> f64 {
    let mut job =
        IntegrationJob::new(Expression::parse(text).unwrap(), a, b, 1.0, kind, 1e-8).unwrap();
    for _ in 0..10000 {
        if let Some(result) = job.advance(batch).unwrap() {
            return result.value;
        }
    }
    panic!("analytic test did not finish");
}

#[test]
fn known_integrals_reversed_bounds_and_area_are_distinct() {
    for (text, a, b, expected) in [
        ("x^2", 0.0, 3.0, 9.0),
        ("sin(x)", 0.0, std::f64::consts::PI, 2.0),
        ("1/(x^2+1)", -1.0, 1.0, std::f64::consts::FRAC_PI_2),
        ("2", 3.0, -2.0, -10.0),
    ] {
        assert!((integral(text, a, b, IntegralKind::Signed, 4) - expected).abs() < 1e-7);
    }
    assert!(integral("x", -1.0, 1.0, IntegralKind::Signed, 4).abs() < 1e-12);
    assert!((integral("x", 1.0, -1.0, IntegralKind::Area, 4) - 1.0).abs() < 1e-8);
    assert_eq!(
        integral("x^2", 0.0, 2.0, IntegralKind::Signed, 1).to_bits(),
        integral("x^2", 0.0, 2.0, IntegralKind::Signed, 32).to_bits()
    );
}

#[test]
fn scheduler_quantum_does_not_reject_an_unfinished_calculation() {
    let mut job = IntegrationJob::new(
        Expression::parse("abs(x-0.123)").unwrap(),
        -1.0,
        1.0,
        1.0,
        IntegralKind::Area,
        1e-10,
    )
    .unwrap();
    assert!(job.advance(0).unwrap().is_none());
    assert!(job.advance(1).unwrap().is_none());
    assert_eq!(job.evaluations(), 48);
    drop(job); // Cancellation needs no forceful thread termination.
}

#[test]
fn geometry_preserves_dependencies_and_invalid_edits_are_atomic() {
    let mut g = Geometry::default();
    let a = g.add_free(Point::new(0.0, 0.0).unwrap()).unwrap();
    let b = g.add_free(Point::new(2.0, 0.0).unwrap()).unwrap();
    let m = g.add_midpoint(a, b).unwrap();
    g.add_shape(Shape::Segment(a, m)).unwrap();
    let expression = Expression::parse("a*x^2").unwrap();
    let f = g.add_on_function(2.0).unwrap();
    assert_eq!(
        g.resolve(&expression, 3.0)[f.index()],
        Some(Point { x: 2.0, y: 12.0 })
    );
    g.move_free(b, Point::new(4.0, 2.0).unwrap()).unwrap();
    assert_eq!(
        g.resolve(&expression, 1.0)[m.index()],
        Some(Point { x: 2.0, y: 1.0 })
    );
    let before = g.clone();
    assert_eq!(
        g.move_free(m, Point { x: 1.0, y: 1.0 }),
        Err(MathError::DerivedPoint)
    );
    assert!(
        g.move_free(
            a,
            Point {
                x: f64::NAN,
                y: 0.0
            }
        )
        .is_err()
    );
    assert_eq!(g, before);
    assert_eq!(
        triangle_area(
            Point { x: 0.0, y: 0.0 },
            Point { x: 3.0, y: 0.0 },
            Point { x: 0.0, y: 4.0 }
        )
        .unwrap(),
        6.0
    );
}

#[test]
fn collections_grow_and_deep_dependencies_resolve_without_recursion() {
    let mut g = Geometry::default();
    let a = g.add_free(Point { x: 0.0, y: 0.0 }).unwrap();
    let mut next = g.add_free(Point { x: 2.0, y: 2.0 }).unwrap();
    for _ in 0..5000 {
        next = g.add_midpoint(a, next).unwrap();
    }
    assert_eq!(g.points().len(), 5002);
    assert!(g.resolve(&Expression::parse("x").unwrap(), 1.0)[next.index()].is_some());
}

#[test]
fn vanished_intersections_propagate_unavailability() {
    let mut g = Geometry::default();
    let ids =
        [(0., 0.), (1., 1.), (0., 1.), (1., 0.)].map(|(x, y)| g.add_free(Point { x, y }).unwrap());
    let intersection = g.add_intersection(ids).unwrap();
    let dependent = g.add_midpoint(ids[0], intersection).unwrap();
    let f = Expression::parse("x").unwrap();
    assert_eq!(
        g.resolve(&f, 0.)[intersection.index()],
        Some(Point { x: 0.5, y: 0.5 })
    );
    g.move_free(ids[3], Point { x: 1., y: 2. }).unwrap();
    let resolved = g.resolve(&f, 0.);
    assert_eq!(resolved[intersection.index()], None);
    assert_eq!(resolved[dependent.index()], None);
}
