use sim_math::{
    Expression, MathError,
    geometry::Point,
    statement::{Statement, definition},
    surface::wireframe,
};
use std::collections::BTreeMap;

#[test]
fn z_is_a_coordinate_and_height_field_uses_both_inputs() {
    assert!(definition("z=2").is_none());
    let Statement::Surface(expression) = Statement::parse("z=x^2-y^2", &BTreeMap::new()).unwrap()
    else {
        panic!("height field");
    };
    let paths = wireframe(
        &expression,
        Point { x: -2.0, y: -2.0 },
        Point { x: 2.0, y: 2.0 },
        [8, 8],
        4,
        || true,
    )
    .unwrap();
    assert_eq!(paths.len(), 18);
    for point in paths.iter().flatten() {
        let [x, y, z] = point.unwrap();
        assert!((z - (x * x - y * y)).abs() < 1e-12);
    }
    let Statement::Surface(constant) = Statement::parse("z=2", &BTreeMap::new()).unwrap() else {
        panic!("plane");
    };
    assert_eq!(constant.evaluate_at(5.0, 3.0).unwrap(), 2.0);
}

#[test]
fn surface_poles_and_invalid_domains_are_not_joined() {
    let expression = Expression::with_variables("1/(x-0.13)", &BTreeMap::new()).unwrap();
    let paths = wireframe(
        &expression,
        Point { x: -1.0, y: -1.0 },
        Point { x: 1.0, y: 1.0 },
        [4, 4],
        4,
        || true,
    )
    .unwrap();
    for path in paths {
        for pair in path.windows(2) {
            if let [Some(a), Some(b)] = pair {
                assert!((a[0] - 0.13) * (b[0] - 0.13) >= 0.0);
            }
        }
    }
    let expression = Expression::with_variables("sqrt(x-y)", &BTreeMap::new()).unwrap();
    let paths = wireframe(
        &expression,
        Point { x: -1.0, y: -1.0 },
        Point { x: 1.0, y: 1.0 },
        [8, 8],
        4,
        || true,
    )
    .unwrap();
    assert!(paths.iter().flatten().any(Option::is_none));
    assert!(
        paths
            .iter()
            .flatten()
            .flatten()
            .all(|[x, y, z]| x >= y && z.is_finite())
    );
}

#[test]
fn surface_sampling_cancels_and_reports_precision_loss() {
    let f = Expression::with_variables("x+y", &BTreeMap::new()).unwrap();
    assert_eq!(
        wireframe(
            &f,
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            [8, 8],
            4,
            || false
        ),
        Err(MathError::Cancelled)
    );
    assert_eq!(
        wireframe(
            &f,
            Point { x: 1e16, y: 0.0 },
            Point {
                x: 1e16 + 2.0,
                y: 1.0
            },
            [8, 8],
            4,
            || true
        ),
        Err(MathError::PrecisionExhausted)
    );
}
