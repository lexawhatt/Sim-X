use super::*;

use crate::menu::{state::Domain, view::Panel};

fn assert_mix_bounded(state: &MenuState) {
    assert!((0.0..=1.0).contains(&state.domains_blend));
    assert!(
        state
            .domain_mix
            .iter()
            .all(|weight| (0.0..=1.0).contains(weight))
    );
    assert!(state.domain_mix.iter().sum::<f32>() <= 1.00001);
}

#[test]
fn centered_picker_rows_remain_clickable_across_supported_viewports() -> LogicResult {
    let mut runner = runner()?;
    let generation = runner.world_generation();
    for (width, height) in [(360.0, 560.0), (1280.0, 720.0), (1920.0, 1080.0)] {
        let viewport = LogicalViewport::new(width, height)?;
        advance(
            &mut runner,
            viewport,
            &[
                over(Target::Domains, Overlay::None, viewport),
                press(),
                release(),
            ],
        )?;
        assert_eq!(state(&runner).overlay, Overlay::Domains);
        let mut previous_bottom = 0.0;
        for domain in Domain::ALL {
            let (entity, _) = runner.components::<Panel>()
                .find(|(_, panel)| matches!(panel, Panel::Button(Target::Domain(value)) if *value == domain))
                .ok_or("domain row panel missing")?;
            let visual = runner.component::<ScreenRectangleVisual>(entity)?;
            let position = visual.position().to_vec2();
            let size = visual.size().to_vec2();
            assert!((position.x() + size.x() * 0.5 - width * 0.5).abs() < 0.001);
            assert!(position.x() >= 0.0 && position.x() + size.x() <= width);
            assert!(position.y() >= previous_bottom && position.y() + size.y() <= height);
            assert!(size.y() >= 42.0);
            previous_bottom = position.y() + size.y();
            // Use extracted component geometry, not the picking helper, for this click.
            if domain == Domain::Math {
                // Geometry is checked above for every row. Math activation now
                // replaces the World; its dedicated navigation test covers it.
                advance(
                    &mut runner,
                    viewport,
                    &[
                        motion(
                            LogicalScreenPosition::new(
                                position.x() + size.x() * 0.5,
                                position.y() + size.y() * 0.5,
                            ),
                            viewport,
                        ),
                        press(),
                    ],
                )?;
                advance(&mut runner, viewport, &[InputEvent::PointerLeft, release()])?;
                assert_eq!(runner.active_world_name(), "main-menu");
                continue;
            }
            advance(
                &mut runner,
                viewport,
                &[
                    motion(
                        LogicalScreenPosition::new(
                            position.x() + size.x() * 0.5,
                            position.y() + size.y() * 0.5,
                        ),
                        viewport,
                    ),
                    press(),
                    release(),
                ],
            )?;
            if domain == Domain::Phys {
                assert_eq!(state(&runner).overlay, Overlay::PhysicsScales);
                advance(&mut runner, viewport, &key_tap(PhysicalKeyCode::Escape))?;
            } else {
                assert_eq!(state(&runner).selected_domain, Some(domain));
            }
            assert_eq!(state(&runner).overlay, Overlay::Domains);
            assert_eq!(runner.world_generation(), generation);
            assert_eq!(runner.active_world_name(), "main-menu");
            assert_eq!(state(&runner).pending_link, None);
        }
        advance(&mut runner, viewport, &key_tap(PhysicalKeyCode::Escape))?;
        assert_eq!(state(&runner).overlay, Overlay::None);
    }
    Ok(())
}

#[test]
fn opening_picker_does_not_preview_the_domain_under_an_old_screen_click() -> LogicResult {
    let mut runner = runner()?;
    // This point lies inside the old Domains button and the new Chemistry row.
    advance(
        &mut runner,
        viewport(),
        &[
            motion(LogicalScreenPosition::new(500.0, 410.0), viewport()),
            press(),
            release(),
        ],
    )?;
    assert_eq!(state(&runner).overlay, Overlay::Domains);
    assert_eq!(state(&runner).preview_domain(), None);
    assert_eq!(state(&runner).domain_mix, [0.0; 4]);
    advance(&mut runner, viewport(), &[])?;
    assert_eq!(state(&runner).preview_domain(), None);
    advance(
        &mut runner,
        viewport(),
        &[motion(LogicalScreenPosition::new(501.0, 410.0), viewport())],
    )?;
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Chem));
    assert_eq!(state(&runner).selected_domain, None);
    Ok(())
}

