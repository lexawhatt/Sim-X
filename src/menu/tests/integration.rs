use super::*;

#[test]
fn tiny_viewports_keep_hidden_geometry_valid_and_show_only_resize_notice() -> LogicResult {
    let mut runner = runner()?;
    for (width, height) in [(1.0, 1.0), (64.0, 64.0), (79.0, 100.0)] {
        advance(&mut runner, LogicalViewport::new(width, height)?, &[])?;
        assert!(
            runner
                .components::<ScreenRectangleVisual>()
                .all(|(_, visual)| { visual.clip() == ScreenClip::Empty })
        );
        assert!(
            runner
                .components::<ScreenImageVisual>()
                .all(|(_, visual)| { visual.clip() == ScreenClip::Empty })
        );
        assert_eq!(state(&runner).overlay, Overlay::None);
    }
    Ok(())
}

fn window(runner: &HeadlessRunner<MenuAction>) -> &WindowControls {
    runner.app_resource::<WindowControls>().unwrap()
}

fn key(key: PhysicalKeyCode, state: ButtonState) -> InputEvent {
    InputEvent::key(key, state)
}

#[test]
fn tab_and_both_shift_keys_follow_physical_edge_order() -> LogicResult {
    for shift in [PhysicalKeyCode::ShiftLeft, PhysicalKeyCode::ShiftRight] {
        let mut runner = runner()?;
        advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
        assert_eq!(state(&runner).focus.focused(), Some(Target::Domains));
        let [tab_press, tab_release] = key_tap(PhysicalKeyCode::Tab);
        advance(
            &mut runner,
            viewport(),
            &[
                key(shift, ButtonState::Pressed),
                tab_press,
                tab_release,
                key(shift, ButtonState::Released),
            ],
        )?;
        assert_eq!(
            state(&runner).focus.focused(),
            Some(Target::Social(SocialLink::Telegram))
        );
        advance(
            &mut runner,
            viewport(),
            &[tab_press, tab_release, key(shift, ButtonState::Pressed)],
        )?;
        assert_eq!(state(&runner).focus.focused(), Some(Target::Domains));
        advance(
            &mut runner,
            viewport(),
            &[key(shift, ButtonState::Released), tab_press, tab_release],
        )?;
        assert_eq!(state(&runner).focus.focused(), Some(Target::Settings));
    }
    Ok(())
}

#[test]
fn releasing_one_shift_preserves_the_other_and_focus_loss_cancels_both() -> LogicResult {
    let mut runner = runner()?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    let [tab_press, tab_release] = key_tap(PhysicalKeyCode::Tab);
    advance(
        &mut runner,
        viewport(),
        &[
            key(PhysicalKeyCode::ShiftLeft, ButtonState::Pressed),
            key(PhysicalKeyCode::ShiftRight, ButtonState::Pressed),
            key(PhysicalKeyCode::ShiftLeft, ButtonState::Released),
            tab_press,
            tab_release,
        ],
    )?;
    assert_eq!(
        state(&runner).focus.focused(),
        Some(Target::Social(SocialLink::Telegram))
    );
    advance(&mut runner, viewport(), &[InputEvent::FocusLost])?;
    assert_eq!(state(&runner).focus.focused(), None);
    assert_eq!(state(&runner).shifts, [false, false]);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::Domains));
    Ok(())
}

#[test]
fn tab_focus_is_trapped_in_the_active_modal() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Settings)?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::ReduceMotion));
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::Close));
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::ReduceMotion));
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert!(state(&runner).reduced_motion);
    assert_eq!(state(&runner).overlay, Overlay::Settings);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    assert_eq!(state(&runner).focus.focused(), None);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    Ok(())
}

#[test]
fn fullscreen_toggle_requests_modes_without_claiming_headless_presentation() -> LogicResult {
    let mut runner = runner()?;
    assert!(state(&runner).fullscreen_requested);
    assert_eq!(
        window(&runner).mode_status(),
        WindowModeStatus::NotSubmitted
    );
    let pressed = key(PhysicalKeyCode::F11, ButtonState::Pressed);
    advance(&mut runner, viewport(), &[pressed, pressed])?;
    assert!(!state(&runner).fullscreen_requested);
    assert_eq!(window(&runner).pending_mode(), Some(WindowMode::Windowed));
    advance(&mut runner, viewport(), &[InputEvent::FocusLost])?;
    assert!(!state(&runner).fullscreen_requested);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::F11))?;
    assert!(state(&runner).fullscreen_requested);
    assert_eq!(
        window(&runner).pending_mode(),
        Some(WindowMode::BorderlessFullscreen(
            FullscreenMonitor::Automatic
        ))
    );
    assert_eq!(
        window(&runner).mode_status(),
        WindowModeStatus::NotSubmitted
    );
    assert_eq!(state(&runner).overlay, Overlay::None);
    Ok(())
}

