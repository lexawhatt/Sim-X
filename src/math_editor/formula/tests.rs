use super::{Atom, Caret, Formula};
use crate::math_editor::formula::layout::FormulaLayout;

fn value(formula: &Formula, x: f64) -> f64 {
    sim_math::Expression::parse(&formula.source().unwrap())
        .unwrap()
        .evaluate(x, 1.0)
        .unwrap()
}

#[test]
fn typed_and_pasted_indexed_roots_have_real_editable_degrees() {
    for (text, expected) in [
        ("root(3,-8)", -2.0),
        ("nthroot(5,-32)", -2.0),
        ("cbrt(-27)", -3.0),
        ("sqrt(16)", 4.0),
        ("root(4,16)", 2.0),
        ("root(-3,-8)", -0.5),
        ("root(1/2,3)", 9.0),
        ("root(1+2,27)", 3.0),
        ("root(3,root(3,512))", 2.0),
        ("root(3,atan2(0,1)+8)", 2.0),
    ] {
        for formula in [Formula::typed(text), Formula::from_text(text).unwrap()] {
            assert!((value(&formula, 0.0) - expected).abs() < 1e-12, "{text}");
            if !text.starts_with("sqrt") {
                assert!(matches!(formula.rows[0][0], Atom::IndexedRoot(..)));
                assert!(formula.source().unwrap().starts_with("root("));
            }
            assert_eq!(formula.caret.row, 0);
        }
    }
}

#[test]
fn indexed_root_template_visits_degree_then_radicand_without_latex() {
    let mut formula = Formula::default();
    formula.template('n');
    let Atom::IndexedRoot(degree, body) = formula.rows[0][0] else {
        panic!()
    };
    assert_eq!(formula.caret.row, degree);
    assert!(formula.source().is_err());
    formula.type_char('3');
    formula.type_char(',');
    assert_eq!(formula.caret.row, body);
    for c in "-8)".chars() {
        formula.type_char(c);
    }
    assert_eq!(formula.source().unwrap(), "root(3,-8)");
    formula.caret = Caret {
        row: degree,
        index: 1,
    };
    formula.erase(false);
    formula.type_char('5');
    assert_eq!(formula.source().unwrap(), "root(5,-8)");
    formula.caret = Caret {
        row: degree,
        index: 1,
    };
    formula.horizontal(true);
    assert_eq!(
        formula.caret,
        Caret {
            row: body,
            index: 0
        }
    );
    formula.horizontal(false);
    assert_eq!(
        formula.caret,
        Caret {
            row: degree,
            index: 1
        }
    );
    formula.anchor = Some(Caret {
        row: degree,
        index: 0,
    });
    formula.next_slot(false);
    assert_eq!(formula.caret.row, body);
    assert!(formula.selection().is_none());
}

#[test]
fn function_palette_wraps_selection_including_indexed_roots() {
    for name in ["sqrt", "cbrt", "sin", "root", "nthroot"] {
        let mut formula = Formula::typed("x+1");
        formula.select_all = true;
        formula.insert_function(name);
        if matches!(name, "root" | "nthroot") {
            formula.type_char('3');
            assert_eq!(formula.source().unwrap(), "root(3,x+1)");
        } else {
            assert_eq!(
                formula.source().unwrap(),
                if name == "cbrt" {
                    "root(3,x+1)".into()
                } else {
                    format!("{name}(x+1)")
                }
            );
        }
        assert!(formula.selection().is_none());
    }
}

#[test]
fn templates_and_operator_keys_wrap_selection_instead_of_erasing_it() {
    for key in ['r', 'n', '^', '/', 'd', 'i'] {
        let mut formula = Formula::typed("x+1");
        formula.select_all = true;
        if matches!(key, '^' | '/') {
            formula.type_char(key);
        } else {
            formula.template(key);
        }
        if matches!(key, '^' | '/' | 'n') {
            formula.type_char('2');
        }
        let source = formula.source().unwrap();
        assert!(source.contains("x+1"), "{key}: {source}");
        match key {
            'r' => assert_eq!(value(&formula, 3.0), 2.0),
            'n' => assert_eq!(value(&formula, 3.0), 2.0),
            '^' => assert_eq!(value(&formula, 3.0), 16.0),
            '/' => assert_eq!(value(&formula, 3.0), 2.0),
            _ => {}
        }
    }
    let mut formula = Formula::typed("x+1");
    formula.select_all = true;
    formula.type_char('(');
    formula.type_char(')');
    formula.type_char('^');
    formula.type_char('2');
    assert_eq!(value(&formula, 3.0), 16.0);
}

