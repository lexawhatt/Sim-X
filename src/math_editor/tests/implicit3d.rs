//! Native input/worker boundary regressions. Clipboard completions are injected
//! by a test system; these tests never install or touch the OS clipboard.
use super::*;
use crate::math_editor::{
    app,
    compute::RowPlot,
    formula,
    interaction::clipboard::ClipboardAction,
    state,
    tests::visual_probe,
    view::{
        assets::Fonts,
        drawing::Scene,
        graph,
        layout::{Layout, Target},
        sidebar,
    },
};

#[test]
fn strict_spatial_comparison_keeps_missing_parameter_suggestions() {
    let mut state = state::MathState::new().unwrap();
    state.document.fields[0] = formula::Formula::typed("x^2+y^2+z^2<r^2");
    state.scan_parameters();
    assert_eq!(state.missing_parameters[0], vec!['r']);
    assert!(state.parameters[0].is_none());
}

fn type_math(runner: &mut HeadlessRunner<crate::actions::AppAction>, source: &str) -> LogicResult {
    use PhysicalKeyCode::*;
    for ch in source.chars() {
        let shifted = match ch {
            '<' => Some(Comma),
            '>' => Some(Period),
            '|' => Some(Backslash),
            _ => None,
        };
        if let Some(code) = shifted {
            frame(
                runner,
                &[
                    key(ShiftLeft, ButtonState::Pressed),
                    key(code, ButtonState::Pressed),
                    key(code, ButtonState::Released),
                    key(ShiftLeft, ButtonState::Released),
                ],
            )?;
        } else {
            type_text(runner, &ch.to_string())?;
        }
    }
    Ok(())
}

fn settled(runner: &mut HeadlessRunner<crate::actions::AppAction>) -> LogicResult {
    // This is a deadlock guard, not a performance assertion: implicit sampling
    // can be substantially more expensive than the scalar/2D test helper.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let state = runner.resource::<state::MathState>().unwrap();
        if !state.dirty && !state.busy && !state.stale {
            return Ok(());
        }
        assert!(
            std::time::Instant::now() < deadline,
            "implicit worker did not settle: {}",
            state.status
        );
        frame(runner, &[])?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn row(runner: &HeadlessRunner<crate::actions::AppAction>, index: usize) -> &RowPlot {
    &runner
        .resource::<state::MathState>()
        .unwrap()
        .plot
        .as_ref()
        .unwrap()
        .rows[index]
}

fn radius(row: &RowPlot) -> f64 {
    assert!(!row.surface.is_empty());
    row.surface
        .iter()
        .flatten()
        .flatten()
        .map(|p| p.iter().map(|v| v * v).sum::<f64>().sqrt())
        .fold(0.0, f64::max)
}

fn preview(runner: &mut HeadlessRunner<crate::actions::AppAction>) -> LogicResult {
    if std::env::var_os("SIM_X_MATH_PREVIEW_SVG").is_none() {
        return Ok(());
    }
    for _ in 0..150 {
        frame(runner, &[])?;
    }
    let state = runner.resource::<state::MathState>().unwrap();
    let fonts = runner.resource::<Fonts>().unwrap();
    let layout = Layout::for_state(viewport(), state);
    let mut scene = Scene::default();
    graph::draw(&mut scene, state, &layout, fonts);
    visual_probe::export(&scene, layout.canvas);
    Ok(())
}

#[test]
fn native_sphere_is_recognized_in_2d_and_sampled_in_3d_without_rewriting_source() -> LogicResult {
    let mut runner = runner()?;
    type_math(&mut runner, "x^2+y^2+z^2=9")?;
    settled(&mut runner)?;
    let original = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .clone();
    assert!(row(&runner, 0).diagnostic.as_ref().unwrap().contains("3D"));
    assert!(
        !row(&runner, 0)
            .diagnostic
            .as_ref()
            .unwrap()
            .contains("syntax")
    );
    assert!(row(&runner, 0).surface.is_empty());
    click(&mut runner, Target::Spatial)?;
    settled(&mut runner)?;
    let sphere = row(&runner, 0);
    assert!((radius(sphere) - 3.0).abs() < 0.15);
    assert!(
        sphere
            .surface
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .all(|v| v.is_finite())
    );
    assert!(sphere.points.is_empty());
    assert_eq!(sphere.boundary, Some(sim_math::relation::Relation::Equal));
    assert!(runner.resource::<state::MathState>().unwrap().document == original);
    preview(&mut runner)?;
    click(&mut runner, Target::Spatial)?;
    for _ in 0..150 {
        frame(&mut runner, &[])?;
    }
    settled(&mut runner)?;
    assert!(runner.resource::<state::MathState>().unwrap().document == original);
    assert!(row(&runner, 0).surface.is_empty());
    assert!(row(&runner, 0).diagnostic.as_ref().unwrap().contains("3D"));
    Ok(())
}

#[test]
fn sphere_parameter_updates_the_existing_radius_and_latest_worker_result() -> LogicResult {
    let mut runner = runner()?;
    type_math(&mut runner, "x^2+y^2+z^2=r^2")?;
    click(&mut runner, Target::Add)?;
    type_text(&mut runner, "r=2")?;
    click(&mut runner, Target::Spatial)?;
    settled(&mut runner)?;
    let first = radius(row(&runner, 0));
    assert!((first - 2.0).abs() < 0.15);
    click(&mut runner, Target::Field(1))?;
    select_all(&mut runner)?;
    type_text(&mut runner, "r=3")?;
    settled(&mut runner)?;
    let updated = radius(row(&runner, 0));
    assert!((updated - 3.0).abs() < 0.15);
    assert!(updated > first + 0.7);
    assert_eq!(row(&runner, 1).scalar, Some(3.0));
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .missing_parameters[0]
            .is_empty()
    );
    Ok(())
}

