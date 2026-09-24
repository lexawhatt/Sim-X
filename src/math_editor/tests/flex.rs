use super::*;
use crate::math_editor::{
    formula::{Atom, Caret, Formula},
    interaction::clipboard::ClipboardAction,
    state,
    view::layout::{self, Target},
};

fn value(f: &Formula, x: f64) -> f64 {
    sim_math::Expression::parse(&f.source().unwrap())
        .unwrap()
        .evaluate(x, 1.0)
        .unwrap()
}
#[test]
fn deleting_integral_preserves_nested_integrand_and_can_rewrap() {
    for forward in [false, true] {
        let mut f = Formula::from_text("integral(1,2,1/(x*sqrt(x^4+2*x^2-1)))").unwrap();
        let Atom::Integral(_, _, body) = f.rows[0][0] else {
            panic!()
        };
        f.caret = if forward {
            Caret { row: 0, index: 0 }
        } else {
            Caret {
                row: body,
                index: 0,
            }
        };
        f.erase(forward);
        assert!(!f.is_integral());
        assert!((value(&f, 1.0) - 1.0 / 2.0_f64.sqrt()).abs() < 1e-12);
        f.caret = Caret { row: 0, index: 0 };
        f.template('i');
        let source = f.source().unwrap();
        assert!(source.starts_with("integral(0,1,"));
        let Atom::Integral(_, _, body) = f.rows[0][0] else {
            panic!()
        };
        assert_eq!(f.parent(body), Some((0, 0)));
        // Repeated unwrap/wrap does not leave stale parent references.
        f.erase(false);
        f.template('i');
        assert_eq!(f.source().unwrap(), source);
    }
}
#[test]
fn paste_parser_preserves_precedence_and_internal_roundtrips() {
    let precise = Formula::from_text("9007199254740993").unwrap();
    assert_eq!(precise.source().unwrap(), "9007199254740993");
    assert!(Formula::from_text("x\ny").is_err());
    for (text, expected) in [
        ("1/2+3", 3.5),
        ("1/(2+3)", 0.2),
        ("2x+sin(pi/2)", 7.0),
        ("(x+1)^2", 16.0),
        ("-x^2", -9.0),
        ("(-x)^2", 9.0),
        ("1e-3+x", 3.001),
        ("x-(1+2)", 0.0),
        ("2×x−1", 5.0),
        ("atan2(0,1)", 0.0),
    ] {
        let f = Formula::from_text(text).unwrap();
        assert!(
            (value(&f, 3.0) - expected).abs() < 1e-12,
            "{text}: {:?}",
            f.source()
        );
        let pasted = Formula::from_text(&f.source().unwrap()).unwrap();
        assert!(
            (value(&pasted, 3.0) - expected).abs() < 1e-12,
            "roundtrip {text}"
        );
    }
    for invalid in ["\\frac{1}{2}", "x🙂", "sin^2(x)", "sqrt(", "1e999"] {
        assert!(Formula::from_text(invalid).is_err(), "{invalid}");
    }
}
#[test]
fn selection_can_copy_a_slot_and_replace_without_losing_structure() {
    let mut f = Formula::from_text("1/(x+2)").unwrap();
    let Atom::Fraction(_, den) = f.rows[0][0] else {
        panic!()
    };
    f.caret = Caret { row: den, index: 0 };
    for _ in 0..3 {
        f.move_horizontal(true, true);
    }
    assert_eq!(f.fragment().source().unwrap(), "x+2");
    f.splice_fragment(&Formula::from_text("x^2").unwrap());
    assert!((value(&f, 2.0) - 0.25).abs() < 1e-12);
    f.select_all = true;
    f.template('i');
    assert!(f.is_integral());
    assert!(f.source().unwrap().contains("integral(0,1,"));
}
#[test]
fn clipboard_failure_and_stale_paste_are_atomic_and_undo_is_one_action() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::from_text("x+1").unwrap();
    s.changed();
    let original = s.document.clone();
    s.request_clipboard(ClipboardAction::Cut);
    s.complete_clipboard(Err("Unavailable".into()));
    assert!(s.document == original);
    s.request_clipboard(ClipboardAction::Paste);
    s.document.fields[0].type_char('2');
    let edited = s.document.clone();
    s.complete_clipboard(Ok("7".into()));
    assert!(s.document == edited);
    s.document.fields[0].select_all = true;
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok("sqrt(9)".into()));
    assert_eq!(value(&s.document.fields[0], 0.0), 3.0);
    assert_eq!(s.undo.len(), 1);
    s.history(false);
    assert_eq!(
        s.document.fields[0].source().unwrap(),
        edited.fields[0].source().unwrap()
    );
    s.request_clipboard(ClipboardAction::Paste);
    let before = s.document.clone();
    s.complete_clipboard(Ok("\\not_a_formula".into()));
    assert!(s.document == before);
}
#[test]
fn internal_clipboard_preserves_structure_and_cut_waits_for_success() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::from_text("integral(1,2,x^2)").unwrap();
    s.request_clipboard(ClipboardAction::Copy);
    let text = s.clipboard_pending.as_ref().unwrap().text.clone();
    s.complete_clipboard(Ok(String::new()));
    let source = s.document.fields[0].source().unwrap();
    s.request_clipboard(ClipboardAction::Cut);
    assert_eq!(s.document.fields[0].source().unwrap(), source);
    s.complete_clipboard(Ok(String::new()));
    assert!(s.document.fields[0].rows[0].is_empty());
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok(text));
    assert_eq!(s.document.fields[0].source().unwrap(), source);
}
#[test]
fn alphabet_keys_have_no_overlaps_and_catalog_anchor_follows_page() {
    let mut s = state::MathState::new().unwrap();
    s.keypad = true;
    for (width, height) in [(900.0, 640.0), (1280.0, 800.0), (1920.0, 1080.0)] {
        s.alphabet = true;
        s.functions = false;
        let viewport = LogicalViewport::new(width, height).unwrap();
        let l = layout::Layout::for_state(viewport, &s);
        let keys: Vec<_> = l
            .controls
            .iter()
            .filter(|(_, r, _)| r.y >= l.keypad_top)
            .collect();
        for (i, (t, r, _)) in keys.iter().enumerate() {
            assert!(r.x >= 0.0 && r.x + r.w <= width);
            assert_eq!(l.hit(r.center(), false), Some(*t));
            for (_, other, _) in keys.iter().skip(i + 1) {
                assert!(r.intersection(*other).is_none(), "overlapping ABC keys");
            }
        }
        assert_eq!(
            keys.iter()
                .filter(|(t, _, _)| matches!(t,Target::Type(c) if c.is_ascii_alphabetic()))
                .count(),
            26
        );
        s.functions = true;
        let l = layout::Layout::for_state(viewport, &s);
        let anchor = l
            .controls
            .iter()
            .find(|(t, _, _)| *t == Target::Functions)
            .unwrap()
            .1;
        let popup = l.popup.unwrap();
        assert!(popup.y + popup.h < anchor.y);
    }
}
#[test]
fn sliders_edit_definitions_offer_unknowns_and_keep_ranges_user_controlled() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::from_text("y=g*x").unwrap();
    s.changed();
    assert_eq!(s.missing_parameters[0], vec!['g']);
    s.add_parameter('g');
    assert!(s.missing_parameters[0].is_empty());
    s.range_edit = Some((1, false, "-200".into()));
    s.commit_range();
    s.range_edit = Some((1, true, "200".into()));
    s.commit_range();
    let track = layout::Rect::new(0.0, 0.0, 100.0, 20.0);
    s.move_slider(1, 75.0, track);
    assert_eq!(s.parameters[1].unwrap().value, 100.0);
    let source = s.document.fields[1].source().unwrap();
    assert_eq!(source, "g=100");
    s.range_edit = Some((1, true, "-999".into()));
    s.commit_range();
    assert_eq!(s.parameters[1].unwrap().range.high, 200.0);
    assert_eq!(s.document.fields[1].source().unwrap(), source);
    s.document.fields[0] = Formula::from_text("integral(0,1,k*x)").unwrap();
    s.changed();
    let layout = layout::Layout::for_state(viewport(), &s);
    let calculate = layout
        .controls
        .iter()
        .find(|(t, _, _)| *t == Target::Calculate(0))
        .unwrap()
        .1;
    let add = layout
        .controls
        .iter()
        .find(|(t, _, _)| *t == Target::CreateParameter('k'))
        .unwrap()
        .1;
    assert!(calculate.intersection(add).is_none());
}
#[test]
fn playback_waits_for_compute_and_is_one_undoable_edit() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::from_text("g=0").unwrap();
    s.changed();
    let before = s.document.fields[0].source().unwrap();
    s.toggle_parameter(0);
    s.advance_parameters(0.1);
    assert_eq!(s.document.fields[0].source().unwrap(), before);
    s.dirty = false;
    s.busy = false;
    s.advance_parameters(0.1);
    assert!(s.parameters[0].unwrap().value > 0.0);
    s.stop_parameter_motion();
    assert_eq!(s.undo.len(), 1);
    s.history(false);
    assert_eq!(s.document.fields[0].source().unwrap(), before);
}
#[test]
fn reduced_motion_skips_ui_easing_without_altering_math() {
    let mut s = state::MathState::new().unwrap();
    s.functions = true;
    s.hovered = Some(Target::Functions);
    s.animate_ui(0.01);
    assert!(s.motion.popup > 0.0 && s.motion.popup < 1.0);
    s.reduced_motion = true;
    s.animate_ui(0.01);
    assert_eq!(s.motion.popup, 1.0);
    assert_eq!(s.motion.hover(Target::Functions), 1.0);
}

