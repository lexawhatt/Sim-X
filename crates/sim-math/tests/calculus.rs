use sim_math::{calculus::IntegralExpression, statement::Statement};
use std::collections::BTreeMap;

fn calculate(source: &str, variables: &BTreeMap<String, f64>, batch: usize) -> f64 {
    let mut job = IntegralExpression::parse(source, variables)
        .unwrap()
        .unwrap()
        .into_job(1e-10)
        .unwrap();
    assert!(job.advance(0).unwrap().is_none());
    loop {
        if let Some(value) = job.advance(batch).unwrap() {
            assert_eq!(job.advance(1).unwrap(), Some(value));
            return value;
        }
    }
}

#[test]
fn difference_of_integrals_respects_arithmetic_and_scheduling() {
    let cases = [
        ("integral(0,1,x^2)-integral(0,1,x)", -1.0 / 6.0),
        ("2*integral(0,1,x^2)-integral(1,0,x)/2", 11.0 / 12.0),
        ("cos(integral(0,1,x))*integral(0,2,1)", 0.5_f64.cos() * 2.0),
        ("(integral(0,1,x))^2", 0.25),
    ];
    for (source, expected) in cases {
        let value = calculate(source, &BTreeMap::new(), 1);
        assert!((value - expected).abs() < 1e-10, "{source}: {value}");
        assert_eq!(value, calculate(source, &BTreeMap::new(), 100));
        assert!(matches!(
            Statement::parse(&format!("y={source}"), &BTreeMap::new()).unwrap(),
            Statement::IntegralExpression {
                graph_result: true,
                ..
            }
        ));
    }
}

#[test]
fn composed_integrals_bind_parameters_without_exposing_internal_slots() {
    for a in [-3.0, 2.0] {
        let variables = BTreeMap::from([
            ("a".into(), a),
            ("b".into(), 2.0),
            ("__quadrature_0".into(), 7.0),
        ]);
        let value = calculate("integral (0,b,a*x)+__quadrature_0", &variables, 3);
        assert!((value - (2.0 * a + 7.0)).abs() < 1e-10);
    }
    let source = std::iter::repeat_n("integral(0,1,x)", 150)
        .collect::<Vec<_>>()
        .join("+");
    assert!((calculate(&source, &BTreeMap::new(), 7) - 75.0).abs() < 1e-9);
}

#[test]
fn invalid_or_improper_composition_never_returns_a_partial_scalar() {
    for source in [
        "integral(0,1,x)-integral(0,1,1/x)",
        "1/(integral(0,1,x)-integral(0,1,x))",
    ] {
        let mut job = IntegralExpression::parse(source, &BTreeMap::new())
            .unwrap()
            .unwrap()
            .into_job(1e-9)
            .unwrap();
        loop {
            match job.advance(1) {
                Ok(None) => {}
                Ok(Some(v)) => panic!("unexpected {v}"),
                Err(e) => {
                    assert_eq!(job.advance(50), Err(e));
                    break;
                }
            }
        }
    }
    for source in [
        "integral(0,1,integral(0,1,x))",
        "integral(0,1,x)+x",
        "integral(0,1,y)",
        "integral(0,x,x)",
        "integral(0,1,x)-integral(0,1,x",
        "integral(0,1,x),integral(0,1,x)",
    ] {
        assert!(
            IntegralExpression::parse(source, &BTreeMap::new()).is_err(),
            "{source}"
        );
    }
    assert!(
        IntegralExpression::parse("cos(2)", &BTreeMap::new())
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        Statement::parse("integral(0,1,x)", &BTreeMap::new()).unwrap(),
        Statement::Integral { .. }
    ));
}