#[test]
fn deleting_a_root_wrapper_preserves_nested_body_and_live_parents() {
    for input in ["sqrt(root(3,x+1))", "root(5,sqrt(x+1))"] {
        for forward in [true, false] {
            let mut formula = Formula::from_text(input).unwrap();
            let body = match formula.rows[0][0] {
                Atom::Root(body) | Atom::IndexedRoot(_, body) => body,
                _ => panic!(),
            };
            formula.caret = if forward {
                Caret { row: 0, index: 0 }
            } else {
                Caret {
                    row: body,
                    index: 0,
                }
            };
            formula.erase(forward);
            let source = formula.source().unwrap();
            let nested_body = match formula.rows[0][0] {
                Atom::Root(body) | Atom::IndexedRoot(_, body) => body,
                _ => panic!(),
            };
            assert_eq!(formula.parent(nested_body), Some((0, 0)));
            formula.select_all = true;
            formula.template('n');
            formula.type_char('5');
            formula.caret = Caret { row: 0, index: 0 };
            formula.erase(true);
            assert_eq!(formula.source().unwrap(), source);
            assert_eq!(formula.parent(nested_body), Some((0, 0)));
        }
    }
}

#[test]
fn copy_root_or_degree_remaps_only_reachable_slots() {
    let mut source = Formula::from_text("root(2+1,sqrt(x+3))").unwrap();
    let Atom::IndexedRoot(degree, _) = source.rows[0][0] else {
        panic!()
    };
    source.anchor = Some(Caret {
        row: degree,
        index: 0,
    });
    source.caret = Caret {
        row: degree,
        index: source.rows[degree].len(),
    };
    assert_eq!(source.fragment().source().unwrap(), "2+1");
    source.select_all = true;
    let copied = source.fragment();
    let mut target = Formula::default();
    target.splice_fragment(&copied);
    assert_eq!(value(&source, 61.0), 2.0);
    assert_eq!(target.source().unwrap(), source.source().unwrap());
    let Atom::IndexedRoot(degree, body) = target.rows[0][0] else {
        panic!()
    };
    assert_eq!(target.parent(degree), Some((0, 0)));
    assert_eq!(target.parent(body), Some((0, 0)));
}

#[test]
fn index_layout_and_caret_hit_share_script_geometry() {
    let formula = Formula::typed("root(12,x+1)");
    let Atom::IndexedRoot(degree, body) = formula.rows[0][0] else {
        panic!()
    };
    let layout = FormulaLayout::build(&formula, |style, s| {
        s.chars().count() as f32 * [12.0, 8.0, 6.0][style]
    });
    let index = layout
        .stop(Caret {
            row: degree,
            index: 1,
        })
        .unwrap();
    let radicand = layout
        .stop(Caret {
            row: body,
            index: 0,
        })
        .unwrap();
    assert!(index.y < radicand.y && index.x < radicand.x);
    assert!(index.height < radicand.height);
    assert_eq!(
        layout.hit(index.x, index.y - index.height * 0.35),
        Some(index.caret)
    );
    assert_eq!(
        layout.hit(radicand.x, radicand.y - radicand.height * 0.35),
        Some(radicand.caret)
    );
    assert!(
        layout
            .vertical(radicand.caret, true)
            .is_some_and(|caret| caret.row == degree)
    );
    assert!(
        layout
            .rules
            .iter()
            .all(|r| r.from.into_iter().chain(r.to).all(f32::is_finite))
    );
}

