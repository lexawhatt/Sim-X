use super::*;
use crate::math_editor::{
    assets::Fonts,
    drawing::Scene,
    graph_style::GraphStyle,
    layout::{Camera, Rect},
    spatial::Spatial,
    spatial_clip::Bounds,
    wireframe_view::Wireframe,
};

#[test]
fn depth_clipping_retains_crossings_instead_of_rejecting_outside_endpoints() {
    let rect = Rect::new(300.0, 66.0, 1000.0, 800.0);
    let projector = Spatial::default()
        .projector(Camera::default(), rect)
        .unwrap();
    let points = [[-1.0, 0.0, -1000.0], [1.0, 0.0, 1000.0]];
    assert!(projector.point(points[0]).is_none());
    assert!(projector.point(points[1]).is_none());
    let segment = projector.segment(points).unwrap();
    assert!((segment.depths[0] - 128.0).abs() < 1e-6);
    assert!((segment.depths[1] - 0.01).abs() < 1e-6);
    assert!(segment.points.iter().flatten().all(|v| v.is_finite()));
    assert!(
        projector
            .segment([[0.0, 0.0, 1000.0], [1.0, 0.0, 2000.0]])
            .is_none()
    );
}

#[test]
fn wireframe_respects_world_bounds_gaps_and_user_opacity() {
    let rect = Rect::new(300.0, 66.0, 1000.0, 800.0);
    let camera = Camera::default();
    let spatial = Spatial {
        enabled: true,
        blend: 1.0,
        ..Default::default()
    };
    let projector = spatial.projector(camera, rect).unwrap();
    let bounds = Bounds {
        ranges: [[-2.0, 2.0]; 3],
    };
    let wire = Wireframe::new(&projector, bounds, rect);
    let mut style = GraphStyle::new(0);
    style.opacity = 0.4;
    let mut out = Scene::default();
    wire.strip(
        &mut out,
        [Some([-1.0, 0.0, 100.0]), Some([1.0, 0.0, 100.0])],
        style,
    );
    assert!(out.lines.is_empty());
    wire.strip(
        &mut out,
        [Some([-1.0, 0.0, -1.0]), None, Some([1.0, 0.0, 1.0])],
        style,
    );
    assert!(out.lines.is_empty());
    wire.strip(
        &mut out,
        [Some([0.0, 0.0, -100.0]), Some([0.0, 0.0, 100.0])],
        style,
    );
    assert!(!out.lines.is_empty());
    let top = projector.point([0.0, 0.0, 2.0]).unwrap()[1];
    let bottom = projector.point([0.0, 0.0, -2.0]).unwrap()[1];
    for line in &out.lines {
        assert!((top - 0.01..=bottom + 0.01).contains(&line.from[1]));
        assert!((top - 0.01..=bottom + 0.01).contains(&line.to[1]));
        assert!(line.color.alpha() <= 0.4 && line.color.alpha() > 0.0);
        assert_eq!(line.depth, 0.8);
    }
    let mut alphas: Vec<_> = out.lines.iter().map(|l| l.color.alpha()).collect();
    alphas.sort_by(f32::total_cmp);
    assert!(alphas.last().unwrap() - alphas.first().unwrap() > 0.01);
}

#[test]
fn home_uses_nearest_turn_and_cancels_old_pointer_ownership() {
    let mut state = state::MathState::new().unwrap();
    state.spatial.yaw = std::f64::consts::TAU * 20.0;
    state.spatial.drag = Some(([0.0, 0.0], [0.0, 0.0]));
    state.reset_view();
    assert!(state.spatial.drag.is_none());
    assert!((state.spatial.target[0] - state.spatial.yaw).abs() <= std::f64::consts::PI);
    assert!((state.spatial.target[1].to_degrees() - 30.0).abs() < 1e-12);
}

#[test]
fn clipped_saddle_cpu_preview_and_protected_labels() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new()?;
    state.spatial.enabled = true;
    state.spatial.blend = 1.0;
    state.keypad = false;
    state.stale = false;
    state.motion.plot_opacity = 1.0;
    state.document.fields[0] = formula::Formula::typed("z=xy");
    let viewport = LogicalViewport::new(1920.0, 1080.0)?;
    let layout = layout::Layout::for_state(viewport, &state);
    let bounds = Bounds::for_view(state.camera, layout.canvas).unwrap();
    let low = sim_math::geometry::Point {
        x: bounds.ranges[0][0],
        y: bounds.ranges[1][0],
    };
    let high = sim_math::geometry::Point {
        x: bounds.ranges[0][1],
        y: bounds.ranges[1][1],
    };
    let surface = sim_math::surface::wireframe(
        &sim_math::Expression::with_variables("x*y", &Default::default()).unwrap(),
        low,
        high,
        [(layout.canvas.w.min(layout.canvas.h) * 0.56 / 40.0).ceil() as usize; 2],
        8,
        || true,
    )
    .unwrap();
    assert!(
        surface
            .iter()
            .flatten()
            .flatten()
            .any(|p| p[2].abs() > bounds.ranges[2][1] * 4.0)
    );
    state.plot = Some(worker::Plot {
        rows: vec![worker::RowPlot {
            surface,
            ..Default::default()
        }],
    });
    let mut out = Scene::default();
    super::super::graph_view::draw(&mut out, &state, &layout, fonts);
    assert!(out.lines.iter().filter(|l| l.depth == 0.8).count() > 30);
    assert!(
        out.lines.len() < 2000,
        "viewport-sized density, not a document cap"
    );
    let hint = out
        .texts
        .iter()
        .find(|t| t.text.starts_with("Orthographic"))
        .unwrap();
    assert!(
        out.panels
            .iter()
            .any(|p| p.depth == 5.0 && p.rect.contains(hint.position))
    );
    assert!(
        out.texts
            .iter()
            .filter(|t| t.text.parse::<f64>().is_ok())
            .all(|t| out
                .panels
                .iter()
                .any(|p| p.depth == 4.0 && p.rect.contains(t.position)))
    );
    super::super::visual_probe::export(&out, layout.canvas);
    Ok(())
}
