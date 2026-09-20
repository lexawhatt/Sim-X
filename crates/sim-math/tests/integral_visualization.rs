use sim_math::{
    Expression,
    calculus::{IntegralExpression, IntegralVisualization},
};
use std::collections::BTreeMap;

fn plan(source: &str) -> IntegralVisualization {
    IntegralExpression::parse(source, &BTreeMap::from([("k".into(), 2.0)]))
        .unwrap()
        .unwrap()
        .visualization()
}
fn separation(plan: IntegralVisualization, x: f64) -> f64 {
    let IntegralVisualization::Between { positive, negative } = plan else {
        panic!("expected a difference");
    };
    assert_eq!(positive.direction(), Some(1.0));
    assert_eq!(negative.direction(), Some(-1.0));
    positive.expression.evaluate_at(x, 0.0).unwrap() * positive.display_scale()
        - negative.expression.evaluate_at(x, 0.0).unwrap() * negative.display_scale()
}
#[test]
fn common_interval_difference_tracks_sign_scale_and_orientation() {
    assert!((separation(plan("integral(0,1,x)-integral(0,1,x^2)"), 0.5) - 0.25).abs() < 1e-12);
    assert!((separation(plan("integral(1,0,x)-integral(1,0,x^2)"), 0.5) + 0.25).abs() < 1e-12);
    assert!((separation(plan("integral(0,1,x)+integral(1,0,x^2)"), 0.5) - 0.25).abs() < 1e-12);
    assert!(
        (separation(plan("k*(integral(0,1,x)-integral(0,1,x^2))/2"), 0.5) - 0.25).abs() < 1e-12
    );
    assert!(
        (separation(plan("cos(0)*integral(0,1,x)-2*integral(0,1,x^2)"), 0.25) - 0.125).abs()
            < 1e-12
    );
    assert!(separation(plan("integral(-1,1,x)-integral(-1,1,0)"), -0.5) < 0.0);
    assert!(separation(plan("integral(-1,1,x)-integral(-1,1,0)"), 0.5) > 0.0);
}
#[test]
fn different_bounds_and_offsets_keep_separate_contributions() {
    for (source, expected) in [
        ("integral(0,1,x)-integral(0,2,x^2)", 0.0),
        ("3+integral(0,1,x)-integral(0,1,x^2)", 3.0),
        ("integral(0,1,x)+integral(0,1,x^2)", 0.0),
        ("integral(0,1,x)+integral(0,1,x^2)-integral(0,1,2*x)", 0.0),
    ] {
        let IntegralVisualization::Terms { operands, offset } = plan(source) else {
            panic!("{source}");
        };
        assert_eq!(offset, Some(expected));
        assert!(operands.iter().all(|o| o.coefficient.is_some()));
    }
}
#[test]
fn nonlinear_outer_operations_do_not_claim_a_linear_area() {
    for source in [
        "sin(integral(0,1,x))",
        "integral(0,1,x)*integral(0,1,x^2)",
        "integral(0,1,x)^2",
        "1/integral(0,1,x)",
        "abs(integral(0,1,x))",
    ] {
        let IntegralVisualization::Terms { operands, offset } = plan(source) else {
            panic!("{source}");
        };
        assert!(offset.is_none());
        assert!(
            operands
                .iter()
                .all(|o| o.coefficient.is_none() && o.direction().is_none())
        );
    }
}
#[test]
fn zero_weight_and_degenerate_intervals_have_no_signed_area() {
    let IntegralVisualization::Terms { operands, .. } = plan("0*integral(0,1,x)") else {
        panic!()
    };
    assert_eq!(operands[0].direction(), Some(0.0));
    let IntegralVisualization::Terms { operands, .. } =
        IntegralVisualization::single(Expression::parse("x").unwrap(), [2.0, 2.0])
    else {
        panic!()
    };
    assert_eq!(operands[0].direction(), Some(0.0));
}