#[test]
fn native_input_adds_and_drags_a_parameter_without_touching_geometry() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "y=gx")?;
    click(&mut runner, Target::CreateParameter('g'))?;
    frame(&mut runner, &[])?;
    let (track, undo_count) = {
        let s = runner.resource::<state::MathState>().unwrap();
        let l = layout::Layout::for_state(viewport(), s);
        assert_eq!(s.parameters[1].unwrap().name, 'g');
        (
            l.controls
                .iter()
                .find(|(t, _, _)| *t == Target::Slider(1))
                .unwrap()
                .1,
            s.undo.len(),
        )
    };
    let from = [track.x + track.w * 0.25, track.y + track.h * 0.5];
    let to = [track.x + track.w * 0.8, from[1]];
    frame(
        &mut runner,
        &[
            pointer(from),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    frame(&mut runner, &[pointer(to)])?;
    frame(
        &mut runner,
        &[InputEvent::mouse_button(
            MouseButton::Left,
            ButtonState::Released,
        )],
    )?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!((s.parameters[1].unwrap().value - 6.0).abs() < 1e-4);
    assert_eq!(s.undo.len(), undo_count + 1);
    assert!(s.document.geometry.points().is_empty());
    click(&mut runner, Target::Undo)?;
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().parameters[1]
            .unwrap()
            .value,
        1.0
    );
    click(&mut runner, Target::ParameterRange(1, true))?;
    select_all(&mut runner)?;
    type_text(&mut runner, "200")?;
    frame(&mut runner, &tap(PhysicalKeyCode::Enter))?;
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().parameters[1]
            .unwrap()
            .range
            .high,
        200.0
    );
    Ok(())
}

