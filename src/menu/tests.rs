use std::time::Duration;

use sim_logic::prelude::*;

use super::{
    assets::MenuAssets,
    backdrop,
    input::MenuAction,
    layout::Layout,
    state::{MenuState, Overlay, SocialLink, Target},
};

mod carousel;
mod domains;
mod integration;
mod navigation;
mod workspaces;

fn runner() -> LogicResult<HeadlessRunner<MenuAction>> {
    let (application, initial) = crate::build_application()?;
    Ok(application.build_headless(initial)?)
}

fn viewport() -> LogicalViewport {
    LogicalViewport::new(1280.0, 800.0).unwrap()
}

fn advance(
    runner: &mut HeadlessRunner<MenuAction>,
    viewport: LogicalViewport,
    events: &[InputEvent],
) -> LogicResult<LogicFrameReport> {
    let request = FrameRequest::new(Duration::from_millis(16), events, viewport);
    match runner.advance_frame(request) {
        FrameOutcome::Advanced(report) => {
            assert!(report.failure().is_none(), "{:?}", report.failure());
            assert_eq!(report.spawned(), 0);
            assert_eq!(report.despawned(), 0);
            Ok(report)
        }
        FrameOutcome::Rejected(error) => Err(error.into()),
    }
}

fn state(runner: &HeadlessRunner<MenuAction>) -> &MenuState {
    runner.resource::<MenuState>().unwrap()
}

fn motion(position: LogicalScreenPosition, viewport: LogicalViewport) -> InputEvent {
    InputEvent::pointer_moved(PointerSample::new(position, viewport).unwrap())
}

fn over(target: Target, overlay: Overlay, viewport: LogicalViewport) -> InputEvent {
    let rect = Layout::new(viewport).target(target, overlay);
    motion(
        LogicalScreenPosition::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5),
        viewport,
    )
}

fn press() -> InputEvent {
    InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed)
}

fn release() -> InputEvent {
    InputEvent::mouse_button(MouseButton::Left, ButtonState::Released)
}

fn key_tap(key: PhysicalKeyCode) -> [InputEvent; 2] {
    [
        InputEvent::key(key, ButtonState::Pressed),
        InputEvent::key(key, ButtonState::Released),
    ]
}

fn click(runner: &mut HeadlessRunner<MenuAction>, target: Target) -> LogicResult<LogicFrameReport> {
    let pointer = over(target, state(runner).overlay, viewport());
    advance(runner, viewport(), &[pointer, press(), release()])
}

#[test]
fn pointer_activation_waits_for_matching_release() -> LogicResult {
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &[over(Target::Domains, Overlay::None, viewport()), press()],
    )?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    assert_eq!(state(&runner).pointer.captured(), Some(Target::Domains));
    advance(&mut runner, viewport(), &[release()])?;
    assert_eq!(state(&runner).overlay, Overlay::Domains);
    assert_eq!(state(&runner).pointer.captured(), None);
    advance(&mut runner, viewport(), &[release()])?;
    assert_eq!(state(&runner).overlay, Overlay::Domains);
    Ok(())
}

#[test]
fn outside_press_and_release_over_different_target_cannot_activate() -> LogicResult {
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &[
            motion(LogicalScreenPosition::new(2.0, 2.0), viewport()),
            press(),
            over(Target::Domains, Overlay::None, viewport()),
            release(),
        ],
    )?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    advance(
        &mut runner,
        viewport(),
        &[
            press(),
            over(Target::Settings, Overlay::None, viewport()),
            release(),
        ],
    )?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    assert_eq!(state(&runner).pointer.captured(), None);
    click(&mut runner, Target::Settings)?;
    assert_eq!(state(&runner).overlay, Overlay::Settings);
    Ok(())
}

