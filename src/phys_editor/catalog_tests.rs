//! Real input routing tests for the category dock and bounded object picker.

use std::time::Duration;

use sim_logic::prelude::*;

use super::{
    catalog::{CatalogState, CatalogTarget, Category, EntryId, MAX_QUERY_BYTES, PAGE_SIZE},
    catalog_layout::CatalogLayout,
    document::{Document, LinkKind, ObjectKind, Point},
    input::EditorAction,
    layout::{Layout, Rect},
    placement::Placement,
    state::{Control, EditorState, Mode, Tool},
};

fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}

fn runner() -> LogicResult<HeadlessRunner<EditorAction>> {
    let (app, initial) = crate::build_phys_editor_application()?;
    Ok(app.build_headless(initial)?)
}

fn state(runner: &HeadlessRunner<EditorAction>) -> &EditorState {
    runner.resource::<EditorState>().unwrap()
}

fn frame_at(
    runner: &mut HeadlessRunner<EditorAction>,
    viewport: LogicalViewport,
    events: &[InputEvent],
) -> LogicResult {
    match runner.advance_frame(FrameRequest::new(
        Duration::from_millis(16),
        events,
        viewport,
    )) {
        FrameOutcome::Advanced(report) => {
            assert!(report.failure().is_none(), "{:?}", report.failure());
            assert_eq!(report.fixed_ticks_attempted(), 0);
            assert_eq!(report.spawned(), 0);
            assert_eq!(report.despawned(), 0);
            Ok(())
        }
        FrameOutcome::Rejected(error) => Err(error.into()),
    }
}

fn frame(runner: &mut HeadlessRunner<EditorAction>, events: &[InputEvent]) -> LogicResult {
    frame_at(runner, viewport(), events)
}

fn settle(runner: &mut HeadlessRunner<EditorAction>) -> LogicResult {
    for _ in 0..32 {
        frame(runner, &[])?;
    }
    Ok(())
}

fn center(rect: Rect) -> LogicalScreenPosition {
    LogicalScreenPosition::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn motion(point: LogicalScreenPosition, viewport: LogicalViewport) -> InputEvent {
    InputEvent::pointer_moved(PointerSample::new(point, viewport).unwrap())
}

fn press() -> InputEvent {
    InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed)
}

fn release() -> InputEvent {
    InputEvent::mouse_button(MouseButton::Left, ButtonState::Released)
}

fn click_at(
    runner: &mut HeadlessRunner<EditorAction>,
    point: LogicalScreenPosition,
) -> LogicResult {
    frame(runner, &[motion(point, viewport()), press(), release()])
}

fn target_point(
    runner: &HeadlessRunner<EditorAction>,
    target: CatalogTarget,
) -> LogicalScreenPosition {
    let state = state(runner);
    let layout = Layout::for_state(viewport(), state);
    center(
        CatalogLayout::new(&layout, &state.catalog)
            .target(target, &state.catalog)
            .unwrap(),
    )
}

fn click_target(runner: &mut HeadlessRunner<EditorAction>, target: CatalogTarget) -> LogicResult {
    click_at(runner, target_point(runner, target))
}

fn world_point(runner: &HeadlessRunner<EditorAction>, point: Point) -> LogicalScreenPosition {
    let state = state(runner);
    state
        .camera
        .project(point, Layout::for_state(viewport(), state).canvas)
}

fn key(key: PhysicalKeyCode) -> [InputEvent; 2] {
    [
        InputEvent::key(key, ButtonState::Pressed),
        InputEvent::key(key, ButtonState::Released),
    ]
}

fn shortcut(key: PhysicalKeyCode) -> [InputEvent; 4] {
    let [down, up] = self::key(key);
    [
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Pressed),
        down,
        up,
        InputEvent::key(PhysicalKeyCode::ControlLeft, ButtonState::Released),
    ]
}

