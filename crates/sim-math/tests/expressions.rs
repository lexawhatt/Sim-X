use sim_math::{Expression, MathError, functions, geometry::Point, statement::Statement};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn catalog_names_are_unique_and_all_entries_validate_arity() {
    let mut names = BTreeSet::new();
    for f in functions::FUNCTIONS {
        assert!(names.insert(f.name));
        assert!(f.accepts(f.min_args));
        if f.min_args > 0 {
            assert!(f.evaluate(&vec![1.0; f.min_args - 1]).is_nan());
        }
        assert!(f.evaluate(&vec![f64::NAN; f.min_args]).is_nan());
    }
    assert_eq!(functions::find("tg").unwrap().name, "tan");
}
#[test]
fn scalar_functions_match_known_values_and_domains() {
    for (text, expected) in [
        ("csc(pi/2)", 1.0),
        ("sec(0)", 1.0),
        ("cot(pi/4)", 1.0),
        ("asin(0.5)", std::f64::consts::FRAC_PI_6),
        ("acot(0)", std::f64::consts::FRAC_PI_2),
        ("cosh(0)", 1.0),
        ("acsch(1)", 1.0_f64.asinh()),
        ("asech(1)", 0.0),
        ("acoth(2)", 0.5_f64.atanh()),
        ("asinh(0)", 0.0),
        ("logbase(8,2)", 3.0),
        ("cbrt(-8)", -2.0),
        ("round(-0.5)", -1.0),
        ("mod(-1,3)", 2.0),
        ("mean(2,4,6)", 4.0),
        ("median(1,9,3,7)", 5.0),
        ("stdev(1,2,3)", 1.0),
        ("varp(1,2,3)", 2.0 / 3.0),
        ("count(1,2,3)", 3.0),
        ("total(1,2,3)", 6.0),
        ("gamma(5)", 24.0),
        ("lngamma(1)", 0.0),
        ("erf(0)", 0.0),
        ("hypot(3,4)", 5.0),
        ("atan2(1,0)", std::f64::consts::FRAC_PI_2),
        ("deg(pi)", 180.0),
        ("rad(180)", std::f64::consts::PI),
        ("clamp(3,0,1)", 1.0),
        ("lerp(2,4,0.5)", 3.0),
        ("smoothstep(0.5)", 0.5),
        ("smootherstep(0.5)", 0.5),
        ("sinc(0)", 1.0),
        ("gaussian(0)", 1.0),
        ("sigmoid(0)", 0.5),
        ("softplus(1000)", 1000.0),
    ] {
        assert!(
            (Expression::constant(text).unwrap() - expected).abs() < 1e-11,
            "{text}"
        );
    }
    for text in [
        "gamma(0)",
        "gamma(-1)",
        "atan2(0,0)",
        "acosh(0)",
        "atanh(1)",
        "asin(2)",
        "stdev(1)",
        "mod(2,0)",
        "clamp(2,3,1)",
        "logbase(2,1)",
    ] {
        assert!(Expression::constant(text).is_err(), "{text}");
    }
}
#[test]
fn functions_screen_continuous_intervals_without_bridging_discontinuities() {
    for text in [
        "asin(x)",
        "atan2(x,2)",
        "var(x,1,2)",
        "stdev(x,1,2)",
        "sigmoid(x)",
        "softplus(x)",
        "sinc(x)",
        "erf(x)",
    ] {
        assert!(
            Expression::parse(text)
                .unwrap()
                .screen_interval(-0.5, 0.5, 0.0)
                .is_ok(),
            "{text}"
        );
    }
    for text in ["floor(x)", "mod(x,1)", "atan2(x,-1)", "csc(x)", "cot(x)"] {
        assert!(
            Expression::parse(text)
                .unwrap()
                .screen_interval(-0.5, 0.5, 0.0)
                .is_err(),
            "{text}"
        );
    }
}
#[test]
fn rows_distinguish_scalars_explicit_curves_implicit_equations_and_calculus() {
    let vars = BTreeMap::from([("b".into(), 3.0)]);
    assert!(matches!(
        Statement::parse("2", &vars),
        Ok(Statement::Scalar(2.0))
    ));
    assert!(matches!(
        Statement::parse("y=2", &vars),
        Ok(Statement::Curve(_))
    ));
    assert!(matches!(
        Statement::parse("x=y^2", &vars),
        Ok(Statement::InverseCurve(_))
    ));
    assert!(matches!(
        Statement::parse("x^2+y^2=3", &vars),
        Ok(Statement::Implicit(_))
    ));
    let Statement::Integral {
        body,
        lower,
        upper,
        graph_result,
    } = Statement::parse("y=integral(0,b,b*x^2)", &vars).unwrap()
    else {
        panic!()
    };
    assert_eq!((lower, upper, graph_result), (0.0, 3.0, true));
    assert_eq!(body.evaluate_at(2.0, 0.0).unwrap(), 12.0);
    assert!(matches!(
        Statement::parse("derivative(x^2)", &vars),
        Ok(Statement::Derivative(_))
    ));
    assert!(Statement::parse("integral(0,1,y)", &vars).is_err());
}
#[test]
fn implicit_circle_is_sampled_with_small_residual_and_poles_are_not_contours() {
    let vars = BTreeMap::new();
    let circle = Expression::with_variables("x^2+y^2-3", &vars).unwrap();
    let lo = Point { x: -2.0, y: -2.0 };
    let hi = Point { x: 2.0, y: 2.0 };
    let segments = sim_math::contour::contours(&circle, lo, hi, [60, 60], || true).unwrap();
    assert!(segments.len() > 100);
    for p in segments.iter().flatten() {
        assert!((p.x * p.x + p.y * p.y - 3.0).abs() < 0.006);
    }
    let pole = Expression::with_variables("1/x", &vars).unwrap();
    assert!(
        sim_math::contour::contours(&pole, lo, hi, [61, 61], || true)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        sim_math::contour::contours(&circle, lo, hi, [60, 60], || false),
        Err(MathError::Cancelled)
    );
}
