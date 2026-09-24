use super::*;
use crate::math_editor::{
    scene::Spatial,
    state,
    tests::visual_probe,
    view::{
        assets,
        drawing::Scene,
        graph,
        layout::{self, Camera, Rect},
    },
};

#[test]
fn frontal_engine_camera_matches_two_dimensional_projection() {
    let spatial = Spatial::default();
    for rect in [
        Rect::new(384.0, 66.0, 896.0, 530.0),
        Rect::new(470.0, 66.0, 1450.0, 810.0),
    ] {
        let camera = Camera {
            x: 2.0,
            y: -3.0,
            scale: 55.0,
        };
        let projection = spatial.projector(camera, rect).unwrap();
        for p in [
            sim_math::geometry::Point { x: 2.0, y: -3.0 },
            sim_math::geometry::Point { x: 0.0, y: 0.0 },
        ] {
            let expected = camera.project(p, rect).unwrap();
            let actual = projection.point([p.x, p.y, 0.0]).unwrap();
            assert!((actual[0] - expected[0]).hypot(actual[1] - expected[1]) < 0.001);
        }
    }
}

#[test]
fn settled_math_basis_is_z_up_and_height_fields_rise_above_xy() {
    let spatial = Spatial {
        enabled: true,
        blend: 1.0,
        ..Spatial::default()
    };
    let rect = Rect::new(470.0, 66.0, 1450.0, 810.0);
    let camera = Camera {
        x: 0.0,
        y: 0.0,
        scale: 62.0,
    };
    let projector = spatial.projector(camera, rect).unwrap();
    let origin = projector.point([0.0, 0.0, 0.0]).unwrap();
    let up = projector.point([0.0, 0.0, 1.0]).unwrap();
    assert!((up[0] - origin[0]).abs() < 0.001);
    assert!(origin[1] - up[1] > camera.scale as f32 * 0.8);
    let x = projector.point([1.0, 0.0, 0.0]).unwrap();
    let y = projector.point([0.0, 1.0, 0.0]).unwrap();
    let area =
        ((x[0] - origin[0]) * (y[1] - origin[1]) - (x[1] - origin[1]) * (y[0] - origin[0])).abs();
    assert!(area > camera.scale.powi(2) as f32 * 0.35);
    assert!(area < camera.scale.powi(2) as f32 * 0.55);
    let surface =
        sim_math::Expression::with_variables("x^2+y^2", &std::collections::BTreeMap::new())
            .unwrap();
    for (x, y) in [(-1.0, -1.0), (0.5, -1.0), (1.0, 2.0)] {
        let z = surface.evaluate_at(x, y).unwrap();
        let foot = projector.point([x, y, 0.0]).unwrap();
        let height = projector.point([x, y, z]).unwrap();
        assert!((height[0] - foot[0]).abs() < 0.001);
        assert!(height[1] < foot[1]);
    }
}

#[test]
fn plane_lowers_continuously_and_toggle_reversal_never_changes_math_coordinates() {
    let mut spatial = Spatial::default();
    let rect = Rect::new(384.0, 66.0, 896.0, 530.0);
    let camera = Camera::default();
    let sample = [1.0, 2.0, 0.0];
    let initial = spatial
        .projector(camera, rect)
        .unwrap()
        .point(sample)
        .unwrap();
    spatial.enabled = true;
    assert_eq!(
        spatial
            .projector(camera, rect)
            .unwrap()
            .point(sample)
            .unwrap(),
        initial
    );
    let mut last = initial;
    for i in 0..90 {
        if i == 18 || i == 24 {
            spatial.enabled = !spatial.enabled;
        }
        let before = spatial
            .projector(camera, rect)
            .unwrap()
            .point(sample)
            .unwrap();
        assert_eq!(before, last);
        spatial.advance(0.016, false);
        let current = spatial
            .projector(camera, rect)
            .unwrap()
            .point(sample)
            .unwrap();
        assert!((current[0] - last[0]).hypot(current[1] - last[1]) < 22.0);
        last = current;
    }
    spatial.enabled = false;
    spatial.advance(0.016, true);
    assert_eq!(spatial.blend, 0.0);
    assert_eq!(
        spatial
            .projector(camera, rect)
            .unwrap()
            .point(sample)
            .unwrap(),
        initial
    );
    spatial.enabled = true;
    spatial.advance(0.016, true);
    assert_eq!(spatial.blend, 1.0);
    let projector = spatial.projector(camera, rect).unwrap();
    let a = projector.point([camera.x, camera.y, 0.0]).unwrap();
    let b = projector.point([camera.x, camera.y, 1.0]).unwrap();
    assert!(b[1] < a[1] && (b[0] - a[0]).abs() < 0.001);
}

