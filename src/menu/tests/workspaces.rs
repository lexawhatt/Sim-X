use super::*;
use crate::menu::state::{Domain, PROJECT_NAME_LIMIT, PhysicsScale};

fn physics(runner: &mut HeadlessRunner<MenuAction>) -> LogicResult {
    click(runner, Target::Domains)?;
    click(runner, Target::Domain(Domain::Phys))?;
    assert_eq!(state(runner).overlay, Overlay::PhysicsScales);
    Ok(())
}

fn form(runner: &mut HeadlessRunner<MenuAction>) -> LogicResult {
    physics(runner)?;
    click(runner, Target::Scale(PhysicsScale::Macro))?;
    click(runner, Target::CreateProject)?;
    assert_eq!(state(runner).overlay, Overlay::CreateProject);
    assert_eq!(state(runner).focus.focused(), Some(Target::ProjectName));
    Ok(())
}

#[test]
fn scale_previews_and_macro_projects_have_explicit_back_navigation() -> LogicResult {
    let mut runner = runner()?;
    physics(&mut runner)?;
    assert_eq!(state(&runner).preview_scale(), None);
    for scale in [PhysicsScale::Micro, PhysicsScale::Astra] {
        click(&mut runner, Target::Scale(scale))?;
        assert_eq!(state(&runner).overlay, Overlay::PhysicsScales);
        assert_eq!(state(&runner).preview_scale(), Some(scale));
        assert!(state(&runner).project_error.contains("Coming soon"));
        assert!(state(&runner).pending_project.is_none());
    }
    click(&mut runner, Target::Scale(PhysicsScale::Macro))?;
    assert_eq!(state(&runner).overlay, Overlay::Projects);
    assert_eq!(state(&runner).preview_scale(), Some(PhysicsScale::Macro));
    click(&mut runner, Target::CreateProject)?;
    for overlay in [
        Overlay::Projects,
        Overlay::PhysicsScales,
        Overlay::Domains,
        Overlay::None,
    ] {
        advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
        assert_eq!(state(&runner).overlay, overlay);
    }
    Ok(())
}

#[test]
fn name_form_owns_physical_editor_shortcuts_and_rejects_blank_name() -> LogicResult {
    let mut runner = runner()?;
    form(&mut runner)?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert_eq!(state(&runner).overlay, Overlay::CreateProject);
    assert_eq!(state(&runner).project_error, "Give your project a name.");
    for key in [
        PhysicalKeyCode::KeyE,
        PhysicalKeyCode::Digit4,
        PhysicalKeyCode::Space,
        PhysicalKeyCode::KeyF,
    ] {
        advance(&mut runner, viewport(), &key_tap(key))?;
    }
    assert_eq!(state(&runner).project_name, "e4 f");
    assert_eq!(state(&runner).overlay, Overlay::CreateProject);
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::Backspace),
    )?;
    assert_eq!(state(&runner).project_name, "e4 ");
    let ctrl = [
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Pressed),
        InputEvent::key(PhysicalKeyCode::KeyV, ButtonState::Pressed),
        InputEvent::key(PhysicalKeyCode::KeyV, ButtonState::Released),
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Released),
    ];
    advance(&mut runner, viewport(), &ctrl)?;
    assert_eq!(state(&runner).project_name, "e4 ");
    Ok(())
}

#[test]
fn name_form_focus_is_trapped_and_cancellation_owns_the_frame_tail() -> LogicResult {
    let mut runner = runner()?;
    form(&mut runner)?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::ConfirmProject));
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::CancelProject));
    let mut events = key_tap(PhysicalKeyCode::Escape).to_vec();
    events.extend(key_tap(PhysicalKeyCode::Enter));
    advance(&mut runner, viewport(), &events)?;
    assert_eq!(state(&runner).overlay, Overlay::Projects);
    assert!(state(&runner).pending_project.is_none());
    Ok(())
}

#[test]
fn project_name_model_is_unicode_bounded_trimmed_and_never_blank() {
    let mut state = MenuState {
        project_name: "   ".into(),
        ..MenuState::default()
    };
    assert!(!state.confirm_project());
    assert!(state.pending_project.is_none());
    assert!(!state.departing);
    state.project_name.clear();
    for _ in 0..PROJECT_NAME_LIMIT + 10 {
        state.append_project_character('я');
    }
    assert_eq!(state.project_name.chars().count(), PROJECT_NAME_LIMIT);
    state.append_project_character('\n');
    assert_eq!(state.project_name.chars().count(), PROJECT_NAME_LIMIT);
    state.project_name = "a".repeat(PROJECT_NAME_LIMIT + 1);
    assert!(!state.confirm_project());
    state.project_name = "Two\nlines".into();
    assert!(!state.confirm_project());
    state.project_name = "  Newton lab  ".into();
    assert!(state.confirm_project());
    assert_eq!(state.pending_project.as_deref(), Some("Newton lab"));
    assert!(state.departing);
}

