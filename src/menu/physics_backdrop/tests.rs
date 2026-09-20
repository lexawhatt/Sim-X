use super::*;
use geometry::{EARTH_RADIUS, ORBIT_RADIUS, PENDULUM_LENGTH, PIVOT, bob, electron, moon, rotate};
use std::f32::consts::{PI, TAU};

#[test]
fn every_path_and_node_fits_one_centered_square() {
    for phase in [0.0, 0.7, 3.0, 17.0, TAU * 10.0] {
        for (path, segments) in PATHS {
            for index in 0..=segments {
                let point = path.point(f32::from(index) / f32::from(segments), phase);
                assert!(point.x().is_finite() && point.y().is_finite());
                assert!(point.x().abs() < 0.5 && point.y().abs() < 0.5);
            }
        }
        for dot in DOTS {
            let (point, radius) = dot.geometry(phase);
            assert!(point.x().abs() + radius <= 0.5 && point.y().abs() + radius <= 0.5);
        }
    }
}

#[test]
fn earth_and_circular_moon_orbit_have_exactly_the_same_center_as_the_atom() {
    for phase in [0.0, 0.7, 3.0, 17.0] {
        assert_eq!(Dot::Earth.geometry(phase), (Vec2::ZERO, EARTH_RADIUS));
        let point = moon(phase);
        assert!((point.x().hypot(point.y()) - ORBIT_RADIUS).abs() < 1e-6);
        for t in [0.0, 0.1, 0.3, 0.5] {
            let opposite = Path::MoonOrbit.point(t, phase) + Path::MoonOrbit.point(t + 0.5, phase);
            assert!(opposite.x().hypot(opposite.y()) < 1e-6);
        }
        assert_eq!(Path::Rod.point(0.0, phase), PIVOT);
        assert_eq!(Path::Rod.point(1.0, phase), bob(phase));
        let delta = bob(phase) - PIVOT;
        assert!((delta.x().hypot(delta.y()) - PENDULUM_LENGTH).abs() < 1e-6);
        assert_eq!(Path::Spring.point(0.0, phase), PIVOT);
        let spring_end = Path::Spring.point(1.0, phase);
        let top_middle = (Path::Mass.point(0.0, phase) + Path::Mass.point(0.25, phase)) * 0.5;
        assert!((spring_end - top_middle).length() < 1e-6);
        for orbit in 0..3 {
            let local = rotate(
                electron(orbit, phase),
                -(f32::from(orbit) * PI / 3.0 + phase * 0.1),
            );
            assert!(((local.x() / 0.164).powi(2) + (local.y() / 0.405).powi(2) - 1.0).abs() < 1e-5);
        }
    }
}

#[test]
fn phase_wrap_keeps_all_connected_geometry_continuous() {
    for (path, segments) in PATHS {
        for step in 0..=segments {
            let t = f32::from(step) / f32::from(segments);
            let delta = path.point(t, 0.0) - path.point(t, TAU * 10.0);
            assert!(delta.x().hypot(delta.y()) < 1e-5);
        }
    }
    for dot in DOTS {
        let delta = dot.geometry(0.0).0 - dot.geometry(TAU * 10.0).0;
        assert!(delta.x().hypot(delta.y()) < 1e-5);
    }
}

fn visible_state() -> MenuState {
    MenuState {
        overlay: Overlay::PhysicsScales,
        scale_blend: 1.0,
        scale_mix: [0.0, 1.0, 0.0],
        ..Default::default()
    }
}