#[test]
fn physical_selection_copy_and_alphabet_share_the_active_formula() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x+12")?;
    frame(
        &mut runner,
        &[
            key(ShiftLeft, ButtonState::Pressed),
            key(ArrowLeft, ButtonState::Pressed),
            key(ArrowLeft, ButtonState::Released),
            key(ArrowLeft, ButtonState::Pressed),
            key(ArrowLeft, ButtonState::Released),
            key(ShiftLeft, ButtonState::Released),
        ],
    )?;
    frame(
        &mut runner,
        &[
            key(ControlLeft, ButtonState::Pressed),
            key(KeyC, ButtonState::Pressed),
            key(KeyC, ButtonState::Released),
            key(ControlLeft, ButtonState::Released),
        ],
    )?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.clipboard_pending.as_ref().unwrap().text, "12");
    // Headless input emits an intent; it never reads or overwrites OS clipboard.
    frame(&mut runner, &[InputEvent::FocusLost])?;
    frame(&mut runner, &tap(Backspace))?;
    click(&mut runner, Target::Alphabet)?;
    click(&mut runner, Target::Type('g'))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields[0].source().unwrap(), "x+g");
    assert!(s.alphabet);
    assert!(s.clipboard_pending.is_none());
    Ok(())
}

#[test]
fn mouse_drag_selects_formula_text_instead_of_moving_graph() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x+12")?;
    let (from, to) = {
        let s = runner.resource::<state::MathState>().unwrap();
        let l = layout::Layout::for_state(viewport(), s);
        let point = |index| {
            let stop = s.layouts[0].stop(Caret { row: 0, index }).unwrap();
            [
                l.fields[0].x + 12.0 - s.offsets[0][0] + stop.x,
                l.fields[0].y + 8.0 - s.offsets[0][1] + stop.y - stop.height * 0.3,
            ]
        };
        (point(2), point(4))
    };
    frame(
        &mut runner,
        &[
            pointer(from),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    frame(&mut runner, &[pointer(to)])?;
    frame(
        &mut runner,
        &[InputEvent::mouse_button(
            MouseButton::Left,
            ButtonState::Released,
        )],
    )?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields[0].fragment().source().unwrap(), "12");
    assert!(s.document.geometry.points().is_empty());
    type_text(&mut runner, "9")?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .unwrap(),
        "x+9"
    );
    Ok(())
}