#[test]
fn empty_power_or_fraction_backspace_restores_the_original_operand() {
    for ch in ['/', '^'] {
        let mut formula = Formula::typed("x+1");
        formula.select_all = true;
        formula.type_char(ch);
        assert!(formula.source().is_err());
        formula.erase(false);
        assert_eq!(formula.source().unwrap(), "x+1");
        assert_eq!(formula.caret, Caret { row: 0, index: 0 });
    }
}

#[test]
fn deeply_nested_indexed_roots_stay_iterative_and_editable() {
    let input = format!("{}x{}", "root(3,".repeat(120), ")".repeat(120));
    let mut formula = Formula::typed(&input);
    let layout = FormulaLayout::build(&formula, |_, s| s.len() as f32 * 8.0);
    assert!(layout.width.is_finite() && layout.width > 1000.0);
    assert!(formula.source().unwrap().starts_with("root(3,root(3,"));
    formula.select_all = true;
    let fragment = formula.fragment();
    assert_eq!(fragment.source().unwrap(), formula.source().unwrap());
}

#[test]
fn implicit_products_after_powers_are_visible_without_extra_editable_atoms() {
    let mut formula = Formula::typed("x^4");
    formula.move_horizontal(true, false);
    formula.type_char('7');
    formula.type_char('x');
    assert_eq!(formula.source().unwrap(), "((x)^(4))*7*x");
    assert_eq!(value(&formula, 2.0), 224.0);
    assert_eq!(formula.rows[0].len(), 3);
    let layout = FormulaLayout::build(&formula, |_, s| s.chars().count() as f32 * 12.0);
    let dots: Vec<_> = layout.glyphs.iter().filter(|g| g.text == "·").collect();
    assert_eq!(dots.len(), 1);
    let seven = layout.glyphs.iter().find(|g| g.text == "7").unwrap();
    assert!(dots[0].x < seven.x && dots[0].y == seven.y);
    let boundary = Caret { row: 0, index: 1 };
    let stop = layout.stop(boundary).unwrap();
    assert_eq!(stop.x, seven.x);
    assert_eq!(
        layout.hit(stop.x, stop.y - stop.height * 0.35),
        Some(boundary)
    );
    let expected_stops: usize = formula
        .postorder()
        .into_iter()
        .map(|row| formula.rows[row].len() + 1)
        .sum();
    assert_eq!(layout.stops.len(), expected_stops);
    assert_eq!(
        layout.stop(Caret { row: 0, index: 3 }).unwrap().x,
        layout.width
    );
}

#[test]
fn multiplication_spacing_preserves_literals_coefficients_and_serialized_meaning() {
    for (text, dots, expected) in [
        ("123.45", 0, 123.45),
        ("2x", 0, 4.0),
        ("ax", 0, 2.0),
        ("x7", 1, 14.0),
        ("2sqrt(9)", 1, 6.0),
        ("sqrt(9)root(3,8)", 1, 6.0),
        ("(x+1)(x-1)", 1, 3.0),
        ("root(3,8)7x", 1, 28.0),
        ("root(3,sqrt(9)root(3,8))", 1, 6.0_f64.cbrt()),
    ] {
        let formula = Formula::typed(text);
        let layout = FormulaLayout::build(&formula, |_, s| s.chars().count() as f32 * 12.0);
        assert_eq!(
            layout.glyphs.iter().filter(|g| g.text == "·").count(),
            dots,
            "{text}"
        );
        assert!((value(&formula, 2.0) - expected).abs() < 1e-12, "{text}");
        assert_eq!(
            layout
                .stop(Caret {
                    row: 0,
                    index: formula.rows[0].len()
                })
                .unwrap()
                .x,
            layout.width
        );
        let pasted = Formula::from_text(&formula.source().unwrap()).unwrap();
        assert!((value(&pasted, 2.0) - expected).abs() < 1e-12, "{text}");
    }
}

