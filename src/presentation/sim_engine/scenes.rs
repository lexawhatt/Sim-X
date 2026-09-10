use std::collections::{BTreeMap, BTreeSet};

use ::sim_engine::{Camera2d, Color, LogicalViewport, ShapeStyle, Vec2};

use crate::{
    domains::phys::{
        electromagnetism::ElectrostaticSnapshot,
        mechanics::{Force2, MechanicsSnapshot},
        thermodynamics::ThermalSnapshot,
        waves_optics::WaveSnapshot,
    },
    presentation::ui::{
        MechanicsProjectSnapshotRef, PhysicsSnapshotRef,
        catalog::{
            MainMenuItem, MechanicsTool, PhysSubdomain, PlaybackRate, ProjectTemplate, Screen,
            SocialLink, TopDomain, ViewExitChoice,
        },
        geometry::{Point, UiRect},
        layout::UiLayout,
        state::{Notice, UiState},
    },
};

use super::{
    pixel_font,
    scene_builder::{BuiltFrame, UiSceneBuilder as Scene},
};

pub(super) fn build(
    layout: UiLayout,
    state: &UiState,
    snapshot: Option<PhysicsSnapshotRef<'_>>,
    physics_error: Option<&str>,
    scientific_camera: Camera2d,
) -> Result<BuiltFrame, String> {
    let has_view_exit_overlay = state.screen == Screen::PhysicsViewExit;
    let scene = match state.screen {
        Screen::MainMenu => build_main_menu(layout, state),
        Screen::Settings => build_settings(layout, state),
        Screen::Domains => build_domain_menu(layout, state),
        Screen::PhysSubdomains => build_phys_menu(layout, state),
        Screen::Projects => build_projects(layout, state),
        Screen::PhysicsEditor => {
            build_physics_editor(layout, state, snapshot, physics_error, scientific_camera)
        }
        Screen::PhysicsView => {
            build_physics_view(layout, state, snapshot, physics_error, scientific_camera)
        }
        Screen::PhysicsViewExit => {
            build_physics_view(layout, state, snapshot, physics_error, scientific_camera)
        }
        Screen::TimeEasterEgg => build_time_easter_egg(layout, state),
    };
    let mut frame = scene.finish()?;
    if has_view_exit_overlay {
        frame.overlay = Some(
            build_physics_view_exit_overlay(layout, state)
                .finish()?
                .screen,
        );
    }
    Ok(frame)
}

fn build_main_menu(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background());
    let compact = layout.height < 700.0;
    let title_y = if compact { 104.0 } else { 138.0 };
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "SIM;X",
        Point::new(layout.width * 0.5, title_y),
        if compact { 8.0 } else { 10.0 },
        text(),
    );
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "SCIENTIFIC WORLDS YOU CAN TOUCH",
        Point::new(layout.width * 0.5, title_y + 66.0),
        2.2,
        muted(),
    );

    for ((item, rect), hover) in MainMenuItem::ALL
        .into_iter()
        .zip(layout.main_menu_buttons())
        .zip(state.animations.main_hover)
    {
        draw_button(&mut scene, layout, rect, item.label(), hover, false);
    }

    draw_social_links(&mut scene, layout, state);
    draw_time_code_hotspot(&mut scene, layout, state);
    let message = match state.notice {
        Some(Notice::ExternalLinkFailed(link)) => match link {
            SocialLink::GitHub => "COULD NOT OPEN GITHUB",
            SocialLink::YouTube => "COULD NOT OPEN YOUTUBE",
            SocialLink::Telegram => "COULD NOT OPEN TELEGRAM",
        },
        _ => "ENTER OR 1   DOMAINS     S OR 2   SETTINGS",
    };
    draw_footer(&mut scene, layout, message);
    scene
}

fn draw_social_links(scene: &mut Scene, layout: UiLayout, state: &UiState) {
    for ((link, rect), hover) in SocialLink::ALL
        .into_iter()
        .zip(layout.social_link_buttons())
        .zip(state.animations.social_hover)
    {
        draw_social_icon(scene, layout, rect, link, hover);
        if hover > 0.02 {
            pixel_font::draw_centered(
                scene,
                layout,
                link.label(),
                Point::new(rect.center().x, rect.min.y - 15.0),
                1.25,
                text().with_alpha(hover),
            );
        }
    }
}

fn draw_social_icon(
    scene: &mut Scene,
    _layout: UiLayout,
    rect: UiRect,
    link: SocialLink,
    hover: f32,
) {
    let amount = hover.clamp(0.0, 1.0);
    let rect = rect.expand(amount * 2.0);
    scene.rect(
        ui_rect(rect),
        10.0,
        ShapeStyle::fill_stroke(
            mix_color(surface(), surface_hover(), amount),
            1.0 + amount,
            mix_color(border(), social_color(link), amount),
        ),
    );
}

fn build_settings(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background());
    draw_back_button(&mut scene, layout, state, "MENU");
    draw_page_title(
        &mut scene,
        layout,
        "SETTINGS PREVIEW",
        "CURRENT DESKTOP PROFILE",
    );

    let panel_width = (layout.width - 80.0).clamp(440.0, 720.0);
    let panel = UiRect::from_min_size(
        Point::new((layout.width - panel_width) * 0.5, 205.0),
        panel_width,
        310.0,
    );
    draw_panel(&mut scene, layout, panel);
    let left = panel.min.x + 26.0;
    let rows = [
        ("THEME", "DARK", "LIGHT THEME IS NOT IN THIS PRODUCT SLICE"),
        (
            "WINDOW",
            "BORDERLESS FULLSCREEN",
            "SIM;ENGINE DRAWS THE COMPLETE SHELL",
        ),
        (
            "SCIENTIFIC CONFIRM",
            "ASK",
            "PREFERENCE OVERRIDE COMES WITH PROJECT STATE",
        ),
        (
            "MOTION",
            "SMOOTH",
            "HOVER TWEENING AFFECTS PRESENTATION ONLY",
        ),
    ];
    for (index, (label, value, detail)) in rows.into_iter().enumerate() {
        let y = panel.min.y + 32.0 + index as f32 * 70.0;
        pixel_font::draw(&mut scene, layout, label, Point::new(left, y), 1.7, faint());
        pixel_font::draw(
            &mut scene,
            layout,
            value,
            Point::new(left + 176.0, y),
            title_size_for(value, 1.9, panel.width() - 210.0),
            accent(),
        );
        pixel_font::draw(
            &mut scene,
            layout,
            detail,
            Point::new(left, y + 27.0),
            title_size_for(detail, 1.25, panel.width() - 50.0),
            muted(),
        );
    }
    draw_footer(
        &mut scene,
        layout,
        "PREVIEW ONLY   PERSISTED PREFERENCES ARE NOT BUILT YET",
    );
    scene
}

fn build_domain_menu(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background());
    draw_back_button(&mut scene, layout, state, "MENU");
    let compact = layout.height < 700.0;
    let title_y = if compact { 92.0 } else { 116.0 };
    let title_size = if compact { 7.0 } else { 8.0 };
    let maximum_hover = state
        .animations
        .domain_hover
        .into_iter()
        .fold(0.0, f32::max);

    pixel_font::draw_centered(
        &mut scene,
        layout,
        "DOMAINS",
        Point::new(layout.width * 0.5, title_y - maximum_hover * 6.0),
        title_size,
        text().with_alpha(1.0 - maximum_hover),
    );
    for domain in TopDomain::ALL {
        let amount = state.animations.domain_hover[domain.index()];
        if amount > 0.001 {
            pixel_font::draw_centered(
                &mut scene,
                layout,
                domain.title(),
                Point::new(layout.width * 0.5, title_y + (1.0 - amount) * 10.0),
                title_size,
                text().with_alpha(amount),
            );
        }
    }
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "CHOOSE A SIMULATION DOMAIN",
        Point::new(layout.width * 0.5, title_y + 54.0),
        2.4,
        muted(),
    );

    for ((domain, rect), amount) in TopDomain::ALL
        .into_iter()
        .zip(layout.domain_buttons())
        .zip(state.animations.domain_hover)
    {
        draw_button(&mut scene, layout, rect, domain.label(), amount, false);
    }

    let message = match state.notice {
        Some(Notice::DomainUnavailable(domain)) => match domain {
            TopDomain::Math => "SIM;MATH FOLLOWS THE PHYSICS SLICE",
            TopDomain::Chem => "SIM;CHEM FOLLOWS THE PHYSICS SLICE",
            TopDomain::Biol => "SIM;BIOL FOLLOWS THE PHYSICS SLICE",
            TopDomain::Phys => "SIM;PHYS IS ACTIVE",
        },
        _ => "2 OR CLICK PHYS   ACTIVE DOMAIN",
    };
    draw_footer(&mut scene, layout, message);
    scene
}

fn build_phys_menu(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background());
    draw_back_button(&mut scene, layout, state, "DOMAINS");
    let compact = layout.height < 700.0;
    let title_y = if compact { 82.0 } else { 100.0 };
    let title_size = if compact { 5.0 } else { 6.0 };
    let maximum_hover = state.animations.phys_hover.into_iter().fold(0.0, f32::max);
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "SIM;PHYS",
        Point::new(layout.width * 0.5, title_y - maximum_hover * 5.0),
        title_size,
        text().with_alpha(1.0 - maximum_hover),
    );
    for subdomain in PhysSubdomain::ALL {
        let amount = state.animations.phys_hover[subdomain.index()];
        if amount > 0.001 {
            pixel_font::draw_centered(
                &mut scene,
                layout,
                subdomain.title(),
                Point::new(layout.width * 0.5, title_y + (1.0 - amount) * 9.0),
                title_size_for(subdomain.title(), title_size, layout.width),
                text().with_alpha(amount),
            );
        }
    }
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "CHOOSE A PHYSICS SUB-DOMAIN",
        Point::new(layout.width * 0.5, title_y + 40.0),
        2.1,
        muted(),
    );

    for ((subdomain, rect), amount) in PhysSubdomain::ALL
        .into_iter()
        .zip(layout.phys_buttons())
        .zip(state.animations.phys_hover)
    {
        draw_button(&mut scene, layout, rect, subdomain.label(), amount, false);
    }
    draw_footer(
        &mut scene,
        layout,
        "SELECT A SUB-DOMAIN TO OPEN ITS PROJECTS",
    );
    scene
}