#[test]
fn integral_bands_fade_near_2d_and_are_absent_from_the_settled_orbit_view() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "int(x)-int(x^2)")?;
    click(&mut runner, layout::Target::Calculate(0))?;
    until(&mut runner, |s| !s.busy && !s.dirty)?;
    for _ in 0..70 {
        frame(&mut runner, &[])?;
    }
    let state = runner.resource::<state::MathState>().unwrap();
    let fonts = runner.resource::<assets::Fonts>().unwrap();
    // Rendering the same successful plot at controlled presentation states does
    // not modify its scalar, integration activation or scientific coordinates.
    let mut view = state::MathState::new()?;
    view.document = state.document.clone();
    view.plot = state.plot.clone();
    view.stale = false;
    view.area_progress = vec![1.0];
    view.motion.plot_opacity = 1.0;
    let mut alpha = Vec::new();
    for blend in [0.0, 0.1, 0.2, 1.0, 0.0] {
        view.spatial.enabled = true;
        view.spatial.blend = blend;
        let layout = layout::Layout::for_state(viewport(), &view);
        let mut scene = Scene::default();
        graph::draw(&mut scene, &view, &layout, fonts);
        alpha.push(
            scene
                .panels
                .iter()
                .filter(|p| p.depth == 0.5)
                .map(|p| p.color.alpha())
                .fold(0.0_f32, f32::max),
        );
        assert!(!scene.lines.iter().any(|line| line.depth == 0.5));
        if blend == 1.0 {
            assert!(
                scene
                    .texts
                    .iter()
                    .any(|text| text.text.contains("XY plane (z=0)"))
            );
            assert!(!scene.lines.is_empty());
        }
    }
    assert!(alpha[0] > alpha[1] && alpha[1] > 0.0);
    assert_eq!(alpha[2], 0.0);
    assert_eq!(alpha[3], 0.0);
    assert_eq!(alpha[4], alpha[0]);
    Ok(())
}

#[test]
fn camera_zoom_is_anchored_smooth_reversible_and_reduced_motion_is_immediate() {
    let mut state = state::MathState::new().unwrap();
    let rect = Rect::new(384.0, 66.0, 896.0, 530.0);
    let pointer = [710.0, 240.0];
    let world = state.camera.unproject(pointer, rect).unwrap();
    let before = state.camera;
    state.zoom_view(2.0, pointer, rect);
    assert_eq!(state.camera, before);
    state.advance_camera(0.016);
    assert!(state.camera.scale > before.scale && state.camera.scale < before.scale * 2.0);
    for _ in 0..150 {
        state.advance_camera(0.016);
        let projected = state.camera.project(world, rect).unwrap();
        assert!((projected[0] - pointer[0]).hypot(projected[1] - pointer[1]) < 0.001);
    }
    assert!(state.camera_target.is_none());
    state.zoom_view(0.5, pointer, rect);
    state.reduced_motion = true;
    state.advance_camera(0.016);
    assert_eq!(state.camera.scale, before.scale);
    assert!((state.camera.x - before.x).abs() < 1e-12);
    assert!((state.camera.y - before.y).abs() < 1e-12);
}

#[test]
fn native_3d_switch_orbit_and_return_preserve_document() -> LogicResult {
    use crate::math_editor::view::layout::Target;
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "z=sin(x)+cos(y)")?;
    until(&mut runner, |s| !s.busy)?;
    let before = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .clone();
    assert!(runner.resource::<state::MathState>().unwrap().parameters[0].is_none());
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .missing_parameters[0]
            .is_empty()
    );
    click(&mut runner, Target::Spatial)?;
    assert!(runner.resource::<state::MathState>().unwrap().spatial.blend > 0.0);
    assert!(runner.resource::<state::MathState>().unwrap().spatial.blend < 1.0);
    until(&mut runner, |s| !s.busy)?;
    for _ in 0..100 {
        frame(&mut runner, &[])?;
    }
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(!s.plot.as_ref().unwrap().rows[0].surface.is_empty());
    let l = layout::Layout::for_state(viewport(), s);
    assert!(
        !l.controls
            .iter()
            .any(|(t, _, _)| matches!(t, Target::Tool(_)))
    );
    let mut scene = Scene::default();
    graph::draw(
        &mut scene,
        s,
        &l,
        runner.resource::<assets::Fonts>().unwrap(),
    );
    assert!(scene.lines.len() > 100);
    visual_probe::export(&scene, l.canvas);
    assert!(
        scene.lines.iter().all(
            |line| line.width >= 0.5 && line.from.iter().chain(&line.to).all(|n| n.is_finite())
        )
    );
    let origin = l.canvas.center();
    let angle = s.spatial.target;
    frame(
        &mut runner,
        &[
            pointer(origin),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    frame(
        &mut runner,
        &[
            pointer([origin[0] + 80.0, origin[1] + 30.0]),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Released),
        ],
    )?;
    assert_ne!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .spatial
            .target,
        angle
    );
    assert!(runner.resource::<state::MathState>().unwrap().document == before);
    click(&mut runner, Target::Spatial)?;
    for _ in 0..100 {
        frame(&mut runner, &[])?;
    }
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(!s.spatial.visible());
    assert!(s.document == before);
    Ok(())
}

#[test]
fn orbit_cancels_on_focus_loss_and_never_edits_constructions() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    click(&mut runner, layout::Target::Spatial)?;
    let s = runner.resource::<state::MathState>().unwrap();
    let l = layout::Layout::for_state(viewport(), s);
    let origin = l.canvas.center();
    frame(
        &mut runner,
        &[
            pointer(origin),
            InputEvent::mouse_button(MouseButton::Left, ButtonState::Pressed),
        ],
    )?;
    frame(&mut runner, &[InputEvent::FocusLost])?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .spatial
            .drag
            .is_none()
    );
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .geometry
            .points()
            .is_empty()
    );
    Ok(())
}
