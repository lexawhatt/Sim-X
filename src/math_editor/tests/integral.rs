use super::*;
use crate::math_editor::{
    compute::{CalculationCache, Plot, Request, RowPlot, integrals::IntegralPlot},
    formula,
    scene::{integrals, style::GraphStyle},
    state,
    tests::visual_probe,
    view::{
        assets,
        drawing::Scene,
        graph,
        layout::{self, Camera, Rect},
    },
};
use sim_math::{calculus::IntegralVisualization, geometry::Point, statement::Statement};
use std::collections::BTreeMap;

fn request() -> Request {
    Request {
        sources: vec![],
        calculate: vec![],
        lower: Point { x: -2.0, y: -2.0 },
        upper: Point { x: 2.0, y: 2.0 },
        pixels: [600.0, 600.0],
        spatial: false,
    }
}
fn plan(source: &str) -> IntegralVisualization {
    match Statement::parse(source, &BTreeMap::new()).unwrap() {
        Statement::IntegralExpression { expression, .. } => expression.visualization(),
        Statement::Integral {
            body, lower, upper, ..
        } => IntegralVisualization::single(body, [lower, upper]),
        _ => panic!("not an integral"),
    }
}
fn picture(source: &str) -> IntegralPlot {
    IntegralPlot::prepare(plan(source), &request(), &|| true).unwrap()
}
fn draw(picture: &IntegralPlot, progress: Option<f32>) -> Scene {
    let mut scene = Scene::default();
    let rect = Rect::new(0.0, 0.0, 600.0, 600.0);
    let camera = Camera {
        x: 0.0,
        y: 0.0,
        scale: 150.0,
    };
    integrals::draw(
        &mut scene,
        picture,
        GraphStyle::new(0),
        rect,
        progress,
        |p| camera.project(p, rect),
        true,
    );
    scene
}
fn signed_area(scene: &Scene) -> f64 {
    scene
        .panels
        .iter()
        .map(|p| {
            f64::from(p.rect.w * p.rect.h) / 150.0_f64.powi(2)
                * if p.color.red() == GraphStyle::new(0).tint().red() {
                    1.0
                } else {
                    -1.0
                }
        })
        .sum()
}

#[test]
fn common_interval_fill_follows_the_two_curves_and_reveal_front() {
    let picture = picture("integral(0,1,x)-integral(0,1,x^2)");
    assert_eq!(picture.curves.len(), 2);
    assert_eq!(picture.bands.len(), 1);
    assert!(draw(&picture, None).panels.is_empty());
    assert!(draw(&picture, Some(0.0)).panels.is_empty());
    let half = draw(&picture, Some(0.5));
    assert!(!half.panels.is_empty());
    assert!(
        half.panels
            .iter()
            .all(|p| p.rect.x >= 300.0 && p.rect.x + p.rect.w <= 375.001)
    );
    let full = draw(&picture, Some(1.0));
    for panel in &full.panels {
        assert!(panel.rect.x >= 300.0 && panel.rect.x + panel.rect.w <= 450.001);
        assert!(panel.rect.y + panel.rect.h < 300.001);
    }
    assert!((signed_area(&full) - 1.0 / 6.0).abs() < 1e-4);
    assert!(signed_area(&half) < signed_area(&full));
}

#[test]
fn crossings_and_reversed_limits_keep_signed_and_geometric_area_distinct() {
    let crossing = draw(&picture("integral(-1,1,x)-integral(-1,1,0)"), Some(1.0));
    assert!(
        crossing
            .panels
            .iter()
            .any(|p| p.color.red() == GraphStyle::new(0).tint().red())
    );
    assert!(
        crossing
            .panels
            .iter()
            .any(|p| p.color.red() == integrals::negative_color(GraphStyle::new(0)).red())
    );
    assert!(signed_area(&crossing).abs() < 1e-6);
    let unsigned: f64 = crossing
        .panels
        .iter()
        .map(|p| f64::from(p.rect.w * p.rect.h) / 22500.0)
        .sum();
    assert!((unsigned - 1.0).abs() < 1e-5);
    let reversed = draw(&picture("integral(1,0,x)-integral(1,0,x^2)"), Some(1.0));
    assert!((signed_area(&reversed) + 1.0 / 6.0).abs() < 1e-4);
    let single = draw(&picture("integral(1,0,x)"), Some(1.0));
    assert!((signed_area(&single) + 0.5).abs() < 1e-5);
    assert!(single.panels.iter().all(|p| p.rect.y < 300.0));
}