#[test]
fn multi_digit_exponents_need_explicit_right_or_space_to_exit() {
    // Automatic exit after a digit would silently turn x^12 into x^1 * 2.
    let mut formula = Formula::typed("x^12");
    let Atom::Power(_, exponent) = formula.rows[0][0] else {
        panic!()
    };
    assert_eq!(formula.caret.row, exponent);
    assert_eq!(formula.source().unwrap(), "((x)^(12))");
    assert_eq!(value(&formula, 2.0), 4096.0);
    formula.type_char(' ');
    formula.type_char('3');
    assert_eq!(formula.source().unwrap(), "((x)^(12))*3");
    for suffix in ["+1", "-1"] {
        let arithmetic = Formula::typed(&format!("x^12{suffix}"));
        assert_eq!(arithmetic.caret.row, 0);
        assert_eq!(arithmetic.source().unwrap(), format!("((x)^(12)){suffix}"));
    }
    let negative = Formula::typed("x^-12");
    assert_eq!(negative.source().unwrap(), "((x)^(-12))");
}

#[test]
fn relations_type_paste_and_copy_as_canonical_ascii_without_implicit_products() {
    for (text, source, sign) in [
        ("x<1", "x<1", '<'),
        ("x>1", "x>1", '>'),
        ("x<=1", "x<=1", '≤'),
        ("x>=1", "x>=1", '≥'),
        ("x≤1", "x<=1", '≤'),
        ("x≥1", "x>=1", '≥'),
    ] {
        for mut formula in [Formula::typed(text), Formula::from_text(text).unwrap()] {
            assert_eq!(formula.source().unwrap(), source, "{text}");
            assert_eq!(
                formula.rows[0],
                vec![Atom::Symbol('x'), Atom::Symbol(sign), Atom::Symbol('1')]
            );
            let layout = FormulaLayout::build(&formula, |_, s| s.chars().count() as f32 * 12.0);
            assert!(layout.glyphs.iter().any(|g| g.text == sign.to_string()));
            assert!(!layout.glyphs.iter().any(|g| g.text == "·"));
            let caret = Caret { row: 0, index: 2 };
            let stop = layout.stop(caret).unwrap();
            assert_eq!(layout.hit(stop.x, stop.y - stop.height * 0.35), Some(caret));
            formula.select_all = true;
            assert_eq!(formula.fragment().source().unwrap(), source);
            formula.select_all = false;
            formula.caret = caret;
            formula.erase(false);
            assert_eq!(formula.source().unwrap(), "x*1");
            formula.type_char(sign);
            assert_eq!(formula.source().unwrap(), source);
            formula.caret = Caret { row: 0, index: 1 };
            formula.erase(true);
            assert_eq!(formula.source().unwrap(), "x*1");
        }
    }
    assert_eq!(Formula::typed("x^2<=1").source().unwrap(), "((x)^(2))<=1");
    for invalid in ["x<", "<=1", "x<y<z", "x==1", "sin(x<1)", "x=>1"] {
        assert!(Formula::from_text(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn absolute_value_relations_keep_all_function_arguments_and_nested_bars() {
    let expected = "max(abs(x),abs(y),abs(z))<=1";
    assert_eq!(
        Formula::typed("max(|x|,|y|,|z|)<=1").source().unwrap(),
        expected
    );
    assert_eq!(
        Formula::from_text("max(|x|, |y|, |z|) ≤ 1")
            .unwrap()
            .source()
            .unwrap(),
        expected
    );
    assert_eq!(
        Formula::from_text(expected).unwrap().source().unwrap(),
        expected
    );
    assert_eq!(
        Formula::typed("|x^2|>=1").source().unwrap(),
        "abs(((x)^(2)))>=1"
    );
    assert_eq!(
        Formula::typed("||x||>=1").source().unwrap(),
        "abs(abs(x))>=1"
    );
    for input in ["|x|", "|x+|x||", "2|x|", "|x||x|", "|.5|", "|-.5|"] {
        let formula = Formula::from_text(input).unwrap_or_else(|error| panic!("{input}: {error}"));
        assert!(
            sim_math::Expression::parse(&formula.source().unwrap()).is_ok(),
            "{input}"
        );
    }
    assert_eq!(Formula::typed("|-.5|<=1").source().unwrap(), "abs(-0.5)<=1");
    assert!(Formula::from_text("max(|x|,|y)").is_err());
    assert_eq!(
        Formula::typed("x^2+y^2+z^2=9").source().unwrap(),
        "((x)^(2))+((y)^(2))+((z)^(2))=9"
    );
}
