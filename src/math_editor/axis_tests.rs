use super::super::{
    assets::Fonts,
    axis_spatial, axis_ticks, axis_view,
    drawing::Scene,
    layout::{Layout, Rect, Target},
};
use super::*;

#[test]
fn adaptive_tick_density_is_zero_anchored_and_labels_keep_precision() {
    for scale in [1e-12, 0.0008, 0.02, 8.1, 62.0, 422.0, 1e9, 1e15] {
        let spacing = axis_ticks::Spacing::new(scale, 30.0).unwrap();
        assert!(spacing.major * scale >= 88.0 - 1e-8);
        let extent = 1450.0 / scale;
        let ticks = axis_ticks::ticks(-extent * 0.5, extent * 0.5, spacing.major);
        assert!(ticks.len() <= 18);
        assert!(ticks.contains(&0.0));
        for v in ticks {
            assert!((v / spacing.major - (v / spacing.major).round()).abs() < 1e-8);
        }
        assert!(axis_ticks::ticks(-extent * 0.5, extent * 0.5, spacing.minor).len() <= 85);
    }
    assert_eq!(axis_ticks::label(-0.0, 0.1), "0");
    assert_eq!(axis_ticks::label(0.2, 0.1), "0.2");
    assert_eq!(axis_ticks::label(20.0, 10.0), "20");
    assert_ne!(
        axis_ticks::label(1e9, 0.2),
        axis_ticks::label(1e9 + 0.2, 0.2)
    );
    assert_ne!(
        axis_ticks::label(1e-7, 2e-8),
        axis_ticks::label(1.2e-7, 2e-8)
    );
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(axis_ticks::Spacing::new(scale, 20.0).is_none());
    }
    assert!(axis_ticks::ticks(1e30, 1e30 + 1e20, 1.0).is_empty());
}

fn assert_label_separation(scene: &Scene, fonts: &Fonts, rect: Rect) {
    let mut boxes = vec![];
    for text in &scene.texts {
        let r = Rect::new(
            text.position[0],
            text.position[1] - 16.0,
            fonts.width(text.style, &text.text),
            20.0,
        );
        assert!(r.x >= rect.x && r.x + r.w <= rect.x + rect.w);
        assert!(r.y >= rect.y && r.y + r.h <= rect.y + rect.h);
        assert!(
            boxes.iter().all(|old: &Rect| old.intersection(r).is_none()),
            "overlap at {}",
            text.text
        );
        boxes.push(r);
    }
}

#[test]
fn zoomed_grid_and_xyz_labels_do_not_overlap_or_flood_the_view() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    for size in [[900.0, 640.0], [1280.0, 800.0], [1920.0, 1080.0]] {
        let viewport = LogicalViewport::new(size[0], size[1])?;
        for scale in [8.1, 62.0, 422.0, 0.00001, 1e9] {
            state.camera.scale = scale;
            state.camera.x = 0.23 / scale;
            state.camera.y = -3.2 / scale;
            let layout = Layout::for_state(viewport, &state);
            let mut scene = Scene::default();
            axis_view::plane(&mut scene, &state, &layout, fonts);
            assert!(scene.lines.len() < 200);
            assert!(
                scene.texts.len() >= 3 && scene.texts.len() < 35,
                "{} labels at {size:?}, {scale}",
                scene.texts.len()
            );
            assert_label_separation(&scene, fonts, layout.canvas);
            state.spatial.blend = 1.0;
            let projector = state.spatial.projector(state.camera, layout.canvas)?;
            let mut scene = Scene::default();
            axis_spatial::draw(&mut scene, &state, &layout, &projector, fonts);
            assert_label_separation(&scene, fonts, layout.canvas);
            let blue = Color::rgb8(119, 177, 241);
            assert!(scene.texts.iter().any(|t| t.color == blue && t.text == "Z"));
            assert!(
                scene
                    .texts
                    .iter()
                    .any(|t| t.color == blue && t.text.parse::<f64>().is_ok()),
                "missing Z numbers at {size:?}, {scale}"
            );
            state.spatial.blend = 0.0;
        }
    }
    Ok(())
}