#[test]
fn differing_bounds_remain_separate_and_nonlinear_results_are_not_filled() {
    let separate = picture("integral(0,1,x)-integral(0,2,x^2)");
    assert_eq!(separate.bands.len(), 2);
    assert_eq!(separate.bands[0].bounds, [0.0, 1.0]);
    assert_eq!(separate.bands[1].bounds, [0.0, 2.0]);
    let nonlinear = picture("sin(integral(0,1,x))");
    assert_eq!(nonlinear.curves.len(), 1);
    assert!(nonlinear.bands.is_empty());
    assert!(draw(&nonlinear, Some(1.0)).panels.is_empty());
}

#[test]
fn exact_subpixel_bounds_domain_gaps_and_sampling_cancellation() {
    let tiny = picture("integral(0.123456,0.123457,1)");
    let samples = &tiny.bands[0].sections;
    assert_eq!(samples.first().unwrap().unwrap().x, 0.123456);
    assert_eq!(samples.last().unwrap().unwrap().x, 0.123457);
    let pole = picture("integral(-1,1,1/x)");
    assert!(pole.bands[0].sections.iter().any(Option::is_none));
    let calls = std::cell::Cell::new(0);
    let result = IntegralPlot::prepare(
        plan("integral(0,1,x)-integral(0,1,x^2)"),
        &request(),
        &|| {
            calls.set(calls.get() + 1);
            calls.get() < 30
        },
    );
    assert!(matches!(result, Err(sim_math::MathError::Cancelled)));
}

#[test]
fn each_row_reveals_only_after_its_own_successful_completion() {
    let mut state = state::MathState::new().unwrap();
    state.document.fields.push(formula::Formula::default());
    let completed = RowPlot {
        scalar: Some(0.5),
        integral_plot: Some(picture("integral(0,1,x)")),
        ..Default::default()
    };
    let pending = RowPlot {
        scalar: None,
        pending: true,
        ..completed.clone()
    };
    state.plot = Some(Plot {
        rows: vec![completed, pending],
    });
    state.stale = false;
    state.advance_integral_reveal(2.0);
    assert_eq!(state.area_progress, [1.0, 0.0]);
    let row = &mut state.plot.as_mut().unwrap().rows[1];
    row.pending = false;
    row.scalar = Some(0.5);
    state.advance_integral_reveal(0.016);
    assert_eq!(state.area_progress[0], 1.0);
    assert!(state.area_progress[1] > 0.0 && state.area_progress[1] < 0.1);
    state.reduced_motion = true;
    state.advance_integral_reveal(0.016);
    assert_eq!(state.area_progress, [1.0, 1.0]);
    state.stale = true;
    state.advance_integral_reveal(1.0);
    assert_eq!(state.area_progress, [0.0, 0.0]);
}

#[test]
fn calculate_only_cancels_the_row_that_is_pending() {
    let mut state = state::MathState::new().unwrap();
    state.document.fields = vec![
        formula::Formula::typed("int(x)"),
        formula::Formula::typed("int(x^2)"),
    ];
    state.calculate = vec![true, true];
    state.busy = true;
    state.plot = Some(Plot {
        rows: vec![
            RowPlot {
                scalar: Some(0.5),
                ..Default::default()
            },
            RowPlot {
                pending: true,
                ..Default::default()
            },
        ],
    });
    state.activate(layout::Target::Calculate(0));
    assert!(
        state.calculate[0],
        "a finished row must not act like Cancel because another row is busy"
    );
    state.activate(layout::Target::Calculate(1));
    assert!(!state.calculate[1]);
}

#[test]
fn starting_another_row_preserves_completed_results_and_reveal() {
    let mut state = state::MathState::new().unwrap();
    state.document.fields = vec![
        formula::Formula::typed("int(x)"),
        formula::Formula::typed("int(x^2)"),
    ];
    state.calculate = vec![true, false];
    state.area_progress = vec![1.0, 0.0];
    state.stale = false;
    let completed = RowPlot {
        scalar: Some(0.5),
        integral_plot: Some(picture("integral(0,1,x)")),
        ..Default::default()
    };
    state.plot = Some(Plot {
        rows: vec![completed, RowPlot::default()],
    });
    state.activate(layout::Target::Calculate(1));
    state.advance_integral_reveal(0.016);
    assert_eq!(state.plot.as_ref().unwrap().rows[0].scalar, Some(0.5));
    assert_eq!(state.area_progress, [1.0, 0.0]);
    assert!(state.plot.as_ref().unwrap().rows[1].pending);
}

