use std::time::Duration;

use sim_logic::prelude::*;

use super::{
    catalog::{CatalogTarget, EntryId},
    catalog_layout::CatalogLayout,
    document::{MAX_OBJECTS, ObjectKind, Point},
    input::EditorAction,
    layout::{Layout, Rect},
    placement::Placement,
    state::{Control, EditorState, Mode, Tool},
    view::{Panel, VisualSlot},
};

fn runner() -> LogicResult<HeadlessRunner<EditorAction>> {
    let (application, initial) = crate::build_phys_editor_application()?;
    Ok(application.build_headless(initial)?)
}

fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}

fn state(runner: &HeadlessRunner<EditorAction>) -> &EditorState {
    runner.resource::<EditorState>().unwrap()
}

fn advance(
    runner: &mut HeadlessRunner<EditorAction>,
    viewport: LogicalViewport,
    events: &[InputEvent],
) -> LogicResult<LogicFrameReport> {
    let request = FrameRequest::new(Duration::from_millis(16), events, viewport);
    match runner.advance_frame(request) {
        FrameOutcome::Advanced(report) => {
            assert!(report.failure().is_none(), "{:?}", report.failure());
            assert_eq!(report.fixed_ticks_attempted(), 0);
            assert_eq!(report.spawned(), 0);
            assert_eq!(report.despawned(), 0);
            Ok(report)
        }
        FrameOutcome::Rejected(error) => Err(error.into()),
    }
}

fn centre(rect: Rect) -> LogicalScreenPosition {
    LogicalScreenPosition::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn canvas_point(point: Point) -> LogicalScreenPosition {
    let layout = Layout::new(viewport(), Mode::Editor);
    super::state::Camera::default().project(point, layout.canvas)
}

fn control_point(control: Control, mode: Mode) -> LogicalScreenPosition {
    centre(Layout::new(viewport(), mode).control(control).unwrap())
}

fn motion(point: LogicalScreenPosition, viewport: LogicalViewport) -> InputEvent {
    InputEvent::pointer_moved(PointerSample::new(point, viewport).unwrap())
}

fn button(button: MouseButton, state: ButtonState) -> InputEvent {
    InputEvent::mouse_button(button, state)
}

fn press() -> InputEvent {
    button(MouseButton::Left, ButtonState::Pressed)
}

fn release() -> InputEvent {
    button(MouseButton::Left, ButtonState::Released)
}

fn key_tap(key: PhysicalKeyCode) -> [InputEvent; 2] {
    [
        InputEvent::key(key, ButtonState::Pressed),
        InputEvent::key(key, ButtonState::Released),
    ]
}

fn shortcut(key: PhysicalKeyCode) -> [InputEvent; 4] {
    let [down, up] = key_tap(key);
    [
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Pressed),
        down,
        up,
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Released),
    ]
}

fn click_at(
    runner: &mut HeadlessRunner<EditorAction>,
    point: LogicalScreenPosition,
) -> LogicResult {
    advance(
        runner,
        viewport(),
        &[motion(point, viewport()), press(), release()],
    )?;
    Ok(())
}

fn click(runner: &mut HeadlessRunner<EditorAction>, control: Control) -> LogicResult {
    if let Control::Palette(kind) = control {
        let entry = match kind {
            ObjectKind::Ball => EntryId::Ball,
            ObjectKind::Box => EntryId::Box,
            ObjectKind::Anchor => EntryId::Anchor,
        };
        let position = open_entry(runner, entry)?;
        click_at(runner, position)?;
        for _ in 0..30 {
            advance(runner, viewport(), &[])?;
        }
        return Ok(());
    }
    click_at(runner, control_point(control, state(runner).mode))
}