#[test]
fn domain_preview_does_not_select_until_mouse_release_or_keyboard_activation() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Domains)?;
    assert_eq!(state(&runner).preview_domain(), None);
    assert_eq!(state(&runner).selected_domain, None);
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Domain(Domain::Chem),
            Overlay::Domains,
            viewport(),
        )],
    )?;
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Chem));
    assert_eq!(state(&runner).selected_domain, None);
    advance(&mut runner, viewport(), &[press()])?;
    assert_eq!(state(&runner).selected_domain, None);
    advance(&mut runner, viewport(), &[release()])?;
    assert_eq!(state(&runner).selected_domain, Some(Domain::Chem));
    advance(&mut runner, viewport(), &[InputEvent::PointerLeft])?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::ArrowUp))?;
    assert_eq!(
        state(&runner).focus.focused(),
        Some(Target::Domain(Domain::Math))
    );
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Math));
    assert_eq!(state(&runner).selected_domain, Some(Domain::Chem));
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Enter))?;
    assert_eq!(runner.active_world_name(), "math-editor");
    Ok(())
}

#[test]
fn keyboard_domain_navigation_overrides_stationary_mouse_preview() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Domains)?;
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Domain(Domain::Math),
            Overlay::Domains,
            viewport(),
        )],
    )?;
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Math));
    assert_eq!(state(&runner).focus.focused(), None);
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowDown),
    )?;
    assert_eq!(
        state(&runner).focus.focused(),
        Some(Target::Domain(Domain::Phys))
    );
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowDown),
    )?;
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Math));
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowDown),
    )?;
    assert_eq!(
        state(&runner).focus.focused(),
        Some(Target::Domain(Domain::Chem))
    );
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Chem));
    advance(&mut runner, viewport(), &[])?;
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Chem));
    assert_eq!(state(&runner).selected_domain, None);
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Domain(Domain::Biol),
            Overlay::Domains,
            viewport(),
        )],
    )?;
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Biol));
    Ok(())
}

#[test]
fn domain_crossfades_are_smooth_normalized_and_keep_resources_bounded() -> LogicResult {
    let mut runner = runner()?;
    let font_bytes = runner.text_font_bytes();
    let image_bytes = runner.image_pixel_bytes();
    let labels = runner.components::<ScreenTextVisual>().count();
    let generation = runner.world_generation();
    click(&mut runner, Target::Domains)?;
    assert!(state(&runner).domains_blend > 0.0 && state(&runner).domains_blend < 1.0);
    assert_eq!(state(&runner).domain_mix, [0.0; 4]);
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Domain(Domain::Phys),
            Overlay::Domains,
            viewport(),
        )],
    )?;
    for _ in 0..20 {
        advance(&mut runner, viewport(), &[])?;
    }
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Domain(Domain::Biol),
            Overlay::Domains,
            viewport(),
        )],
    )?;
    assert!(state(&runner).domain_mix[Domain::Phys.index()] > 0.0);
    assert!(state(&runner).domain_mix[Domain::Biol.index()] > 0.0);
    assert!(state(&runner).domain_mix[Domain::Biol.index()] < 1.0);
    for _ in 0..4 {
        for domain in Domain::ALL {
            advance(
                &mut runner,
                viewport(),
                &[over(Target::Domain(domain), Overlay::Domains, viewport())],
            )?;
            for _ in 0..8 {
                let previous = state(&runner).domain_mix[domain.index()];
                advance(&mut runner, viewport(), &[])?;
                assert!(state(&runner).domain_mix[domain.index()] >= previous);
                assert_mix_bounded(state(&runner));
            }
            assert_eq!(runner.text_font_count(), 6);
            assert_eq!(runner.image_asset_count(), 5);
            assert_eq!(runner.text_font_bytes(), font_bytes);
            assert_eq!(runner.image_pixel_bytes(), image_bytes);
            assert_eq!(runner.components::<ScreenTextVisual>().count(), labels);
            assert_eq!(
                runner.components::<ScreenLineVisual>().count(),
                backdrop::LINE_COUNT + crate::menu::physics_backdrop::LINE_COUNT
            );
            assert_eq!(
                runner.components::<ScreenCircleVisual>().count(),
                backdrop::CIRCLE_COUNT + crate::menu::physics_backdrop::CIRCLE_COUNT
            );
            assert_eq!(runner.world_generation(), generation);
        }
    }
    let previous = state(&runner).domains_blend;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    assert!(state(&runner).domains_blend > 0.0 && state(&runner).domains_blend < previous);
    assert_mix_bounded(state(&runner));
    Ok(())
}

