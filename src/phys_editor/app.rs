use sim_logic::prelude::*;

use super::{
    assets::EditorAssets,
    catalog_view,
    input::{self, EditorAction},
    session,
    state::EditorState,
    view,
};

pub(crate) const ENTITY_COUNT: usize = view::ENTITY_COUNT + catalog_view::ENTITY_COUNT + 8;
pub(crate) const LINE_COUNT: usize = view::LINE_COUNT + catalog_view::LINE_COUNT;
pub(crate) const CIRCLE_COUNT: usize = view::CIRCLE_COUNT + catalog_view::CIRCLE_COUNT;
pub(crate) const RECT_COUNT: usize = view::RECT_COUNT + catalog_view::RECT_COUNT;
pub(crate) const TEXT_COUNT: usize = view::TEXT_COUNT + catalog_view::TEXT_COUNT;

/// Builds the unified Physics authoring prototype without opening a window.
///
/// Registers bounded retained visuals and editor interactions. Run constructs
/// an independent 2D mechanics world with finite contacts; Editor never
/// advances that world. The document is in memory, capped at 128 objects and
/// 256 links with 64 history entries. Shared by desktop and headless tests.
pub fn build_phys_editor_application() -> LogicResult<(Application<EditorAction>, WorldFactoryId)> {
    let mut config = AppConfig::default();
    // The domain adapter schedules bounded physical steps only in View. Logic's
    // unused fixed schedule stays paused; unscaled frame input/UI remain live.
    config.set_time(TimeConfig::default().with_time_scale(0.0)?);
    config.set_entity_limit(view::ENTITY_COUNT + catalog_view::ENTITY_COUNT + 8)?;
    config.set_input_event_limit(128)?;
    config.set_command_limit(16)?;
    config.set_text_limits(TextLimits::new(3, EditorAssets::font_bytes()));
    let defaults = RenderLimits::default();
    config.set_render_limits(
        RenderLimits::new(
            0,
            defaults.world_scene_budget(),
            FrameLimits::new(128, 8192, 300_000, 16 * 1024 * 1024, 32 * 1024 * 1024, 4096),
        )
        .with_max_world_lines(0)
        .with_max_world_rectangles(0)
        .with_max_screen_lines(view::LINE_COUNT + catalog_view::LINE_COUNT)
        .with_max_screen_circles(view::CIRCLE_COUNT + catalog_view::CIRCLE_COUNT)
        .with_max_screen_rectangles(view::RECT_COUNT + catalog_view::RECT_COUNT)
        .with_max_screen_texts(view::TEXT_COUNT + catalog_view::TEXT_COUNT)
        .with_max_screen_text_bytes(8192)
        .with_max_screen_text_glyphs(4096)
        .with_screen_scene_budget(defaults.world_scene_budget()),
    );
    let mut app = Application::new(config)?;
    app.register_app_resource(WindowControls::default())?;
    let assets = EditorAssets::register(&mut app)?;
    input::bind(&mut app)?;
    let initial = install(&mut app, assets)?;
    Ok((app, initial))
}

/// Installs a scoped editor in either the menu host or the standalone launcher.
pub(crate) fn install(
    app: &mut Application<EditorAction>,
    assets: EditorAssets,
) -> LogicResult<WorldFactoryId> {
    app.approve_component::<view::VisualSlot>()?;
    app.approve_component::<catalog_view::CatalogVisualSlot>()?;
    app.add_fallible_frame_system(input::route);
    app.add_frame_system(session::advance);
    app.add_fallible_frame_system(view::refresh);
    app.add_fallible_frame_system(catalog_view::refresh);
    Ok(app.register_world("physics-editor", move |world| {
        world.insert_resource(EditorState::default())?;
        view::spawn(world, &assets).map_err(|error| WorldBuildError::user(error.to_string()))?;
        catalog_view::spawn(world, &assets)
            .map_err(|error| WorldBuildError::user(error.to_string()))
    })?)
}