fn build_projects(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background());
    draw_back_button(&mut scene, layout, state, "SUB-DOMAINS");
    draw_page_title(
        &mut scene,
        layout,
        "PROJECTS",
        state.selected_subdomain.title(),
    );

    let active = state.selected_subdomain.has_runtime();
    for ((project, rect), hover) in ProjectTemplate::ALL
        .into_iter()
        .zip(layout.project_cards())
        .zip(state.animations.project_hover)
    {
        let disabled = !project.is_available(state.selected_subdomain);
        draw_project_card(
            &mut scene,
            layout,
            rect,
            project,
            state.selected_subdomain,
            hover,
            disabled,
        );
    }

    let message = match state.notice {
        Some(Notice::SubdomainUnavailable(subdomain)) => match subdomain {
            PhysSubdomain::Thermodynamics => "THERMODYNAMICS PROJECT RUNTIME IS NOT BUILT YET",
            PhysSubdomain::WavesAndOptics => "WAVES AND OPTICS PROJECT RUNTIME IS NOT BUILT YET",
            PhysSubdomain::Electromagnetism => "ELECTROMAGNETISM PROJECT RUNTIME IS NOT BUILT YET",
            PhysSubdomain::Relativity => "RELATIVITY PROJECT RUNTIME IS NOT BUILT YET",
            PhysSubdomain::FluidDynamics => "FLUID DYNAMICS PROJECT RUNTIME IS NOT BUILT YET",
            PhysSubdomain::Sandbox => "PHYS;SANDBOX WAITS FOR MULTIPLE CAPABILITY SETS",
            PhysSubdomain::Mechanics => "PHYS;MECHANICS PROJECTS ARE ACTIVE",
        },
        Some(Notice::ProjectUnavailable(_)) => "THIS PROJECT WAITS FOR ITS NEXT SCIENCE GATE",
        _ if active => "1 NEW PROJECT   2 PRIMARY SCIENCE LAB",
        _ => "PROJECT BROWSER READY   DOMAIN RUNTIME PENDING",
    };
    draw_footer(&mut scene, layout, message);
    scene
}

fn draw_project_card(
    scene: &mut Scene,
    layout: UiLayout,
    rect: UiRect,
    project: ProjectTemplate,
    subdomain: PhysSubdomain,
    hover: f32,
    disabled: bool,
) {
    let amount = hover.clamp(0.0, 1.0);
    let display = rect.expand(amount * 2.0);
    let outline = if disabled {
        mix_color(border(), faint(), amount)
    } else {
        mix_color(border(), accent(), amount)
    };
    scene.rect(
        ui_rect(display),
        12.0,
        ShapeStyle::fill_stroke(
            mix_color(surface(), surface_hover(), amount),
            1.0 + amount,
            outline,
        ),
    );
    pixel_font::draw_centered(
        scene,
        layout,
        project.label(subdomain),
        Point::new(display.center().x, display.min.y + 54.0),
        title_size_for(project.label(subdomain), 2.7, display.width() - 28.0),
        if disabled { muted() } else { text() },
    );
    pixel_font::draw_centered(
        scene,
        layout,
        project.description(subdomain),
        Point::new(display.center().x, display.min.y + 103.0),
        title_size_for(project.description(subdomain), 1.45, display.width() - 30.0),
        faint(),
    );
    pixel_font::draw_centered(
        scene,
        layout,
        if disabled { "LOCKED" } else { "OPEN" },
        Point::new(display.center().x, display.max.y - 30.0),
        1.45,
        if disabled { warning() } else { accent() },
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PhysicsWorkspace {
    Editor,
    View,
}

const THERMAL_COLD_REFERENCE_KELVIN: f64 = 250.0;
const THERMAL_HOT_REFERENCE_KELVIN: f64 = 500.0;
const MAX_RENDERED_MECHANICS_BODIES: usize = 800;
const MAX_RENDERED_THERMAL_BODIES: usize = 256;
const MAX_THERMAL_TEMPERATURE_LABELS: usize = 12;
const MAX_RENDERED_WAVE_POINTS: usize = 2_048;
const MAX_RENDERED_ELECTROSTATIC_CHARGES: usize = 700;

fn build_physics_editor(
    layout: UiLayout,
    state: &UiState,
    snapshot: Option<PhysicsSnapshotRef<'_>>,
    physics_error: Option<&str>,
    scientific_camera: Camera2d,
) -> Scene {
    match (state.selected_subdomain, snapshot) {
        (PhysSubdomain::Mechanics, Some(PhysicsSnapshotRef::Mechanics(project))) => {
            build_mechanics_editor(
                layout,
                state,
                Some(project),
                physics_error,
                scientific_camera,
            )
        }
        (PhysSubdomain::Thermodynamics, Some(PhysicsSnapshotRef::Thermodynamics(snapshot))) => {
            build_thermal_lab(
                layout,
                state,
                snapshot,
                physics_error,
                PhysicsWorkspace::Editor,
                scientific_camera,
            )
        }
        (PhysSubdomain::WavesAndOptics, Some(PhysicsSnapshotRef::WavesAndOptics(snapshot))) => {
            build_wave_lab(
                layout,
                state,
                snapshot,
                physics_error,
                PhysicsWorkspace::Editor,
                scientific_camera,
            )
        }
        (PhysSubdomain::Electromagnetism, Some(PhysicsSnapshotRef::Electromagnetism(snapshot))) => {
            build_electrostatic_lab(
                layout,
                state,
                snapshot,
                physics_error,
                PhysicsWorkspace::Editor,
                scientific_camera,
            )
        }
        (PhysSubdomain::Mechanics, None) => {
            build_mechanics_editor(layout, state, None, physics_error, scientific_camera)
        }
        _ => build_missing_physics_lab(layout, state, physics_error, scientific_camera),
    }
}

fn build_physics_view(
    layout: UiLayout,
    state: &UiState,
    snapshot: Option<PhysicsSnapshotRef<'_>>,
    physics_error: Option<&str>,
    scientific_camera: Camera2d,
) -> Scene {
    match (state.selected_subdomain, snapshot) {
        (PhysSubdomain::Mechanics, Some(PhysicsSnapshotRef::Mechanics(project))) => {
            build_mechanics_view(
                layout,
                state,
                Some(project),
                physics_error,
                scientific_camera,
            )
        }
        (PhysSubdomain::Thermodynamics, Some(PhysicsSnapshotRef::Thermodynamics(snapshot))) => {
            build_thermal_lab(
                layout,
                state,
                snapshot,
                physics_error,
                PhysicsWorkspace::View,
                scientific_camera,
            )
        }
        (PhysSubdomain::WavesAndOptics, Some(PhysicsSnapshotRef::WavesAndOptics(snapshot))) => {
            build_wave_lab(
                layout,
                state,
                snapshot,
                physics_error,
                PhysicsWorkspace::View,
                scientific_camera,
            )
        }
        (PhysSubdomain::Electromagnetism, Some(PhysicsSnapshotRef::Electromagnetism(snapshot))) => {
            build_electrostatic_lab(
                layout,
                state,
                snapshot,
                physics_error,
                PhysicsWorkspace::View,
                scientific_camera,
            )
        }
        (PhysSubdomain::Mechanics, None) => {
            build_mechanics_view(layout, state, None, physics_error, scientific_camera)
        }
        _ => build_missing_physics_lab(layout, state, physics_error, scientific_camera),
    }
}

fn build_physics_view_exit_overlay(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new_overlay(Color::TRANSPARENT);
    scene.rect(
        UiRect::from_min_size(Point::new(0.0, 0.0), layout.width, layout.height),
        0.0,
        ShapeStyle::filled(Color::rgba8(2, 5, 10, 196)),
    );
    let panel = layout.view_exit_panel();
    draw_panel(&mut scene, layout, panel);
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "LEAVE SIMULATION VIEW?",
        Point::new(panel.center().x, panel.min.y + 48.0),
        2.8,
        text(),
    );
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "APPLY KEEPS THIS STATE   DISCARD RESTORES EDITOR",
        Point::new(panel.center().x, panel.min.y + 92.0),
        1.35,
        muted(),
    );
    for ((choice, rect), hover) in ViewExitChoice::ALL
        .into_iter()
        .zip(layout.view_exit_choice_buttons())
        .zip(state.animations.view_exit_hover)
    {
        draw_button(
            &mut scene,
            layout,
            rect,
            choice.label(),
            hover,
            choice == ViewExitChoice::Apply,
        );
    }
    scene
}

fn build_mechanics_editor(
    layout: UiLayout,
    state: &UiState,
    mechanics: Option<MechanicsProjectSnapshotRef<'_>>,
    mechanics_error: Option<&str>,
    scientific_camera: Camera2d,
) -> Scene {
    let mut scene = Scene::new(background());
    draw_back_button(&mut scene, layout, state, "PROJECTS");
    draw_button(
        &mut scene,
        layout,
        layout.view_simulation_button(),
        "VIEW SIMULATION",
        state.animations.view_simulation_hover,
        false,
    );
    draw_button(
        &mut scene,
        layout,
        layout.lab_reset_button(),
        "RESET",
        state.animations.reset_hover,
        false,
    );
    pixel_font::draw_centered(
        &mut scene,
        layout,
        state.selected_project.label(state.selected_subdomain),
        Point::new(layout.width * 0.5, 43.0),
        title_size_for(
            state.selected_project.label(state.selected_subdomain),
            4.0,
            layout.width - 720.0,
        ),
        text(),
    );

    let left_panel = layout.editor_left_panel();
    let right_panel = layout.editor_right_panel();
    let canvas = layout.editor_canvas();
    draw_panel(&mut scene, layout, left_panel);
    draw_panel(&mut scene, layout, right_panel);
    draw_canvas(&mut scene, layout, canvas, scientific_camera);

    pixel_font::draw(
        &mut scene,
        layout,
        "BUILD",
        Point::new(left_panel.min.x + 16.0, left_panel.min.y + 20.0),
        3.0,
        muted(),
    );
    for ((tool, rect), hover) in MechanicsTool::ALL
        .into_iter()
        .zip(layout.mechanics_tool_buttons())
        .zip(state.animations.tool_hover)
    {
        if matches!(tool, MechanicsTool::Pendulum | MechanicsTool::Spring) {
            draw_disabled_button(&mut scene, layout, rect, tool.label());
        } else {
            draw_button(
                &mut scene,
                layout,
                rect,
                tool.label(),
                hover,
                state.selected_tool == tool,
            );
        }
    }
    pixel_font::draw(
        &mut scene,
        layout,
        "MECHANICS CORE",
        Point::new(left_panel.min.x + 16.0, left_panel.max.y - 78.0),
        1.8,
        muted(),
    );
    pixel_font::draw(
        &mut scene,
        layout,
        "F=MA SOLVER READY",
        Point::new(left_panel.min.x + 16.0, left_panel.max.y - 52.0),
        1.45,
        accent(),
    );
    pixel_font::draw(
        &mut scene,
        layout,
        "CONSTRAINTS NEXT",
        Point::new(left_panel.min.x + 16.0, left_panel.max.y - 30.0),
        1.45,
        faint(),
    );

    draw_mechanics_world(
        &mut scene,
        layout,
        canvas,
        mechanics.map(|project| project.world),
        mechanics.map(|project| project.persistent_forces),
        state.selected_body,
        scientific_camera,
    );
    if let Some((start, end)) = state.force_drag_preview {
        scene.scientific_line(start, end, 4.0, warning());
    }
    draw_inspector(
        &mut scene,
        layout,
        right_panel,
        state,
        mechanics,
        mechanics_error,
    );
    let message = match state.notice {
        Some(Notice::ToolAwaitingDomain(tool)) => {
            format!("{} TOOL WAITS FOR APP COMMAND BINDING", tool.label())
        }
        Some(Notice::ViewStateDiscarded) => "VIEW RUN DISCARDED   EDITOR STATE RESTORED".to_owned(),
        Some(Notice::ViewStateApplied) => "VIEW STATE APPLIED TO EDITOR".to_owned(),
        Some(Notice::EditorActionCommitted) => "EDITOR CHANGE COMMITTED   NO STEP".to_owned(),
        Some(Notice::EditorActionFailed) => "EDITOR CHANGE REJECTED   STATE UNCHANGED".to_owned(),
        _ if mechanics_error.is_some() => "DOMAIN ERROR   EDITOR STATE UNCHANGED".to_owned(),
        _ => "EDITOR MODE   V TO VIEW SIMULATION".to_owned(),
    };
    pixel_font::draw_centered(
        &mut scene,
        layout,
        &message,
        Point::new(canvas.center().x, canvas.max.y - 30.0),
        title_size_for(&message, 1.7, canvas.width() - 30.0),
        muted(),
    );
    scene
}

