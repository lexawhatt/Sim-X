use super::*;
use crate::math_editor::{
    state,
    view::{
        assets::Fonts,
        drawing::Scene,
        layout::{Layout, Rect, Target},
        sidebar,
    },
};

fn rectangle(layout: &Layout, target: Target) -> Option<Rect> {
    layout
        .controls
        .iter()
        .find(|(t, _, _)| *t == target)
        .map(|(_, r, _)| *r)
}

#[test]
fn toolbar_and_z_keep_fading_geometry_but_guard_ineligible_clicks() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    state.keypad = true;
    for (enabled, blend) in [
        (false, 0.0),
        (true, 0.0),
        (true, 0.3),
        (true, 0.75),
        (true, 1.0),
        (false, 1.0),
        (false, 0.75),
        (false, 0.3),
        (false, 0.0),
    ] {
        state.spatial.enabled = enabled;
        state.spatial.blend = blend;
        let layout = Layout::for_state(viewport(), &state);
        let mut scene = Scene::default();
        sidebar::draw(&mut scene, &state, &layout, fonts);
        if blend < 1.0 {
            let rect = rectangle(&layout, Target::Tool(1)).unwrap();
            assert!((rect.y - (78.0 - blend as f32 * 8.0)).abs() < 1e-5);
            let expected = if !enabled && blend == 0.0 {
                Target::Tool(1)
            } else {
                Target::TransitionGuard
            };
            assert_eq!(layout.hit(rect.center(), false), Some(expected));
            let text = scene.texts.iter().find(|t| t.text == "Point").unwrap();
            assert!((text.color.alpha() - (1.0 - blend as f32)).abs() < 1e-6);
        } else {
            assert!(rectangle(&layout, Target::Tool(1)).is_none());
            assert!(!scene.texts.iter().any(|t| t.text == "Point"));
        }
        if enabled || blend > 0.0 {
            let rect = rectangle(&layout, Target::SpatialZ).unwrap();
            let expected = if enabled && blend >= 0.6 {
                Target::SpatialZ
            } else {
                Target::TransitionGuard
            };
            assert_eq!(layout.hit(rect.center(), false), Some(expected));
            let text = scene.texts.iter().find(|t| t.text == "z").unwrap();
            assert!((text.color.alpha() - blend as f32).abs() < 1e-6);
        } else {
            assert!(rectangle(&layout, Target::SpatialZ).is_none());
        }
    }
    Ok(())
}

#[test]
fn mode_toggle_label_stays_stable_and_indicator_tracks_reversal() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    let mut centers = vec![];
    for blend in [0.0, 0.25, 0.75, 1.0, 0.75, 0.25, 0.0] {
        state.spatial.blend = blend;
        let layout = Layout::for_state(viewport(), &state);
        assert_eq!(
            layout
                .controls
                .iter()
                .find(|(t, _, _)| *t == Target::Spatial)
                .unwrap()
                .2,
            "2D / 3D"
        );
        let rect = rectangle(&layout, Target::Spatial).unwrap();
        let mut scene = Scene::default();
        sidebar::draw(&mut scene, &state, &layout, fonts);
        let labels: Vec<_> = scene
            .texts
            .iter()
            .filter(|t| rect.contains(t.position))
            .collect();
        assert!(labels.iter().any(|t| t.text == "2D"));
        assert!(labels.iter().any(|t| t.text == "3D"));
        let line = scene
            .lines
            .iter()
            .find(|line| rect.contains(line.from) && rect.contains(line.to))
            .unwrap();
        centers.push((line.from[0] + line.to[0]) * 0.5);
    }
    assert!(centers[..4].windows(2).all(|p| p[1] > p[0]));
    assert_eq!(centers[0], centers[6]);
    assert_eq!(centers[1], centers[5]);
    assert_eq!(centers[2], centers[4]);
    Ok(())
}

#[test]
fn reduced_motion_switches_tool_eligibility_without_intermediate_frames() {
    let mut state = state::MathState::new().unwrap();
    state.keypad = true;
    state.reduced_motion = true;
    state.activate(Target::Spatial);
    state.advance_camera(0.016);
    let layout = Layout::for_state(viewport(), &state);
    assert_eq!(state.spatial.blend, 1.0);
    assert!(rectangle(&layout, Target::Tool(0)).is_none());
    assert_eq!(
        layout.hit(
            rectangle(&layout, Target::SpatialZ).unwrap().center(),
            false
        ),
        Some(Target::SpatialZ)
    );
    state.activate(Target::Spatial);
    state.advance_camera(0.016);
    let layout = Layout::for_state(viewport(), &state);
    assert_eq!(state.spatial.blend, 0.0);
    assert!(rectangle(&layout, Target::SpatialZ).is_none());
    assert_eq!(
        layout.hit(rectangle(&layout, Target::Tool(0)).unwrap().center(), false),
        Some(Target::Tool(0))
    );
}

#[test]
fn fading_toolbar_press_cannot_turn_into_a_tool_or_canvas_action() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    click(&mut runner, Target::Tool(1))?;
    click(&mut runner, Target::Spatial)?;
    let state = runner.resource::<state::MathState>().unwrap();
    let layout = Layout::for_state(viewport(), state);
    let rect = rectangle(&layout, Target::Tool(3)).unwrap();
    assert_eq!(
        layout.hit(rect.center(), false),
        Some(Target::TransitionGuard)
    );
    frame(
        &mut runner,
        &[
            pointer(rect.center()),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )?;
    let state = runner.resource::<state::MathState>().unwrap();
    assert_eq!(state.tool, 1);
    assert!(state.document.geometry.points().is_empty());
    assert!(state.spatial.drag.is_none());
    click(&mut runner, Target::Spatial)?;
    let state = runner.resource::<state::MathState>().unwrap();
    let layout = Layout::for_state(viewport(), state);
    let rect = rectangle(&layout, Target::Tool(3)).unwrap();
    assert_eq!(
        layout.hit(rect.center(), false),
        Some(Target::TransitionGuard)
    );
    frame(
        &mut runner,
        &[
            pointer(rect.center()),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    for _ in 0..150 {
        frame(&mut runner, &[])?;
    }
    let state = runner.resource::<state::MathState>().unwrap();
    let layout = Layout::for_state(viewport(), state);
    let rect = rectangle(&layout, Target::Tool(3)).unwrap();
    frame(
        &mut runner,
        &[
            pointer(rect.center()),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )?;
    let state = runner.resource::<state::MathState>().unwrap();
    assert_eq!(state.tool, 1);
    assert!(state.document.geometry.points().is_empty());
    assert!(!state.canvas_pressed);
    click(&mut runner, Target::Tool(3))?;
    assert_eq!(runner.resource::<state::MathState>().unwrap().tool, 3);
    Ok(())
}

#[test]
fn integral_key_has_readable_consistent_font_and_nonoverlapping_bounds() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    state.keypad = true;
    let layout = Layout::for_state(viewport(), &state);
    let mut scene = Scene::default();
    sidebar::draw(&mut scene, &state, &layout, fonts);
    let label = scene.texts.iter().find(|t| t.text == "Integral").unwrap();
    assert_eq!(label.style, 1);
    let rect = rectangle(&layout, Target::Integral).unwrap();
    assert!(fonts.width(label.style, &label.text) <= rect.w - 10.0);
    for (_, other, _) in layout
        .controls
        .iter()
        .filter(|(t, _, _)| *t != Target::Integral)
    {
        assert!(rect.intersection(*other).is_none());
    }
    Ok(())
}