fn open_entry(
    runner: &mut HeadlessRunner<EditorAction>,
    entry: EntryId,
) -> LogicResult<LogicalScreenPosition> {
    let editor = state(runner);
    let layout = Layout::for_state(viewport(), editor);
    let catalog = CatalogLayout::new(&layout, &editor.catalog);
    click_at(runner, centre(catalog.opener()))?;
    for _ in 0..30 {
        advance(runner, viewport(), &[])?;
    }
    let editor = state(runner);
    let catalog = CatalogLayout::new(&Layout::for_state(viewport(), editor), &editor.catalog);
    click_at(runner, centre(catalog.category(entry.entry().category)))?;
    let editor = state(runner);
    let catalog = CatalogLayout::new(&Layout::for_state(viewport(), editor), &editor.catalog);
    Ok(centre(
        catalog
            .target(CatalogTarget::Entry(entry), &editor.catalog)
            .unwrap(),
    ))
}

fn visual_counts(runner: &HeadlessRunner<EditorAction>) -> [usize; 4] {
    [
        runner.components::<ScreenRectangleVisual>().count(),
        runner.components::<ScreenCircleVisual>().count(),
        runner.components::<ScreenLineVisual>().count(),
        runner.components::<ScreenTextVisual>().count(),
    ]
}

#[test]
fn idle_editor_is_static_and_reuses_preallocated_visuals() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::new(1.0, 2.0)))?;
    let document = state(&runner).document.objects().to_vec();
    let counts = visual_counts(&runner);
    assert_eq!(
        counts,
        [
            super::view::RECT_COUNT + super::catalog_view::RECT_COUNT,
            super::view::CIRCLE_COUNT + super::catalog_view::CIRCLE_COUNT,
            super::view::LINE_COUNT + super::catalog_view::LINE_COUNT,
            super::view::TEXT_COUNT + super::catalog_view::TEXT_COUNT,
        ]
    );
    assert_eq!(runner.components::<ActiveCamera2d>().count(), 1);
    assert_eq!(
        counts.iter().sum::<usize>() + 1,
        super::view::ENTITY_COUNT + super::catalog_view::ENTITY_COUNT
    );
    for _ in 0..120 {
        advance(&mut runner, viewport(), &[])?;
        assert_eq!(state(&runner).document.objects(), document);
        assert_eq!(visual_counts(&runner), counts);
        assert_eq!(state(&runner).mode, Mode::Editor);
    }
    Ok(())
}

#[test]
fn canvas_stamping_commits_once_on_ordinary_release() -> LogicResult {
    let mut runner = runner()?;
    let position = Point::new(1.0, 2.0);
    advance(
        &mut runner,
        viewport(),
        &[motion(canvas_point(position), viewport()), press()],
    )?;
    assert!(state(&runner).document.objects().is_empty());
    assert!(state(&runner).drag.is_some());
    advance(&mut runner, viewport(), &[release()])?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    assert_eq!(state(&runner).document.objects()[0].position, position);
    assert!(state(&runner).drag.is_none());
    advance(&mut runner, viewport(), &[release()])?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    Ok(())
}

#[test]
fn focus_loss_pointer_leave_and_resize_cancel_uncommitted_gestures() -> LogicResult {
    for cancellation in [InputEvent::FocusLost, InputEvent::PointerLeft] {
        let mut runner = runner()?;
        let point = canvas_point(Point::default());
        advance(
            &mut runner,
            viewport(),
            &[motion(point, viewport()), press()],
        )?;
        advance(&mut runner, viewport(), &[cancellation])?;
        assert!(state(&runner).drag.is_none());
        advance(
            &mut runner,
            viewport(),
            &[motion(point, viewport()), release()],
        )?;
        assert!(state(&runner).document.objects().is_empty());
        click_at(&mut runner, point)?;
        assert_eq!(state(&runner).document.objects().len(), 1);
    }
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &[motion(canvas_point(Point::default()), viewport()), press()],
    )?;
    advance(
        &mut runner,
        LogicalViewport::new(1000.0, 700.0)?,
        &[release()],
    )?;
    assert!(state(&runner).document.objects().is_empty());
    assert!(state(&runner).drag.is_none());
    Ok(())
}