#[test]
fn focus_loss_and_pointer_leave_cancel_held_gestures() -> LogicResult {
    for cancellation in [InputEvent::FocusLost, InputEvent::PointerLeft] {
        let mut runner = runner()?;
        advance(
            &mut runner,
            viewport(),
            &[over(Target::Domains, Overlay::None, viewport()), press()],
        )?;
        advance(&mut runner, viewport(), &[cancellation])?;
        assert_eq!(state(&runner).pointer.captured(), None);
        advance(
            &mut runner,
            viewport(),
            &[over(Target::Domains, Overlay::None, viewport()), release()],
        )?;
        assert_eq!(state(&runner).overlay, Overlay::None);
        click(&mut runner, Target::Domains)?;
        assert_eq!(state(&runner).overlay, Overlay::Domains);
    }
    Ok(())
}

#[test]
fn modal_blocks_background_actions_and_links() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Settings)?;
    for target in [
        Target::Domains,
        Target::Social(SocialLink::Github),
        Target::Social(SocialLink::Youtube),
        Target::Social(SocialLink::Telegram),
    ] {
        // Resolve the obscured menu geometry deliberately, not modal controls.
        advance(
            &mut runner,
            viewport(),
            &[over(target, Overlay::None, viewport()), press(), release()],
        )?;
        assert_eq!(state(&runner).overlay, Overlay::Settings);
        assert_eq!(state(&runner).pending_link, None);
    }
    Ok(())
}

#[test]
fn closing_modal_does_not_pass_remaining_batch_to_background() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Settings)?;
    let [escape_press, escape_release] = key_tap(PhysicalKeyCode::Escape);
    advance(
        &mut runner,
        viewport(),
        &[
            escape_press,
            escape_release,
            over(
                Target::Social(SocialLink::Github),
                Overlay::None,
                viewport(),
            ),
            press(),
            release(),
        ],
    )?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    assert_eq!(state(&runner).pending_link, None);
    click(&mut runner, Target::Social(SocialLink::Github))?;
    assert_eq!(state(&runner).pending_link, Some(SocialLink::Github));
    Ok(())
}

#[test]
fn changing_modal_cancels_previous_mouse_capture() -> LogicResult {
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &[over(Target::Settings, Overlay::None, viewport()), press()],
    )?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    assert_eq!(state(&runner).overlay, Overlay::Quit);
    assert_eq!(state(&runner).pointer.captured(), None);
    let report = advance(
        &mut runner,
        viewport(),
        &[
            over(Target::ConfirmQuit, Overlay::Quit, viewport()),
            release(),
        ],
    )?;
    assert!(!report.exit_requested());
    assert_eq!(state(&runner).overlay, Overlay::Quit);
    Ok(())
}

#[test]
fn arrow_enter_escape_navigation_requires_explicit_quit_confirmation() -> LogicResult {
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowDown),
    )?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::Domains));
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowRight),
    )?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::Settings));
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert_eq!(state(&runner).overlay, Overlay::Settings);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    assert_eq!(state(&runner).overlay, Overlay::None);
    let report = advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    assert!(!report.exit_requested());
    assert_eq!(state(&runner).overlay, Overlay::Quit);
    assert_eq!(state(&runner).focus.focused(), Some(Target::Close));
    let report = advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert!(!report.exit_requested());
    assert_eq!(state(&runner).overlay, Overlay::None);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowRight),
    )?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::ConfirmQuit));
    let report = advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert!(report.exit_requested());
    assert!(state(&runner).departing);
    Ok(())
}

#[test]
fn social_clicks_queue_only_the_selected_allowlisted_destination() -> LogicResult {
    let mut runner = runner()?;
    for link in [
        SocialLink::Github,
        SocialLink::Youtube,
        SocialLink::Telegram,
    ] {
        click(&mut runner, Target::Social(link))?;
        assert_eq!(state(&runner).pending_link, Some(link));
        assert_eq!(state(&runner).overlay, Overlay::None);
    }
    // The native link opener is not installed by build_application().
    Ok(())
}

#[test]
fn queued_mouse_edges_use_their_positions_instead_of_final_hover() -> LogicResult {
    let mut runner = runner()?;
    advance(
        &mut runner,
        viewport(),
        &[
            over(Target::Domains, Overlay::None, viewport()),
            press(),
            release(),
            over(Target::Settings, Overlay::None, viewport()),
        ],
    )?;
    assert_eq!(state(&runner).overlay, Overlay::Domains);
    Ok(())
}