fn build_mechanics_view(
    layout: UiLayout,
    state: &UiState,
    mechanics: Option<MechanicsProjectSnapshotRef<'_>>,
    mechanics_error: Option<&str>,
    scientific_camera: Camera2d,
) -> Scene {
    let mut scene = Scene::new(background());
    draw_back_button(&mut scene, layout, state, "EDITOR");
    draw_button(
        &mut scene,
        layout,
        layout.lab_reset_button(),
        "RESET",
        state.animations.reset_hover,
        false,
    );
    draw_lab_title(&mut scene, layout, state);

    let left = layout.view_left_panel();
    let right = layout.view_right_panel();
    let canvas = layout.view_canvas();
    draw_panel(&mut scene, layout, left);
    draw_panel(&mut scene, layout, right);
    draw_canvas(&mut scene, layout, canvas, scientific_camera);
    pixel_font::draw(
        &mut scene,
        layout,
        "VIEW TOOLS",
        Point::new(left.min.x + 16.0, left.min.y + 20.0),
        2.5,
        muted(),
    );
    for (index, (tool, status)) in [
        ("HAND", "NOT IN THIS BUILD"),
        ("FORCE", "NOT IN THIS BUILD"),
        ("MEASURE", "READ ONLY PANEL"),
    ]
    .into_iter()
    .enumerate()
    {
        let y = left.min.y + 82.0 + index as f32 * 76.0;
        pixel_font::draw(
            &mut scene,
            layout,
            tool,
            Point::new(left.min.x + 16.0, y),
            2.0,
            if index == 2 { accent() } else { muted() },
        );
        pixel_font::draw(
            &mut scene,
            layout,
            status,
            Point::new(left.min.x + 16.0, y + 29.0),
            1.25,
            if index == 2 {
                faint()
            } else {
                warning().with_alpha(0.65)
            },
        );
    }

    draw_mechanics_world(
        &mut scene,
        layout,
        canvas,
        mechanics.map(|project| project.world),
        None,
        state.selected_body,
        scientific_camera,
    );
    let body = mechanics.and_then(|project| {
        state
            .selected_body
            .and_then(|selected| project.world.bodies.iter().find(|body| body.id == selected))
            .or_else(|| project.world.bodies.first())
    });
    let rows = [
        (
            "POSITION X",
            body.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.position.x().get(), 3, "M"),
            ),
        ),
        (
            "VELOCITY X",
            body.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.velocity.x().get(), 3, "M/S"),
            ),
        ),
        (
            "ACCEL X",
            body.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.acceleration.x().get(), 3, "M/S/S"),
            ),
        ),
        (
            "STEP",
            mechanics.map_or_else(
                || "--".to_owned(),
                |value| value.world.step.get().to_string(),
            ),
        ),
        (
            "TIME",
            mechanics.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.world.elapsed.get(), 3, "S"),
            ),
        ),
    ];
    draw_metric_panel(
        &mut scene,
        layout,
        right,
        "MEASUREMENTS",
        rows,
        mechanics_error,
    );
    draw_playback_controls(&mut scene, layout, state, true);
    draw_canvas_status(
        &mut scene,
        layout,
        canvas,
        if mechanics_error.is_some() {
            "DOMAIN ERROR   VIEW PAUSED"
        } else {
            "VIEW MODE   ESC RETURNS TO EDITOR"
        },
    );
    scene
}

fn draw_physics_workspace_header(
    scene: &mut Scene,
    layout: UiLayout,
    state: &UiState,
    workspace: PhysicsWorkspace,
) {
    draw_back_button(
        scene,
        layout,
        state,
        if workspace == PhysicsWorkspace::Editor {
            "PROJECTS"
        } else {
            "EDITOR"
        },
    );
    if workspace == PhysicsWorkspace::Editor {
        draw_button(
            scene,
            layout,
            layout.view_simulation_button(),
            if state.selected_subdomain == PhysSubdomain::Electromagnetism {
                "VIEW FIELD"
            } else {
                "VIEW SIMULATION"
            },
            state.animations.view_simulation_hover,
            false,
        );
    }
    draw_button(
        scene,
        layout,
        layout.lab_reset_button(),
        "RESET",
        state.animations.reset_hover,
        false,
    );
    draw_lab_title(scene, layout, state);
}

fn physics_workspace_regions(
    layout: UiLayout,
    workspace: PhysicsWorkspace,
) -> (UiRect, UiRect, UiRect) {
    if workspace == PhysicsWorkspace::Editor {
        (
            layout.editor_left_panel(),
            layout.editor_right_panel(),
            layout.editor_canvas(),
        )
    } else {
        (
            layout.view_left_panel(),
            layout.view_right_panel(),
            layout.view_canvas(),
        )
    }
}

fn build_thermal_lab(
    layout: UiLayout,
    state: &UiState,
    thermal: &ThermalSnapshot,
    physics_error: Option<&str>,
    workspace: PhysicsWorkspace,
    scientific_camera: Camera2d,
) -> Scene {
    let mut scene = Scene::new(background());
    draw_physics_workspace_header(&mut scene, layout, state, workspace);
    let (left, right, canvas) = physics_workspace_regions(layout, workspace);
    draw_panel(&mut scene, layout, left);
    draw_panel(&mut scene, layout, right);
    draw_canvas(&mut scene, layout, canvas, scientific_camera);

    pixel_font::draw(
        &mut scene,
        layout,
        "THERMAL BODIES",
        Point::new(left.min.x + 16.0, left.min.y + 20.0),
        2.2,
        muted(),
    );
    for (index, body) in thermal.bodies.iter().take(6).enumerate() {
        let label = format!("BODY {}", body.entity.get());
        let temperature = format_si(body.temperature.get(), 2, "K");
        let y = left.min.y + 70.0 + index as f32 * 58.0;
        pixel_font::draw(
            &mut scene,
            layout,
            &label,
            Point::new(left.min.x + 16.0, y),
            1.5,
            faint(),
        );
        pixel_font::draw(
            &mut scene,
            layout,
            &temperature,
            Point::new(left.min.x + 16.0, y + 24.0),
            1.7,
            thermal_color(body.temperature.get()),
        );
    }
    draw_thermal_world(&mut scene, layout, canvas, thermal);
    draw_thermal_inspector(&mut scene, layout, right, thermal, physics_error, workspace);
    if workspace == PhysicsWorkspace::View {
        draw_playback_controls(&mut scene, layout, state, true);
    }
    draw_canvas_status(
        &mut scene,
        layout,
        canvas,
        match (workspace, physics_error.is_some()) {
            (PhysicsWorkspace::Editor, true) => "DOMAIN ERROR   EDITOR STATE UNCHANGED",
            (PhysicsWorkspace::Editor, false) => "CURATED LAB SETUP   READ ONLY   NO STEPS",
            (PhysicsWorkspace::View, true) => "DOMAIN ERROR   VIEW PAUSED",
            (PhysicsWorkspace::View, false) => "ENERGY CONSERVING CONDUCTION",
        },
    );
    scene
}

fn draw_thermal_world(
    scene: &mut Scene,
    layout: UiLayout,
    canvas: UiRect,
    thermal: &ThermalSnapshot,
) {
    if thermal.bodies.is_empty() {
        pixel_font::draw_centered(
            scene,
            layout,
            "EMPTY THERMAL WORLD",
            canvas.center(),
            2.0,
            muted(),
        );
        return;
    }
    pixel_font::draw_centered(
        scene,
        layout,
        "250 K COLD   FIXED SCALE   500 K HOT",
        Point::new(canvas.center().x, canvas.min.y + 28.0),
        1.3,
        faint(),
    );
    let count = thermal.bodies.len() as f32;
    let spacing = (canvas.width() - 100.0) / count.max(1.0);
    let rendered_indices = evenly_spaced_indices(thermal.bodies.len(), MAX_RENDERED_THERMAL_BODIES);
    let rendered_link_count = thermal_rendered_link_count(thermal, &rendered_indices);
    let rendered_bodies: Vec<_> = rendered_indices
        .iter()
        .copied()
        .map(|index| {
            let body = &thermal.bodies[index];
            let point = Point::new(
                canvas.min.x + 50.0 + spacing * (index as f32 + 0.5),
                canvas.center().y,
            );
            (body, point)
        })
        .collect();
    let points_by_entity: BTreeMap<_, _> = rendered_bodies
        .iter()
        .map(|(body, point)| (body.entity, *point))
        .collect();
    for link in &thermal.links {
        if let (Some(first), Some(second)) = (
            points_by_entity.get(&link.first),
            points_by_entity.get(&link.second),
        ) {
            scene.scientific_line(
                ui_point(*first),
                ui_point(*second),
                8.0,
                text().with_alpha(0.24),
            );
        }
    }
    let label_stride = rendered_bodies
        .len()
        .div_ceil(MAX_THERMAL_TEMPERATURE_LABELS)
        .max(1);
    let mut labels = 0_usize;
    for (display_index, (body, point)) in rendered_bodies.iter().enumerate() {
        let color = thermal_color(body.temperature.get());
        scene.scientific_circle(
            ui_point(*point),
            46.0,
            ShapeStyle::fill_stroke(color.with_alpha(0.32), 3.0, color),
        );
        if display_index % label_stride == 0 && labels < MAX_THERMAL_TEMPERATURE_LABELS {
            let temperature = format_si(body.temperature.get(), 1, "K");
            pixel_font::draw_centered(
                scene,
                layout,
                &temperature,
                Point::new(point.x, point.y + 76.0),
                1.8,
                color,
            );
            labels += 1;
        }
    }
    if rendered_bodies.len() < thermal.bodies.len() {
        let label = format!(
            "DISPLAY LOD   {} OF {} BODIES",
            rendered_bodies.len(),
            thermal.bodies.len()
        );
        pixel_font::draw_centered(
            scene,
            layout,
            &label,
            Point::new(canvas.center().x, canvas.min.y + 52.0),
            1.25,
            warning(),
        );
    }
    if rendered_link_count < thermal.links.len() {
        let label = format!("LINKS   {rendered_link_count} OF {}", thermal.links.len());
        pixel_font::draw_centered(
            scene,
            layout,
            &label,
            Point::new(canvas.center().x, canvas.min.y + 70.0),
            1.25,
            warning(),
        );
    }
}

fn thermal_color(temperature: f64) -> Color {
    let amount = ((temperature - THERMAL_COLD_REFERENCE_KELVIN)
        / (THERMAL_HOT_REFERENCE_KELVIN - THERMAL_COLD_REFERENCE_KELVIN)) as f32;
    mix_color(accent(), Color::rgb8(255, 94, 72), amount.clamp(0.0, 1.0))
}