#[test]
fn palette_click_selects_brush_and_palette_drag_places_that_kind_once() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Control::Palette(ObjectKind::Box))?;
    assert_eq!(state(&runner).brush, Placement::Primitive(ObjectKind::Box));
    assert_eq!(state(&runner).tool, Tool::Build);
    assert!(state(&runner).document.objects().is_empty());
    let palette = open_entry(&mut runner, EntryId::Anchor)?;
    let destination = Point::new(6.0, -1.0);
    advance(
        &mut runner,
        viewport(),
        &[motion(palette, viewport()), press()],
    )?;
    assert!(state(&runner).document.objects().is_empty());
    advance(
        &mut runner,
        viewport(),
        &[motion(canvas_point(destination), viewport()), release()],
    )?;
    let object = &state(&runner).document.objects()[0];
    assert_eq!(object.kind, ObjectKind::Anchor);
    assert_eq!(object.position, destination);
    assert_eq!(state(&runner).document.objects().len(), 1);
    Ok(())
}

#[test]
fn palette_press_cannot_activate_another_control_on_release() -> LogicResult {
    let mut runner = runner()?;
    let palette = open_entry(&mut runner, EntryId::Box)?;
    let run = control_point(Control::Run, Mode::Editor);
    advance(
        &mut runner,
        viewport(),
        &[
            motion(palette, viewport()),
            press(),
            motion(run, viewport()),
            release(),
        ],
    )?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert!(state(&runner).document.objects().is_empty());
    assert!(state(&runner).drag.is_none());
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    click(&mut runner, Control::Run)?;
    assert_eq!(state(&runner).mode, Mode::Preview);
    Ok(())
}

#[test]
fn selected_drag_is_one_edit_and_commits_the_release_position() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    click(&mut runner, Control::Tool(Tool::Select))?;
    advance(
        &mut runner,
        viewport(),
        &[motion(canvas_point(Point::default()), viewport()), press()],
    )?;
    for position in [Point::new(1.0, 1.0), Point::new(2.0, 2.0)] {
        advance(
            &mut runner,
            viewport(),
            &[motion(canvas_point(position), viewport())],
        )?;
        assert_eq!(
            state(&runner).document.objects()[0].position,
            Point::default()
        );
    }
    let released = Point::new(3.0, 2.0);
    advance(
        &mut runner,
        viewport(),
        &[
            motion(canvas_point(released), viewport()),
            release(),
            motion(canvas_point(Point::new(-2.0, -2.0)), viewport()),
        ],
    )?;
    assert_eq!(state(&runner).document.objects()[0].position, released);
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyZ))?;
    assert_eq!(
        state(&runner).document.objects()[0].position,
        Point::default()
    );
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyY))?;
    assert_eq!(state(&runner).document.objects()[0].position, released);
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyZ))?;
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyZ))?;
    assert!(state(&runner).document.objects().is_empty());
    Ok(())
}

#[test]
fn scroll_is_pointer_anchored_and_middle_drag_changes_only_camera() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    let document = state(&runner).document.objects().to_vec();
    let layout = Layout::new(viewport(), Mode::Editor);
    let pointer = canvas_point(Point::new(2.0, 1.0));
    let before = state(&runner).camera.unproject(pointer, layout.canvas);
    advance(
        &mut runner,
        viewport(),
        &[
            motion(pointer, viewport()),
            InputEvent::mouse_wheel(ScrollDelta::lines(0.0, 2.0)?),
        ],
    )?;
    let after = state(&runner).camera.unproject(pointer, layout.canvas);
    assert!((before.x - after.x).abs() < 1e-10);
    assert!((before.y - after.y).abs() < 1e-10);
    let camera = state(&runner).camera;
    let moved =
        LogicalScreenPosition::new(pointer.to_vec2().x() + 60.0, pointer.to_vec2().y() + 30.0);
    advance(
        &mut runner,
        viewport(),
        &[
            button(MouseButton::Middle, ButtonState::Pressed),
            motion(moved, viewport()),
            button(MouseButton::Middle, ButtonState::Released),
        ],
    )?;
    assert!(
        (state(&runner).camera.center.x - (camera.center.x - 60.0 / camera.pixels_per_m)).abs()
            < 1e-10
    );
    assert!(
        (state(&runner).camera.center.y - (camera.center.y + 30.0 / camera.pixels_per_m)).abs()
            < 1e-10
    );
    assert!(state(&runner).pan.is_none());
    assert_eq!(state(&runner).document.objects(), document);
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyZ))?;
    assert!(state(&runner).document.objects().is_empty());
    Ok(())
}

