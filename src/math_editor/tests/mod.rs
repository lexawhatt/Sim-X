use crate::math_editor::{
    app,
    formula::{self, layout as formula_layout},
    scene::style,
    state,
    view::layout,
};
use sim_logic::prelude::*;

mod axis;
mod editing;
mod flex;
mod implicit3d;
mod integral;
mod row_flow;
mod row_interpretation;
mod spatial;
mod transition_controls;
mod wireframe;

#[test]
fn direct_math_does_not_retain_unused_fixed_input() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    // Far more than the 128 retained-edge limit, but only two edges per frame.
    // Math uses frame input; it must not accumulate an unused fixed copy.
    for _ in 0..200 {
        frame(&mut runner, &tap(PhysicalKeyCode::ArrowLeft))?;
    }
    Ok(())
}

#[test]
fn repeated_thickness_decrease_stays_renderable() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "x")?;
    until(&mut runner, |s| !s.busy)?;
    let s = runner.resource::<state::MathState>().unwrap();
    let l = layout::Layout::for_state(viewport(), s);
    let icon = l
        .controls
        .iter()
        .find(|(t, _, _)| *t == layout::Target::Visible(0))
        .unwrap()
        .1;
    frame(
        &mut runner,
        &[
            pointer(icon.center()),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    for _ in 0..32 {
        frame(&mut runner, &[])?;
    }
    frame(
        &mut runner,
        &[InputEvent::mouse_button(
            MouseButton::Left,
            ButtonState::Released,
        )],
    )?;
    for _ in 0..100 {
        click(&mut runner, layout::Target::Thickness(false))?;
        let width = runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .styles[0]
            .width;
        assert!(width.is_finite() && width >= 0.5);
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .styles[0]
            .width,
        0.5
    );
    Ok(())
}

#[test]
fn natural_notation_is_scientific_not_latex() {
    for (text, x, expected) in [
        ("2x+1", 3.0, 7.0),
        ("x^2+1", 3.0, 10.0),
        ("sin(x)^2+cos(x)^2", 0.8, 1.0),
        ("1/2", 0.0, 0.5),
        ("sqrt(9)", 0.0, 3.0),
        ("(x+1)^2", 3.0, 16.0),
        ("2tg(x)", 0.0, 0.0),
        ("sin^2(x)+cos^2(x)", 0.8, 1.0),
        ("x^-2", 2.0, 0.25),
        ("ax^2", 3.0, 9.0),
        ("2pi", 0.0, std::f64::consts::TAU),
        ("pi^2", 0.0, std::f64::consts::PI.powi(2)),
    ] {
        let f = formula::Formula::typed(text);
        let source = f.source().unwrap();
        let expression = sim_math::Expression::parse(&source).unwrap();
        assert!(
            (expression.evaluate(x, 1.0).unwrap() - expected).abs() < 1e-12,
            "{text} -> {source}"
        );
    }
}
#[test]
fn incomplete_slots_and_caret_follow_the_structure() {
    let mut formula = formula::Formula::typed("1/");
    assert!(formula.source().is_err());
    formula.type_char('2');
    let layout = formula_layout::FormulaLayout::build(&formula, |_, s| s.len() as f32 * 12.0);
    let stop = layout.stop(formula.caret).unwrap();
    assert!(stop.y > 0.0);
    assert_eq!(
        layout.hit(stop.x, stop.y - stop.height * 0.35),
        Some(formula.caret)
    );
    formula.horizontal(true);
    assert_eq!(formula.caret.row, 0);
    formula.type_char('+');
    formula.type_char('3');
    assert_eq!(
        sim_math::Expression::parse(&formula.source().unwrap())
            .unwrap()
            .evaluate(0.0, 0.0)
            .unwrap(),
        3.5
    );
}
#[test]
fn native_headless_math_world_renders() -> LogicResult {
    let (app, initial) = app::build_math_editor_application()?;
    let mut runner = app.build_headless(initial)?;
    let viewport = LogicalViewport::new(1280.0, 800.0)?;
    for _ in 0..3 {
        match runner.advance_frame(FrameRequest::new(
            std::time::Duration::from_millis(16),
            &[],
            viewport,
        )) {
            FrameOutcome::Advanced(report) => {
                assert!(report.failure().is_none(), "{:?}", report.failure())
            }
            FrameOutcome::Rejected(error) => return Err(error.into()),
        }
    }
    assert!(runner.resource::<state::MathState>().is_some());
    Ok(())
}

fn runner() -> LogicResult<HeadlessRunner<crate::actions::AppAction>> {
    let (app, world) = app::build_math_editor_application()?;
    let mut runner = app.build_headless(world)?;
    frame(&mut runner, &[])?;
    // Keyboard interaction tests explicitly open the optional desktop dock.
    click(&mut runner, layout::Target::Keypad)?;
    Ok(runner)
}
fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}
fn frame(
    runner: &mut HeadlessRunner<crate::actions::AppAction>,
    events: &[InputEvent],
) -> LogicResult {
    match runner.advance_frame(FrameRequest::new(
        std::time::Duration::from_millis(16),
        events,
        viewport(),
    )) {
        FrameOutcome::Advanced(report) => {
            assert!(report.failure().is_none(), "{:?}", report.failure());
            Ok(())
        }
        FrameOutcome::Rejected(error) => Err(error.into()),
    }
}
fn key(key: PhysicalKeyCode, state: ButtonState) -> InputEvent {
    InputEvent::key(key, state)
}
fn tap(code: PhysicalKeyCode) -> [InputEvent; 2] {
    [
        key(code, ButtonState::Pressed),
        key(code, ButtonState::Released),
    ]
}
fn type_text(runner: &mut HeadlessRunner<crate::actions::AppAction>, text: &str) -> LogicResult {
    use PhysicalKeyCode::*;
    for c in text.chars() {
        let (code, shifted) = match c {
            'a'..='z' => (
                [
                    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM,
                    KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
                ][c as usize - 'a' as usize],
                false,
            ),
            '0'..='9' => (
                [
                    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
                ][c as usize - '0' as usize],
                false,
            ),
            '(' => (Digit9, true),
            ')' => (Digit0, true),
            '^' => (Digit6, true),
            '*' => (Digit8, true),
            '+' => (Equal, true),
            '=' => (Equal, false),
            '-' => (Minus, false),
            '/' => (Slash, false),
            '.' => (Period, false),
            ',' => (Comma, false),
            _ => panic!("test input {c}"),
        };
        if shifted {
            frame(runner, &[key(ShiftLeft, ButtonState::Pressed)])?;
        }
        frame(runner, &tap(code))?;
        if shifted {
            frame(runner, &[key(ShiftLeft, ButtonState::Released)])?;
        }
    }
    Ok(())
}
fn select_all(runner: &mut HeadlessRunner<crate::actions::AppAction>) -> LogicResult {
    use PhysicalKeyCode::*;
    frame(
        runner,
        &[
            key(ControlLeft, ButtonState::Pressed),
            key(KeyA, ButtonState::Pressed),
            key(KeyA, ButtonState::Released),
            key(ControlLeft, ButtonState::Released),
        ],
    )
}
fn pointer(p: [f32; 2]) -> InputEvent {
    InputEvent::pointer_moved(
        PointerSample::new(LogicalScreenPosition::new(p[0], p[1]), viewport()).unwrap(),
    )
}
fn click(
    runner: &mut HeadlessRunner<crate::actions::AppAction>,
    target: layout::Target,
) -> LogicResult {
    let s = runner.resource::<state::MathState>().unwrap();
    let l = layout::Layout::for_state(viewport(), s);
    let rect = match target {
        layout::Target::Field(i) => l.fields[i],
        _ => {
            l.popup_controls
                .iter()
                .chain(&l.controls)
                .find(|(t, _, _)| *t == target)
                .unwrap()
                .1
        }
    };
    frame(
        runner,
        &[
            pointer(rect.center()),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )
}
fn until(
    runner: &mut HeadlessRunner<crate::actions::AppAction>,
    predicate: impl Fn(&state::MathState) -> bool,
) -> LogicResult {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !predicate(runner.resource::<state::MathState>().unwrap()) {
        assert!(
            std::time::Instant::now() < deadline,
            "worker did not finish: {}",
            runner.resource::<state::MathState>().unwrap().status
        );
        frame(runner, &[])?;
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    Ok(())
}

#[test]
fn native_physical_typing_fraction_undo_and_click_caret() -> LogicResult {
    use PhysicalKeyCode::*;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    frame(
        &mut runner,
        &[
            key(ControlLeft, ButtonState::Pressed),
            key(KeyA, ButtonState::Pressed),
            key(KeyA, ButtonState::Released),
            key(ControlLeft, ButtonState::Released),
        ],
    )?;
    for code in [Digit1, Slash, Digit2] {
        frame(&mut runner, &tap(code))?;
    }
    let state = runner.resource::<state::MathState>().unwrap();
    assert_eq!(
        sim_math::Expression::parse(&state.document.fields[0].source().unwrap())
            .unwrap()
            .evaluate(0.0, 0.0)
            .unwrap(),
        0.5
    );
    assert!(
        state.layouts[0]
            .stop(state.document.fields[0].caret)
            .unwrap()
            .y
            > 0.0
    );
    frame(
        &mut runner,
        &[
            key(ControlLeft, ButtonState::Pressed),
            key(KeyZ, ButtonState::Pressed),
            key(KeyZ, ButtonState::Released),
            key(ControlLeft, ButtonState::Released),
        ],
    )?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .is_err()
    );
    frame(
        &mut runner,
        &[
            key(ControlLeft, ButtonState::Pressed),
            key(KeyY, ButtonState::Pressed),
            key(KeyY, ButtonState::Released),
            key(ControlLeft, ButtonState::Released),
        ],
    )?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .is_ok()
    );
    let s = runner.resource::<state::MathState>().unwrap();
    let stop = s.layouts[0]
        .stops
        .iter()
        .find(|stop| stop.y < 0.0 && stop.caret.index == 0)
        .unwrap();
    let expected = stop.caret;
    let rect = layout::Layout::for_state(viewport(), s).fields[0];
    let p = [
        rect.x + 12.0 - s.offsets[0][0] + stop.x,
        rect.y + 8.0 - s.offsets[0][1] + stop.y - stop.height * 0.35,
    ];
    frame(
        &mut runner,
        &[
            pointer(p),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .caret,
        expected
    );
    Ok(())
}

#[test]
fn inline_integral_requires_calculate_and_tracks_named_parameter() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "int(b*x^2)")?;
    click(&mut runner, layout::Target::Add)?;
    type_text(&mut runner, "b=2")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .plot
            .as_ref()
            .unwrap()
            .rows[0]
            .integral
            .is_none()
    );
    click(&mut runner, layout::Target::Calculate(0))?;
    until(&mut runner, |s| {
        !s.busy
            && s.plot
                .as_ref()
                .is_some_and(|p| p.rows[0].integral.is_some())
    })?;
    let value = |s: &state::MathState| s.plot.as_ref().unwrap().rows[0].integral.unwrap().0.value;
    assert!((value(runner.resource::<state::MathState>().unwrap()) - 2.0 / 3.0).abs() < 1e-8);
    click(&mut runner, layout::Target::Field(1))?;
    select_all(&mut runner)?;
    type_text(&mut runner, "b=3")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    assert!((value(runner.resource::<state::MathState>().unwrap()) - 1.0).abs() < 1e-8);
    select_all(&mut runner)?;
    type_text(&mut runner, "b=")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(s.plot.as_ref().unwrap().rows[0].integral.is_none());
    assert!(s.plot.as_ref().unwrap().rows[0].diagnostic.is_some());
    Ok(())
}