#[test]
fn paste_does_not_apply_after_edit_and_undo_to_same_formula() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::typed("x");
    s.changed();
    s.request_clipboard(ClipboardAction::Paste);
    let before = s.document.clone();
    s.document.fields[0].type_char('2');
    s.remember(before);
    s.history(false);
    s.complete_clipboard(Ok("7".into()));
    assert_eq!(s.document.fields[0].source().unwrap(), "x");
    s.document.fields[0].select_all = true;
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok(String::new()));
    assert_eq!(s.document.fields[0].source().unwrap(), "x");
}

#[test]
fn multiline_paste_is_one_transaction_with_crlf_and_forward_dependencies() {
    let mut s = state::MathState::new().unwrap();
    let before = s.document.clone();
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok(" \r\n y=a*x \r\n\r\n a=2\r\n".into()));
    assert_eq!(s.document.fields.len(), 2);
    assert_eq!(s.document.fields[0].source().unwrap(), "y=a*x");
    assert_eq!(s.document.fields[1].source().unwrap(), "a=2");
    assert_eq!(s.focus, Some(1));
    assert!(s.missing_parameters.iter().all(Vec::is_empty));
    assert_eq!(s.parameters[1].unwrap().value, 2.0);
    assert_eq!(s.undo.len(), 1);
    assert_eq!(s.calculate, vec![false; 2]);
    let pasted = s.document.clone();
    s.history(false);
    assert!(s.document == before);
    s.history(true);
    assert!(s.document == pasted);

    s.submit(&layout::Layout::for_state(viewport(), &s));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while s.busy {
        assert!(std::time::Instant::now() < deadline);
        s.poll();
        std::thread::yield_now();
    }
    let plot = s.plot.as_ref().unwrap();
    assert!(plot.rows.iter().all(|r| r.diagnostic.is_none()));
    assert_eq!(
        plot.rows[0]
            .expression
            .as_ref()
            .unwrap()
            .evaluate_at(3.0, 0.0)
            .unwrap(),
        6.0
    );
}

#[test]
fn batch_paste_preserves_neighbor_styles_visibility_and_calculate_ownership() {
    for replace in [false, true] {
        let mut s = state::MathState::new().unwrap();
        s.document.fields[0] = Formula::typed("x+1");
        s.insert_expression(1);
        s.document.fields[1] = Formula::typed("int(x)");
        s.document.visible[1] = false;
        s.document.styles[1].width = 7.0;
        s.calculate[1] = true;
        s.changed();
        s.focus_expression(0, true);
        s.document.fields[0].select_all = replace;
        let before = s.document.clone();
        let history = s.undo.len();
        s.request_clipboard(ClipboardAction::Paste);
        s.complete_clipboard(Ok("root(3,x)\nx^2+y^2+z^2=9".into()));
        let count = if replace { 3 } else { 4 };
        let last = count - 1;
        assert_eq!(s.document.fields.len(), count);
        assert_eq!(s.document.fields[last], before.fields[1]);
        assert!(!s.document.visible[last]);
        assert_eq!(s.document.styles[last].width, 7.0);
        assert!(s.calculate[last]);
        assert!(s.calculate[..last].iter().all(|v| !v));
        assert_eq!(s.focus, Some(last - 1));
        assert_eq!(s.undo.len(), history + 1);
        assert_eq!(s.layouts.len(), count);
        assert_eq!(s.offsets.len(), count);
        assert_eq!(s.parameters.len(), count);
        if !replace {
            assert_eq!(s.document.fields[0], before.fields[0]);
        }
        s.history(false);
        assert!(s.document == before);
    }
}