#[test]
fn reduced_motion_snaps_picker_transitions_and_freezes_rotating_geometry() -> LogicResult {
    let mut runner = runner()?;
    advance(&mut runner, viewport(), &[])?;
    let positions = |runner: &HeadlessRunner<MenuAction>| {
        runner
            .components::<ScreenCircleVisual>()
            .map(|(entity, visual)| (entity, visual.center()))
            .collect::<Vec<_>>()
    };
    let initial = positions(&runner);
    for _ in 0..8 {
        advance(&mut runner, viewport(), &[])?;
    }
    assert_ne!(
        initial,
        positions(&runner),
        "decorative motif should animate before reduced motion"
    );
    click(&mut runner, Target::Settings)?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Space))?;
    let phase = state(&runner).phase;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    click(&mut runner, Target::Domains)?;
    assert_eq!(state(&runner).domains_blend, 1.0);
    assert_eq!(state(&runner).preview_pulse, 0.0);
    for domain in Domain::ALL {
        advance(
            &mut runner,
            viewport(),
            &[over(Target::Domain(domain), Overlay::Domains, viewport())],
        )?;
        for other in Domain::ALL {
            assert_eq!(
                state(&runner).domain_mix[other.index()],
                if other == domain { 1.0 } else { 0.0 }
            );
        }
        assert_eq!(state(&runner).phase, phase);
        assert_eq!(state(&runner).preview_pulse, 0.0);
    }
    // Screen visuals consume the current frame's motion setting immediately.
    let frozen = positions(&runner);
    for _ in 0..8 {
        advance(&mut runner, viewport(), &[])?;
    }
    assert_eq!(frozen, positions(&runner));
    assert_eq!(state(&runner).phase, phase);
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    assert_eq!(state(&runner).domains_blend, 0.0);
    assert_eq!(state(&runner).domain_mix, [0.0; 4]);
    Ok(())
}

#[test]
fn domain_preview_pulse_decays_once_and_briefly_expands_the_selected_motif() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Domains)?;
    assert_eq!(state(&runner).preview_pulse, 0.0);
    advance(
        &mut runner,
        viewport(),
        &[over(
            Target::Domain(Domain::Phys),
            Overlay::Domains,
            viewport(),
        )],
    )?;
    assert_eq!(state(&runner).preview_pulse, 1.0);
    let mut previous = 1.0;
    for _ in 0..80 {
        advance(&mut runner, viewport(), &[])?;
        let pulse = state(&runner).preview_pulse;
        assert!((0.0..=previous).contains(&pulse));
        previous = pulse;
    }
    assert_eq!(state(&runner).preview_pulse, 0.0);
    let hover = over(Target::Domain(Domain::Math), Overlay::Domains, viewport());
    advance(&mut runner, viewport(), &[hover])?;
    assert_eq!(state(&runner).preview_pulse, 1.0);
    let tint = crate::menu::theme::domain(Domain::Math);
    let (entity, circle) = runner
        .components::<ScreenCircleVisual>()
        .find(|(_, circle)| {
            let color = circle.color();
            color.red() == tint.red()
                && color.green() == tint.green()
                && color.blue() == tint.blue()
        })
        .ok_or("Math motif circle missing")?;
    let baseline_radius = circle.radius();
    for _ in 0..16 {
        // Even repeated platform motion at the same coordinates cannot restart it.
        advance(&mut runner, viewport(), &[hover])?;
    }
    assert!(state(&runner).preview_pulse > 0.0 && state(&runner).preview_pulse < 1.0);
    let expanded = runner.component::<ScreenCircleVisual>(entity)?.radius();
    assert!(expanded > baseline_radius * 1.02);
    for _ in 0..17 {
        advance(&mut runner, viewport(), &[])?;
    }
    assert_eq!(state(&runner).preview_pulse, 0.0);
    let settled = runner.component::<ScreenCircleVisual>(entity)?.radius();
    assert!(settled < expanded);
    assert!((settled - baseline_radius).abs() < 0.002);
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowDown),
    )?;
    // No domain is focused automatically; the first key chooses Physics.
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Phys));
    assert_eq!(state(&runner).preview_pulse, 1.0);
    advance(
        &mut runner,
        viewport(),
        &key_tap(PhysicalKeyCode::ArrowDown),
    )?;
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Math));
    assert_eq!(state(&runner).preview_pulse, 1.0);
    Ok(())
}
