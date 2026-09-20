use std::time::Duration;

use sim_logic::prelude::*;

use crate::{
    MenuAction,
    menu::{
        state::{Domain, MenuState, Overlay, PhysicsScale, Target},
        view::Panel,
    },
    phys_editor::state::{EditorState, Mode},
};

fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}

fn frame(runner: &mut HeadlessRunner<MenuAction>, events: &[InputEvent]) -> LogicResult {
    match runner.advance_frame(FrameRequest::new(
        Duration::from_millis(16),
        events,
        viewport(),
    )) {
        FrameOutcome::Advanced(report) => {
            assert!(report.failure().is_none(), "{:?}", report.failure());
            assert_eq!(report.fixed_ticks_attempted(), 0);
            Ok(())
        }
        FrameOutcome::Rejected(error) => Err(error.into()),
    }
}

fn tap(runner: &mut HeadlessRunner<MenuAction>, key: PhysicalKeyCode) -> LogicResult {
    frame(
        runner,
        &[
            InputEvent::key(key, ButtonState::Pressed),
            InputEvent::key(key, ButtonState::Released),
        ],
    )
}

fn click(runner: &mut HeadlessRunner<MenuAction>, target: Target) -> LogicResult {
    let (entity, _) = runner
        .components::<Panel>()
        .find(|(_, panel)| matches!(panel, Panel::Button(value) if *value == target))
        .ok_or("missing menu button")?;
    let visual = runner.component::<ScreenRectangleVisual>(entity)?;
    let position = visual.position().to_vec2();
    let size = visual.size().to_vec2();
    let pointer = PointerSample::new(
        LogicalScreenPosition::new(position.x() + size.x() * 0.5, position.y() + size.y() * 0.5),
        viewport(),
    )?;
    frame(
        runner,
        &[
            InputEvent::pointer_moved(pointer),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )
}

#[test]
fn named_project_enters_real_editor_without_old_input_or_simulation() -> LogicResult {
    let (app, initial) = crate::build_application()?;
    let mut runner = app.build_headless(initial)?;
    frame(&mut runner, &[])?;
    let generation = runner.world_generation();
    for target in [
        Target::Domains,
        Target::Domain(Domain::Phys),
        Target::Scale(PhysicsScale::Macro),
        Target::CreateProject,
    ] {
        click(&mut runner, target)?;
    }
    assert_eq!(
        runner
            .resource::<MenuState>()
            .ok_or("missing menu")?
            .overlay,
        Overlay::CreateProject
    );
    for key in [
        PhysicalKeyCode::KeyN,
        PhysicalKeyCode::KeyE,
        PhysicalKeyCode::KeyW,
        PhysicalKeyCode::Space,
        PhysicalKeyCode::KeyL,
        PhysicalKeyCode::KeyA,
        PhysicalKeyCode::KeyB,
    ] {
        tap(&mut runner, key)?;
    }
    let mut events = vec![
        InputEvent::key(PhysicalKeyCode::Enter, ButtonState::Pressed),
        InputEvent::key(PhysicalKeyCode::Enter, ButtonState::Released),
    ];
    // This tail must not run physics or place an object in the new World.
    events.extend([
        InputEvent::key(PhysicalKeyCode::F5, ButtonState::Pressed),
        InputEvent::key(PhysicalKeyCode::F5, ButtonState::Released),
        InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
    ]);
    frame(&mut runner, &events)?;
    frame(&mut runner, &[])?;
    assert_ne!(runner.world_generation(), generation);
    assert_eq!(runner.active_world_name(), "physics-editor");
    assert!(runner.resource::<MenuState>().is_none());
    let editor = runner.resource::<EditorState>().ok_or("missing editor")?;
    assert_eq!(editor.project_name, "new lab");
    assert_eq!(editor.mode, Mode::Editor);
    assert!(editor.run.is_none());
    assert!(editor.document.objects().is_empty());
    assert!(!editor.environment_open);
    assert!(
        runner
            .components::<ScreenTextVisual>()
            .any(|(_, text)| text.text() == "new lab")
    );
    tap(&mut runner, PhysicalKeyCode::F5)?;
    assert_eq!(
        runner
            .resource::<EditorState>()
            .ok_or("missing editor")?
            .mode,
        Mode::Preview
    );
    tap(&mut runner, PhysicalKeyCode::Escape)?;
    assert_eq!(
        runner
            .resource::<EditorState>()
            .ok_or("missing editor")?
            .mode,
        Mode::Editor
    );
    Ok(())
}

#[test]
fn keyboard_path_and_mouse_confirmation_preserve_name_and_window_mode() -> LogicResult {
    let (app, initial) = crate::build_application()?;
    let mut runner = app.build_headless(initial)?;
    frame(&mut runner, &[])?;
    use PhysicalKeyCode::*;
    for key in [
        ArrowDown, Enter, ArrowDown, Enter, ArrowDown, ArrowDown, Enter, Enter,
    ] {
        tap(&mut runner, key)?;
    }
    assert_eq!(
        runner
            .resource::<MenuState>()
            .ok_or("missing menu")?
            .overlay,
        Overlay::CreateProject
    );
    tap(&mut runner, F11)?;
    for key in [Space, KeyA, Minus, Digit1, Space] {
        tap(&mut runner, key)?;
    }
    click(&mut runner, Target::ConfirmProject)?;
    frame(&mut runner, &[])?;
    let editor = runner.resource::<EditorState>().ok_or("missing editor")?;
    assert_eq!(editor.project_name, "a-1");
    assert!(!editor.fullscreen_requested);
    Ok(())
}