fn open(runner: &mut HeadlessRunner<EditorAction>, category: Category) -> LogicResult {
    if !state(runner).catalog.open {
        click_target(runner, CatalogTarget::Open)?;
        settle(runner)?;
    }
    click_target(runner, CatalogTarget::Category(category))?;
    assert!(state(runner).catalog.open);
    assert_eq!(state(runner).catalog.category, category);
    settle(runner)?;
    assert_eq!(state(runner).catalog.blend, 1.0);
    Ok(())
}

fn assert_document(actual: &Document, expected: &Document) {
    assert_eq!(actual.objects(), expected.objects());
    assert_eq!(actual.links(), expected.links());
    assert_eq!(actual.environment(), expected.environment());
    assert_eq!(actual.can_undo(), expected.can_undo());
    assert_eq!(actual.can_redo(), expected.can_redo());
}

fn pools(runner: &HeadlessRunner<EditorAction>) -> [usize; 4] {
    [
        runner.components::<ScreenRectangleVisual>().count(),
        runner.components::<ScreenCircleVisual>().count(),
        runner.components::<ScreenLineVisual>().count(),
        runner.components::<ScreenTextVisual>().count(),
    ]
}

#[test]
fn full_catalog_open_category_switch_and_close_do_not_touch_authoring() -> LogicResult {
    let mut runner = runner()?;
    let document = state(&runner).document.clone();
    open(&mut runner, Category::Bodies)?;
    click_target(&mut runner, CatalogTarget::Category(Category::Connections))?;
    assert!(state(&runner).catalog.open);
    assert_eq!(state(&runner).catalog.category, Category::Connections);
    click_target(&mut runner, CatalogTarget::Category(Category::Connections))?;
    assert!(state(&runner).catalog.open);
    click_target(&mut runner, CatalogTarget::Close)?;
    assert!(!state(&runner).catalog.open);
    assert_document(&state(&runner).document, &document);
    assert!(state(&runner).catalog.pressed.is_none());
    Ok(())
}

#[test]
fn primitive_entry_click_arms_placement_without_inserting_an_object() -> LogicResult {
    let mut runner = runner()?;
    open(&mut runner, Category::Bodies)?;
    click_target(&mut runner, CatalogTarget::Entry(EntryId::Box))?;
    assert!(!state(&runner).catalog.open);
    assert_eq!(state(&runner).brush, Placement::Primitive(ObjectKind::Box));
    assert_eq!(state(&runner).tool, Tool::Build);
    assert!(state(&runner).document.objects().is_empty());
    assert!(state(&runner).drag.is_none());
    let point = world_point(&runner, Point::new(8.0, 2.0));
    click_at(&mut runner, point)?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    assert_eq!(state(&runner).document.objects()[0].kind, ObjectKind::Box);
    assert_eq!(
        state(&runner).document.objects()[0].position,
        Point::new(8.0, 2.0)
    );
    Ok(())
}

#[test]
fn assembly_card_places_a_complete_graph_at_the_chosen_anchor_as_one_edit() -> LogicResult {
    for (entry, placement, spring) in [
        (EntryId::Pendulum, Placement::Pendulum, false),
        (EntryId::Oscillator, Placement::Oscillator, true),
    ] {
        let mut runner = runner()?;
        open(&mut runner, Category::Assemblies)?;
        click_target(&mut runner, CatalogTarget::Entry(entry))?;
        assert_eq!(state(&runner).brush, placement);
        assert!(state(&runner).document.objects().is_empty());
        settle(&mut runner)?;
        let origin = Point::new(4.0, 2.0);
        let point = world_point(&runner, origin);
        click_at(&mut runner, point)?;
        let document = &state(&runner).document;
        assert_eq!(document.objects().len(), 2);
        assert_eq!(document.objects()[0].kind, ObjectKind::Anchor);
        assert_eq!(document.objects()[0].position, origin);
        assert_eq!(document.objects()[1].kind, ObjectKind::Ball);
        assert_eq!(document.links().len(), 1);
        assert_eq!(
            matches!(document.links()[0].kind, LinkKind::Spring { .. }),
            spring
        );
        frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
        assert!(state(&runner).document.objects().is_empty());
        assert!(state(&runner).document.links().is_empty());
        assert!(!state(&runner).document.can_undo());
        frame(&mut runner, &shortcut(PhysicalKeyCode::KeyY))?;
        assert_eq!(state(&runner).document.objects().len(), 2);
    }
    Ok(())
}