#[test]
fn two_structured_integrals_calculate_and_update_from_a_parameter() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "y=int(b*x^2)-int(x)")?;
    click(&mut runner, layout::Target::Add)?;
    type_text(&mut runner, "b=2")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .plot
            .as_ref()
            .unwrap()
            .rows[0]
            .scalar
            .is_none()
    );
    click(&mut runner, layout::Target::Calculate(0))?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    let value = |s: &state::MathState| s.plot.as_ref().unwrap().rows[0].scalar.unwrap();
    assert!((value(runner.resource::<state::MathState>().unwrap()) - 1.0 / 6.0).abs() < 1e-8);
    click(&mut runner, layout::Target::Field(1))?;
    select_all(&mut runner)?;
    type_text(&mut runner, "b=3")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    assert!((value(runner.resource::<state::MathState>().unwrap()) - 0.5).abs() < 1e-8);
    Ok(())
}

#[test]
fn focus_loss_and_return_modal_do_not_leak_input() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    frame(
        &mut runner,
        &[
            key(PhysicalKeyCode::ControlLeft, ButtonState::Pressed),
            InputEvent::FocusLost,
        ],
    )?;
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().controls,
        [false; 2]
    );
    click(&mut runner, layout::Target::Back)?;
    frame(
        &mut runner,
        &[
            key(PhysicalKeyCode::Escape, ButtonState::Pressed),
            pointer([800.0, 400.0]),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(!s.confirm_back && !s.canvas_pressed);
    frame(
        &mut runner,
        &[InputEvent::mouse_button(
            MouseButton::Left,
            ButtonState::Released,
        )],
    )?;
    click(&mut runner, layout::Target::Back)?;
    click(&mut runner, layout::Target::ConfirmBack)?;
    frame(&mut runner, &[])?;
    assert_eq!(runner.active_world_name(), "main-menu");
    Ok(())
}

#[test]
fn long_and_deep_formulas_are_not_rejected_or_recursively_laid_out() {
    let text = format!("{}x{}", "(".repeat(500), ")".repeat(500));
    let formula = formula::Formula::typed(&text);
    let layout = formula_layout::FormulaLayout::build(&formula, |_, s| s.len() as f32 * 12.0);
    assert!(layout.width > 1000.0);
    assert!(sim_math::Expression::parse(&formula.source().unwrap()).is_ok());
    let formula =
        formula::Formula::typed(&std::iter::repeat_n("x", 2500).collect::<Vec<_>>().join("+"));
    assert!(formula.source().unwrap().len() > 4000);
    let layout = formula_layout::FormulaLayout::build(&formula, |_, s| s.len() as f32 * 12.0);
    assert_eq!(layout.stops.len(), 5000);
}

#[test]
fn equal_expression_rows_keypad_visibility_and_structural_history() -> LogicResult {
    use crate::math_editor::view::layout::Target;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    click(&mut runner, Target::Add)?;
    click(&mut runner, Target::Type('2'))?;
    until(&mut runner, |s| !s.busy)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.focus, Some(1));
    assert_eq!(s.plot.as_ref().unwrap().rows[1].scalar, Some(2.0));
    click(&mut runner, Target::Visible(1))?;
    assert!(
        !runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .visible[1]
    );
    click(&mut runner, Target::Remove(1))?;
    for code in [
        PhysicalKeyCode::KeyZ,
        PhysicalKeyCode::KeyY,
        PhysicalKeyCode::KeyZ,
    ] {
        frame(
            &mut runner,
            &[
                key(PhysicalKeyCode::ControlLeft, ButtonState::Pressed),
                key(code, ButtonState::Pressed),
                key(code, ButtonState::Released),
                key(PhysicalKeyCode::ControlLeft, ButtonState::Released),
            ],
        )?;
    }
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields
            .len(),
        2
    );
    click(&mut runner, Target::Remove(0))?;
    until(&mut runner, |s| !s.busy)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.plot.as_ref().unwrap().rows[0].scalar, Some(2.0));
    Ok(())
}

