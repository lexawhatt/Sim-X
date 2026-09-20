use super::*;
use crate::math_editor::{
    assets::Fonts,
    drawing::Scene,
    formula::Formula,
    formula_layout::FormulaLayout,
    layout::{Layout, Target},
    worker::{Plot, RowPlot},
};

fn scene(state: &state::MathState, fonts: &Fonts) -> Scene {
    let layout = Layout::for_state(viewport(), state);
    let mut scene = Scene::default();
    super::super::sidebar_view::draw(&mut scene, state, &layout, fonts);
    scene
}

fn hints(scene: &Scene) -> Vec<&super::super::drawing::Text> {
    scene
        .texts
        .iter()
        .filter(|text| text.text.contains("XY curve"))
        .collect()
}

#[test]
fn bare_x_gets_explicit_xy_interpretation_without_editing_the_formula() -> LogicResult {
    let mut runner = runner()?;
    type_text(&mut runner, "x")?;
    until(&mut runner, |s| !s.busy && !s.dirty)?;
    let state = runner.resource::<state::MathState>().unwrap();
    let before = state.document.clone();
    let fonts = runner.resource::<Fonts>().unwrap();
    assert!(hints(&scene(state, fonts)).is_empty());
    click(&mut runner, Target::Spatial)?;
    until(&mut runner, |s| !s.busy && !s.dirty)?;
    let state = runner.resource::<state::MathState>().unwrap();
    let fonts = runner.resource::<Fonts>().unwrap();
    let scene = scene(state, fonts);
    let labels = hints(&scene);
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].text, "y = x  |  XY curve (z = 0)");
    assert!(state.document == before);
    assert!(state.plot.as_ref().unwrap().rows[0].surface.is_empty());
    Ok(())
}

#[test]
fn xy_hint_fades_in_and_out_without_any_row_layout_change() -> LogicResult {
    let runner = runner()?;
    let fonts = runner.resource::<Fonts>().unwrap();
    let mut state = state::MathState::new().unwrap();
    state.document.fields[0] = Formula::typed("x^2");
    state.layouts[0] = FormulaLayout::build(&state.document.fields[0], |style, text| {
        fonts.width(style, text)
    });
    state.plot = Some(Plot {
        rows: vec![RowPlot {
            expression: Some(sim_math::Expression::parse("x^2").unwrap()),
            ..Default::default()
        }],
    });
    state.stale = false;
    let original_rows = Layout::for_state(viewport(), &state).rows;
    let original_fields = Layout::for_state(viewport(), &state).fields;
    let mut previous_y = None;
    for blend in [0.0, 0.25, 0.75, 1.0, 0.75, 0.25, 0.0] {
        state.spatial.blend = blend;
        let layout = Layout::for_state(viewport(), &state);
        assert_eq!(layout.rows, original_rows);
        assert_eq!(layout.fields, original_fields);
        let scene = scene(&state, fonts);
        let labels = hints(&scene);
        if blend == 0.0 {
            assert!(labels.is_empty());
            continue;
        }
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].text, "y = f(x)  |  XY curve (z = 0)");
        assert!((labels[0].color.alpha() - blend as f32).abs() < 1e-6);
        assert!(fonts.width(labels[0].style, &labels[0].text) < layout.sidebar - 62.0);
        if let Some(y) = previous_y {
            assert_eq!(labels[0].position[1], y);
        }
        previous_y = Some(labels[0].position[1]);
    }
    state.spatial.blend = 1.0;
    state.stale = true;
    assert!(hints(&scene(&state, fonts)).is_empty());
    state.stale = false;
    state.plot.as_mut().unwrap().rows[0].diagnostic =
        Some("No finite real values in this view".into());
    assert!(hints(&scene(&state, fonts)).is_empty());
    Ok(())
}

#[test]
fn scalars_surfaces_calculus_and_errors_never_masquerade_as_xy_curves() -> LogicResult {
    let mut runner = runner()?;
    click(&mut runner, Target::Spatial)?;
    for (source, expected) in [
        ("y=x", Some("y = x  |  XY curve (z = 0)")),
        ("y=2", Some("y = f(x)  |  XY curve (z = 0)")),
        ("2", None),
        ("a=2", None),
        ("z=x", None),
        ("int(x)", None),
        ("y=int(x)", None),
        ("1/", None),
    ] {
        click(&mut runner, Target::Field(0))?;
        select_all(&mut runner)?;
        type_text(&mut runner, source)?;
        until(&mut runner, |s| !s.busy && !s.dirty)?;
        let state = runner.resource::<state::MathState>().unwrap();
        let fonts = runner.resource::<Fonts>().unwrap();
        let scene = scene(state, fonts);
        let labels = hints(&scene);
        assert_eq!(
            labels.first().map(|text| text.text.as_str()),
            expected,
            "{source}"
        );
        assert!(labels.len() <= 1);
    }
    Ok(())
}
