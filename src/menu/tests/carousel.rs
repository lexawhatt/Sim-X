use super::*;

use crate::menu::{input::home_mix, state::Domain};

fn quarter_second(runner: &mut HeadlessRunner<MenuAction>) -> LogicResult {
    match runner.advance_frame(FrameRequest::new(
        Duration::from_millis(250),
        &[],
        viewport(),
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

#[test]
fn carousel_has_five_second_slots_and_smooth_normalized_one_second_crossfades() {
    for (seconds, expected) in [
        (0.0, [1.0, 0.0, 0.0, 0.0]),
        (4.0, [1.0, 0.0, 0.0, 0.0]),
        (4.5, [0.5, 0.5, 0.0, 0.0]),
        (5.0, [0.0, 1.0, 0.0, 0.0]),
        (9.5, [0.0, 0.5, 0.5, 0.0]),
        (20.0, [1.0, 0.0, 0.0, 0.0]),
    ] {
        assert_eq!(home_mix(seconds), expected);
    }
    for sample in 0..=800 {
        let weights = home_mix(sample as f32 * 0.025);
        assert!(weights.iter().all(|weight| (0.0..=1.0).contains(weight)));
        assert!((weights.iter().sum::<f32>() - 1.0).abs() < 0.00001);
        assert!(weights.iter().filter(|&&weight| weight > 0.0).count() <= 2);
    }
    // Crossfades meet each held image with a flat slope, including cycle wrap.
    for boundary in [4.0, 5.0, 9.0, 10.0, 14.0, 15.0, 19.0, 20.0] {
        let before = home_mix(boundary - 0.001);
        let after = home_mix(boundary + 0.001);
        for index in 0..4 {
            assert!((after[index] - before[index]).abs() < 0.00001);
        }
    }
}

#[test]
fn actual_home_frames_advance_carousel_and_reduced_motion_freezes_it() -> LogicResult {
    let mut runner = runner()?;
    runner.set_paused(true);
    for _ in 0..16 {
        quarter_second(&mut runner)?;
    }
    assert_eq!(state(&runner).home_seconds, 4.0);
    assert_eq!(state(&runner).home_mix, [1.0, 0.0, 0.0, 0.0]);
    quarter_second(&mut runner)?;
    quarter_second(&mut runner)?;
    assert_eq!(state(&runner).home_mix, [0.5, 0.5, 0.0, 0.0]);
    quarter_second(&mut runner)?;
    quarter_second(&mut runner)?;
    assert_eq!(state(&runner).home_seconds, 5.0);
    assert_eq!(state(&runner).home_mix, [0.0, 1.0, 0.0, 0.0]);
    let math_tint = crate::menu::theme::domain(Domain::Math);
    let visible_circles: Vec<_> = runner
        .components::<ScreenCircleVisual>()
        .map(|(_, circle)| circle)
        .filter(|circle| circle.clip() != ScreenClip::Empty && circle.color().alpha() > 0.0)
        .collect();
    assert!(!visible_circles.is_empty());
    assert!(visible_circles.iter().all(|circle| {
        circle.color().red() == math_tint.red()
            && circle.color().green() == math_tint.green()
            && circle.color().blue() == math_tint.blue()
    }));
    click(&mut runner, Target::Settings)?;
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Space))?;
    assert!(state(&runner).reduced_motion);
    let frozen = (
        state(&runner).home_seconds,
        state(&runner).home_mix,
        state(&runner).phase,
    );
    for _ in 0..24 {
        quarter_second(&mut runner)?;
    }
    assert_eq!(
        (
            state(&runner).home_seconds,
            state(&runner).home_mix,
            state(&runner).phase
        ),
        frozen
    );
    Ok(())
}

#[test]
fn domains_start_blank_and_unselected_hover_fades_back_to_blank() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Domains)?;
    assert_eq!(state(&runner).preview_domain(), None);
    assert_eq!(state(&runner).focus.focused(), None);
    assert_eq!(state(&runner).selected_domain, None);
    assert_eq!(state(&runner).preview_pulse, 0.0);
    for _ in 0..20 {
        advance(&mut runner, viewport(), &[])?;
        assert_eq!(state(&runner).domain_mix, [0.0; 4]);
    }
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
    assert!(state(&runner).domain_mix[Domain::Math.index()] > 0.0);
    advance(&mut runner, viewport(), &[InputEvent::PointerLeft])?;
    assert_eq!(state(&runner).preview_domain(), None);
    assert_eq!(state(&runner).selected_domain, None);
    let mut previous = state(&runner).domain_mix;
    for _ in 0..80 {
        advance(&mut runner, viewport(), &[])?;
        for (current, previous) in state(&runner).domain_mix.iter().zip(previous) {
            assert!(*current <= previous);
        }
        previous = state(&runner).domain_mix;
    }
    assert!(
        state(&runner)
            .domain_mix
            .iter()
            .all(|&weight| weight < 0.0001)
    );
    click(&mut runner, Target::Domain(Domain::Biol))?;
    advance(&mut runner, viewport(), &[InputEvent::FocusLost])?;
    assert_eq!(state(&runner).focus.focused(), None);
    assert_eq!(state(&runner).selected_domain, Some(Domain::Biol));
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Biol));
    advance(&mut runner, viewport(), &key_tap(PhysicalKeyCode::Escape))?;
    click(&mut runner, Target::Domains)?;
    assert_eq!(state(&runner).selected_domain, None);
    assert_eq!(state(&runner).preview_domain(), None);
    Ok(())
}

#[test]
fn pointer_keyboard_batch_policy_and_post_click_movement_do_not_activate() -> LogicResult {
    let mut runner = runner()?;
    let mut events = key_tap(PhysicalKeyCode::Tab).to_vec();
    events.push(over(Target::Settings, Overlay::None, viewport()));
    advance(&mut runner, viewport(), &events)?;
    assert_eq!(state(&runner).focus.focused(), Some(Target::Domains));
    // Keyboard wins ambiguous mixed batches: Logic has no ordered motion stream.
    assert_eq!(state(&runner).hovered, None);
    assert_eq!(state(&runner).overlay, Overlay::None);

    advance(
        &mut runner,
        viewport(),
        &[
            over(Target::Domains, Overlay::None, viewport()),
            press(),
            release(),
            over(Target::Domain(Domain::Math), Overlay::Domains, viewport()),
        ],
    )?;
    assert_eq!(state(&runner).overlay, Overlay::Domains);
    assert_eq!(state(&runner).preview_domain(), Some(Domain::Math));
    assert_eq!(state(&runner).selected_domain, None);
    Ok(())
}