#[test]
fn entry_drag_commits_once_and_an_assembly_drag_is_one_undo_transaction() -> LogicResult {
    for (category, entry, count) in [
        (Category::Bodies, EntryId::Ball, 1),
        (Category::Assemblies, EntryId::Pendulum, 2),
    ] {
        let mut runner = runner()?;
        open(&mut runner, category)?;
        let card = target_point(&runner, CatalogTarget::Entry(entry));
        let destination = world_point(&runner, Point::new(8.0, 2.0));
        let layout = Layout::for_state(viewport(), state(&runner));
        assert!(
            !CatalogLayout::new(&layout, &state(&runner).catalog)
                .panel
                .contains(destination)
        );
        frame(&mut runner, &[motion(card, viewport()), press()])?;
        assert!(state(&runner).drag.is_some());
        assert!(state(&runner).document.objects().is_empty());
        frame(&mut runner, &[motion(destination, viewport())])?;
        assert!(state(&runner).document.objects().is_empty());
        frame(&mut runner, &[release()])?;
        assert_eq!(state(&runner).document.objects().len(), count);
        assert!(!state(&runner).catalog.open);
        assert!(state(&runner).drag.is_none());
        frame(&mut runner, &[release()])?;
        assert_eq!(state(&runner).document.objects().len(), count);
        frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
        assert!(state(&runner).document.objects().is_empty());
        assert!(state(&runner).document.links().is_empty());
        assert!(!state(&runner).document.can_undo());
    }
    Ok(())
}

#[test]
fn focused_search_consumes_editor_shortcuts_including_digits_and_modifiers() -> LogicResult {
    let mut runner = runner()?;
    let point = world_point(&runner, Point::default());
    click_at(&mut runner, point)?;
    let document = state(&runner).document.clone();
    let tool = state(&runner).tool;
    let snap = state(&runner).snap;
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyF))?;
    assert!(state(&runner).catalog.open && state(&runner).catalog.search_focused);
    for key_code in [
        PhysicalKeyCode::Digit1,
        PhysicalKeyCode::KeyE,
        PhysicalKeyCode::Delete,
        PhysicalKeyCode::Space,
        PhysicalKeyCode::KeyG,
        PhysicalKeyCode::F5,
    ] {
        frame(&mut runner, &key(key_code))?;
    }
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyD))?;
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyZ))?;
    assert_eq!(state(&runner).catalog.query, "1e g");
    assert_eq!(state(&runner).tool, tool);
    assert_eq!(state(&runner).snap, snap);
    assert!(!state(&runner).environment_open);
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert_document(&state(&runner).document, &document);
    frame(&mut runner, &key(PhysicalKeyCode::Backspace))?;
    assert_eq!(state(&runner).catalog.query, "1e ");
    frame(&mut runner, &shortcut(PhysicalKeyCode::Backspace))?;
    assert!(state(&runner).catalog.query.is_empty());
    frame(&mut runner, &key(PhysicalKeyCode::Escape))?;
    assert!(!state(&runner).catalog.open);
    assert!(!state(&runner).catalog.search_focused);
    assert_document(&state(&runner).document, &document);
    Ok(())
}