#[test]
fn inspector_mass_is_authoring_only_and_anchors_disable_it() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    click(&mut runner, Control::MassUp)?;
    assert_eq!(state(&runner).document.objects()[0].mass_kg, 2.0);
    click(&mut runner, Control::MassDown)?;
    assert_eq!(state(&runner).document.objects()[0].mass_kg, 1.0);
    click(&mut runner, Control::Palette(ObjectKind::Anchor))?;
    click_at(&mut runner, canvas_point(Point::new(2.0, 0.0)))?;
    assert!(!state(&runner).enabled(Control::MassUp));
    let document = state(&runner).document.objects().to_vec();
    click(&mut runner, Control::MassUp)?;
    assert_eq!(state(&runner).document.objects(), document);
    Ok(())
}

#[test]
fn run_opens_separate_physics_view_and_back_preserves_the_scene() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    click(&mut runner, Control::MassUp)?;
    let document = state(&runner).document.objects().to_vec();
    let counts = visual_counts(&runner);
    click(&mut runner, Control::Run)?;
    assert_eq!(state(&runner).mode, Mode::Preview);
    assert!(state(&runner).ghost().is_none());
    let preview_layout = Layout::new(viewport(), Mode::Preview);
    for control in Control::ALL {
        if !matches!(
            control,
            Control::Back
                | Control::Home
                | Control::Environment
                | Control::Pause
                | Control::Slow
                | Control::Normal
                | Control::Fast
        ) {
            assert!(!state(&runner).enabled(control));
            assert!(preview_layout.control(control).is_none());
        }
    }
    for (entity, slot) in runner.components::<VisualSlot>() {
        if matches!(slot, VisualSlot::Panel(Panel::Inspector | Panel::Palette)) {
            assert_eq!(
                runner.component::<ScreenRectangleVisual>(entity)?.clip(),
                ScreenClip::Empty
            );
        }
    }
    click_at(&mut runner, centre(preview_layout.canvas))?;
    click_at(&mut runner, control_point(Control::MassUp, Mode::Editor))?;
    for key in [
        PhysicalKeyCode::Delete,
        PhysicalKeyCode::Digit2,
        PhysicalKeyCode::KeyG,
    ] {
        advance(&mut runner, viewport(), &key_tap(key))?;
    }
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyD))?;
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyZ))?;
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyY))?;
    for _ in 0..120 {
        advance(&mut runner, viewport(), &[])?;
        assert_eq!(state(&runner).document.objects(), document);
        assert_eq!(visual_counts(&runner), counts);
    }
    click(&mut runner, Control::Back)?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert_eq!(state(&runner).document.objects(), document);
    click(&mut runner, Control::MassUp)?;
    assert_eq!(state(&runner).document.objects()[0].mass_kg, 4.0);
    Ok(())
}

#[test]
fn preview_mode_change_does_not_pass_remaining_batch_into_editor() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    click(&mut runner, Control::Run)?;
    let document = state(&runner).document.objects().to_vec();
    let [escape_down, escape_up] = key_tap(PhysicalKeyCode::Escape);
    let [delete_down, delete_up] = key_tap(PhysicalKeyCode::Delete);
    advance(
        &mut runner,
        viewport(),
        &[escape_down, escape_up, delete_down, delete_up],
    )?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert_eq!(state(&runner).document.objects(), document);
    Ok(())
}

#[test]
fn run_cancels_a_held_stamp_before_entering_preview() -> LogicResult {
    let mut runner = runner()?;
    let pointer = canvas_point(Point::default());
    advance(
        &mut runner,
        viewport(),
        &[motion(pointer, viewport()), press()],
    )?;
    assert!(state(&runner).drag.is_some());
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::F5))?;
    assert_eq!(state(&runner).mode, Mode::Preview);
    assert!(state(&runner).drag.is_none());
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    advance(
        &mut runner,
        viewport(),
        &[motion(pointer, viewport()), release()],
    )?;
    assert!(state(&runner).document.objects().is_empty());
    click_at(&mut runner, pointer)?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    Ok(())
}