#[test]
fn zoomed_out_grid_cpu_preview() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    state.camera.scale = 8.1;
    let layout = Layout::for_state(LogicalViewport::new(1920.0, 1080.0)?, &state);
    let mut scene = Scene::default();
    axis_view::plane(&mut scene, &state, &layout, fonts);
    super::super::visual_probe::export(&scene, layout.canvas);
    Ok(())
}

#[test]
fn axis_layers_crossfade_without_toggle_pop_or_unbounded_line_counts() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    let mut counts = vec![];
    for blend in [0.0, 0.01, 0.15, 0.5, 0.85, 0.99, 1.0] {
        state.spatial.enabled = true;
        state.spatial.blend = blend;
        let layout = Layout::for_state(viewport(), &state);
        let projector = state.spatial.projector(state.camera, layout.canvas)?;
        let mut scene = Scene::default();
        axis_view::transition(&mut scene, &state, &layout, &projector, fonts);
        assert!(scene.lines.len() < 400);
        assert!(scene.texts.len() < 80);
        assert!(
            scene
                .lines
                .iter()
                .all(|line| line.color.alpha() >= 0.0 && line.color.alpha() <= 1.0)
        );
        if blend == 0.0 {
            let mut planar = Scene::default();
            axis_view::plane(&mut planar, &state, &layout, fonts);
            assert_eq!(scene.lines.len(), planar.lines.len());
            for (a, b) in scene.lines.iter().zip(planar.lines) {
                assert_eq!((a.from, a.to, a.color), (b.from, b.to, b.color));
            }
        }
        if blend > 0.0 && blend < 1.0 {
            assert!(scene.lines.iter().all(|line| line.color.alpha() < 1.0));
        }
        counts.push(scene.lines.len());
    }
    assert!(counts.iter().all(|count| *count > 10));
    Ok(())
}

#[test]
fn supplemental_z_key_fades_and_cannot_activate_while_leaving_3d() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    state.keypad = true;
    let key = |s: &state::MathState| {
        Layout::for_state(viewport(), s)
            .controls
            .iter()
            .find(|(t, _, _)| *t == Target::SpatialZ)
            .map(|(_, r, _)| *r)
    };
    assert!(key(&state).is_none());
    state.activate(Target::Spatial);
    state.advance_camera(0.016);
    let start = key(&state).unwrap();
    let mut scene = Scene::default();
    let layout = Layout::for_state(viewport(), &state);
    super::super::sidebar_view::draw(&mut scene, &state, &layout, fonts);
    let glyph = scene.texts.iter().find(|t| t.text == "z").unwrap();
    assert!(glyph.color.alpha() > 0.0 && glyph.color.alpha() < 1.0);
    assert_eq!(
        layout.hit(start.center(), false),
        Some(Target::TransitionGuard)
    );
    for _ in 0..16 {
        state.advance_camera(0.016);
    }
    let layout = Layout::for_state(viewport(), &state);
    assert_eq!(
        layout.hit(key(&state).unwrap().center(), false),
        Some(Target::SpatialZ)
    );
    state.activate(Target::SpatialZ);
    assert_eq!(state.document.fields[0].source().unwrap(), "z");
    for _ in 0..150 {
        state.advance_camera(0.016);
    }
    assert!(key(&state).unwrap().y < start.y);
    state.activate(Target::Spatial);
    let layout = Layout::for_state(viewport(), &state);
    assert_ne!(
        layout.hit(key(&state).unwrap().center(), false),
        Some(Target::SpatialZ)
    );
    state.reduced_motion = true;
    state.advance_camera(0.016);
    assert!(key(&state).is_none());
    state.activate(Target::Spatial);
    state.advance_camera(0.016);
    assert_eq!(state.spatial.blend, 1.0);
    Ok(())
}