#[test]
fn scale_layers_crossfade_with_bounded_retained_resources() -> LogicResult {
    let mut runner = runner()?;
    physics(&mut runner)?;
    let labels = runner.components::<ScreenTextVisual>().count();
    for scale in PhysicsScale::ALL {
        advance(
            &mut runner,
            viewport(),
            &[over(
                Target::Scale(scale),
                Overlay::PhysicsScales,
                viewport(),
            )],
        )?;
        for _ in 0..12 {
            advance(&mut runner, viewport(), &[])?;
            let menu = state(&runner);
            assert!(
                menu.scale_mix
                    .iter()
                    .all(|value| (0.0..=1.0).contains(value))
            );
            assert!(menu.scale_mix.iter().sum::<f32>() <= 1.00001);
            assert!(menu.picker_mix.iter().sum::<f32>() <= 1.00001);
            assert_eq!(runner.components::<ScreenTextVisual>().count(), labels);
        }
    }
    Ok(())
}

#[test]
fn reduced_motion_snaps_scale_selection_and_freezes_decorative_phase() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Settings)?;
    click(&mut runner, Target::ReduceMotion)?;
    click(&mut runner, Target::Close)?;
    let phase = state(&runner).phase;
    let macro_seconds = state(&runner).macro_seconds;
    physics(&mut runner)?;
    assert_eq!(state(&runner).scale_blend, 1.0);
    assert_eq!(state(&runner).picker_mix, [0.0, 1.0, 0.0, 0.0]);
    for scale in PhysicsScale::ALL {
        advance(
            &mut runner,
            viewport(),
            &[over(
                Target::Scale(scale),
                Overlay::PhysicsScales,
                viewport(),
            )],
        )?;
        let mut expected = [0.0; 3];
        expected[scale.index()] = 1.0;
        assert_eq!(state(&runner).scale_mix, expected);
        assert_eq!(state(&runner).phase, phase);
        assert_eq!(state(&runner).macro_seconds, macro_seconds);
    }
    Ok(())
}

#[test]
fn macro_carousel_advances_only_while_its_scale_is_previewed() -> LogicResult {
    let mut runner = runner()?;
    physics(&mut runner)?;
    let initial = state(&runner).macro_seconds;
    advance(&mut runner, viewport(), &[])?;
    assert_eq!(state(&runner).macro_seconds, initial);
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Scale(PhysicsScale::Macro),
            Overlay::PhysicsScales,
            viewport(),
        )],
    )?;
    for _ in 0..70 {
        advance(&mut runner, viewport(), &[])?;
    }
    assert!(state(&runner).macro_seconds > initial + 1.0);
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Scale(PhysicsScale::Astra),
            Overlay::PhysicsScales,
            viewport(),
        )],
    )?;
    let frozen = state(&runner).macro_seconds;
    for _ in 0..10 {
        advance(&mut runner, viewport(), &[])?;
    }
    assert_eq!(state(&runner).macro_seconds, frozen);
    click(&mut runner, Target::Scale(PhysicsScale::Macro))?;
    for _ in 0..10 {
        advance(&mut runner, viewport(), &[])?;
    }
    assert_eq!(state(&runner).macro_seconds, frozen);
    Ok(())
}

#[test]
fn all_new_targets_fit_minimum_window_and_hidden_geometry_survives_resize() -> LogicResult {
    let minimum = LogicalViewport::new(360.0, 560.0)?;
    let layout = Layout::new(minimum);
    let mut model = MenuState::default();
    for overlay in [
        Overlay::PhysicsScales,
        Overlay::Projects,
        Overlay::CreateProject,
    ] {
        model.show(overlay)?;
        for &target in model.targets() {
            let rect = layout.target(target, overlay);
            assert!(rect.x >= 0.0 && rect.y >= 0.0);
            assert!(rect.x + rect.width <= minimum.width());
            assert!(rect.y + rect.height <= minimum.height());
            assert!(rect.width > 0.0 && rect.height >= 42.0);
        }
    }
    let mut runner = runner()?;
    form(&mut runner)?;
    for (width, height) in [(1.0, 1.0), (64.0, 64.0), (360.0, 560.0)] {
        advance(&mut runner, LogicalViewport::new(width, height)?, &[])?;
    }
    assert!(state(&runner).pending_project.is_none());
    Ok(())
}