#[test]
fn macro_shows_three_coherent_mechanisms_but_never_overlays_them() -> LogicResult {
    let layout = Layout::new(LogicalViewport::new(1280.0, 800.0)?);
    let mut state = visible_state();
    for index in 0..1801 {
        state.macro_seconds = index as f32 * 0.01;
        let frame = frames(&layout, &state)[PhysicsScale::Macro.index()];
        let weights = [Mechanism::Pendulum, Mechanism::Oscillator, Mechanism::Gears]
            .map(|part| mechanism_frame(frame, Some(part), &state).opacity);
        assert!(weights.iter().filter(|&&weight| weight > 0.0).count() <= 1);
        assert!(weights.iter().all(|&weight| (0.0..=0.48).contains(&weight)));
    }
    for (seconds, expected) in [
        (2.0, Mechanism::Pendulum),
        (8.0, Mechanism::Oscillator),
        (14.0, Mechanism::Gears),
    ] {
        assert_eq!(Mechanism::current(seconds), (expected, 1.0));
    }
    for boundary in [6.0, 12.0, 18.0] {
        assert!(Mechanism::current(boundary - 0.001).1 < 0.0001);
        assert!(Mechanism::current(boundary + 0.001).1 < 0.0001);
    }
    Ok(())
}

#[test]
fn planets_and_macro_bodies_have_no_opaque_fill_and_share_the_atom_line_contrast() -> LogicResult {
    let frame = Frame {
        placement: Rect::new(0.0, 0.0, 768.0, 768.0),
        opacity: 0.48,
        color: theme::ACCENT,
    };
    for dot in [Dot::Earth, Dot::Moon, Dot::Weight] {
        let visual = circle_visual(PhysicsBackdropDot { dot, glow: false }, frame, 0.0)?;
        assert_eq!(visual.color().alpha(), 0.0);
        assert!(visual.stroke().is_some());
        assert_eq!(
            circle_visual(PhysicsBackdropDot { dot, glow: true }, frame, 0.0)?.clip(),
            ScreenClip::Empty
        );
    }
    for (path, _) in PATHS {
        assert!(path.opacity() <= 0.42);
    }
    Ok(())
}

#[test]
fn responsive_frames_stay_centered_and_small_decorative_segments_are_culled() -> LogicResult {
    let state = visible_state();
    for (width, height) in [
        (360.0, 560.0),
        (900.0, 600.0),
        (1280.0, 800.0),
        (1920.0, 1080.0),
    ] {
        let layout = Layout::new(LogicalViewport::new(width, height)?);
        for frame in frames(&layout, &state) {
            assert!((frame.placement.x + frame.placement.width * 0.5 - width * 0.5).abs() < 1e-4);
            assert!((frame.placement.y + frame.placement.height * 0.5 - height * 0.5).abs() < 1e-4);
            assert_eq!(frame.placement.width, frame.placement.height);
        }
    }
    let visual = line_visual(
        PhysicsBackdropLine {
            path: Path::Rod,
            segment: 0,
            segments: 100,
        },
        Frame {
            placement: Rect::new(0.0, 0.0, 1.0, 1.0),
            opacity: 1.0,
            color: theme::ACCENT,
        },
        0.0,
    )?;
    assert_eq!(visual.clip(), ScreenClip::Empty);
    assert_ne!(visual.start(), visual.end());
    Ok(())
}

#[test]
fn idle_and_other_screens_hide_motifs_and_reduced_motion_has_no_pulse() -> LogicResult {
    let layout = Layout::new(LogicalViewport::new(1280.0, 800.0)?);
    let mut state = visible_state();
    state.scale_mix = [0.0; 3];
    assert!(
        frames(&layout, &state)
            .iter()
            .all(|frame| frame.opacity == 0.0)
    );
    state.scale_mix = [0.0, 0.5, 0.0];
    state.hovered = Some(crate::menu::state::Target::Scale(PhysicsScale::Macro));
    assert!(frames(&layout, &state)[1].placement.width > layout.backdrop(1.0).width);
    state.reduced_motion = true;
    assert_eq!(
        frames(&layout, &state)[1].placement.width,
        layout.backdrop(1.0).width
    );
    state.macro_seconds = 6.0;
    assert!(
        mechanism_frame(
            frames(&layout, &state)[1],
            Some(Mechanism::Oscillator),
            &state
        )
        .opacity
            > 0.0
    );
    for overlay in [
        Overlay::None,
        Overlay::Domains,
        Overlay::Projects,
        Overlay::CreateProject,
    ] {
        state.overlay = overlay;
        assert!(
            frames(&layout, &state)
                .iter()
                .all(|frame| frame.opacity == 0.0)
        );
    }
    Ok(())
}