#[test]
fn sidebar_scroll_is_not_graph_zoom_and_hidden_rows_cannot_be_clicked() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    for _ in 0..8 {
        click(&mut runner, layout::Target::Add)?;
    }
    let s = runner.resource::<state::MathState>().unwrap();
    let camera = s.camera;
    let scroll = s.sidebar_scroll;
    frame(
        &mut runner,
        &[
            pointer([180.0, 210.0]),
            InputEvent::mouse_wheel(ScrollDelta::lines(0.0, 2.0)?),
        ],
    )?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.camera, camera);
    assert!(s.sidebar_scroll < scroll);
    let layout = layout::Layout::for_state(viewport(), s);
    assert_ne!(
        layout.hit(layout.fields[0].center(), false),
        Some(layout::Target::Field(0))
    );
    Ok(())
}

#[test]
fn many_rows_have_no_product_quota_and_reveal_focused_input() -> LogicResult {
    let mut s = state::MathState::new()?;
    for _ in 0..300 {
        s.document.fields.push(formula::Formula::typed("x"));
        s.document.visible.push(true);
        s.document.styles.push(style::GraphStyle::new(0));
    }
    s.changed();
    let layout = layout::Layout::for_state(viewport(), &s);
    assert_eq!(layout.fields.len(), 301);
    assert!(layout.max_scroll > 20000.0);
    Ok(())
}