#[test]
fn native_absolute_cube_retains_open_and_closed_inequality_semantics() -> LogicResult {
    let mut runner = runner()?;
    type_math(&mut runner, "max(|x|,|y|,|z|)<=1")?;
    let source = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .fields[0]
        .source()
        .unwrap();
    assert!(source.contains("abs(x)") && source.ends_with("<=1"));
    click(&mut runner, Target::Spatial)?;
    settled(&mut runner)?;
    let closed = row(&runner, 0).diagnostic.clone().unwrap();
    assert!(closed.to_lowercase().contains("boundary"), "{closed}");
    assert!(!row(&runner, 0).surface.is_empty());
    assert_eq!(
        row(&runner, 0).boundary,
        Some(sim_math::relation::Relation::LessOrEqual)
    );
    for p in row(&runner, 0).surface.iter().flatten().flatten() {
        let extent = p.iter().map(|v| v.abs()).fold(0.0, f64::max);
        assert!((extent - 1.0).abs() < 0.15, "cube point {p:?}");
    }
    preview(&mut runner)?;
    click(&mut runner, Target::Field(0))?;
    select_all(&mut runner)?;
    type_math(&mut runner, "max(|x|,|y|,|z|)<1")?;
    settled(&mut runner)?;
    let open = row(&runner, 0).diagnostic.as_ref().unwrap();
    assert!(open.to_lowercase().contains("boundary"), "{open}");
    assert_ne!(
        open, &closed,
        "strict inequality must not look closed in its explanation"
    );
    assert!(!row(&runner, 0).surface.is_empty());
    assert_eq!(
        row(&runner, 0).boundary,
        Some(sim_math::relation::Relation::Less)
    );
    let source = runner
        .resource::<state::MathState>()
        .unwrap()
        .document
        .fields[0]
        .source()
        .unwrap();
    assert!(source.ends_with("<1") && !source.contains("<="));
    Ok(())
}

fn complete_test_paste(state: Option<ResMut<state::MathState>>) -> LogicResult {
    if let Some(mut state) = state
        && state
            .clipboard_pending
            .as_ref()
            .is_some_and(|request| request.action == ClipboardAction::Paste)
    {
        state.complete_clipboard(Ok("max(|x|,|y|,|z|) \u{2264} 1".into()));
    }
    Ok(())
}

#[test]
fn unicode_cube_paste_uses_native_input_and_replacing_it_with_y_x_never_extrudes() -> LogicResult {
    use PhysicalKeyCode::*;
    let (mut app, initial) = app::build_math_editor_application()?;
    app.add_fallible_frame_system(complete_test_paste);
    let mut runner = app.build_headless(initial)?;
    frame(&mut runner, &[])?;
    frame(
        &mut runner,
        &[
            key(ControlLeft, ButtonState::Pressed),
            key(KeyV, ButtonState::Pressed),
            key(KeyV, ButtonState::Released),
            key(ControlLeft, ButtonState::Released),
        ],
    )?;
    settled(&mut runner)?;
    assert!(
        runner
            .resource::<state::MathState>()
            .unwrap()
            .document
            .fields[0]
            .source()
            .unwrap()
            .ends_with("<=1")
    );
    click(&mut runner, Target::Spatial)?;
    settled(&mut runner)?;
    assert!(!row(&runner, 0).surface.is_empty());
    click(&mut runner, Target::Field(0))?;
    select_all(&mut runner)?;
    type_text(&mut runner, "y=x")?;
    settled(&mut runner)?;
    assert!(row(&runner, 0).boundary.is_none());
    assert!(row(&runner, 0).surface.is_empty());
    assert!(row(&runner, 0).points.iter().flatten().any(|p| p.x != 0.0));
    assert!(
        row(&runner, 0)
            .points
            .iter()
            .flatten()
            .all(|p| (p.x - p.y).abs() < 1e-12)
    );
    for _ in 0..15 {
        frame(&mut runner, &[])?;
    }
    assert!(row(&runner, 0).surface.is_empty());
    Ok(())
}

#[test]
fn implicit_boundary_status_fits_the_narrow_sidebar_without_overlapping_the_canvas() -> LogicResult
{
    let mut runner = runner()?;
    click(&mut runner, Target::Spatial)?;
    for source in ["x^2+y^2+z^2=9", "max(|x|,|y|,|z|)<=1", "max(|x|,|y|,|z|)<1"] {
        click(&mut runner, Target::Field(0))?;
        select_all(&mut runner)?;
        type_math(&mut runner, source)?;
        settled(&mut runner)?;
        let state = runner.resource::<state::MathState>().unwrap();
        let fonts = runner.resource::<Fonts>().unwrap();
        let layout = Layout::for_state(LogicalViewport::new(900.0, 640.0)?, state);
        let mut scene = Scene::default();
        sidebar::draw(&mut scene, state, &layout, fonts);
        let message = row(&runner, 0).diagnostic.as_ref().unwrap();
        let rendered = scene
            .texts
            .iter()
            .find(|text| &text.text == message)
            .unwrap();
        assert!(
            rendered.position[0] + fonts.width(rendered.style, message) <= layout.sidebar - 12.0,
            "status does not fit: {message}"
        );
    }
    Ok(())
}