#[test]
fn search_filters_real_cards_caps_input_and_no_results_cannot_select() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyF))?;
    for key_code in [
        PhysicalKeyCode::KeyB,
        PhysicalKeyCode::KeyO,
        PhysicalKeyCode::KeyX,
    ] {
        frame(&mut runner, &key(key_code))?;
    }
    assert_eq!(state(&runner).catalog.visible(), vec![EntryId::Box]);
    frame(&mut runner, &key(PhysicalKeyCode::Enter))?;
    assert_eq!(state(&runner).brush, Placement::Primitive(ObjectKind::Box));
    assert!(state(&runner).document.objects().is_empty());
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyF))?;
    frame(&mut runner, &shortcut(PhysicalKeyCode::Backspace))?;
    for _ in 0..MAX_QUERY_BYTES + 2 {
        frame(&mut runner, &key(PhysicalKeyCode::KeyQ))?;
    }
    assert_eq!(state(&runner).catalog.query.len(), MAX_QUERY_BYTES);
    assert!(state(&runner).catalog.visible().is_empty());
    let brush = state(&runner).brush;
    frame(&mut runner, &key(PhysicalKeyCode::Enter))?;
    assert!(state(&runner).catalog.open);
    assert_eq!(state(&runner).brush, brush);
    assert!(state(&runner).document.objects().is_empty());
    Ok(())
}

#[test]
fn outside_click_dismisses_without_placing_and_only_the_next_click_edits() -> LogicResult {
    let mut runner = runner()?;
    open(&mut runner, Category::Bodies)?;
    let destination = world_point(&runner, Point::new(8.0, 2.0));
    click_at(&mut runner, destination)?;
    assert!(!state(&runner).catalog.open);
    assert!(state(&runner).document.objects().is_empty());
    click_at(&mut runner, destination)?;
    assert_eq!(state(&runner).document.objects().len(), 1);
    Ok(())
}

#[test]
fn focus_loss_and_resize_cancel_held_entries_before_a_late_canvas_release() -> LogicResult {
    for resize in [false, true] {
        let mut runner = runner()?;
        open(&mut runner, Category::Bodies)?;
        let card = target_point(&runner, CatalogTarget::Entry(EntryId::Box));
        frame(&mut runner, &[motion(card, viewport()), press()])?;
        assert!(state(&runner).drag.is_some());
        let current = if resize {
            LogicalViewport::new(1000.0, 700.0)?
        } else {
            viewport()
        };
        let events = if resize {
            vec![]
        } else {
            vec![InputEvent::FocusLost]
        };
        frame_at(&mut runner, current, &events)?;
        assert!(state(&runner).catalog.pressed.is_none());
        assert!(state(&runner).drag.is_none());
        let layout = Layout::for_state(current, state(&runner));
        let destination = LogicalScreenPosition::new(
            layout.canvas.x + layout.canvas.width - 8.0,
            layout.canvas.y + 8.0,
        );
        frame_at(
            &mut runner,
            current,
            &[motion(destination, current), release()],
        )?;
        assert!(state(&runner).document.objects().is_empty());
        assert_eq!(state(&runner).brush, Placement::Primitive(ObjectKind::Ball));
    }
    Ok(())
}

#[test]
fn scrolling_a_popup_never_zooms_or_moves_the_canvas_camera() -> LogicResult {
    let mut runner = runner()?;
    open(&mut runner, Category::Bodies)?;
    let position = target_point(&runner, CatalogTarget::Entry(EntryId::Ball));
    let camera = state(&runner).camera;
    for scroll in [-3.0, 4.0, -100.0] {
        frame(
            &mut runner,
            &[
                motion(position, viewport()),
                InputEvent::mouse_wheel(ScrollDelta::lines(0.0, scroll)?),
            ],
        )?;
        assert_eq!(state(&runner).camera.center, camera.center);
        assert_eq!(state(&runner).camera.pixels_per_m, camera.pixels_per_m);
        assert_eq!(state(&runner).catalog.page, 0);
    }
    Ok(())
}