#[test]
fn bottom_keyboard_and_anchored_catalog_fit_supported_viewports() {
    for (width, height) in [(900.0, 640.0), (1280.0, 800.0), (1920.0, 1080.0)] {
        let mut s = state::MathState::new().unwrap();
        s.keypad = true;
        let viewport = LogicalViewport::new(width, height).unwrap();
        let l = layout::Layout::for_state(viewport, &s);
        assert_eq!(l.canvas.y + l.canvas.h, l.keypad_top);
        for target in [
            layout::Target::Type('7'),
            layout::Target::Root,
            layout::Target::Enter,
            layout::Target::Functions,
        ] {
            let r = l.controls.iter().find(|(t, _, _)| *t == target).unwrap().1;
            assert!(r.y >= l.keypad_top && r.y + r.h <= height - 28.0);
            assert!(r.x >= 0.0 && r.x + r.w <= width);
            assert!(!l.canvas.contains(r.center()));
        }
        s.functions = true;
        let l = layout::Layout::for_state(viewport, &s);
        let popup = l.popup.unwrap();
        let anchor = l
            .controls
            .iter()
            .find(|(t, _, _)| *t == layout::Target::Functions)
            .unwrap()
            .1;
        assert!(popup.y >= 112.0 && popup.y + popup.h < anchor.y);
        assert!(popup.x >= l.sidebar && popup.x + popup.w <= width);
        s.activate(layout::Target::Keypad);
        let collapsed = layout::Layout::for_state(viewport, &s);
        assert!(!s.keypad && !s.functions);
        assert!(collapsed.canvas.h > l.canvas.h);
    }
}