fn draw_thermal_inspector(
    scene: &mut Scene,
    layout: UiLayout,
    panel: UiRect,
    thermal: &ThermalSnapshot,
    physics_error: Option<&str>,
    workspace: PhysicsWorkspace,
) {
    let total_energy = match thermal.total_energy {
        crate::domains::phys::thermodynamics::ThermalEnergyTotal::Empty => "0 J".to_owned(),
        crate::domains::phys::thermodynamics::ThermalEnergyTotal::Representable(total) => {
            format_si(total.get(), 2, "J")
        }
        crate::domains::phys::thermodynamics::ThermalEnergyTotal::AboveBinary64Range => {
            "ABOVE F64 RANGE".to_owned()
        }
    };
    let rows = [
        ("BODIES", thermal.bodies.len().to_string()),
        ("LINKS", thermal.links.len().to_string()),
        ("ENERGY", total_energy),
        ("STEP", thermal.step.get().to_string()),
        ("TIME", format_si(thermal.elapsed.get(), 3, "S")),
    ];
    draw_metric_panel(
        scene,
        layout,
        panel,
        if workspace == PhysicsWorkspace::Editor {
            "SETUP VALUES"
        } else {
            "MEASUREMENTS"
        },
        rows,
        physics_error,
    );
}

fn build_wave_lab(
    layout: UiLayout,
    state: &UiState,
    wave: &WaveSnapshot,
    physics_error: Option<&str>,
    workspace: PhysicsWorkspace,
    scientific_camera: Camera2d,
) -> Scene {
    let mut scene = Scene::new(background());
    draw_physics_workspace_header(&mut scene, layout, state, workspace);
    let (left, right, canvas) = physics_workspace_regions(layout, workspace);
    draw_panel(&mut scene, layout, left);
    draw_panel(&mut scene, layout, right);
    draw_canvas(&mut scene, layout, canvas, scientific_camera);
    let notes = [
        ("MODEL", "1D TRANSVERSE"),
        ("BOUNDARY", "FIXED ZERO"),
        ("METHOD", "SEMI IMPLICIT"),
        ("SAFETY", "COURANT MAX 1"),
    ];
    pixel_font::draw(
        &mut scene,
        layout,
        "WAVE FIELD",
        Point::new(left.min.x + 16.0, left.min.y + 20.0),
        2.5,
        muted(),
    );
    for (index, (name, value)) in notes.into_iter().enumerate() {
        let y = left.min.y + 78.0 + index as f32 * 64.0;
        pixel_font::draw(
            &mut scene,
            layout,
            name,
            Point::new(left.min.x + 16.0, y),
            1.4,
            faint(),
        );
        pixel_font::draw(
            &mut scene,
            layout,
            value,
            Point::new(left.min.x + 16.0, y + 24.0),
            title_size_for(value, 1.55, left.width() - 32.0),
            accent(),
        );
    }
    draw_wave_world(&mut scene, layout, canvas, wave);
    let maximum = wave
        .samples
        .iter()
        .map(|sample| sample.displacement.get().abs())
        .fold(0.0_f64, f64::max);
    let rows = [
        ("SAMPLES", wave.samples.len().to_string()),
        ("SPEED", format_si(wave.speed.get(), 2, "M/S")),
        ("MAX U", format_si(maximum, 4, "M")),
        ("STEP", wave.step.get().to_string()),
        ("TIME", format_si(wave.elapsed.get(), 3, "S")),
    ];
    draw_metric_panel(
        &mut scene,
        layout,
        right,
        if workspace == PhysicsWorkspace::Editor {
            "SETUP VALUES"
        } else {
            "MEASUREMENTS"
        },
        rows,
        physics_error,
    );
    if workspace == PhysicsWorkspace::View {
        draw_playback_controls(&mut scene, layout, state, true);
    }
    draw_canvas_status(
        &mut scene,
        layout,
        canvas,
        match (workspace, physics_error.is_some()) {
            (PhysicsWorkspace::Editor, true) => "DOMAIN ERROR   EDITOR STATE UNCHANGED",
            (PhysicsWorkspace::Editor, false) => "CURATED LAB SETUP   READ ONLY   NO STEPS",
            (PhysicsWorkspace::View, true) => "DOMAIN ERROR   VIEW PAUSED",
            (PhysicsWorkspace::View, false) => "LIVE FIXED BOUNDARY PROPAGATION",
        },
    );
    scene
}

fn draw_wave_world(scene: &mut Scene, layout: UiLayout, canvas: UiRect, wave: &WaveSnapshot) {
    if wave.samples.len() < 2 {
        pixel_font::draw_centered(
            scene,
            layout,
            "EMPTY WAVE FIELD",
            canvas.center(),
            2.0,
            muted(),
        );
        return;
    }
    let rendered_indices = wave_render_indices(wave);
    let mut previous = None;
    let mut range_rendered_points = 0_usize;
    for index in &rendered_indices {
        let sample = &wave.samples[*index];
        let (Some(x), Some(displacement)) = (
            f64_to_f32(sample.x.get()),
            f64_to_f32(sample.displacement.get()),
        ) else {
            previous = None;
            continue;
        };
        range_rendered_points += 1;
        let point = Point::new(x, displacement);
        if let Some(previous) = previous {
            scene.scientific_world_line(previous, point, 3.0, accent());
        }
        previous = Some(point);
    }
    pixel_font::draw(
        scene,
        layout,
        "X: M   U: M   FIXED SCALE",
        Point::new(canvas.min.x + 22.0, canvas.min.y + 22.0),
        1.45,
        faint(),
    );
    if rendered_indices.len() < wave.samples.len() {
        let label = format!(
            "EXTREMA LOD   {} OF {} POINTS",
            rendered_indices.len(),
            wave.samples.len()
        );
        pixel_font::draw(
            scene,
            layout,
            &label,
            Point::new(canvas.min.x + 22.0, canvas.min.y + 45.0),
            1.25,
            warning(),
        );
    }
    if range_rendered_points < rendered_indices.len() {
        let label = format!(
            "DISPLAY RANGE   {range_rendered_points} OF {} POINTS",
            rendered_indices.len()
        );
        pixel_font::draw(
            scene,
            layout,
            &label,
            Point::new(canvas.min.x + 22.0, canvas.min.y + 68.0),
            1.25,
            warning(),
        );
    }
}

fn build_electrostatic_lab(
    layout: UiLayout,
    state: &UiState,
    electrostatic: &ElectrostaticSnapshot,
    physics_error: Option<&str>,
    workspace: PhysicsWorkspace,
    scientific_camera: Camera2d,
) -> Scene {
    let mut scene = Scene::new(background());
    draw_physics_workspace_header(&mut scene, layout, state, workspace);
    let (left, right, canvas) = physics_workspace_regions(layout, workspace);
    draw_panel(&mut scene, layout, left);
    draw_panel(&mut scene, layout, right);
    draw_canvas(&mut scene, layout, canvas, scientific_camera);
    pixel_font::draw(
        &mut scene,
        layout,
        "ELECTROSTATICS",
        Point::new(left.min.x + 16.0, left.min.y + 20.0),
        2.2,
        muted(),
    );
    for (index, charge) in electrostatic.charges.iter().take(6).enumerate() {
        let sign = if charge.charge.get() > 0.0 {
            "POSITIVE"
        } else {
            "NEGATIVE"
        };
        let value = format!("{:.2E} C", charge.charge.get());
        let y = left.min.y + 74.0 + index as f32 * 72.0;
        pixel_font::draw(
            &mut scene,
            layout,
            sign,
            Point::new(left.min.x + 16.0, y),
            1.55,
            charge_color(charge.charge.get()),
        );
        pixel_font::draw(
            &mut scene,
            layout,
            &value,
            Point::new(left.min.x + 16.0, y + 27.0),
            1.35,
            muted(),
        );
    }
    draw_electrostatic_world(&mut scene, layout, canvas, electrostatic, scientific_camera);
    let maximum = electrostatic
        .probes
        .iter()
        .map(|probe| probe.magnitude.get())
        .fold(0.0_f64, f64::max);
    let rows = [
        ("CHARGES", electrostatic.charges.len().to_string()),
        ("PROBES", electrostatic.probes.len().to_string()),
        ("MAX FIELD", format!("{maximum:.2E} N/C")),
        (
            "CONSTANTS",
            match electrostatic.constants_source {
                crate::foundation::ConstantsSource::Codata2022 => "CODATA 2022".to_owned(),
                crate::foundation::ConstantsSource::Custom => "CUSTOM".to_owned(),
            },
        ),
        (
            "EPS0",
            format!("{:.6E} F/M", electrostatic.vacuum_permittivity.get()),
        ),
    ];
    draw_metric_panel(
        &mut scene,
        layout,
        right,
        if workspace == PhysicsWorkspace::Editor {
            "SETUP VALUES"
        } else {
            "MEASUREMENTS"
        },
        rows,
        physics_error,
    );
    if workspace == PhysicsWorkspace::View {
        draw_playback_controls(&mut scene, layout, state, false);
    }
    draw_canvas_status(
        &mut scene,
        layout,
        canvas,
        match (workspace, physics_error.is_some()) {
            (PhysicsWorkspace::Editor, true) => "EVALUATION ERROR   EDITOR STATE UNCHANGED",
            (PhysicsWorkspace::Editor, false) => "CURATED LAB SETUP   READ ONLY   NO STEPS",
            (PhysicsWorkspace::View, true) => "EVALUATION ERROR",
            (PhysicsWorkspace::View, false) => "PURE BOUNDED FIELD EVALUATION",
        },
    );
    scene
}

fn draw_electrostatic_world(
    scene: &mut Scene,
    layout: UiLayout,
    canvas: UiRect,
    electrostatic: &ElectrostaticSnapshot,
    camera: Camera2d,
) {
    let marker_world_radius = 16.0 / camera.zoom();
    let maximum_field = electrostatic
        .probes
        .iter()
        .map(|probe| probe.magnitude.get())
        .fold(0.0_f64, f64::max);
    let mut omitted_probe_positions = 0_usize;
    for probe in &electrostatic.probes {
        let (Some(x), Some(y)) = (
            f64_to_f32(probe.position.x().get()),
            f64_to_f32(probe.position.y().get()),
        ) else {
            omitted_probe_positions += 1;
            continue;
        };
        let origin = Point::new(x, y);
        if !world_point_is_visible(origin, canvas, camera, 48.0) {
            continue;
        }
        let magnitude = probe.magnitude.get();
        if magnitude == 0.0 || maximum_field == 0.0 {
            continue;
        }
        let normalized_x = probe.field.x().get() / magnitude;
        let normalized_y = probe.field.y().get() / magnitude;
        let visual_length =
            (18.0 + 28.0 * (magnitude / maximum_field).sqrt()) / f64::from(camera.zoom());
        let (Some(dx), Some(dy), Some(length)) = (
            finite_f64_to_f32(normalized_x),
            finite_f64_to_f32(normalized_y),
            f64_to_f32(visual_length),
        ) else {
            continue;
        };
        let end = Point::new(origin.x + dx * length, origin.y + dy * length);
        let color = accent().with_alpha(0.62);
        scene.scientific_world_line(origin, end, 1.5, color);
        let normal = Point::new(-dy, dx);
        for side in [-1.0_f32, 1.0] {
            let arrow = Point::new(
                end.x - dx * 9.0 / camera.zoom() + normal.x * side * 5.0 / camera.zoom(),
                end.y - dy * 9.0 / camera.zoom() + normal.y * side * 5.0 / camera.zoom(),
            );
            scene.scientific_world_line(end, arrow, 1.5, color);
        }
    }
    let mut visible_charges = 0_usize;
    let mut rendered_charges = 0_usize;
    let mut omitted_charge_positions = 0_usize;
    for charge in &electrostatic.charges {
        let (Some(x), Some(y)) = (
            f64_to_f32(charge.position.x().get()),
            f64_to_f32(charge.position.y().get()),
        ) else {
            omitted_charge_positions += 1;
            continue;
        };
        let point = Point::new(x, y);
        if !world_point_is_visible(point, canvas, camera, 24.0) {
            continue;
        }
        visible_charges += 1;
        if rendered_charges >= MAX_RENDERED_ELECTROSTATIC_CHARGES {
            continue;
        }
        let color = charge_color(charge.charge.get());
        scene.scientific_world_circle(
            point,
            marker_world_radius,
            ShapeStyle::fill_stroke(color.with_alpha(0.34), 3.0, color),
        );
        rendered_charges += 1;
    }
    pixel_font::draw(
        scene,
        layout,
        "POSITION: M   ARROWS: SQRT RELATIVE FIELD",
        Point::new(canvas.min.x + 22.0, canvas.min.y + 22.0),
        1.25,
        faint(),
    );
    if rendered_charges < visible_charges {
        let label =
            format!("DISPLAY LOD   {rendered_charges} OF {visible_charges} VISIBLE CHARGES");
        pixel_font::draw(
            scene,
            layout,
            &label,
            Point::new(canvas.min.x + 22.0, canvas.min.y + 45.0),
            1.25,
            warning(),
        );
    }
    if omitted_probe_positions > 0 || omitted_charge_positions > 0 {
        let label = format!(
            "DISPLAY RANGE   {omitted_probe_positions} PROBES   {omitted_charge_positions} CHARGES OMITTED"
        );
        pixel_font::draw(
            scene,
            layout,
            &label,
            Point::new(canvas.min.x + 22.0, canvas.min.y + 68.0),
            1.25,
            warning(),
        );
    }
}