#[test]
fn empty_actuators_do_not_duplicate_the_real_environment_panel() -> LogicResult {
    let mut runner = runner()?;
    open(&mut runner, Category::Actuators)?;
    assert!(state(&runner).catalog.visible().is_empty());
    assert!(!state(&runner).environment_open);
    frame(&mut runner, &key(PhysicalKeyCode::Enter))?;
    assert!(state(&runner).catalog.open);
    assert!(!state(&runner).environment_open);
    click_target(&mut runner, CatalogTarget::Close)?;
    let layout = Layout::for_state(viewport(), state(&runner));
    click_at(
        &mut runner,
        center(layout.control(Control::Environment).unwrap()),
    )?;
    assert!(state(&runner).environment_open);
    assert!(!state(&runner).catalog.open);
    assert!(state(&runner).document.objects().is_empty());
    frame(&mut runner, &shortcut(PhysicalKeyCode::KeyF))?;
    assert!(!state(&runner).catalog.open);
    let opener = target_point(&runner, CatalogTarget::Open);
    click_at(&mut runner, opener)?;
    assert!(!state(&runner).catalog.open);
    assert!(state(&runner).environment_open);
    let layout = Layout::for_state(viewport(), state(&runner));
    click_at(
        &mut runner,
        center(layout.control(Control::CloseEnvironment).unwrap()),
    )?;
    assert!(!state(&runner).environment_open);
    assert!(state(&runner).document.objects().is_empty());
    Ok(())
}

#[test]
fn same_frame_entry_choice_or_close_does_not_activate_actions_behind_the_picker() -> LogicResult {
    let mut runner = runner()?;
    let initial_point = world_point(&runner, Point::default());
    click_at(&mut runner, initial_point)?;
    let document = state(&runner).document.clone();
    open(&mut runner, Category::Bodies)?;
    let card = target_point(&runner, CatalogTarget::Entry(EntryId::Box));
    let [delete, delete_up] = key(PhysicalKeyCode::Delete);
    let [run, run_up] = key(PhysicalKeyCode::F5);
    frame(
        &mut runner,
        &[
            motion(card, viewport()),
            press(),
            release(),
            delete,
            delete_up,
            run,
            run_up,
        ],
    )?;
    assert!(!state(&runner).catalog.open);
    assert_eq!(state(&runner).mode, Mode::Editor);
    assert_document(&state(&runner).document, &document);
    open(&mut runner, Category::Bodies)?;
    let [escape, escape_up] = key(PhysicalKeyCode::Escape);
    let destination = world_point(&runner, Point::new(8.0, 2.0));
    frame(
        &mut runner,
        &[
            escape,
            escape_up,
            motion(destination, viewport()),
            press(),
            release(),
        ],
    )?;
    assert!(!state(&runner).catalog.open);
    assert_document(&state(&runner).document, &document);
    Ok(())
}

#[test]
fn opening_every_category_and_searching_reuses_the_retained_visual_pool() -> LogicResult {
    let mut runner = runner()?;
    let initial = pools(&runner);
    assert!(initial.iter().all(|count| *count > 0));
    for category in Category::ALL {
        open(&mut runner, category)?;
        assert_eq!(pools(&runner), initial);
        assert!(state(&runner).catalog.visible().len() <= PAGE_SIZE);
        click_target(&mut runner, CatalogTarget::Search)?;
        frame(&mut runner, &key(PhysicalKeyCode::KeyQ))?;
        assert_eq!(pools(&runner), initial);
        frame(&mut runner, &key(PhysicalKeyCode::Escape))?;
        assert!(state(&runner).document.objects().is_empty());
    }
    Ok(())
}

#[test]
fn quick_dock_choices_arm_placement_and_favorites_reorder_without_scene_edits() -> LogicResult {
    let mut runner = runner()?;
    let document = state(&runner).document.clone();
    click_target(&mut runner, CatalogTarget::Quick(EntryId::Box))?;
    assert!(!state(&runner).catalog.open);
    assert_eq!(state(&runner).brush, Placement::Primitive(ObjectKind::Box));
    assert_eq!(state(&runner).catalog.recent[0], EntryId::Box);
    assert_document(&state(&runner).document, &document);
    open(&mut runner, Category::Assemblies)?;
    click_target(&mut runner, CatalogTarget::Favorite(EntryId::Pendulum))?;
    assert!(state(&runner).catalog.open);
    assert_eq!(state(&runner).catalog.favorites, vec![EntryId::Pendulum]);
    assert_eq!(state(&runner).catalog.quick_entries()[0], EntryId::Pendulum);
    assert_eq!(state(&runner).brush, Placement::Primitive(ObjectKind::Box));
    assert_document(&state(&runner).document, &document);
    click_target(&mut runner, CatalogTarget::Close)?;
    click_target(&mut runner, CatalogTarget::Quick(EntryId::Pendulum))?;
    assert_eq!(state(&runner).brush, Placement::Pendulum);
    assert_document(&state(&runner).document, &document);
    Ok(())
}