#[test]
fn cursor_requests_follow_eligible_pointer_targets_and_current_viewport() -> LogicResult {
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &[over(Target::Domains, Overlay::None, viewport())],
    )?;
    assert_eq!(window(&runner).pending_cursor(), Some(CursorShape::Pointer));
    advance(&mut runner, LogicalViewport::new(320.0, 480.0)?, &[])?;
    assert_eq!(window(&runner).pending_cursor(), Some(CursorShape::Default));
    click(&mut runner, Target::Settings)?;
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Social(SocialLink::Github),
            Overlay::None,
            viewport(),
        )],
    )?;
    assert_eq!(window(&runner).pending_cursor(), Some(CursorShape::Default));
    advance(
        &mut runner,
        viewport(),
        &[over(Target::ReduceMotion, Overlay::Settings, viewport())],
    )?;
    assert_eq!(window(&runner).pending_cursor(), Some(CursorShape::Pointer));
    advance(&mut runner, viewport(), &[InputEvent::FocusLost])?;
    assert_eq!(window(&runner).pending_cursor(), Some(CursorShape::Default));
    Ok(())
}

#[test]
fn resize_recomputes_hover_and_cursor_from_current_layout_without_mouse_motion() -> LogicResult {
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &[over(Target::Domains, Overlay::None, viewport())],
    )?;
    assert_eq!(state(&runner).hovered, Some(Target::Domains));
    advance(&mut runner, LogicalViewport::new(1920.0, 1080.0)?, &[])?;
    assert_eq!(state(&runner).hovered, None);
    assert_eq!(window(&runner).pending_cursor(), Some(CursorShape::Default));
    advance(&mut runner, viewport(), &[])?;
    assert_eq!(state(&runner).hovered, Some(Target::Domains));
    assert_eq!(window(&runner).pending_cursor(), Some(CursorShape::Pointer));
    assert_eq!(state(&runner).overlay, Overlay::None);
    Ok(())
}

#[test]
fn narrow_focus_indicator_avoids_rounded_tessellation() -> LogicResult {
    let mut runner = runner()?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Tab))?;
    let (entity, _) = runner
        .components::<crate::menu::view::Panel>()
        .find(|(_, panel)| matches!(panel, crate::menu::view::Panel::Focus(Target::Domains)))
        .ok_or("Domains focus indicator missing")?;
    let visual = runner.component::<ScreenRectangleVisual>(entity)?;
    assert!(visual.color().alpha() > 0.0);
    assert!(visual.size().to_vec2().x() <= 2.0);
    assert_eq!(visual.corner_radius(), 0.0);
    Ok(())
}

#[test]
fn screen_animation_and_resize_update_immediately_while_fixed_simulation_is_paused() -> LogicResult
{
    let mut runner = runner()?;
    runner.set_paused(true);
    let centers = |runner: &HeadlessRunner<MenuAction>| {
        runner
            .components::<ScreenCircleVisual>()
            .map(|(entity, circle)| (entity, circle.center()))
            .collect::<Vec<_>>()
    };
    let (physics_image, _) = runner
        .components::<backdrop::BackdropImage>()
        .next()
        .ok_or("Physics orbit image missing")?;
    advance(&mut runner, viewport(), &[])?;
    let initial = centers(&runner);
    let initial_rotation = runner
        .component::<ScreenImageVisual>(physics_image)?
        .rotation();
    let report = advance(&mut runner, viewport(), &[])?;
    assert_eq!(report.fixed_ticks_attempted(), 0);
    assert_ne!(centers(&runner), initial);
    assert!(
        runner
            .component::<ScreenImageVisual>(physics_image)?
            .rotation()
            > initial_rotation
    );
    for (width, height) in [(360.0, 560.0), (1920.0, 1080.0), (1280.0, 720.0)] {
        let viewport = LogicalViewport::new(width, height)?;
        let report = advance(&mut runner, viewport, &[])?;
        assert_eq!(report.fixed_ticks_attempted(), 0);
        assert_eq!(runner.components::<Transform2d>().count(), 0);
        assert_eq!(runner.components::<LineVisual>().count(), 0);
        assert_eq!(runner.components::<CircleVisual>().count(), 0);
        let placement = Layout::new(viewport).backdrop(0.0);
        let orbit = runner.component::<ScreenImageVisual>(physics_image)?;
        if !Layout::new(viewport).compact {
            assert_eq!(orbit.clip(), ScreenClip::Unclipped);
            assert_eq!(orbit.position(), placement.position());
            assert_eq!(orbit.size(), placement.size());
        }
        for (_, circle) in runner.components::<ScreenCircleVisual>() {
            if circle.clip() == ScreenClip::Empty {
                continue;
            }
            let center = circle.center().to_vec2();
            assert!(center.x() >= placement.x && center.x() <= placement.x + placement.width);
            assert!(center.y() >= placement.y && center.y() <= placement.y + placement.height);
        }
        let frame = runner.extracted_frame().unwrap();
        assert!(frame.resolved_lines().is_empty());
        assert!(frame.resolved_circles().is_empty());
        assert_eq!(
            runner.components::<ScreenLineVisual>().count(),
            backdrop::LINE_COUNT + crate::menu::physics_backdrop::LINE_COUNT
        );
        assert_eq!(
            runner.components::<ScreenCircleVisual>().count(),
            backdrop::CIRCLE_COUNT + crate::menu::physics_backdrop::CIRCLE_COUNT
        );
    }
    Ok(())
}