fn charge_color(charge: f64) -> Color {
    if charge > 0.0 {
        Color::rgb8(255, 96, 76)
    } else {
        Color::rgb8(88, 155, 255)
    }
}

fn build_missing_physics_lab(
    layout: UiLayout,
    state: &UiState,
    physics_error: Option<&str>,
    _scientific_camera: Camera2d,
) -> Scene {
    let mut scene = Scene::new(background());
    draw_back_button(&mut scene, layout, state, "PROJECTS");
    draw_lab_title(&mut scene, layout, state);
    pixel_font::draw_centered(
        &mut scene,
        layout,
        physics_error.unwrap_or("NO MATCHING CANONICAL SNAPSHOT"),
        Point::new(layout.width * 0.5, layout.height * 0.5),
        2.1,
        warning(),
    );
    scene
}

fn draw_lab_title(scene: &mut Scene, layout: UiLayout, state: &UiState) {
    let label = state.selected_project.label(state.selected_subdomain);
    pixel_font::draw_centered(
        scene,
        layout,
        label,
        Point::new(layout.width * 0.5, 43.0),
        title_size_for(label, 4.0, layout.width - 720.0),
        text(),
    );
}

fn draw_metric_panel<const COUNT: usize>(
    scene: &mut Scene,
    layout: UiLayout,
    panel: UiRect,
    title: &str,
    rows: [(&str, String); COUNT],
    physics_error: Option<&str>,
) {
    let left = panel.min.x + 16.0;
    pixel_font::draw(
        scene,
        layout,
        title,
        Point::new(left, panel.min.y + 20.0),
        2.4,
        muted(),
    );
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let y = panel.min.y + 74.0 + index as f32 * 58.0;
        pixel_font::draw(scene, layout, label, Point::new(left, y), 1.5, faint());
        pixel_font::draw(
            scene,
            layout,
            &value,
            Point::new(left, y + 25.0),
            title_size_for(&value, 1.55, panel.width() - 32.0),
            muted(),
        );
    }
    pixel_font::draw(
        scene,
        layout,
        physics_error.unwrap_or("DOMAIN SNAPSHOT LIVE"),
        Point::new(left, panel.max.y - 30.0),
        1.2,
        if physics_error.is_some() {
            warning()
        } else {
            time_accent()
        },
    );
}

fn draw_canvas_status(scene: &mut Scene, layout: UiLayout, canvas: UiRect, message: &str) {
    pixel_font::draw_centered(
        scene,
        layout,
        message,
        Point::new(canvas.center().x, canvas.max.y - 24.0),
        title_size_for(message, 1.5, canvas.width() - 24.0),
        muted(),
    );
}

fn draw_playback_controls(
    scene: &mut Scene,
    layout: UiLayout,
    state: &UiState,
    time_evolving: bool,
) {
    if !time_evolving {
        pixel_font::draw_centered(
            scene,
            layout,
            "STATIC EVALUATION   RESET AVAILABLE",
            Point::new(layout.width * 0.5, layout.height - 45.0),
            1.7,
            muted(),
        );
        return;
    }
    for ((rate, rect), hover) in PlaybackRate::ALL
        .into_iter()
        .zip(layout.playback_buttons())
        .zip(state.animations.playback_hover)
    {
        draw_button(
            scene,
            layout,
            rect,
            rate.label(),
            hover,
            state.playback_rate == rate,
        );
    }
}

fn draw_mechanics_world(
    scene: &mut Scene,
    layout: UiLayout,
    canvas: UiRect,
    mechanics: Option<&MechanicsSnapshot>,
    persistent_forces: Option<
        &BTreeMap<crate::foundation::EntityId, crate::domains::phys::mechanics::ForceContribution>,
    >,
    selected_body: Option<crate::foundation::EntityId>,
    camera: Camera2d,
) {
    let Some(snapshot) = mechanics else {
        pixel_font::draw_centered(
            scene,
            layout,
            "NO CANONICAL SNAPSHOT",
            canvas.center(),
            2.2,
            warning(),
        );
        return;
    };
    if snapshot.bodies.is_empty() {
        pixel_font::draw_centered(scene, layout, "EMPTY WORLD", canvas.center(), 2.2, muted());
        return;
    }

    let representable_bodies: Vec<_> = snapshot
        .bodies
        .iter()
        .filter_map(|body| mechanics_body_point(body).map(|point| (body, point)))
        .collect();
    let visible_bodies: Vec<_> = representable_bodies
        .iter()
        .copied()
        .filter(|(_, point)| world_point_is_visible(*point, canvas, camera, 32.0))
        .collect();
    let selected_visible = selected_body.and_then(|selected| {
        visible_bodies
            .iter()
            .copied()
            .find(|(body, _)| body.id == selected)
    });
    let mut rendered_bodies = 0_usize;
    if let Some((body, point)) = selected_visible {
        draw_mechanics_body(scene, body, point, persistent_forces, selected_body, camera);
        rendered_bodies += 1;
    }
    for (body, point) in visible_bodies.iter().copied() {
        if selected_body == Some(body.id) || rendered_bodies >= MAX_RENDERED_MECHANICS_BODIES {
            continue;
        }
        draw_mechanics_body(scene, body, point, persistent_forces, selected_body, camera);
        rendered_bodies += 1;
    }
    if representable_bodies.len() != snapshot.bodies.len() {
        let omitted = snapshot.bodies.len() - representable_bodies.len();
        let label = format!(
            "DISPLAY RANGE   {omitted} OF {} BODIES OMITTED",
            snapshot.bodies.len()
        );
        pixel_font::draw_centered(
            scene,
            layout,
            &label,
            Point::new(canvas.center().x, canvas.min.y + 36.0),
            1.35,
            warning(),
        );
    }
    if rendered_bodies < visible_bodies.len() {
        let label = format!(
            "DISPLAY LOD   {rendered_bodies} OF {} VISIBLE BODIES",
            visible_bodies.len()
        );
        pixel_font::draw_centered(
            scene,
            layout,
            &label,
            Point::new(canvas.center().x, canvas.min.y + 58.0),
            1.35,
            warning(),
        );
    }
    pixel_font::draw_centered(
        scene,
        layout,
        "F = M A",
        Point::new(canvas.center().x, canvas.center().y + 118.0),
        3.2,
        text().with_alpha(0.84),
    );
    draw_mechanics_scale(scene, layout, canvas, camera);
}

fn draw_force_vector(scene: &mut Scene, body: Point, force: Force2, camera: Camera2d) {
    let force_x = force.x().get();
    let force_y = force.y().get();
    let scale = force_x.abs().max(force_y.abs());
    if scale == 0.0 || !scale.is_finite() {
        return;
    }
    let scaled_x = force_x / scale;
    let scaled_y = force_y / scale;
    let normalized_magnitude = scaled_x.hypot(scaled_y);
    let unit_x = scaled_x / normalized_magnitude;
    let unit_y = scaled_y / normalized_magnitude;
    let (Some(direction_x), Some(direction_y)) =
        (finite_f64_to_f32(unit_x), finite_f64_to_f32(unit_y))
    else {
        return;
    };
    let marker_radius = 22.0 / camera.zoom();
    let shaft_pixels = f64_to_f32((36.0 + 12.0 * scale.ln_1p()).clamp(36.0, 96.0)).unwrap_or(36.0);
    let displayed_length = marker_radius + shaft_pixels / camera.zoom();
    let start = Point::new(
        body.x + direction_x * marker_radius,
        body.y + direction_y * marker_radius,
    );
    let force_end = Point::new(
        body.x + direction_x * displayed_length,
        body.y + direction_y * displayed_length,
    );
    let normal = Point::new(-direction_y, direction_x);
    scene.scientific_world_line(start, force_end, 4.0, warning());
    let arrow_length = 12.0 / camera.zoom();
    let arrow_width = 7.0 / camera.zoom();
    for side in [-1.0_f32, 1.0] {
        let arrow = Point::new(
            force_end.x - direction_x * arrow_length + normal.x * side * arrow_width,
            force_end.y - direction_y * arrow_length + normal.y * side * arrow_width,
        );
        scene.scientific_world_line(force_end, arrow, 3.0, warning());
    }
}

fn draw_mechanics_body(
    scene: &mut Scene,
    body_snapshot: &crate::domains::phys::mechanics::BodySnapshot,
    body: Point,
    persistent_forces: Option<
        &BTreeMap<crate::foundation::EntityId, crate::domains::phys::mechanics::ForceContribution>,
    >,
    selected_body: Option<crate::foundation::EntityId>,
    camera: Camera2d,
) {
    scene.scientific_world_circle(
        body,
        18.0 / camera.zoom(),
        ShapeStyle::fill_stroke(accent().with_alpha(0.30), 2.0, accent()),
    );
    if selected_body == Some(body_snapshot.id) {
        scene.scientific_world_circle(body, 24.0 / camera.zoom(), ShapeStyle::stroked(2.0, text()));
    }
    let force = persistent_forces.map_or(body_snapshot.net_force, |forces| {
        forces
            .get(&body_snapshot.id)
            .map_or(Force2::ZERO, |contribution| contribution.force)
    });
    draw_force_vector(scene, body, force, camera);
}

fn mechanics_body_point(body: &crate::domains::phys::mechanics::BodySnapshot) -> Option<Point> {
    Some(Point::new(
        f64_to_f32(body.position.x().get())?,
        f64_to_f32(body.position.y().get())?,
    ))
}