#[test]
fn all_catalog_shortcuts_open_focused_global_search_without_leaking_commands() -> LogicResult {
    let mut runner = runner()?;
    for events in [
        key(PhysicalKeyCode::Slash).to_vec(),
        shortcut(PhysicalKeyCode::KeyK).to_vec(),
        shortcut(PhysicalKeyCode::KeyF).to_vec(),
    ] {
        frame(&mut runner, &events)?;
        assert!(state(&runner).catalog.open && state(&runner).catalog.search_focused);
        assert!(state(&runner).catalog.query.is_empty());
        assert!(state(&runner).document.objects().is_empty());
        frame(&mut runner, &key(PhysicalKeyCode::Escape))?;
        assert!(!state(&runner).catalog.open);
    }
    open(&mut runner, Category::Measure)?;
    assert!(state(&runner).catalog.visible().is_empty());
    click_target(&mut runner, CatalogTarget::Search)?;
    for character in [
        PhysicalKeyCode::KeyP,
        PhysicalKeyCode::KeyN,
        PhysicalKeyCode::KeyD,
        PhysicalKeyCode::KeyL,
        PhysicalKeyCode::KeyM,
    ] {
        frame(&mut runner, &key(character))?;
    }
    assert!(
        state(&runner)
            .catalog
            .visible()
            .contains(&EntryId::Pendulum)
    );
    frame(&mut runner, &key(PhysicalKeyCode::Enter))?;
    assert_eq!(state(&runner).brush, Placement::Pendulum);
    assert!(state(&runner).document.objects().is_empty());
    Ok(())
}

#[test]
fn unavailable_ideal_filter_has_no_hit_target_or_placeholder() -> LogicResult {
    let mut runner = runner()?;
    open(&mut runner, Category::Actuators)?;
    let document = state(&runner).document.clone();
    let layout = Layout::for_state(viewport(), state(&runner));
    let geometry = CatalogLayout::new(&layout, &state(&runner).catalog);
    assert!(
        geometry
            .target(CatalogTarget::IdealFilter, &state(&runner).catalog)
            .is_none()
    );
    assert!(state(&runner).catalog.visible().is_empty());
    click_target(&mut runner, CatalogTarget::Category(Category::Bodies))?;
    assert!(!state(&runner).catalog.ideal_only);
    assert_eq!(state(&runner).catalog.visible().len(), 3);
    assert_document(&state(&runner).document, &document);
    Ok(())
}

#[test]
fn a_filtered_out_keyboard_target_cannot_be_activated_with_enter() -> LogicResult {
    let mut runner = runner()?;
    open(&mut runner, Category::Connections)?;
    frame(&mut runner, &key(PhysicalKeyCode::ArrowDown))?;
    assert_eq!(
        state(&runner).catalog.focused,
        Some(CatalogTarget::Entry(EntryId::Rod))
    );
    click_target(&mut runner, CatalogTarget::Section(2))?;
    assert_eq!(state(&runner).catalog.visible(), vec![EntryId::Spring]);
    let brush = state(&runner).brush;
    frame(&mut runner, &key(PhysicalKeyCode::Enter))?;
    assert!(
        state(&runner).catalog.open,
        "Enter activated a hidden filtered-out entry"
    );
    assert_eq!(state(&runner).brush, brush);
    assert!(state(&runner).catalog.recent.is_empty());
    assert!(state(&runner).document.objects().is_empty());
    Ok(())
}

