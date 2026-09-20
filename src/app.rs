use sim_logic::prelude::*;

use crate::menu::{self, assets::MenuAssets, input::MenuAction};
use crate::{
    navigation,
    phys_editor::{app as editor, assets::EditorAssets},
};

/// Builds the same menu for native presentation and deterministic headless tests.
///
/// Registers six font sizes sharing two faces, five images, and bounded retained
/// screen-space visuals. Decorative animation runs only in FrameUpdate, even
/// during pause. OS link opening is installed separately by the desktop entry point.
pub fn build_application() -> LogicResult<(Application<MenuAction>, WorldFactoryId)> {
    build_with_math(false)
}

pub(crate) fn build_with_math(
    start_math: bool,
) -> LogicResult<(Application<MenuAction>, WorldFactoryId)> {
    let mut config = AppConfig::default();
    config.set_time(TimeConfig::default().with_time_scale(0.0)?);
    // A Math document has no object quota. Retained visual capacity grows from
    // visible demand, not from this ceiling; allocation/device failures remain
    // errors. The standalone Physics configuration and document are unchanged.
    config.set_entity_limit(usize::MAX)?;
    config.set_input_event_limit(128)?;
    // Math's visible graph pool can expand in a short batch rather than
    // revealing a dense plot twelve strokes at a time for several seconds.
    config.set_command_limit(128)?;
    config.set_text_limits(TextLimits::new(6, MenuAssets::font_bytes()));
    config.set_image_asset_limits(ImageAssetLimits::new(5, 768, 768, 5 * 1024 * 1024));
    let defaults = RenderLimits::default();
    let static_entities = editor::ENTITY_COUNT
        .max(menu::backdrop::ENTITY_COUNT + menu::physics_backdrop::ENTITY_COUNT + 128);
    let static_visuals = editor::LINE_COUNT
        .max(menu::backdrop::LINE_COUNT + menu::physics_backdrop::LINE_COUNT)
        + editor::CIRCLE_COUNT
            .max(menu::backdrop::CIRCLE_COUNT + menu::physics_backdrop::CIRCLE_COUNT)
        + editor::RECT_COUNT.max(64)
        + editor::TEXT_COUNT.max(45 + menu::workspace_view::TEXT_COUNT);
    if static_entities.max(static_visuals) > defaults.world_scene_budget().max_commands() {
        return Err("Fixed UI scenes exceed the available Logic scene budget".into());
    }
    config.set_render_limits(
        RenderLimits::new(
            0,
            defaults.world_scene_budget(),
            FrameLimits::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ),
        )
        .with_max_world_lines(0)
        .with_max_world_rectangles(0)
        .with_max_screen_lines(usize::MAX)
        .with_max_screen_circles(usize::MAX)
        // Logic does not re-export the SceneBudget constructor (LOGIC-009).
        // Keep the supported engine work envelope, not a Math document quota.
        .with_screen_scene_budget(defaults.world_scene_budget())
        .with_max_screen_rectangles(usize::MAX)
        .with_max_screen_images(5 + menu::physics_backdrop::IMAGE_COUNT)
        .with_max_screen_texts(usize::MAX)
        .with_max_screen_text_bytes(usize::MAX)
        .with_max_screen_text_glyphs(usize::MAX),
    );
    let mut application = Application::new(config)?;
    application.register_app_resource(WindowControls::default())?;
    application.approve_components::<(
        menu::view::Panel,
        menu::view::Label,
        menu::view::Picture,
        menu::domains_view::DomainCaption,
        menu::backdrop::BackdropLine,
        menu::backdrop::BackdropDot,
        menu::backdrop::BackdropImage,
        menu::workspace_view::WorkspaceCaption,
        menu::physics_backdrop::PhysicsBackdropLine,
        menu::physics_backdrop::PhysicsBackdropDot,
        menu::physics_backdrop::PhysicsBackdropImage,
    )>()?;
    let assets = MenuAssets::register(&mut application)?;
    let editor_assets =
        EditorAssets::register_shared(&mut application, &assets.body, &assets.small)?;
    menu::input::bind(&mut application)?;
    application.add_fallible_frame_system(menu::input::route);
    application.add_fallible_frame_system(menu::backdrop::refresh);
    application.add_fallible_frame_system(menu::view::refresh);
    application.add_fallible_frame_system(menu::domains_view::refresh);
    application.add_fallible_frame_system(menu::workspace_view::refresh);
    application.add_fallible_frame_system(menu::physics_backdrop::refresh);
    application.add_fallible_frame_system(navigation::open_project);
    application.add_frame_system(navigation::initialize_project);
    let editor_factory = editor::install(&mut application, editor_assets)?;
    let math_factory = crate::math_editor::app::install(
        &mut application,
        [
            assets.heading.clone(),
            assets.body.clone(),
            assets.small.clone(),
        ],
    )?;
    application.register_app_resource(navigation::Navigation {
        editor: editor_factory,
        pending: None,
    })?;
    let initial = application.register_world("main-menu", move |world| {
        menu::view::spawn(world, &assets)
            .and_then(|()| menu::domains_view::spawn(world, &assets))
            .and_then(|()| menu::backdrop::spawn(world, &assets))
            .and_then(|()| menu::workspace_view::spawn(world, &assets))
            .and_then(|()| menu::physics_backdrop::spawn(world, &assets))
            .map_err(|error| WorldBuildError::user(error.to_string()))
    })?;
    application.register_app_resource(crate::math_editor::app::Navigation {
        menu: initial,
        editor: math_factory,
        preferences: None,
    })?;
    Ok((application, if start_math { math_factory } else { initial }))
}