#[test]
fn queued_pointer_viewport_survives_resize_before_frame_delivery() -> LogicResult {
    for (pointer_size, frame_size) in [
        ((360.0, 560.0), (1920.0, 1080.0)),
        ((1920.0, 1080.0), (360.0, 560.0)),
    ] {
        let mut runner = runner()?;
        let pointer_viewport = LogicalViewport::new(pointer_size.0, pointer_size.1)?;
        let frame_viewport = LogicalViewport::new(frame_size.0, frame_size.1)?;
        advance(
            &mut runner,
            frame_viewport,
            &[
                over(Target::Settings, Overlay::None, pointer_viewport),
                press(),
                release(),
            ],
        )?;
        assert_eq!(state(&runner).overlay, Overlay::Settings);
    }
    Ok(())
}

#[test]
fn reduced_motion_snaps_feedback_and_freezes_ambient_phase() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Settings)?;
    let phase = state(&runner).phase;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Space))?;
    assert!(state(&runner).reduced_motion);
    assert_eq!(state(&runner).phase, phase);
    assert_eq!(state(&runner).emphasis[Target::ReduceMotion.index()], 1.0);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    advance(
        &mut runner,
        viewport(),
        &[over(Target::Domains, Overlay::None, viewport())],
    )?;
    assert_eq!(state(&runner).emphasis[Target::Domains.index()], 1.0);
    for _ in 0..60 {
        advance(&mut runner, viewport(), &[])?;
    }
    assert_eq!(state(&runner).phase, phase);
    click(&mut runner, Target::Settings)?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Space))?;
    assert!(!state(&runner).reduced_motion);
    assert_ne!(state(&runner).phase, phase);
    Ok(())
}

#[test]
fn resize_and_repeated_modal_use_retain_bounded_real_text_and_images() -> LogicResult {
    let mut runner = runner()?;
    assert_eq!(runner.text_font_count(), 6);
    assert_eq!(runner.text_font_bytes(), MenuAssets::font_bytes());
    assert_eq!(runner.image_asset_count(), 5);
    assert!(runner.image_pixel_bytes() <= 5 * 1024 * 1024);
    let image_bytes = runner.image_pixel_bytes();
    let font_bytes = runner.text_font_bytes();
    let original_labels = runner.components::<ScreenTextVisual>().count();
    for _ in 0..3 {
        for (width, height) in [(360.0, 560.0), (1280.0, 720.0), (1920.0, 1080.0)] {
            let viewport = LogicalViewport::new(width, height)?;
            for overlay_target in [Target::Settings, Target::Domains, Target::Quit] {
                advance(
                    &mut runner,
                    viewport,
                    &[
                        over(overlay_target, Overlay::None, viewport),
                        press(),
                        release(),
                    ],
                )?;
                let snapshot = runner.extracted_frame().unwrap();
                assert!(snapshot.resolved_circles().is_empty());
                assert!(snapshot.resolved_rectangles().is_empty());
                assert!(snapshot.resolved_lines().is_empty());
                assert_eq!(runner.components::<Transform2d>().count(), 0);
                assert!(
                    snapshot.screen_primitives().count()
                        <= backdrop::ENTITY_COUNT
                            + crate::menu::physics_backdrop::ENTITY_COUNT
                            + 128
                );
                assert_eq!(
                    runner.components::<ScreenCircleVisual>().count(),
                    backdrop::CIRCLE_COUNT + crate::menu::physics_backdrop::CIRCLE_COUNT
                );
                assert_eq!(
                    runner.components::<ScreenLineVisual>().count(),
                    backdrop::LINE_COUNT + crate::menu::physics_backdrop::LINE_COUNT
                );
                assert!(
                    snapshot.resolved_screen_images().len()
                        <= 5 + crate::menu::physics_backdrop::IMAGE_COUNT
                );
                assert!(snapshot.resolved_screen_rectangles().len() <= 64);
                assert!(snapshot.resolved_screen_texts().len() <= 64);
                for text in snapshot.resolved_screen_texts() {
                    if text.tint().alpha() > 0.0 && !text.text().is_empty() {
                        let baseline = text.baseline_origin().to_vec2();
                        assert!(
                            baseline.x() >= 0.0 && baseline.x() + text.metrics().advance() <= width,
                            "label {:?} exceeds viewport width {width}",
                            text.text()
                        );
                        assert!(
                            baseline.y() - text.metrics().ascent() >= 0.0
                                && baseline.y() - text.metrics().descent() <= height,
                            "label {:?} exceeds viewport height {height}",
                            text.text()
                        );
                    }
                }
                assert!(
                    snapshot
                        .resolved_screen_texts()
                        .iter()
                        .any(|text| { text.text() == "Sim;X" && text.metrics().advance() > 100.0 })
                );
                assert_eq!(runner.text_font_count(), 6);
                assert_eq!(runner.text_font_bytes(), font_bytes);
                assert_eq!(runner.image_asset_count(), 5);
                assert_eq!(runner.image_pixel_bytes(), image_bytes);
                assert_eq!(
                    runner.components::<ScreenTextVisual>().count(),
                    original_labels
                );
                advance(&mut runner, viewport, &key_tap(PhysicalKeyCode::Escape))?;
                assert_eq!(state(&runner).overlay, Overlay::None);
            }
        }
    }
    Ok(())
}