fn world_point_is_visible(point: Point, canvas: UiRect, camera: Camera2d, margin: f32) -> bool {
    let Ok(viewport) = LogicalViewport::new(canvas.width(), canvas.height()) else {
        return false;
    };
    let Ok(screen) = camera.world_to_screen(Vec2::new(point.x, point.y), viewport) else {
        return false;
    };
    let screen = screen.to_vec2();
    screen.x() >= -margin
        && screen.y() >= -margin
        && screen.x() <= canvas.width() + margin
        && screen.y() <= canvas.height() + margin
}

fn evenly_spaced_indices(length: usize, maximum: usize) -> Vec<usize> {
    if length <= maximum {
        return (0..length).collect();
    }
    if maximum <= 1 {
        return vec![0];
    }
    (0..maximum)
        .map(|slot| slot * (length - 1) / (maximum - 1))
        .collect()
}

fn thermal_rendered_link_count(thermal: &ThermalSnapshot, rendered_indices: &[usize]) -> usize {
    let rendered_entities: BTreeSet<_> = rendered_indices
        .iter()
        .map(|index| thermal.bodies[*index].entity)
        .collect();
    thermal
        .links
        .iter()
        .filter(|link| {
            rendered_entities.contains(&link.first) && rendered_entities.contains(&link.second)
        })
        .count()
}

fn wave_render_indices(wave: &WaveSnapshot) -> Vec<usize> {
    let length = wave.samples.len();
    if length <= MAX_RENDERED_WAVE_POINTS {
        return (0..length).collect();
    }
    let bucket_count = (MAX_RENDERED_WAVE_POINTS - 2) / 2;
    let interior = length - 2;
    let mut indices = Vec::with_capacity(MAX_RENDERED_WAVE_POINTS);
    indices.push(0);
    for bucket in 0..bucket_count {
        let start = 1 + bucket * interior / bucket_count;
        let end = 1 + (bucket + 1) * interior / bucket_count;
        let mut minimum = start;
        let mut maximum = start;
        for index in (start + 1)..end {
            let value = wave.samples[index].displacement.get();
            if value < wave.samples[minimum].displacement.get() {
                minimum = index;
            }
            if value > wave.samples[maximum].displacement.get() {
                maximum = index;
            }
        }
        for index in [minimum.min(maximum), minimum.max(maximum)] {
            if indices.last().copied() != Some(index) {
                indices.push(index);
            }
        }
    }
    if indices.last().copied() != Some(length - 1) {
        indices.push(length - 1);
    }
    indices
}

fn draw_mechanics_scale(scene: &mut Scene, layout: UiLayout, canvas: UiRect, camera: Camera2d) {
    let Ok(spacing) = super::scene_builder::adaptive_grid_spacing(camera.zoom()) else {
        return;
    };
    let length_pixels = spacing * camera.zoom();
    let start = Point::new(canvas.min.x + 22.0, canvas.max.y - 28.0);
    scene.rect(
        UiRect::from_min_size(start, length_pixels, 2.0),
        0.0,
        ShapeStyle::filled(text().with_alpha(0.78)),
    );
    let label = format!("GRID {}", format_metric_length(spacing));
    pixel_font::draw(
        scene,
        layout,
        &label,
        Point::new(start.x, start.y - 18.0),
        1.5,
        muted(),
    );
}

fn format_metric_length(meters: f32) -> String {
    if meters >= 1_000.0 {
        format!("{:.3} KM", meters / 1_000.0)
    } else if meters >= 1.0 {
        format!("{meters:.3} M")
    } else if meters >= 0.01 {
        format!("{:.3} CM", meters * 100.0)
    } else if meters >= 0.001 {
        format!("{:.3} MM", meters * 1_000.0)
    } else if meters >= 0.000_001 {
        format!("{:.3} UM", meters * 1_000_000.0)
    } else {
        format!("{:.3} NM", meters * 1_000_000_000.0)
    }
}

fn f64_to_f32(value: f64) -> Option<f32> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        None
    } else {
        let converted = value as f32;
        if value != 0.0 && converted == 0.0 {
            None
        } else {
            Some(converted)
        }
    }
}

fn finite_f64_to_f32(value: f64) -> Option<f32> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        None
    } else {
        Some(value as f32)
    }
}

fn format_si(value: f64, decimal_places: usize, unit: &str) -> String {
    let absolute = value.abs();
    let fixed_threshold = 10.0_f64.powi(-(decimal_places as i32));
    let number = if !value.is_finite() {
        "NON FINITE".to_owned()
    } else if value == 0.0 || (absolute >= fixed_threshold && absolute < 1_000_000.0) {
        format!("{value:.decimal_places$}")
    } else {
        format!("{value:.decimal_places$E}")
    };
    if unit.is_empty() {
        number
    } else {
        format!("{number} {unit}")
    }
}

fn draw_canvas(scene: &mut Scene, _layout: UiLayout, canvas: UiRect, scientific_camera: Camera2d) {
    scene.start_scientific_canvas(canvas, scientific_camera);
    scene.rect(
        ui_rect(canvas),
        12.0,
        ShapeStyle::fill_stroke(
            Color::rgb8(10, 14, 20),
            1.0,
            Color::rgba8(112, 211, 255, 45),
        ),
    );
    scene.start_heads_up_overlay();
}

fn draw_inspector(
    scene: &mut Scene,
    layout: UiLayout,
    panel: UiRect,
    state: &UiState,
    mechanics: Option<MechanicsProjectSnapshotRef<'_>>,
    mechanics_error: Option<&str>,
) {
    let left = panel.min.x + 16.0;
    pixel_font::draw(
        scene,
        layout,
        "INSPECTOR",
        Point::new(left, panel.min.y + 20.0),
        2.7,
        muted(),
    );
    pixel_font::draw(
        scene,
        layout,
        state.selected_tool.label(),
        Point::new(left, panel.min.y + 64.0),
        2.4,
        accent(),
    );
    let body = state.selected_body.and_then(|selected| {
        mechanics.and_then(|project| project.world.bodies.iter().find(|body| body.id == selected))
    });
    let force = body.map(|body| {
        mechanics
            .and_then(|project| project.persistent_forces.get(&body.id))
            .map_or(Force2::ZERO, |contribution| contribution.force)
    });
    let rows = [
        (
            "MASS",
            body.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.mass.get(), 3, "KG"),
            ),
        ),
        (
            "POSITION X",
            body.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.position.x().get(), 3, "M"),
            ),
        ),
        (
            "POSITION Y",
            body.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.position.y().get(), 3, "M"),
            ),
        ),
        (
            "FORCE X",
            force.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.x().get(), 3, "N"),
            ),
        ),
        (
            "FORCE Y",
            force.map_or_else(
                || "--".to_owned(),
                |value| format_si(value.y().get(), 3, "N"),
            ),
        ),
    ];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let y = if index == 0 {
            panel.min.y + 112.0
        } else {
            panel.min.y + 230.0 + (index - 1) as f32 * 58.0
        };
        pixel_font::draw(scene, layout, label, Point::new(left, y), 1.8, faint());
        pixel_font::draw(
            scene,
            layout,
            &value,
            Point::new(left, y + 25.0),
            title_size_for(&value, 1.7, panel.width() - 32.0),
            muted(),
        );
    }
    if body.is_some() {
        for (label, rect) in ["MASS -", "MASS +"]
            .into_iter()
            .zip(layout.inspector_mass_buttons())
        {
            draw_button(scene, layout, rect, label, 0.0, false);
        }
    } else {
        pixel_font::draw(
            scene,
            layout,
            "SELECT A BODY",
            Point::new(left, panel.min.y + 174.0),
            1.5,
            faint(),
        );
    }
    pixel_font::draw(
        scene,
        layout,
        if mechanics_error.is_some() {
            "EDITOR ERROR"
        } else {
            "EDITOR READY"
        },
        Point::new(left, panel.max.y - 54.0),
        1.5,
        if mechanics_error.is_some() {
            warning()
        } else {
            time_accent()
        },
    );
    pixel_font::draw(
        scene,
        layout,
        mechanics_error.unwrap_or("NO SIMULATION STEP"),
        Point::new(left, panel.max.y - 30.0),
        1.25,
        if mechanics_error.is_some() {
            warning()
        } else {
            accent()
        },
    );
}

fn build_time_easter_egg(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(Color::rgb8(3, 10, 8));
    draw_back_button(&mut scene, layout, state, "SIM;X");
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "SIM;TIME",
        Point::new(layout.width * 0.5, layout.height * 0.30),
        9.0,
        time_accent(),
    );
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "TEMPORAL DOMAIN UNSTABLE",
        Point::new(layout.width * 0.5, layout.height * 0.41),
        2.5,
        time_accent().with_alpha(0.64),
    );
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "01:04:08:05:09:06",
        Point::new(layout.width * 0.5, layout.height * 0.54),
        4.0,
        text().with_alpha(0.88),
    );
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "FUNCTIONALITY EXISTS IN ANOTHER WORLD LINE",
        Point::new(layout.width * 0.5, layout.height * 0.66),
        2.0,
        muted(),
    );
    draw_footer(&mut scene, layout, "ESC TO RETURN   EL PSY KONGROO");
    scene
}

fn draw_page_title(scene: &mut Scene, layout: UiLayout, title: &str, subtitle: &str) {
    pixel_font::draw_centered(
        scene,
        layout,
        title,
        Point::new(layout.width * 0.5, 104.0),
        title_size_for(title, 6.2, layout.width - 400.0),
        text(),
    );
    pixel_font::draw_centered(
        scene,
        layout,
        subtitle,
        Point::new(layout.width * 0.5, 154.0),
        title_size_for(subtitle, 2.0, layout.width - 100.0),
        muted(),
    );
}

fn draw_back_button(scene: &mut Scene, layout: UiLayout, state: &UiState, label: &str) {
    draw_button(
        scene,
        layout,
        layout.back_button(),
        label,
        state.animations.back_hover,
        false,
    );
}

fn draw_time_code_hotspot(scene: &mut Scene, layout: UiLayout, state: &UiState) {
    let hover = state.animations.time_code_hover;
    pixel_font::draw_centered(
        scene,
        layout,
        "?",
        layout.time_code_hotspot().center(),
        2.6 + hover * 0.35,
        time_accent().with_alpha(0.09 + hover * 0.20),
    );
}

fn draw_panel(scene: &mut Scene, _layout: UiLayout, rect: UiRect) {
    scene.rect(
        ui_rect(rect),
        12.0,
        ShapeStyle::fill_stroke(surface(), 1.0, border()),
    );
}

fn draw_button(
    scene: &mut Scene,
    layout: UiLayout,
    rect: UiRect,
    label: &str,
    hover: f32,
    selected: bool,
) {
    let amount = hover.clamp(0.0, 1.0);
    let display_rect = rect.expand(amount * 2.0);
    let fill = if selected {
        accent().with_alpha(0.22 + amount * 0.12)
    } else {
        mix_color(surface(), surface_hover(), amount)
    };
    let outline = if selected {
        accent()
    } else {
        mix_color(border(), accent(), amount)
    };
    scene.rect(
        ui_rect(display_rect),
        9.0,
        ShapeStyle::fill_stroke(fill, 1.0 + amount, outline),
    );
    pixel_font::draw_centered(
        scene,
        layout,
        label,
        display_rect.center(),
        title_size_for(label, 3.0, rect.width() - 28.0),
        mix_color(text(), Color::WHITE, amount),
    );
}

