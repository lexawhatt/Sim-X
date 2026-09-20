use sim_math::{Expression, IntegralKind, IntegrationJob, MathError, functions, geometry::Point};
use std::collections::BTreeMap;

#[test]
fn indexed_roots_share_the_registry_alias_and_real_domain_convention() {
    let function = functions::find("root").unwrap();
    assert_eq!(function.signature, "n,x");
    assert_eq!(functions::find("nthroot").unwrap().name, "root");
    for (source, expected) in [
        ("root(3,-8)", -2.0),
        ("root(5,-32)", -2.0),
        ("nthroot(4,81)", 3.0),
        ("root(-3,-8)", -0.5),
        ("root(-5,-32)", -0.5),
        ("root(-2,16)", 0.25),
        ("root(0.5,3)", 9.0),
        ("root(-0.5,2)", 0.25),
        ("root(2.5,32)", 4.0),
        ("root(1,-7)", -7.0),
        ("root(-1,-4)", -0.25),
        ("root(3,0)", 0.0),
        ("root(0.25,0)", 0.0),
        ("root(3,root(5,-32768))", -2.0),
    ] {
        let actual = Expression::constant(source).unwrap();
        assert!((actual - expected).abs() < 1e-12, "{source}: {actual}");
    }
    for source in [
        "root(0,1)",
        "root(0,0)",
        "root(2,-1)",
        "root(-2,-1)",
        "root(2.5,-1)",
        "root(-3,0)",
        "root(-0.5,0)",
        "root(3.000000000000001,-8)",
        "root(9007199254740992,-1)",
        "root(9007199254740993,-1)",
        "root(1e30,-1)",
    ] {
        assert_eq!(
            Expression::constant(source),
            Err(MathError::Undefined),
            "{source}"
        );
    }
    assert_eq!(Expression::constant("root(9007199254740991,-1)"), Ok(-1.0));
    assert_eq!(Expression::constant("root(-9007199254740991,-1)"), Ok(-1.0));
    assert!(Expression::parse("root(3)").is_err());
    assert!(Expression::parse("root(3,8,1)").is_err());
}

#[test]
fn indexed_roots_handle_float_extremes_without_false_parity_or_intermediate_overflow() {
    let root = functions::find("root").unwrap();
    assert_eq!(root.evaluate(&[f64::from_bits(1), 1.0]), 1.0);
    assert_eq!(root.evaluate(&[-f64::from_bits(1), 1.0]), 1.0);
    assert_eq!(root.evaluate(&[1.0, f64::MAX]), f64::MAX);
    assert_eq!(root.evaluate(&[1.0, f64::from_bits(1)]), f64::from_bits(1));
    assert_eq!(
        root.evaluate(&[2.0, f64::from_bits(1)]),
        f64::from_bits(1).sqrt()
    );
    assert_eq!(
        root.evaluate(&[-2.0, f64::from_bits(1)]),
        f64::from_bits(1).sqrt().recip()
    );
    assert_eq!(
        root.evaluate(&[3.0, -f64::from_bits(1)]),
        -f64::from_bits(1).cbrt()
    );
    // Computing a positive root first would overflow; the final reciprocal
    // result is subnormal but representable and must not be replaced by zero.
    let subnormal = root.evaluate(&[-0.5, 1e155]);
    assert!(subnormal > 0.0 && (subnormal / 1e-310 - 1.0).abs() < 1e-10);
    for args in [
        [0.5, 1e155],
        [0.0, 1.0],
        [f64::INFINITY, 1.0],
        [f64::NAN, 1.0],
        [3.0, f64::NEG_INFINITY],
        [3.0, f64::NAN],
    ] {
        assert!(root.evaluate(&args).is_nan(), "{args:?}");
    }
}