#[test]
fn bottom_keyboard_does_not_create_points_and_enter_matches_physical_key() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    click(&mut runner, layout::Target::Type('x'))?;
    click(&mut runner, layout::Target::Square)?;
    click(&mut runner, layout::Target::Enter)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields.len(), 2);
    assert!(s.document.geometry.points().is_empty());
    assert_eq!(
        sim_math::Expression::parse(&s.document.fields[0].source().unwrap())
            .unwrap()
            .evaluate(3.0, 0.0)
            .unwrap(),
        9.0
    );
    click(&mut runner, layout::Target::Keypad)?;
    click(&mut runner, layout::Target::Functions)?;
    frame(&mut runner, &tap(PhysicalKeyCode::Escape))?;
    assert!(!runner.resource::<state::MathState>().unwrap().functions);
    Ok(())
}

#[test]
fn inverse_trig_absolute_values_and_multi_argument_functions_type_naturally() {
    for (text, expected) in [
        ("sin^-1(0.5)", std::f64::consts::FRAC_PI_6),
        ("sin(0.5)^-1", 1.0 / 0.5_f64.sin()),
        ("atan2(1,1)", std::f64::consts::FRAC_PI_4),
        ("2sin(pi/2)", 2.0),
        ("mean(1,2,3)", 2.0),
        ("|(-3)|", 3.0),
    ] {
        let source = formula::Formula::typed(text).source().unwrap();
        let actual = sim_math::Expression::constant(&source).unwrap();
        assert!(
            (actual - expected).abs() < 1e-12,
            "{text}: {source} = {actual}"
        );
    }
}
#[test]
fn nested_radicals_expand_rows_and_keep_caret_reachable() {
    let mut s = state::MathState::new().unwrap();
    let baseline = layout::Layout::for_state(viewport(), &s).fields[0].h;
    s.document.fields[0] =
        formula::Formula::typed(&format!("{}x{}", "sqrt(".repeat(18), ")".repeat(18)));
    s.layouts[0] =
        formula_layout::FormulaLayout::build(&s.document.fields[0], |_, t| t.len() as f32 * 12.0);
    assert!(layout::Layout::for_state(viewport(), &s).fields[0].h > baseline);
    assert!(s.layouts[0].stop(s.document.fields[0].caret).is_some());
    assert!(sim_math::Expression::parse(&s.document.fields[0].source().unwrap()).is_ok());
}
#[test]
fn icon_hold_opens_style_without_toggling_and_cancel_does_not_leak() -> LogicResult {
    use crate::math_editor::view::layout::Target;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    let icon = {
        let s = runner.resource::<state::MathState>().unwrap();
        layout::Layout::for_state(viewport(), s)
            .controls
            .iter()
            .find(|(t, _, _)| *t == Target::Visible(0))
            .unwrap()
            .1
            .center()
    };
    frame(
        &mut runner,
        &[
            pointer(icon),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    for _ in 0..30 {
        frame(&mut runner, &[])?;
    }
    assert_eq!(
        runner.resource::<state::MathState>().unwrap().style_popup,
        Some(0)
    );
    frame(
        &mut runner,
        &[InputEvent::mouse_button(
            MouseButton::Left,
            ButtonState::Released,
        )],
    )?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .visible[0]
    );
    click(&mut runner, Target::Color(4))?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .styles[0]
            .color,
        4,
        "after swatch click: popup {:?}, suppress {}",
        runner.resource::<state::MathState>().unwrap().style_popup,
        runner
            .resource::<state::MathState>()
            .unwrap()
            .suppress_release
    );
    click(&mut runner, Target::Pattern(1))?;
    click(&mut runner, Target::Thickness(true))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.styles[0].color, 4);
    assert_eq!(s.document.styles[0].pattern, 1);
    assert!(s.document.fields[0].rows[0].is_empty());
    click(&mut runner, Target::ClosePopup)?;
    click(&mut runner, Target::Visible(0))?;
    assert!(
        !runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .visible[0]
    );
    frame(
        &mut runner,
        &[
            pointer(icon),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            InputEvent::FocusLost,
        ],
    )?;
    for _ in 0..30 {
        frame(&mut runner, &[])?;
    }
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .style_popup
            .is_none()
    );
    assert!(
        !runner
            .resource::<state::MathState>()
            .unwrap()
            .suppress_release
    );
    Ok(())
}
#[test]
fn catalog_is_scrollable_and_inserts_into_the_active_row() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    click(&mut runner, layout::Target::Functions)?;
    click(&mut runner, layout::Target::Function("sin"))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(!s.functions);
    assert!(s.document.fields[0].caret.row != 0);
    frame(&mut runner, &tap(PhysicalKeyCode::Digit0))?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert_eq!(s.document.fields[0].source().unwrap(), "sin(0)");
    click(&mut runner, layout::Target::Functions)?;
    let s = runner.resource::<state::MathState>().unwrap();
    let popup = layout::Layout::for_state(viewport(), s).popup.unwrap();
    frame(
        &mut runner,
        &[
            pointer(popup.center()),
            InputEvent::mouse_wheel(ScrollDelta::lines(0.0, -100.0)?),
        ],
    )?;
    let s = runner.resource::<state::MathState>().unwrap();
    let l = layout::Layout::for_state(viewport(), s);
    assert_eq!(s.catalog_scroll, l.catalog_max);
    assert!(
        l.popup_controls
            .iter()
            .any(|(t, r, _)| *t == layout::Target::Integral && l.popup_clip.contains(r.center()))
    );
    Ok(())
}

mod visual_probe;