#[test]
fn catalog_drag_release_consumes_queued_editor_actions_after_closing() -> LogicResult {
    let mut runner = runner()?;
    open(&mut runner, Category::Assemblies)?;
    let card = target_point(&runner, CatalogTarget::Entry(EntryId::Pendulum));
    let destination = world_point(&runner, Point::new(8.0, 2.0));
    frame(&mut runner, &[motion(card, viewport()), press()])?;
    frame(&mut runner, &[motion(destination, viewport())])?;
    let [delete, delete_up] = key(PhysicalKeyCode::Delete);
    let [run, run_up] = key(PhysicalKeyCode::F5);
    frame(&mut runner, &[release(), delete, delete_up, run, run_up])?;
    assert!(!state(&runner).catalog.open);
    assert_eq!(
        state(&runner).mode,
        Mode::Editor,
        "Run leaked through a catalog-closing drag"
    );
    assert_eq!(
        state(&runner).document.objects().len(),
        2,
        "Delete leaked through a catalog-closing drag"
    );
    assert_eq!(state(&runner).document.links().len(), 1);
    Ok(())
}

fn contained(inner: Rect, outer: Rect) -> bool {
    let epsilon = 0.001;
    inner.width > 0.0
        && inner.height > 0.0
        && inner.x + epsilon >= outer.x
        && inner.y + epsilon >= outer.y
        && inner.x + inner.width <= outer.x + outer.width + epsilon
        && inner.y + inner.height <= outer.y + outer.height + epsilon
}

#[test]
fn responsive_catalog_geometry_keeps_targets_visible_scoped_and_pickable() -> LogicResult {
    for (width, height) in [(900.0, 600.0), (1280.0, 800.0), (1920.0, 1080.0)] {
        let viewport = LogicalViewport::new(width, height)?;
        let layout = Layout::new(viewport, Mode::Editor);
        let viewport_rect = Rect::new(0.0, 0.0, width, height);
        for category in Category::ALL {
            for blend in [0.0, 0.5, 1.0] {
                let catalog = CatalogState {
                    open: true,
                    category,
                    blend,
                    ..CatalogState::default()
                };
                let geometry = CatalogLayout::new(&layout, &catalog);
                assert!(contained(geometry.panel, viewport_rect));
                assert!(contained(geometry.search, geometry.panel));
                assert!(contained(geometry.grid, geometry.panel));
                assert!(contained(geometry.rail, geometry.panel));
                for dock_category in Category::ALL {
                    let rect = geometry.category(dock_category);
                    assert!(contained(rect, geometry.rail));
                    assert_eq!(
                        geometry.hit(center(rect), &catalog),
                        Some(CatalogTarget::Category(dock_category))
                    );
                }
                for (index, id) in catalog.visible().into_iter().enumerate() {
                    let card = geometry.card(index).unwrap();
                    assert!(contained(card, geometry.grid));
                    assert_eq!(
                        geometry.hit(center(card), &catalog),
                        Some(CatalogTarget::Entry(id))
                    );
                }
                for index in 0..category.sections().len() {
                    let Some(section) = geometry.target(CatalogTarget::Section(index), &catalog)
                    else {
                        continue;
                    };
                    assert!(contained(section, geometry.rail));
                    assert_eq!(
                        geometry.hit(center(section), &catalog),
                        Some(CatalogTarget::Section(index))
                    );
                }
                for target in [
                    CatalogTarget::Search,
                    CatalogTarget::ClearSearch,
                    CatalogTarget::Close,
                    CatalogTarget::PreviousPage,
                    CatalogTarget::NextPage,
                ] {
                    let rect = geometry.target(target, &catalog).unwrap();
                    assert!(contained(rect, geometry.panel));
                    assert_eq!(geometry.hit(center(rect), &catalog), Some(target));
                }
                assert!(geometry.card(PAGE_SIZE).is_none());
                assert!(contained(geometry.opener(), layout.palette));
                for (index, id) in catalog
                    .quick_entries()
                    .into_iter()
                    .enumerate()
                    .take(geometry.quick_count)
                {
                    let rect = geometry.quick(index).unwrap();
                    assert!(contained(rect, layout.palette));
                    assert_eq!(
                        geometry.hit(center(rect), &catalog),
                        Some(CatalogTarget::Quick(id))
                    );
                }
            }
        }
    }
    Ok(())
}
