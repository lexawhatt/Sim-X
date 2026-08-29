use ::sim_engine::{Color, Rect as EngineRect, Scene, ShapeStyle, Vec2};

use crate::presentation::ui::{
    catalog::{MechanicsTool, PhysSubdomain, Screen, TopDomain},
    geometry::{Point, UiRect},
    layout::UiLayout,
    state::{Notice, UiState},
};

use super::pixel_font;

pub(super) fn build(layout: UiLayout, state: &UiState) -> Scene {
    match state.screen {
        Screen::Domains => build_domain_menu(layout, state),
        Screen::PhysSubdomains => build_phys_menu(layout, state),
        Screen::MechanicsEditor => build_mechanics_editor(layout, state),
        Screen::TimeEasterEgg => build_time_easter_egg(layout, state),
    }
}

fn build_time_easter_egg(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(Color::rgb8(3, 10, 8)).expect("UI background is valid");
    draw_button(
        &mut scene,
        layout,
        layout.domains_back_button(),
        "SIM;X",
        state.animations.back_hover,
        false,
    );
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

fn build_domain_menu(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background()).expect("UI background is valid");
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
        "SIM;X",
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
            TopDomain::Math => "SIM;MATH FOLLOWS THE MECHANICS SLICE",
            TopDomain::Chem => "SIM;CHEM FOLLOWS THE MECHANICS SLICE",
            TopDomain::Biol => "SIM;BIOL FOLLOWS THE MECHANICS SLICE",
            TopDomain::Phys => "SIM;PHYS IS ACTIVE",
        },
        _ => "2 OR CLICK PHYS   FIRST ACTIVE DOMAIN",
    };
    draw_footer(&mut scene, layout, message);
    draw_time_code_hotspot(&mut scene, layout, state);
    scene
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

fn build_phys_menu(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background()).expect("UI background is valid");
    let compact = layout.height < 700.0;
    let title_y = if compact { 82.0 } else { 100.0 };
    let title_size = if compact { 5.0 } else { 6.0 };
    draw_button(
        &mut scene,
        layout,
        layout.domains_back_button(),
        "DOMAINS",
        state.animations.back_hover,
        false,
    );

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

    let message = match state.notice {
        Some(Notice::SubdomainUnavailable(subdomain)) => match subdomain {
            PhysSubdomain::Thermodynamics => "THERMODYNAMICS IS PLANNED NEXT",
            PhysSubdomain::WavesAndOptics => "WAVES & OPTICS IS PLANNED",
            PhysSubdomain::Electromagnetism => "ELECTROMAGNETISM IS PLANNED",
            PhysSubdomain::Relativity => "RELATIVITY IS PLANNED",
            PhysSubdomain::FluidDynamics => "FLUID DYNAMICS IS PLANNED",
            PhysSubdomain::Sandbox => "PHYS;SANDBOX OPENS AFTER CORE EFFECTS",
            PhysSubdomain::Mechanics => "PHYS;MECHANICS IS ACTIVE",
        },
        _ => "1 OR CLICK MECHANICS   7 OPENS SANDBOX LATER",
    };
    draw_footer(&mut scene, layout, message);
    scene
}

fn build_mechanics_editor(layout: UiLayout, state: &UiState) -> Scene {
    let mut scene = Scene::new(background()).expect("UI background is valid");
    draw_button(
        &mut scene,
        layout,
        layout.phys_back_button(),
        "PHYS",
        state.animations.back_hover,
        false,
    );
    draw_button(
        &mut scene,
        layout,
        layout.editor_reset_button(),
        "RESET",
        state.animations.reset_hover,
        false,
    );
    pixel_font::draw_centered(
        &mut scene,
        layout,
        "PHYS;MECHANICS",
        Point::new(layout.width * 0.5, 43.0),
        4.2,
        text(),
    );

    let left_panel = layout.editor_left_panel();
    let right_panel = layout.editor_right_panel();
    let canvas = layout.editor_canvas();
    draw_panel(&mut scene, layout, left_panel);
    draw_panel(&mut scene, layout, right_panel);
    draw_canvas(&mut scene, layout, canvas);

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
        draw_button(
            &mut scene,
            layout,
            rect,
            tool.label(),
            hover,
            state.selected_tool == tool,
        );
    }
    pixel_font::draw(
        &mut scene,
        layout,
        "MECHANICS SET",
        Point::new(left_panel.min.x + 16.0, left_panel.max.y - 78.0),
        2.0,
        muted(),
    );
    pixel_font::draw(
        &mut scene,
        layout,
        "BODIES CONSTRAINTS",
        Point::new(left_panel.min.x + 16.0, left_panel.max.y - 52.0),
        1.6,
        faint(),
    );
    pixel_font::draw(
        &mut scene,
        layout,
        "FORCES MOTION",
        Point::new(left_panel.min.x + 16.0, left_panel.max.y - 30.0),
        1.6,
        faint(),
    );

    draw_pendulum(&mut scene, layout, canvas, state.pendulum_anchor);
    draw_inspector(&mut scene, layout, right_panel, state);

    if let Some(Notice::ToolAwaitingDomain(tool)) = state.notice {
        pixel_font::draw_centered(
            &mut scene,
            layout,
            &format!("{} TOOL AFTER F=MA CONTRACTS", tool.label()),
            Point::new(canvas.center().x, canvas.max.y - 30.0),
            1.8,
            warning(),
        );
    } else {
        pixel_font::draw_centered(
            &mut scene,
            layout,
            "SELECT PENDULUM THEN CLICK TO PLACE",
            Point::new(canvas.center().x, canvas.max.y - 30.0),
            1.8,
            muted(),
        );
    }

    scene
}

