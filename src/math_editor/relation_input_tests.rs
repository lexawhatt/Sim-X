use super::*;
use crate::math_editor::app;

#[test]
fn physical_comparison_keys_preserve_unshifted_comma_and_decimal() {
    use PhysicalKeyCode::*;
    assert_eq!(character(Comma, false), Some(','));
    assert_eq!(character(Comma, true), Some('<'));
    assert_eq!(character(Period, false), Some('.'));
    assert_eq!(character(Period, true), Some('>'));
}

#[test]
fn numeric_dock_exposes_all_relations_without_overlapping_existing_controls() -> LogicResult {
    let mut state = MathState::new()?;
    state.keypad = true;
    for (width, height) in [(900.0, 640.0), (1280.0, 800.0), (1920.0, 1080.0)] {
        let layout = Layout::for_state(LogicalViewport::new(width, height)?, &state);
        let keys: Vec<_> = layout
            .controls
            .iter()
            .filter(|(_, rect, _)| rect.y >= layout.keypad_top)
            .collect();
        for sign in ['<', '>', '≤', '≥'] {
            let (_, rect, _) = keys
                .iter()
                .find(|(t, _, _)| *t == Target::Type(sign))
                .unwrap();
            assert_eq!(layout.hit(rect.center(), false), Some(Target::Type(sign)));
        }
        assert!(keys.iter().any(|(t, _, _)| *t == Target::Fraction));
        assert!(keys.iter().any(|(t, _, _)| *t == Target::NthRoot));
        for (i, (_, rect, _)) in keys.iter().enumerate() {
            assert!(rect.x >= 0.0 && rect.x + rect.w <= width);
            assert!(rect.y + rect.h <= height - 28.0);
            for (_, other, _) in keys.iter().skip(i + 1) {
                assert!(rect.intersection(*other).is_none());
            }
        }
    }
    Ok(())
}

#[test]
fn native_physical_absolute_value_relation_reaches_the_document_intact() -> LogicResult {
    use PhysicalKeyCode::*;
    let (app, initial) = app::build_math_editor_application()?;
    let mut runner = app.build_headless(initial)?;
    let viewport = LogicalViewport::new(1280.0, 800.0)?;
    let frame = |runner: &mut HeadlessRunner<AppAction>, events: &[InputEvent]| -> LogicResult {
        match runner.advance_frame(FrameRequest::new(
            std::time::Duration::from_millis(16),
            events,
            viewport,
        )) {
            FrameOutcome::Rejected(error) => Err(error.into()),
            _ => Ok(()),
        }
    };
    frame(&mut runner, &[])?;
    for ch in "max(|x|,|y|,|z|)<=1".chars() {
        let (code, shift) = match ch {
            'm' => (KeyM, false),
            'a' => (KeyA, false),
            'x' => (KeyX, false),
            'y' => (KeyY, false),
            'z' => (KeyZ, false),
            '1' => (Digit1, false),
            '(' => (Digit9, true),
            ')' => (Digit0, true),
            '|' => (Backslash, true),
            ',' => (Comma, false),
            '<' => (Comma, true),
            '=' => (Equal, false),
            _ => panic!(),
        };
        let mut events = vec![];
        if shift {
            events.push(InputEvent::key(ShiftLeft, ButtonState::Pressed));
        }
        events.extend([
            InputEvent::key(code, ButtonState::Pressed),
            InputEvent::key(code, ButtonState::Released),
        ]);
        if shift {
            events.push(InputEvent::key(ShiftLeft, ButtonState::Released));
        }
        frame(&mut runner, &events)?;
    }
    let state = runner.resource::<MathState>().unwrap();
    assert_eq!(
        state.document.fields[0].source().unwrap(),
        "max(abs(x),abs(y),abs(z))<=1"
    );
    assert!(
        state.layouts[0]
            .glyphs
            .iter()
            .any(|glyph| glyph.text == "≤")
    );
    Ok(())
}