#[test]
fn tiny_viewports_disable_document_editing_and_remain_renderable() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    let document = state(&runner).document.objects().to_vec();
    for (width, height) in [(1.0, 1.0), (360.0, 560.0), (899.0, 599.0)] {
        let tiny = LogicalViewport::new(width, height)?;
        let [delete_down, delete_up] = key_tap(PhysicalKeyCode::Delete);
        advance(
            &mut runner,
            tiny,
            &[
                motion(LogicalScreenPosition::new(width * 0.5, height * 0.5), tiny),
                press(),
                release(),
                delete_down,
                delete_up,
            ],
        )?;
        assert_eq!(state(&runner).document.objects(), document);
        assert!(state(&runner).drag.is_none());
        assert!(state(&runner).hovered.is_none());
    }
    Ok(())
}

#[test]
fn control_shortcuts_need_live_modifier_and_cancel_on_focus_loss() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::KeyZ))?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    advance(
        &mut runner,
        viewport(),
        &[InputEvent::key(
            PhysicalKeyCode::ControlLeft,
            ButtonState::Pressed,
        )],
    )?;
    advance(&mut runner, viewport(), &[InputEvent::FocusLost])?;
    assert_eq!(state(&runner).controls_held, [false, false]);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::KeyZ))?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyZ))?;
    assert!(state(&runner).document.objects().is_empty());
    advance(&mut runner, viewport(), &shortcut(PhysicalKeyCode::KeyY))?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    Ok(())
}

#[test]
fn bound_keyboard_events_do_not_accumulate_into_an_unconsumed_fixed_queue() -> LogicResult {
    let mut runner = runner()?;
    click_at(&mut runner, canvas_point(Point::default()))?;
    let document = state(&runner).document.objects().to_vec();
    // Each frame is below the 128-event budget, but the complete stream is
    // four times larger. Fixed-input retention must not grow while authoring.
    for frame in 0..256 {
        let (key, tool) = if frame % 2 == 0 {
            (PhysicalKeyCode::Digit1, Tool::Select)
        } else {
            (PhysicalKeyCode::Digit2, Tool::Build)
        };
        advance(&mut runner, viewport(), &key_tap(key))?;
        assert_eq!(state(&runner).tool, tool);
        assert_eq!(state(&runner).document.objects(), document);
    }
    click(&mut runner, Control::Run)?;
    for _ in 0..80 {
        advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Digit3))?;
        assert_eq!(state(&runner).mode, Mode::Preview);
        assert_eq!(state(&runner).document.objects(), document);
    }
    click(&mut runner, Control::Back)?;
    assert_eq!(state(&runner).mode, Mode::Editor);
    Ok(())
}

#[test]
fn full_mixed_scene_and_build_ghost_fit_retained_budgets_in_editor_and_preview() -> LogicResult {
    fn populate_scene(mut state: ResMut<EditorState>) {
        for index in 0..MAX_OBJECTS {
            let kind = ObjectKind::ALL[index % ObjectKind::ALL.len()];
            let position = Point::new((index % 16) as f64 - 7.5, (index / 16) as f64 - 3.5);
            let id = state.document.add(kind, position).unwrap();
            state.document.set_size(id, 0.6).unwrap();
            state
                .document
                .rotate(id, (index % 12) as f64 * 15.0)
                .unwrap();
            if kind != ObjectKind::Anchor {
                state.document.set_mass(id, index as f64 + 1.0).unwrap();
            }
            state.selected = Some(id);
        }
    }

    let (mut application, initial) = crate::build_phys_editor_application()?;
    application.add_system(Stage::Startup, populate_scene);
    let mut runner = application.build_headless(initial)?;
    let counts = visual_counts(&runner);
    let document = state(&runner).document.objects().to_vec();
    assert_eq!(document.len(), MAX_OBJECTS);
    for kind in ObjectKind::ALL {
        assert!(document.iter().any(|object| object.kind == kind));
    }
    let point = canvas_point(Point::new(0.0, 4.5));
    advance(&mut runner, viewport(), &[motion(point, viewport())])?;
    assert!(state(&runner).ghost().is_some());
    assert!(
        runner
            .extracted_frame()
            .unwrap()
            .screen_primitives()
            .count()
            > MAX_OBJECTS
    );
    assert_eq!(visual_counts(&runner), counts);
    assert_eq!(
        counts.iter().sum::<usize>() + 1,
        super::view::ENTITY_COUNT + super::catalog_view::ENTITY_COUNT
    );
    click_at(&mut runner, point)?;
    assert_eq!(state(&runner).document.objects(), document);
    assert!(
        state(&runner)
            .status
            .starts_with("Scene limit reached: 128")
    );
    click(&mut runner, Control::Run)?;
    assert_eq!(state(&runner).mode, Mode::Preview);
    assert!(state(&runner).ghost().is_none());
    assert!(
        runner
            .extracted_frame()
            .unwrap()
            .screen_primitives()
            .count()
            > MAX_OBJECTS
    );
    for _ in 0..20 {
        advance(&mut runner, viewport(), &[])?;
        assert_eq!(state(&runner).document.objects(), document);
        assert_eq!(visual_counts(&runner), counts);
    }
    click(&mut runner, Control::Back)?;
    assert_eq!(state(&runner).document.objects(), document);
    assert_eq!(visual_counts(&runner), counts);
    Ok(())
}

