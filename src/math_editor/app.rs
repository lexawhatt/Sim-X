//! Independent Math world installed into the same Sim;Logic application host.
use crate::math_editor::{
    interaction,
    state::MathState,
    view::{self, assets::Fonts},
};
use crate::{actions::AppAction, menu::state::MenuState};
use sim_logic::prelude::*;

/// Opens the experimental Euclidean Math workspace directly, without a window
/// until the caller chooses native or headless execution. Physics is unchanged.
pub fn build_math_editor_application() -> LogicResult<(Application<AppAction>, WorldFactoryId)> {
    crate::app::build_with_math(true)
}

pub(crate) struct Navigation {
    pub menu: WorldFactoryId,
    pub editor: WorldFactoryId,
    pub preferences: Option<(bool, bool)>,
}

pub(crate) fn install(
    app: &mut Application<AppAction>,
    fonts: [TextFont; 3],
) -> LogicResult<WorldFactoryId> {
    let fonts = Fonts::new(fonts)?;
    app.approve_component::<view::Slot>()?;
    for key in [
        PhysicalKeyCode::Equal,
        PhysicalKeyCode::Period,
        PhysicalKeyCode::End,
        PhysicalKeyCode::Comma,
        PhysicalKeyCode::Backslash,
    ] {
        app.bind_key(key, AppAction::CatalogInput)?;
    }
    app.add_fallible_frame_system(navigate);
    app.add_fallible_frame_system(interaction::route);
    app.add_fallible_frame_system(view::refresh);
    Ok(app.register_world("math-editor", move |world| {
        world
            .insert_resource(MathState::new().map_err(|e| WorldBuildError::user(e.to_string()))?)?;
        world.insert_resource(fonts.clone())?;
        view::spawn(world, &fonts).map_err(|e| WorldBuildError::user(e.to_string()))
    })?)
}
fn navigate(
    menu: Option<ResMut<MenuState>>,
    math: Option<ResMut<MathState>>,
    mut navigation: AppResMut<Navigation>,
    mut commands: Commands,
) -> LogicResult {
    if let Some(mut menu) = menu {
        if let Some((fullscreen, reduced_motion)) = navigation.preferences.take() {
            menu.fullscreen_requested = fullscreen;
            menu.reduced_motion = reduced_motion;
        }
        if menu.pending_math {
            menu.pending_math = false;
            navigation.preferences = Some((menu.fullscreen_requested, menu.reduced_motion));
            let intent = commands.new_transition_intent()?;
            commands.replace_world(intent, navigation.editor)?;
        }
    }
    if let Some(mut math) = math {
        // Math owns frame input and asynchronous calculations, not a fixed
        // simulation. A zero time scale alone retains fixed edges forever.
        // Pause clears that unused queue while frame UI/motion stay active.
        commands.set_paused(true)?;
        if let Some((fullscreen, reduced_motion)) = navigation.preferences.take() {
            math.fullscreen = fullscreen;
            math.reduced_motion = reduced_motion;
        }
        if math.leaving {
            navigation.preferences = Some((math.fullscreen, math.reduced_motion));
            let intent = commands.new_transition_intent()?;
            commands.replace_world(intent, navigation.menu)?;
        }
    }
    Ok(())
}