fn draw_disabled_button(scene: &mut Scene, layout: UiLayout, rect: UiRect, label: &str) {
    scene.rect(
        ui_rect(rect),
        9.0,
        ShapeStyle::fill_stroke(surface().with_alpha(0.62), 1.0, border().with_alpha(0.60)),
    );
    pixel_font::draw_centered(
        scene,
        layout,
        label,
        Point::new(rect.center().x, rect.center().y - 7.0),
        title_size_for(label, 2.55, rect.width() - 28.0),
        muted(),
    );
    pixel_font::draw_centered(
        scene,
        layout,
        "LOCKED",
        Point::new(rect.center().x, rect.center().y + 12.0),
        1.1,
        warning().with_alpha(0.68),
    );
}

fn draw_footer(scene: &mut Scene, layout: UiLayout, message: &str) {
    pixel_font::draw_centered(
        scene,
        layout,
        message,
        Point::new(layout.width * 0.5, layout.height - 26.0),
        title_size_for(message, 1.8, layout.width - 390.0),
        muted(),
    );
}

fn title_size_for(text: &str, desired: f32, available_width: f32) -> f32 {
    let columns = text
        .chars()
        .count()
        .checked_sub(1)
        .map_or(0, |spacing_count| spacing_count * 4)
        + 3;
    desired.min(available_width / columns as f32).max(0.8)
}

fn ui_rect(rect: UiRect) -> UiRect {
    rect
}

fn ui_point(point: Point) -> Point {
    point
}

fn mix_color(start: Color, end: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::rgba(
        start.red() + (end.red() - start.red()) * amount,
        start.green() + (end.green() - start.green()) * amount,
        start.blue() + (end.blue() - start.blue()) * amount,
        start.alpha() + (end.alpha() - start.alpha()) * amount,
    )
}

fn background() -> Color {
    Color::rgb8(8, 10, 15)
}

fn surface() -> Color {
    Color::rgb8(18, 22, 30)
}

fn surface_hover() -> Color {
    Color::rgb8(38, 46, 59)
}

fn border() -> Color {
    Color::rgb8(52, 61, 75)
}

fn accent() -> Color {
    Color::rgb8(112, 211, 255)
}

fn text() -> Color {
    Color::rgb8(236, 241, 248)
}

fn muted() -> Color {
    Color::rgb8(137, 149, 166)
}

fn faint() -> Color {
    Color::rgb8(90, 102, 119)
}

fn warning() -> Color {
    Color::rgb8(255, 190, 94)
}

fn time_accent() -> Color {
    Color::rgb8(91, 240, 154)
}