#[test]
fn editor_panels_render_behind_controls_and_accents() -> LogicResult {
    let mut runner = runner()?;
    advance(&mut runner, viewport(), &[])?;
    let mut checked = [0; 4];
    for (entity, slot) in runner.components::<VisualSlot>() {
        let (expected_depth, group) = match slot {
            VisualSlot::Panel(_) => (3.0, 0),
            VisualSlot::Button(_) => (4.0, 1),
            VisualSlot::Accent(_) => (5.0, 2),
            VisualSlot::PaletteIcon(_) => (5.0, 3),
            _ => continue,
        };
        let depth = if let Ok(visual) = runner.component::<ScreenRectangleVisual>(entity) {
            if visual.clip() == ScreenClip::Empty {
                continue;
            }
            visual.draw_order_depth()
        } else {
            let visual = runner.component::<ScreenCircleVisual>(entity)?;
            if visual.clip() == ScreenClip::Empty {
                continue;
            }
            visual.draw_order_depth()
        };
        assert_eq!(depth, expected_depth);
        checked[group] += 1;
    }
    assert!(checked[..3].iter().all(|count| *count > 0));
    assert_eq!(checked[0], 4);
    assert_eq!(checked[3], 0, "the new catalog owns object icons");
    Ok(())
}

#[test]
fn overlapping_shapes_draw_in_the_same_order_as_picking() -> LogicResult {
    let (mut application, initial) = crate::build_phys_editor_application()?;
    application.add_fallible_frame_system(|mut state: ResMut<EditorState>| -> LogicResult {
        if state.document.objects().is_empty() {
            for kind in [ObjectKind::Box, ObjectKind::Ball, ObjectKind::Anchor] {
                state.document.add(kind, Point::default())?;
            }
        }
        Ok(())
    });
    let mut runner = application.build_headless(initial)?;
    advance(&mut runner, viewport(), &[])?;
    advance(&mut runner, viewport(), &[])?;
    assert_eq!(state(&runner).pick(Point::default()), Some(3));
    let mut depths = [None; 3];
    for (entity, slot) in runner.components::<VisualSlot>() {
        match slot {
            VisualSlot::ObjectEdge {
                index: index @ (0 | 2),
                edge: 0,
            } => {
                let visual = runner.component::<ScreenLineVisual>(entity)?;
                assert_ne!(visual.clip(), ScreenClip::Empty);
                depths[*index] = Some(visual.draw_order_depth());
            }
            VisualSlot::ObjectDisc(1) => {
                let visual = runner.component::<ScreenCircleVisual>(entity)?;
                assert_ne!(visual.clip(), ScreenClip::Empty);
                depths[1] = Some(visual.draw_order_depth());
            }
            _ => {}
        }
    }
    let depths = depths.map(Option::unwrap);
    assert!(depths[0] > 0.0 && depths[2] < 3.0);
    assert!(depths[0] < depths[1] && depths[1] < depths[2]);
    Ok(())
}