#[test]
fn undersized_viewport_shows_notice_and_rejects_queued_menu_actions() -> LogicResult {
    for (width, height) in [
        (359.0, 560.0),
        (360.0, 559.0),
        (640.0, 480.0),
        (320.0, 300.0),
    ] {
        let mut runner = runner()?;
        let small = LogicalViewport::new(width, height)?;
        let image_bytes = runner.image_pixel_bytes();
        let font_bytes = runner.text_font_bytes();
        // A held gesture from before the resize must not survive the notice.
        advance(
            &mut runner,
            viewport(),
            &[over(Target::Domains, Overlay::None, viewport()), press()],
        )?;
        let [down_press, down_release] = key_tap(PhysicalKeyCode::ArrowDown);
        let [enter_press, enter_release] = key_tap(PhysicalKeyCode::Enter);
        let report = advance(
            &mut runner,
            small,
            &[
                release(),
                down_press,
                down_release,
                enter_press,
                enter_release,
                over(
                    Target::Social(SocialLink::Github),
                    Overlay::None,
                    viewport(),
                ),
                press(),
                release(),
            ],
        )?;
        assert!(!report.exit_requested());
        assert_eq!(state(&runner).overlay, Overlay::None);
        assert_eq!(state(&runner).focus.focused(), None);
        assert_eq!(state(&runner).pointer.captured(), None);
        assert_eq!(state(&runner).pending_link, None);
        let snapshot = runner.extracted_frame().unwrap();
        let mut visible_text: Vec<_> = snapshot
            .resolved_screen_texts()
            .iter()
            .filter(|text| text.tint().alpha() > 0.0 && !text.text().is_empty())
            .map(|text| text.text())
            .collect();
        visible_text.sort_unstable();
        assert_eq!(
            visible_text,
            ["Enlarge the window", "Minimum size: 360 x 560"]
        );
        assert!(
            snapshot
                .resolved_screen_images()
                .iter()
                .all(|image| image.tint().alpha() == 0.0 || image.clip() == ScreenClip::Empty)
        );
        assert!(
            snapshot
                .resolved_screen_rectangles()
                .iter()
                .all(|panel| panel.color().alpha() == 0.0)
        );
        assert_eq!(runner.image_asset_count(), 5);
        assert_eq!(runner.image_pixel_bytes(), image_bytes);
        assert_eq!(runner.text_font_count(), 6);
        assert_eq!(runner.text_font_bytes(), font_bytes);
        advance(
            &mut runner,
            viewport(),
            &[over(Target::Domains, Overlay::None, viewport()), release()],
        )?;
        assert_eq!(state(&runner).overlay, Overlay::None);
        // Old pointer events from an unusable viewport remain ineligible after resize.
        advance(
            &mut runner,
            viewport(),
            &[
                over(Target::Domains, Overlay::None, small),
                press(),
                release(),
            ],
        )?;
        assert_eq!(state(&runner).overlay, Overlay::None);
        click(&mut runner, Target::Domains)?;
        assert_eq!(state(&runner).overlay, Overlay::Domains);
    }
    Ok(())
}