fn social_color(link: SocialLink) -> Color {
    match link {
        SocialLink::GitHub => Color::rgb8(240, 246, 252),
        SocialLink::YouTube => Color::rgb8(255, 45, 45),
        SocialLink::Telegram => Color::rgb8(52, 170, 230),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        MAX_RENDERED_THERMAL_BODIES, MAX_RENDERED_WAVE_POINTS, PhysicsWorkspace, build,
        build_electrostatic_lab, build_thermal_lab, build_wave_lab, evenly_spaced_indices,
        f64_to_f32, finite_f64_to_f32, format_si, thermal_rendered_link_count, wave_render_indices,
    };
    use crate::presentation::sim_engine::{UI_FRAME_BUDGET, default_camera};
    use crate::{
        domains::phys::{
            electromagnetism::{
                Coulombs, ElectrostaticWorld, MAX_CHARGES, MAX_ELECTROSTATIC_INTERACTIONS,
                PointChargeSpec, Position2 as ElectrostaticPosition2,
            },
            mechanics::{
                BodySpec, Force2, ForceContribution, ForceSourceId, MAX_BODIES, MechanicsWorld,
                Mobility, Position2, Velocity2,
            },
            thermodynamics::{
                ConductiveLinkSpec, JoulesPerKelvin, Kelvin, MAX_THERMAL_BODIES, MAX_THERMAL_LINKS,
                ThermalBodySpec, ThermalWorld, WattsPerKelvin,
            },
            waves_optics::{MAX_WAVE_SAMPLES, SampleSpacing, WaveSpec, WaveSpeed, WaveWorld},
        },
        foundation::{Kilograms, Meters, MetersPerSecond, Newtons, PhysicalConstants},
        presentation::ui::{
            MechanicsProjectSnapshotRef, PhysicsSnapshotRef,
            catalog::{PhysSubdomain, ProjectTemplate, Screen},
            layout::UiLayout,
            state::UiState,
        },
    };

    #[test]
    fn every_screen_builds_a_non_empty_sim_engine_scene() {
        let layout = UiLayout::new(1920.0, 1080.0);
        for screen in [
            Screen::MainMenu,
            Screen::Settings,
            Screen::Domains,
            Screen::PhysSubdomains,
            Screen::Projects,
            Screen::PhysicsEditor,
            Screen::PhysicsView,
            Screen::PhysicsViewExit,
            Screen::TimeEasterEgg,
        ] {
            let state = UiState {
                screen,
                ..UiState::default()
            };
            let scene = build(
                layout,
                &state,
                None,
                None,
                default_camera(state.selected_subdomain),
            )
            .expect("bounded UI scene");
            assert!(scene.command_count() > 20);
            assert!(scene.budget().is_some());
            assert_eq!(
                scene.scientific.is_some(),
                matches!(
                    screen,
                    Screen::PhysicsEditor | Screen::PhysicsView | Screen::PhysicsViewExit
                )
            );
        }
    }

    #[test]
    fn scientific_position_conversion_never_collapses_nonzero_to_zero() {
        let smallest_binary64 = f64::from_bits(1);
        assert_eq!(f64_to_f32(0.0), Some(0.0));
        assert_eq!(f64_to_f32(smallest_binary64), None);
        assert_eq!(f64_to_f32(-smallest_binary64), None);
        assert_eq!(f64_to_f32(f64::from(f32::MAX)), Some(f32::MAX));
        assert_eq!(f64_to_f32(f64::MAX), None);

        assert_eq!(finite_f64_to_f32(smallest_binary64), Some(0.0));
    }

    #[test]
    fn mechanics_canvas_renders_every_body_in_the_snapshot() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let state = state_for(PhysSubdomain::Mechanics);
        let mut world = MechanicsWorld::new();
        world
            .create_body(BodySpec::new(
                Position2::new(Meters::new(-1.0).expect("X"), Meters::ZERO),
                Velocity2::ZERO,
                Kilograms::new(1.0).expect("mass"),
                Mobility::Dynamic,
            ))
            .expect("first body");
        let one_body = world.snapshot();
        world
            .create_body(BodySpec::new(
                Position2::new(Meters::new(1.0).expect("X"), Meters::ZERO),
                Velocity2::ZERO,
                Kilograms::new(1.0).expect("mass"),
                Mobility::Dynamic,
            ))
            .expect("second body");
        let two_bodies = world.snapshot();

        let forces = BTreeMap::new();
        let one_scene = super::build_mechanics_view(
            layout,
            &state,
            Some(MechanicsProjectSnapshotRef {
                world: &one_body,
                persistent_forces: &forces,
            }),
            None,
            default_camera(PhysSubdomain::Mechanics),
        );
        let two_scene = super::build_mechanics_view(
            layout,
            &state,
            Some(MechanicsProjectSnapshotRef {
                world: &two_bodies,
                persistent_forces: &forces,
            }),
            None,
            default_camera(PhysSubdomain::Mechanics),
        );
        assert!(two_scene.command_count() > one_scene.command_count());
    }

    #[test]
    fn maximum_mechanics_snapshot_and_exit_modal_fit_frame_budgets() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let mut world = MechanicsWorld::new();
        let mut forces = BTreeMap::new();
        let force = Force2::new(
            Newtons::new(1.0).expect("force"),
            Newtons::new(1.0).expect("force"),
        );
        for source in 1..=MAX_BODIES {
            let entity = world
                .create_body(BodySpec::new(
                    Position2::ZERO,
                    Velocity2::ZERO,
                    Kilograms::new(1.0).expect("mass"),
                    Mobility::Dynamic,
                ))
                .expect("body within public maximum");
            forces.insert(
                entity,
                ForceContribution::new(
                    entity,
                    ForceSourceId::new(source as u64).expect("non-zero source"),
                    force,
                ),
            );
        }
        let snapshot = world.snapshot();

        for screen in [Screen::PhysicsView, Screen::PhysicsViewExit] {
            let state = UiState {
                screen,
                selected_subdomain: PhysSubdomain::Mechanics,
                selected_body: snapshot.bodies.last().map(|body| body.id),
                ..UiState::default()
            };
            let frame = build(
                layout,
                &state,
                Some(PhysicsSnapshotRef::Mechanics(MechanicsProjectSnapshotRef {
                    world: &snapshot,
                    persistent_forces: &forces,
                })),
                None,
                default_camera(PhysSubdomain::Mechanics),
            )
            .expect("maximum valid Mechanics snapshot must degrade through LOD");
            assert!(frame.command_count() <= UI_FRAME_BUDGET.max_commands());
            assert_eq!(frame.overlay.is_some(), screen == Screen::PhysicsViewExit);
            assert!(frame.heads_up.is_some());
        }
    }

    #[test]
    fn maximum_wave_snapshot_preserves_a_narrow_extreme_and_fits_budgets() {
        let layout = UiLayout::new(1920.0, 1080.0);
        let spike = MAX_WAVE_SAMPLES / 2 + 7;
        let mut displacement = vec![Meters::ZERO; MAX_WAVE_SAMPLES];
        displacement[spike] = Meters::new(1.0).expect("spike");
        let wave = WaveWorld::new(WaveSpec::fixed_zero(
            SampleSpacing::new(0.01).expect("spacing"),
            WaveSpeed::new(1.0).expect("speed"),
            displacement,
            vec![MetersPerSecond::ZERO; MAX_WAVE_SAMPLES],
        ))
        .expect("maximum wave")
        .snapshot();
        let rendered = wave_render_indices(&wave);
        assert!(rendered.len() <= MAX_RENDERED_WAVE_POINTS);
        assert!(rendered.contains(&spike));

        for screen in [Screen::PhysicsView, Screen::PhysicsViewExit] {
            let state = UiState {
                screen,
                selected_subdomain: PhysSubdomain::WavesAndOptics,
                ..UiState::default()
            };
            let frame = build(
                layout,
                &state,
                Some(PhysicsSnapshotRef::WavesAndOptics(&wave)),
                None,
                default_camera(PhysSubdomain::WavesAndOptics),
            )
            .expect("maximum valid Wave snapshot must degrade through extrema LOD");
            assert!(frame.command_count() <= UI_FRAME_BUDGET.max_commands());
            assert_eq!(frame.overlay.is_some(), screen == Screen::PhysicsViewExit);
        }
    }

    #[test]
    fn maximum_thermal_and_electrostatic_snapshots_fit_exit_frame_budgets() {
        let layout = UiLayout::new(1920.0, 1080.0);

        let mut thermal_world = ThermalWorld::new();
        let mut thermal_bodies = Vec::with_capacity(MAX_THERMAL_BODIES);
        for _ in 0..MAX_THERMAL_BODIES {
            thermal_bodies.push(
                thermal_world
                    .create_body(ThermalBodySpec::new(
                        Kelvin::new(300.0).expect("temperature"),
                        JoulesPerKelvin::new(1_000_000.0).expect("capacity"),
                    ))
                    .expect("thermal body within maximum"),
            );
        }
        let mut links = 0_usize;
        'links: for first in 0..thermal_bodies.len() {
            for second in (first + 1)..thermal_bodies.len() {
                thermal_world
                    .create_link(ConductiveLinkSpec::new(
                        thermal_bodies[first],
                        thermal_bodies[second],
                        WattsPerKelvin::new(1.0).expect("conductance"),
                    ))
                    .expect("thermal link within maximum");
                links += 1;
                if links == MAX_THERMAL_LINKS {
                    break 'links;
                }
            }
        }
        let thermal = thermal_world.snapshot().expect("maximum thermal snapshot");
        let rendered_thermal_indices =
            evenly_spaced_indices(thermal.bodies.len(), MAX_RENDERED_THERMAL_BODIES);
        let rendered_thermal_links =
            thermal_rendered_link_count(&thermal, &rendered_thermal_indices);
        assert_eq!(rendered_thermal_indices.len(), MAX_RENDERED_THERMAL_BODIES);
        assert!(rendered_thermal_links < thermal.links.len());
        let thermal_state = UiState {
            screen: Screen::PhysicsViewExit,
            selected_subdomain: PhysSubdomain::Thermodynamics,
            ..UiState::default()
        };
        let thermal_frame = build(
            layout,
            &thermal_state,
            Some(PhysicsSnapshotRef::Thermodynamics(&thermal)),
            None,
            default_camera(PhysSubdomain::Thermodynamics),
        )
        .expect("maximum valid Thermal snapshot must degrade through LOD");
        assert!(thermal_frame.command_count() <= UI_FRAME_BUDGET.max_commands());
        assert!(thermal_frame.heads_up.is_some());
        assert!(thermal_frame.overlay.is_some());

        let mut electrostatic_world = ElectrostaticWorld::new();
        for _ in 0..MAX_CHARGES {
            electrostatic_world
                .create_charge(PointChargeSpec::new(
                    ElectrostaticPosition2::new(Meters::new(-1.0).expect("X"), Meters::ZERO),
                    Coulombs::new(1.0e-15).expect("charge"),
                ))
                .expect("charge within maximum");
        }
        let probe_count = MAX_ELECTROSTATIC_INTERACTIONS / MAX_CHARGES;
        for _ in 0..probe_count {
            electrostatic_world
                .create_probe(ElectrostaticPosition2::ZERO)
                .expect("probe within pair-work maximum");
        }
        let electrostatic = electrostatic_world
            .evaluate(&PhysicalConstants::codata_2022())
            .expect("maximum electrostatic interaction workload");
        let electrostatic_state = UiState {
            screen: Screen::PhysicsViewExit,
            selected_subdomain: PhysSubdomain::Electromagnetism,
            ..UiState::default()
        };
        let electrostatic_frame = build(
            layout,
            &electrostatic_state,
            Some(PhysicsSnapshotRef::Electromagnetism(&electrostatic)),
            None,
            default_camera(PhysSubdomain::Electromagnetism),
        )
        .expect("maximum valid Electrostatic workload must fit renderer budgets");
        assert!(electrostatic_frame.command_count() <= UI_FRAME_BUDGET.max_commands());
        assert!(electrostatic_frame.heads_up.is_some());
        assert!(electrostatic_frame.overlay.is_some());
    }

    #[test]
    fn extreme_scientific_values_use_bounded_text_and_build_complete_frames() {
        for value in [f64::MAX, f64::MIN, f64::from_bits(1), -f64::from_bits(1)] {
            let formatted = format_si(value, 3, "M/S/S");
            assert!(formatted.len() <= 20);
            assert!(formatted.contains('E'));
        }

        let layout = UiLayout::new(1920.0, 1080.0);
        let mut thermal_world = ThermalWorld::new();
        thermal_world
            .create_body(ThermalBodySpec::new(
                Kelvin::new(f64::MAX).expect("finite maximum temperature"),
                JoulesPerKelvin::new(1.0).expect("capacity"),
            ))
            .expect("representable maximum energy");
        let thermal = thermal_world.snapshot().expect("thermal snapshot");
        let state = UiState {
            screen: Screen::PhysicsViewExit,
            selected_subdomain: PhysSubdomain::Thermodynamics,
            ..UiState::default()
        };
        let frame = build(
            layout,
            &state,
            Some(PhysicsSnapshotRef::Thermodynamics(&thermal)),
            None,
            default_camera(PhysSubdomain::Thermodynamics),
        )
        .expect("maximum finite thermal text must remain bounded");
        assert!(frame.command_count() <= UI_FRAME_BUDGET.max_commands());

        let mut mechanics_world = MechanicsWorld::new();
        let entity = mechanics_world
            .create_body(BodySpec::new(
                Position2::ZERO,
                Velocity2::ZERO,
                Kilograms::new(f64::MAX).expect("maximum finite mass"),
                Mobility::Dynamic,
            ))
            .expect("mechanics body");
        mechanics_world
            .create_body(BodySpec::new(
                Position2::new(
                    Meters::new(f64::MAX).expect("finite but non-renderable X"),
                    Meters::new(f64::from_bits(1)).expect("finite but collapsing Y"),
                ),
                Velocity2::ZERO,
                Kilograms::new(1.0).expect("mass"),
                Mobility::Dynamic,
            ))
            .expect("out-of-visual-range mechanics body");
        let mechanics = mechanics_world.snapshot();
        let mut forces = BTreeMap::new();
        forces.insert(
            entity,
            ForceContribution::new(
                entity,
                ForceSourceId::new(1).expect("source"),
                Force2::new(
                    Newtons::new(f64::MAX).expect("force"),
                    Newtons::new(f64::MAX).expect("force"),
                ),
            ),
        );
        let state = UiState {
            screen: Screen::PhysicsEditor,
            selected_subdomain: PhysSubdomain::Mechanics,
            selected_body: Some(entity),
            ..UiState::default()
        };
        let frame = build(
            layout,
            &state,
            Some(PhysicsSnapshotRef::Mechanics(MechanicsProjectSnapshotRef {
                world: &mechanics,
                persistent_forces: &forces,
            })),
            None,
            default_camera(PhysSubdomain::Mechanics),
        )
        .expect("maximum finite Mechanics text and vector must remain bounded");
        assert!(frame.command_count() <= UI_FRAME_BUDGET.max_commands());

        let tiny = f64::from_bits(1);
        let wave = WaveWorld::new(WaveSpec::fixed_zero(
            SampleSpacing::new(tiny).expect("smallest positive spacing"),
            WaveSpeed::new(1.0).expect("speed"),
            vec![Meters::ZERO; 3],
            vec![MetersPerSecond::ZERO; 3],
        ))
        .expect("finite tiny grid")
        .snapshot();
        let state = UiState {
            screen: Screen::PhysicsEditor,
            selected_subdomain: PhysSubdomain::WavesAndOptics,
            ..UiState::default()
        };
        let frame = build(
            layout,
            &state,
            Some(PhysicsSnapshotRef::WavesAndOptics(&wave)),
            None,
            default_camera(PhysSubdomain::WavesAndOptics),
        )
        .expect("nonzero f64 positions below f32 range are diagnosed, not collapsed");
        assert!(frame.command_count() <= UI_FRAME_BUDGET.max_commands());
    }

    #[test]
    fn thermal_palette_contrast_shrinks_as_temperatures_converge() {
        fn color_distance(first: ::sim_engine::Color, second: ::sim_engine::Color) -> f32 {
            first
                .to_array()
                .into_iter()
                .zip(second.to_array())
                .map(|(left, right)| (left - right).abs())
                .sum()
        }

        let initial_contrast =
            color_distance(super::thermal_color(280.0), super::thermal_color(420.0));
        let near_equilibrium =
            color_distance(super::thermal_color(349.0), super::thermal_color(351.0));
        assert!(near_equilibrium < initial_contrast * 0.02);
    }

    #[test]
    fn every_active_non_mechanics_snapshot_builds_its_scientific_scene() {
        let layout = UiLayout::new(1920.0, 1080.0);

        let mut thermal_world = ThermalWorld::new();
        let hot = thermal_world
            .create_body(ThermalBodySpec::new(
                Kelvin::new(400.0).expect("temperature"),
                JoulesPerKelvin::new(20.0).expect("capacity"),
            ))
            .expect("hot body");
        let cold = thermal_world
            .create_body(ThermalBodySpec::new(
                Kelvin::new(280.0).expect("temperature"),
                JoulesPerKelvin::new(20.0).expect("capacity"),
            ))
            .expect("cold body");
        thermal_world
            .create_link(ConductiveLinkSpec::new(
                hot,
                cold,
                WattsPerKelvin::new(2.0).expect("conductance"),
            ))
            .expect("link");
        let thermal = thermal_world.snapshot().expect("thermal snapshot");
        let thermal_state = state_for(PhysSubdomain::Thermodynamics);
        assert!(
            build_thermal_lab(
                layout,
                &thermal_state,
                &thermal,
                None,
                PhysicsWorkspace::View,
                default_camera(PhysSubdomain::Thermodynamics),
            )
            .command_count()
                > 100
        );

        let displacement = (0..65)
            .map(|index| Meters::new(if index == 32 { 1.0 } else { 0.0 }).expect("displacement"))
            .collect();
        let wave = WaveWorld::new(WaveSpec::fixed_zero(
            SampleSpacing::new(0.1).expect("spacing"),
            WaveSpeed::new(1.0).expect("speed"),
            displacement,
            vec![MetersPerSecond::ZERO; 65],
        ))
        .expect("wave")
        .snapshot();
        let wave_state = state_for(PhysSubdomain::WavesAndOptics);
        assert!(
            build_wave_lab(
                layout,
                &wave_state,
                &wave,
                None,
                PhysicsWorkspace::View,
                default_camera(PhysSubdomain::WavesAndOptics),
            )
            .command_count()
                > 100
        );

        let mut electrostatic_world = ElectrostaticWorld::new();
        electrostatic_world
            .create_charge(PointChargeSpec::new(
                ElectrostaticPosition2::ZERO,
                Coulombs::new(1.0e-9).expect("charge"),
            ))
            .expect("charge");
        electrostatic_world
            .create_probe(ElectrostaticPosition2::new(
                Meters::new(1.0).expect("X"),
                Meters::ZERO,
            ))
            .expect("probe");
        let electrostatic = electrostatic_world
            .evaluate(&PhysicalConstants::codata_2022())
            .expect("electrostatic snapshot");
        let electrostatic_state = state_for(PhysSubdomain::Electromagnetism);
        assert!(
            build_electrostatic_lab(
                layout,
                &electrostatic_state,
                &electrostatic,
                None,
                PhysicsWorkspace::View,
                default_camera(PhysSubdomain::Electromagnetism),
            )
            .command_count()
                > 70
        );
    }

    fn state_for(subdomain: PhysSubdomain) -> UiState {
        UiState {
            screen: Screen::PhysicsView,
            selected_subdomain: subdomain,
            selected_project: ProjectTemplate::PrimaryLab,
            ..UiState::default()
        }
    }
}