#[test]
fn batch_paste_does_not_split_or_overwrite_a_partially_selected_fraction() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::typed("1/23");
    s.document.fields[0].move_horizontal(false, true);
    let before = s.document.fields[0].clone();
    assert!(before.selection().is_some());
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok("y=x\nz=x*y".into()));
    assert_eq!(s.document.fields[0], before);
    assert_eq!(s.document.fields[1].source().unwrap(), "y=x");
    assert_eq!(s.document.fields[2].source().unwrap(), "z=x*y");
    assert_eq!(s.focus, Some(2));
}

#[test]
fn batch_paste_replaces_a_whole_row_selected_by_arrows() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::typed("x+1");
    s.document.fields[0].anchor = Some(Caret { row: 0, index: 0 });
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok("a=3\ny=a*x".into()));
    assert_eq!(s.document.fields.len(), 2);
    assert_eq!(s.document.fields[0].source().unwrap(), "a=3");
}

#[test]
fn invalid_batch_reports_the_original_line_without_partial_edits() {
    for text in ["x\n\nsqrt(\ny", "a=2\n\n\\frac{1}{2}\ny=a*x"] {
        let mut s = state::MathState::new().unwrap();
        s.document.fields[0] = Formula::typed("int(x)");
        s.calculate[0] = true;
        let before = s.document.clone();
        let epoch = s.edit_epoch;
        s.request_clipboard(ClipboardAction::Paste);
        s.complete_clipboard(Ok(text.into()));
        assert!(s.document == before);
        assert_eq!(s.edit_epoch, epoch);
        assert!(s.undo.is_empty());
        assert_eq!(s.calculate, vec![true]);
        assert!(s.notice.as_ref().unwrap().starts_with("Paste: Line 3:"));
    }
}

#[test]
fn trailing_newline_still_pastes_one_fragment_at_the_caret() {
    let mut s = state::MathState::new().unwrap();
    s.document.fields[0] = Formula::typed("1/");
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok("\n 2 \r\n\r\n".into()));
    assert_eq!(s.document.fields.len(), 1);
    assert_eq!(value(&s.document.fields[0], 0.0), 0.5);
    s.request_clipboard(ClipboardAction::Paste);
    let before = s.document.clone();
    s.complete_clipboard(Ok("\r\n \t\n".into()));
    assert!(s.document == before);
}

#[test]
fn stale_batch_cannot_insert_rows_after_focus_change_or_navigation() {
    for modal in [false, true] {
        let mut s = state::MathState::new().unwrap();
        s.request_clipboard(ClipboardAction::Paste);
        let before = s.document.clone();
        if modal {
            s.confirm_back = true;
        } else {
            s.focus = None;
        }
        s.complete_clipboard(Ok("x\ny=x^2".into()));
        assert!(s.document == before);
        assert!(s.undo.is_empty());
    }
}

#[test]
fn batch_paste_has_no_expression_count_quota() -> LogicResult {
    let mut s = state::MathState::new().unwrap();
    let text = (0..512)
        .map(|i| format!("y=x+{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    s.request_clipboard(ClipboardAction::Paste);
    s.complete_clipboard(Ok(text));
    assert_eq!(s.document.fields.len(), 512);
    assert_eq!(s.focus, Some(511));
    assert_eq!(s.undo.len(), 1);
    let rendered = layout::Layout::for_state(viewport(), &s);
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    let fonts = runner
        .resource::<crate::math_editor::view::assets::Fonts>()
        .unwrap();
    crate::math_editor::interaction::fields(&mut s, fonts, &rendered);
    assert_eq!(s.layouts.len(), 512);
    assert!(s.layouts[511].width > 0.0);
    Ok(())
}