#[test]
fn view_only_resampling_reuses_scalars_but_edit_or_cancel_invalidates_them() {
    let mut cache = CalculationCache::default();
    let mut request = request();
    request.sources = vec![Ok("integral(0,1,x)".into()), Ok("b=2".into())];
    request.calculate = vec![true, false];
    cache.prepare(&request);
    cache.record(0, (0.5, None));
    request.lower.x = -100.0;
    request.upper.x = 100.0;
    request.pixels = [1900.0, 900.0];
    request.spatial = true;
    cache.prepare(&request);
    assert_eq!(cache.get(0).unwrap().0, 0.5);
    request.calculate[1] = true;
    cache.prepare(&request);
    assert_eq!(cache.get(0).unwrap().0, 0.5);
    request.sources[1] = Ok("b=3".into());
    cache.prepare(&request);
    assert!(cache.get(0).is_none());
    cache.record(0, (0.5, None));
    request.calculate[0] = false;
    cache.prepare(&request);
    assert!(cache.get(0).is_none());
}

#[test]
fn native_input_difference_displays_curves_then_signed_fill_and_follows_edits() -> LogicResult {
    let mut runner = runner()?;
    frame(&mut runner, &[])?;
    type_text(&mut runner, "int(x)-int(x^2)")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    assert_eq!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .plot
            .as_ref()
            .unwrap()
            .rows[0]
            .integral_plot
            .as_ref()
            .unwrap()
            .curves
            .len(),
        2
    );
    click(&mut runner, layout::Target::Calculate(0))?;
    until(&mut runner, |s| !s.busy && !s.dirty)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!((s.plot.as_ref().unwrap().rows[0].scalar.unwrap() - 1.0 / 6.0).abs() < 1e-8);
    for _ in 0..60 {
        frame(&mut runner, &[])?;
    }
    let s = runner.resource::<state::MathState>().unwrap();
    let layout = layout::Layout::for_state(viewport(), s);
    let mut scene = Scene::default();
    let fonts = runner.resource::<assets::Fonts>().unwrap().clone();
    graph::draw(&mut scene, s, &layout, &fonts);
    assert!(!scene.panels.is_empty());
    let projector = s.spatial.projector(s.camera, layout.canvas)?;
    let mut spatial = Scene::default();
    integrals::draw(
        &mut spatial,
        s.plot.as_ref().unwrap().rows[0]
            .integral_plot
            .as_ref()
            .unwrap(),
        GraphStyle::new(0),
        layout.canvas,
        Some(1.0),
        |p| projector.point([p.x, p.y, 0.0]),
        false,
    );
    assert!(spatial.panels.is_empty());
    assert!(spatial.lines.iter().all(|line| line.depth != 0.5));
    // A failed subsequent integral must remove the previously successful fill.
    click(&mut runner, layout::Target::Field(0))?;
    select_all(&mut runner)?;
    type_text(&mut runner, "int(1/x)")?;
    until(&mut runner, |s| !s.dirty && !s.busy)?;
    let s = runner.resource::<state::MathState>().unwrap();
    assert!(s.plot.as_ref().unwrap().rows[0].scalar.is_none());
    assert!(s.plot.as_ref().unwrap().rows[0].diagnostic.is_some());
    let mut scene = Scene::default();
    graph::draw(&mut scene, s, &layout, &fonts);
    assert!(scene.panels.is_empty());
    Ok(())
}

#[test]
fn integral_region_cpu_preview() -> LogicResult {
    let runner = runner()?;
    let mut state = state::MathState::new().unwrap();
    state.camera = Camera {
        x: 0.5,
        y: 0.5,
        scale: 300.0,
    };
    state.stale = false;
    state.area_progress = vec![1.0];
    let mut request = request();
    request.lower = Point { x: -1.0, y: -1.0 };
    request.upper = Point { x: 2.0, y: 2.0 };
    request.pixels = [900.0, 900.0];
    state.plot = Some(Plot {
        rows: vec![RowPlot {
            scalar: Some(1.0 / 6.0),
            integral_plot: Some(
                IntegralPlot::prepare(plan("integral(0,1,x)-integral(0,1,x^2)"), &request, &|| {
                    true
                })
                .unwrap(),
            ),
            ..Default::default()
        }],
    });
    state.motion.plot_opacity = 1.0;
    let layout = layout::Layout::for_state(viewport(), &state);
    let mut scene = Scene::default();
    graph::draw(
        &mut scene,
        &state,
        &layout,
        runner.resource::<assets::Fonts>().unwrap(),
    );
    visual_probe::export(&scene, layout.canvas);
    Ok(())
}