fn draw_canvas(scene: &mut Scene, layout: UiLayout, canvas: UiRect) {
    scene.rect(
        screen_rect_to_world(layout, canvas),
        12.0,
        ShapeStyle::fill_stroke(
            Color::rgb8(10, 14, 20),
            1.0,
            Color::rgba8(112, 211, 255, 45),
        ),
    );
    let grid_color = Color::rgba8(112, 211, 255, 22);
    let spacing = 32.0;
    let mut x = canvas.min.x + spacing;
    while x < canvas.max.x {
        scene.line(
            screen_to_world(layout, Point::new(x, canvas.min.y)),
            screen_to_world(layout, Point::new(x, canvas.max.y)),
            1.0,
            grid_color,
        );
        x += spacing;
    }
    let mut y = canvas.min.y + spacing;
    while y < canvas.max.y {
        scene.line(
            screen_to_world(layout, Point::new(canvas.min.x, y)),
            screen_to_world(layout, Point::new(canvas.max.x, y)),
            1.0,
            grid_color,
        );
        y += spacing;
    }
}

fn draw_pendulum(scene: &mut Scene, layout: UiLayout, canvas: UiRect, normalized_anchor: Point) {
    let anchor = canvas.point_from_normalized(normalized_anchor);
    let length = canvas
        .height()
        .min(canvas.width())
        .mul_add(0.34, 0.0)
        .clamp(90.0, 170.0);
    let angle = 0.42_f32;
    let bob = Point::new(
        anchor.x + angle.sin() * length,
        anchor.y + angle.cos() * length,
    );
    scene.line(
        screen_to_world(layout, Point::new(anchor.x, anchor.y + 4.0)),
        screen_to_world(layout, bob),
        4.0,
        text().with_alpha(0.86),
    );
    scene.circle(
        screen_to_world(layout, anchor),
        8.0,
        ShapeStyle::fill_stroke(surface_hover(), 2.0, accent()),
    );
    scene.circle(
        screen_to_world(layout, bob),
        24.0,
        ShapeStyle::fill_stroke(accent().with_alpha(0.74), 2.0, accent()),
    );
    scene.line(
        screen_to_world(layout, Point::new(anchor.x - 30.0, anchor.y - 10.0)),
        screen_to_world(layout, Point::new(anchor.x + 30.0, anchor.y - 10.0)),
        4.0,
        accent().with_alpha(0.72),
    );
}

fn draw_inspector(scene: &mut Scene, layout: UiLayout, panel: UiRect, state: &UiState) {
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
    let rows = [
        ("MASS", "DOMAIN VALUE"),
        ("LENGTH", "DOMAIN VALUE"),
        ("GRAVITY", "CONSTANTS REGISTRY"),
        ("STATE", "DESIGN MODE"),
    ];
    for (index, (label, value)) in rows.into_iter().enumerate() {
        let y = panel.min.y + 118.0 + index as f32 * 72.0;
        pixel_font::draw(scene, layout, label, Point::new(left, y), 1.8, faint());
        pixel_font::draw(
            scene,
            layout,
            value,
            Point::new(left, y + 25.0),
            title_size_for(value, 1.7, panel.width()),
            muted(),
        );
    }
    pixel_font::draw(
        scene,
        layout,
        "SIMULATION DISABLED",
        Point::new(left, panel.max.y - 54.0),
        1.5,
        warning(),
    );
    pixel_font::draw(
        scene,
        layout,
        "UNTIL DOMAIN STEP EXISTS",
        Point::new(left, panel.max.y - 30.0),
        1.3,
        faint(),
    );
}

fn draw_panel(scene: &mut Scene, layout: UiLayout, rect: UiRect) {
    scene.rect(
        screen_rect_to_world(layout, rect),
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
        screen_rect_to_world(layout, display_rect),
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

fn draw_footer(scene: &mut Scene, layout: UiLayout, message: &str) {
    pixel_font::draw_centered(
        scene,
        layout,
        message,
        Point::new(layout.width * 0.5, layout.height - 26.0),
        title_size_for(message, 1.8, layout.width - 48.0),
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

fn screen_rect_to_world(layout: UiLayout, rect: UiRect) -> EngineRect {
    EngineRect::new(
        screen_to_world(layout, rect.max),
        screen_to_world(layout, rect.min),
    )
    .normalized()
}

fn screen_to_world(layout: UiLayout, point: Point) -> Vec2 {
    Vec2::new(point.x - layout.width * 0.5, layout.height * 0.5 - point.y)
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

#[cfg(test)]
mod tests {
    use super::build;
    use crate::presentation::ui::{catalog::Screen, layout::UiLayout, state::UiState};

    #[test]
    fn every_screen_builds_a_non_empty_sim_engine_scene() {
        let layout = UiLayout::new(1920.0, 1080.0);

        for screen in [
            Screen::Domains,
            Screen::PhysSubdomains,
            Screen::MechanicsEditor,
            Screen::TimeEasterEgg,
        ] {
            let state = UiState {
                screen,
                ..UiState::default()
            };
            assert!(build(layout, &state).command_count() > 20);
        }
    }
}