#[test]
fn social_hover_preview_takes_priority_over_previous_keyboard_focus() -> LogicResult {
    let mut runner = runner()?;
    for _ in 0..4 {
        advance(
            &mut runner,
            viewport(),
            &key_tap(PhysicalKeyCode::ArrowDown),
        )?;
    }
    assert_eq!(
        state(&runner).focus.focused(),
        Some(Target::Social(SocialLink::Github))
    );
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Social(SocialLink::Telegram),
            Overlay::None,
            viewport(),
        )],
    )?;
    let snapshot = runner.extracted_frame().unwrap();
    assert!(snapshot.resolved_screen_texts().iter().any(|text| {
        text.tint().alpha() > 0.0 && text.text().contains("https://t.me/Simulation_X")
    }));
    assert!(!snapshot.resolved_screen_texts().iter().any(|text| {
        text.tint().alpha() > 0.0 && text.text().contains("https://github.com/lexawhatt/Sim-X")
    }));
    assert_eq!(state(&runner).pending_link, None);
    assert_eq!(state(&runner).overlay, Overlay::None);
    Ok(())
}

#[test]
fn compact_layout_preserves_visible_link_opening_failure_feedback() -> LogicResult {
    fn report_link_failure(mut state: ResMut<MenuState>) {
        state.link_status = "Browser could not open this link.";
    }

    let (mut application, initial) = crate::build_application()?;
    application.add_frame_system(report_link_failure);
    let mut runner = application.build_headless(initial)?;
    for (width, height) in [(360.0, 560.0), (720.0, 720.0)] {
        let viewport = LogicalViewport::new(width, height)?;
        // The injected result is reported after view refresh, as the native adapter does.
        advance(&mut runner, viewport, &[])?;
        advance(&mut runner, viewport, &[])?;
        let snapshot = runner.extracted_frame().unwrap();
        let text = snapshot
            .resolved_screen_texts()
            .iter()
            .find(|text| text.text() == "Browser could not open this link.")
            .ok_or("link failure feedback missing")?;
        assert!(text.tint().alpha() > 0.0);
        let baseline = text.baseline_origin().to_vec2();
        assert!(baseline.x() >= 0.0 && baseline.x() + text.metrics().advance() <= width);
        assert!(baseline.y() - text.metrics().ascent() >= 0.0);
        assert!(baseline.y() - text.metrics().descent() <= height);
    }
    Ok(())
}

#[test]
fn compact_link_tooltip_temporarily_takes_priority_over_persistent_status() -> LogicResult {
    fn report_link_failure(mut state: ResMut<MenuState>) {
        state.link_status = "Browser could not open this link.";
    }
    let (mut application, initial) = crate::build_application()?;
    application.add_frame_system(report_link_failure);
    let mut runner = application.build_headless(initial)?;
    let small = LogicalViewport::new(360.0, 560.0)?;
    advance(&mut runner, small, &[])?;
    advance(
        &mut runner,
        small,
        &[over(
            Target::Social(SocialLink::Telegram),
            Overlay::None,
            small,
        )],
    )?;
    for (text, visible) in [
        ("Telegram", true),
        ("Browser could not open this link.", false),
    ] {
        assert!(
            runner
                .extracted_frame()
                .unwrap()
                .resolved_screen_texts()
                .iter()
                .any(|label| { label.text() == text && (label.tint().alpha() > 0.0) == visible })
        );
    }
    advance(&mut runner, small, &[InputEvent::PointerLeft])?;
    assert!(
        runner
            .extracted_frame()
            .unwrap()
            .resolved_screen_texts()
            .iter()
            .any(|label| {
                label.text() == "Browser could not open this link." && label.tint().alpha() > 0.0
            })
    );
    Ok(())
}