#[test]
fn root_domain_screening_keeps_odd_negative_branches_and_rejects_poles() {
    for (source, interval) in [
        ("root(3,x)", [-8.0, 8.0]),
        ("nthroot(5,x)", [-32.0, 0.0]),
        ("root(1+2,x)", [-8.0, 8.0]),
        ("root(6/2,x)", [-8.0, 8.0]),
        ("root(a,x)", [-8.0, 8.0]),
        ("root(3,root(5,x))", [-8.0, 8.0]),
        ("root(2,root(4,x))", [0.0, 256.0]),
        ("sqrt(cbrt(x))", [0.0, 8.0]),
        ("root(-3,x)", [-8.0, -1.0]),
        ("root(-3,x)", [1.0, 8.0]),
        ("root(0.5,x)", [0.0, 4.0]),
        ("root(x,16)", [2.0, 4.0]),
        ("root(x,0)", [0.5, 8.0]),
        ("root(x,0.5)", [-4.0, -2.0]),
    ] {
        assert!(
            Expression::parse(source)
                .unwrap()
                .screen_interval(interval[0], interval[1], 3.0)
                .is_ok(),
            "{source}"
        );
    }
    for (source, interval) in [
        ("root(2,x)", [-1.0, 1.0]),
        ("root(2.5,x)", [-1.0, 1.0]),
        ("root(-3,x)", [-1.0, 1.0]),
        ("root(-3,x)", [0.0, 1.0]),
        ("root(-3,x)", [-1.0, 0.0]),
        ("root(0,x)", [1.0, 2.0]),
        ("root(x,16)", [-1.0, 1.0]),
        ("root(x,-8)", [3.0, 5.0]),
        ("root(x,0)", [-4.0, -2.0]),
    ] {
        assert!(
            Expression::parse(source)
                .unwrap()
                .screen_interval(interval[0], interval[1], 0.0)
                .is_err(),
            "{source}"
        );
    }
    let variable = Expression::with_variables("root(x,y)", &BTreeMap::new()).unwrap();
    assert!(variable.screen_box([0.5, 3.0], [0.0, 4.0]).is_ok());
    assert!(variable.screen_box([-3.0, -0.5], [0.25, 4.0]).is_ok());
    assert!(variable.screen_box([2.0, 4.0], [-8.0, -1.0]).is_err());
}

#[test]
fn roots_support_public_integral_surface_and_cancellation_paths() {
    for (source, from, to, expected) in [
        ("root(3,x)", -8.0, -1.0, -11.25),
        ("root(-3,x)", 1.0, 8.0, 4.5),
        ("root(4,x)", 0.0, 16.0, 25.6),
    ] {
        let mut job = IntegrationJob::new(
            Expression::parse(source).unwrap(),
            from,
            to,
            0.0,
            IntegralKind::Signed,
            1e-8,
        )
        .unwrap();
        let result = loop {
            if let Some(result) = job.advance(16).unwrap() {
                break result;
            }
        };
        assert!(
            (result.value - expected).abs() < 1e-7,
            "{source}: {}",
            result.value
        );
    }
    assert!(
        IntegrationJob::new(
            Expression::parse("root(-3,x)").unwrap(),
            -1.0,
            1.0,
            0.0,
            IntegralKind::Signed,
            1e-8
        )
        .is_err()
    );
    let expression = Expression::with_variables("root(3,x)-y", &BTreeMap::new()).unwrap();
    let lower = Point { x: -8.0, y: -2.1 };
    let upper = Point { x: 8.0, y: 2.1 };
    let segments =
        sim_math::contour::contours(&expression, lower, upper, [32, 32], || true).unwrap();
    assert!(segments.iter().flatten().any(|p| p.x < -1.0 && p.y < -1.0));
    assert!(segments.iter().flatten().any(|p| p.x > 1.0 && p.y > 1.0));
    assert_eq!(
        sim_math::contour::contours(&expression, lower, upper, [32, 32], || false),
        Err(MathError::Cancelled)
    );
    let height = Expression::with_variables("root(3,x)", &BTreeMap::new()).unwrap();
    let paths = sim_math::surface::wireframe(&height, lower, upper, [8, 4], 2, || true).unwrap();
    assert!(paths.iter().flatten().all(Option::is_some));
    for [x, _, z] in paths.iter().flatten().flatten() {
        assert!((z * z * z - x).abs() < 1e-12);
    }
    assert_eq!(
        sim_math::surface::wireframe(&height, lower, upper, [8, 4], 2, || false),
        Err(MathError::Cancelled)
    );
}
