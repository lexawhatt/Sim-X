use sim_logic::prelude::*;

use super::{
    assets::MenuAssets,
    layout::{Layout, Rect},
    state::{MenuState, Overlay, PhysicsScale, SocialLink, Target},
    theme,
};

#[derive(Component, Clone, Copy)]
pub(crate) enum Panel {
    HeaderLine,
    TitleLine,
    Button(Target),
    Focus(Target),
    Scrim,
    Modal,
    ModalAccent,
}

#[derive(Component, Clone, Copy)]
pub(crate) enum Label {
    Eyebrow,
    Logo,
    Tagline,
    Edition,
    Hint,
    Tooltip,
    LinkStatus,
    Button(Target),
    Arrow(Target),
    ModalTitle,
    ModalLineOne,
    ModalLineTwo,
    ModalDetail,
    MotionValue,
    MinimumTitle,
    MinimumDetail,
}

#[derive(Component, Clone, Copy)]
pub(crate) enum Picture {
    Halo,
    Social(SocialLink),
}

fn panel(depth: f32) -> LogicResult<ScreenRectangleVisual> {
    let mut result = ScreenRectangleVisual::new(
        LogicalScreenPosition::new(0.0, 0.0),
        LogicalScreenVector::new(1.0, 1.0),
        Color::TRANSPARENT,
    )?;
    result.set_draw_order_depth(depth)?;
    Ok(result)
}

fn label(font: &TextFont, text: &str, depth: f32) -> LogicResult<ScreenTextVisual> {
    let mut result =
        ScreenTextVisual::new(font.clone(), text, LogicalScreenPosition::new(0.0, 0.0))?;
    result.set_tint(Color::TRANSPARENT)?;
    result.set_draw_order_depth(depth)?;
    Ok(result)
}

pub(crate) fn spawn(world: &mut WorldBuilder, assets: &MenuAssets) -> LogicResult {
    world.spawn(ActiveCamera2d::centered(1.0)?)?;
    world.insert_resource(WorldBackground::new(theme::BACKGROUND)?)?;
    world.insert_resource(MenuState::default())?;
    for (kind, depth) in [
        (Panel::HeaderLine, 0.0),
        (Panel::TitleLine, 0.0),
        (Panel::Scrim, 10.0),
        (Panel::Modal, 11.0),
        (Panel::ModalAccent, 12.0),
    ] {
        world.spawn((kind, panel(depth)?))?;
    }
    for target in Target::ALL {
        let depth = if target.index() >= 6 { 12.0 } else { 1.0 };
        world.spawn((Panel::Button(target), panel(depth)?))?;
        world.spawn((Panel::Focus(target), panel(depth + 0.1)?))?;
        if !matches!(target, Target::Social(_)) {
            let font = if matches!(target, Target::Domain(_) | Target::Scale(_)) {
                &assets.heading
            } else {
                &assets.body
            };
            world.spawn((Label::Button(target), label(font, "", depth + 1.0)?))?;
        }
    }
    for target in [Target::Domains, Target::Settings, Target::Quit] {
        world.spawn((Label::Arrow(target), label(&assets.body, "\u{2192}", 2.0)?))?;
    }
    for (kind, font, text, depth) in [
        (
            Label::Eyebrow,
            &assets.small,
            "PHYS / MATH / CHEM / BIOL",
            2.0,
        ),
        (Label::Logo, &assets.logo, "Sim;X", 2.0),
        (
            Label::Tagline,
            &assets.body,
            "Scientific worlds you can touch",
            2.0,
        ),
        (
            Label::Edition,
            &assets.small,
            "EARLY DEVELOPMENT  /  0.2",
            2.0,
        ),
        (
            Label::Hint,
            &assets.small,
            "Tab / Arrows to navigate   Enter to select",
            2.0,
        ),
        (Label::Tooltip, &assets.small, "", 2.0),
        (Label::LinkStatus, &assets.small, "", 2.0),
        (Label::ModalTitle, &assets.heading, "", 13.0),
        (Label::ModalLineOne, &assets.small, "", 13.0),
        (Label::ModalLineTwo, &assets.small, "", 13.0),
        (Label::ModalDetail, &assets.small, "", 13.0),
        (Label::MotionValue, &assets.body, "", 13.0),
        (
            Label::MinimumTitle,
            &assets.body,
            "Enlarge the window",
            14.0,
        ),
        (
            Label::MinimumDetail,
            &assets.small,
            "Minimum size: 360 x 560",
            14.0,
        ),
    ] {
        world.spawn((kind, label(font, text, depth)?))?;
    }
    for kind in [
        Picture::Halo,
        Picture::Social(SocialLink::Github),
        Picture::Social(SocialLink::Youtube),
        Picture::Social(SocialLink::Telegram),
    ] {
        let image = match kind {
            Picture::Halo => assets.halo,
            Picture::Social(link) => assets.icons[link.index()],
        };
        let mut visual = ScreenImageVisual::new(
            image,
            LogicalScreenPosition::new(0.0, 0.0),
            LogicalScreenVector::new(1.0, 1.0),
        )?;
        visual.set_filter(ImageFilter::Linear);
        visual.set_tint(Color::TRANSPARENT)?;
        visual.set_draw_order_depth(if matches!(kind, Picture::Halo) {
            -6.0
        } else {
            1.5
        })?;
        world.spawn((kind, visual))?;
    }
    Ok(())
}

fn visible_target(state: &MenuState, target: Target) -> bool {
    (target.index() < 6 && !state.exploration()) || state.targets().contains(&target)
}

fn target_opacity(state: &MenuState, target: Target) -> f32 {
    match target {
        Target::Domain(_) | Target::BackToMenu => state.picker_mix[0],
        Target::Scale(_) | Target::BackToDomains => state.picker_mix[1],
        Target::CreateProject | Target::BackToScales => state.picker_mix[2],
        Target::ProjectName | Target::ConfirmProject | Target::CancelProject => state.picker_mix[3],
        Target::Domains | Target::Settings | Target::Quit | Target::Social(_) => {
            1.0 - state.domains_blend
        }
        _ => {
            if visible_target(state, target) {
                1.0
            } else {
                0.0
            }
        }
    }
}

fn panel_style(kind: Panel, layout: &Layout, state: &MenuState) -> (Rect, Color) {
    let modal_visible = matches!(
        state.overlay,
        Overlay::Settings | Overlay::Quit | Overlay::CreateProject
    );
    match kind {
        Panel::HeaderLine => (
            Rect::new(28.0, 57.0, (layout.width - 56.0).max(1.0), 1.0),
            theme::LINE.with_alpha(0.5),
        ),
        Panel::TitleLine => (
            Rect::new(
                layout.left,
                layout.top + if layout.short { 110.0 } else { 133.0 },
                44.0,
                2.0,
            ),
            theme::ACCENT.scale_alpha(1.0 - state.domains_blend),
        ),
        Panel::Button(target) | Panel::Focus(target) => {
            let mut rect = layout.target(target, state.overlay);
            let selected =
                matches!(target, Target::Domain(domain) if state.selected_domain == Some(domain));
            let emphasis = state.emphasis[target.index()].max(if selected { 0.5 } else { 0.0 });
            let color = if matches!(kind, Panel::Focus(_)) {
                rect.x += 8.0;
                rect.y += 15.0;
                rect.width = 2.0;
                rect.height -= 30.0;
                let accent = match target {
                    Target::Domain(domain) => theme::domain(domain),
                    _ => theme::ACCENT,
                };
                accent.with_alpha(emphasis)
            } else {
                theme::button(emphasis, state.pointer.captured() == Some(target))
            };
            (rect, color.scale_alpha(target_opacity(state, target)))
        }
        Panel::Scrim => (
            Rect::new(0.0, 0.0, layout.width, layout.height),
            Color::BLACK.with_alpha(if modal_visible { 0.78 } else { 0.0 }),
        ),
        Panel::Modal => (
            layout.modal,
            if modal_visible {
                theme::PANEL
            } else {
                Color::TRANSPARENT
            },
        ),
        Panel::ModalAccent => (
            Rect::new(
                layout.modal.x + 24.0,
                layout.modal.y + 1.0,
                (layout.modal.width - 48.0).max(1.0),
                2.0,
            ),
            theme::ACCENT.with_alpha(if modal_visible { 1.0 } else { 0.0 }),
        ),
    }
}

fn button_text(target: Target, state: &MenuState) -> &str {
    match target {
        Target::Domains => "Domains",
        Target::Settings => "Settings",
        Target::Quit | Target::ConfirmQuit => "Quit",
        Target::ReduceMotion => "Reduce motion",
        Target::Close if state.overlay == Overlay::Quit => "Stay here",
        Target::Close => "Back to menu",
        Target::BackToMenu => "Back to main menu",
        Target::Social(link) => link.name(),
        Target::Domain(domain) => domain.title(),
        Target::Scale(scale) => scale.label(),
        Target::BackToDomains => "Back to domains",
        Target::BackToScales => "Back to Phys;X",
        Target::CreateProject => "+  Create new project",
        Target::ProjectName if state.project_name.is_empty() => "Project name",
        Target::ProjectName => &state.project_name,
        Target::ConfirmProject => "Create",
        Target::CancelProject => "Cancel",
    }
}

fn refresh_label(
    kind: Label,
    visual: &mut ScreenTextVisual,
    layout: &Layout,
    state: &MenuState,
) -> LogicResult {
    let modal_visible = matches!(
        state.overlay,
        Overlay::Settings | Overlay::Quit | Overlay::CreateProject
    );
    let (x, y, tint) = match kind {
        Label::Eyebrow => (layout.left, layout.top, theme::ACCENT),
        Label::Logo => (
            layout.left - 5.0,
            layout.top + if layout.short { 94.0 } else { 104.0 },
            theme::INK,
        ),
        Label::Tagline => (
            layout.left,
            layout.top + if layout.short { 143.0 } else { 175.0 },
            theme::INK,
        ),
        Label::Edition => (28.0, 36.0, theme::MUTED),
        Label::Hint => {
            let text = if layout.compact {
                "Tab / Arrows / Enter   F11 fullscreen"
            } else {
                "Tab / Arrows to navigate   Enter to select   F11 fullscreen"
            };
            visual.set_text(text)?;
            (
                layout.left,
                layout.top + if layout.short { 406.0 } else { 459.0 },
                theme::HINT,
            )
        }
        Label::Tooltip => {
            let target = state.hovered.or(state.focus.focused());
            visual.set_text(match target {
                Some(Target::Social(link)) if layout.compact => link.name(),
                Some(Target::Social(link)) => link.url(),
                _ => "",
            })?;
            (28.0, layout.footer - 12.0, theme::ACCENT)
        }
        Label::LinkStatus => {
            visual.set_text(state.link_status)?;
            if layout.compact {
                let previewing_link = matches!(
                    state.hovered.or(state.focus.focused()),
                    Some(Target::Social(_))
                );
                (
                    28.0,
                    layout.footer - 12.0,
                    if previewing_link {
                        Color::TRANSPARENT
                    } else {
                        theme::MUTED
                    },
                )
            } else {
                (190.0, layout.footer + 27.0, theme::MUTED)
            }
        }
        Label::Button(target) => {
            visual.set_text(button_text(target, state))?;
            let rect = layout.target(target, state.overlay);
            let centered = matches!(
                target,
                Target::Domain(_)
                    | Target::Scale(_)
                    | Target::BackToMenu
                    | Target::BackToDomains
                    | Target::BackToScales
                    | Target::CreateProject
                    | Target::ConfirmProject
                    | Target::CancelProject
            );
            visual.set_alignment(if centered {
                TextAlignment::Center
            } else {
                TextAlignment::Left
            })?;
            let tint = match target {
                Target::Domain(domain) => theme::domain(domain),
                Target::Scale(PhysicsScale::Micro | PhysicsScale::Astra) => theme::MUTED,
                Target::ProjectName if state.project_name.is_empty() => theme::MUTED,
                _ => theme::INK,
            };
            let metrics = visual.metrics();
            let baseline =
                rect.y + rect.height * 0.5 + (metrics.ascent() + metrics.descent()) * 0.5;
            (
                if centered {
                    rect.x + rect.width * 0.5
                } else {
                    rect.x + 20.0
                },
                baseline,
                tint.scale_alpha(target_opacity(state, target)),
            )
        }
        Label::Arrow(target) => {
            let rect = layout.target(target, state.overlay);
            (
                rect.x + rect.width - 36.0 + state.emphasis[target.index()] * 4.0,
                rect.y + 34.0,
                theme::ACCENT.with_alpha(0.45 + 0.55 * state.emphasis[target.index()]),
            )
        }
        Label::ModalTitle | Label::ModalLineOne | Label::ModalLineTwo | Label::ModalDetail => {
            let lines = match state.overlay {
                Overlay::None => ["", "", "", ""],
                Overlay::Domains => ["", "", "", ""],
                Overlay::PhysicsScales | Overlay::Projects => ["", "", "", ""],
                Overlay::CreateProject => [
                    "New Macro project",
                    "Start with an empty physical scene.",
                    "Latin input. Up to 64 characters.",
                    if state.project_error.is_empty() {
                        "In memory only. No project saving yet."
                    } else {
                        state.project_error
                    },
                ],
                Overlay::Settings => [
                    "Settings",
                    "Display and interaction.",
                    "Preferences apply to this session.",
                    "Disables decorative animation.",
                ],
                Overlay::Quit => ["See you soon?", "Leave Sim;X for now?", "", ""],
            };
            let index = match kind {
                Label::ModalTitle => 0,
                Label::ModalLineOne => 1,
                Label::ModalLineTwo => 2,
                _ => 3,
            };
            visual.set_text(lines[index])?;
            let offsets = [54.0, 92.0, 115.0, 212.0];
            let tint = if index == 0 { theme::INK } else { theme::MUTED };
            (
                layout.modal.x + 24.0,
                layout.modal.y + offsets[index],
                if modal_visible {
                    tint
                } else {
                    Color::TRANSPARENT
                },
            )
        }
        Label::MotionValue => {
            visual.set_text(if state.reduced_motion { "On" } else { "Off" })?;
            visual.set_alignment(TextAlignment::Right)?;
            (
                layout.modal.x + layout.modal.width - 44.0,
                layout.modal.y + 164.0,
                if state.overlay == Overlay::Settings {
                    theme::ACCENT
                } else {
                    Color::TRANSPARENT
                },
            )
        }
        Label::MinimumTitle | Label::MinimumDetail => {
            visual.set_alignment(TextAlignment::Center)?;
            let offset = if matches!(kind, Label::MinimumTitle) {
                -10.0
            } else {
                18.0
            };
            (
                layout.width * 0.5,
                layout.height * 0.5 + offset,
                if layout.usable() {
                    Color::TRANSPARENT
                } else {
                    theme::INK
                },
            )
        }
    };
    visual.set_position(LogicalScreenPosition::new(x, y))?;
    let home_label = matches!(
        kind,
        Label::Eyebrow
            | Label::Logo
            | Label::Tagline
            | Label::Hint
            | Label::Tooltip
            | Label::LinkStatus
            | Label::Arrow(_)
    );
    visual.set_tint(if home_label {
        tint.scale_alpha(1.0 - state.domains_blend)
    } else {
        tint
    })?;
    Ok(())
}

pub(crate) fn refresh(
    viewport: FrameViewport,
    state: Option<Res<MenuState>>,
    mut panels: Query<(&Panel, &mut ScreenRectangleVisual)>,
    mut labels: Query<(&Label, &mut ScreenTextVisual)>,
    mut pictures: Query<(&Picture, &mut ScreenImageVisual)>,
) -> LogicResult {
    let Some(state) = state else {
        return Ok(());
    };
    let layout = Layout::new(viewport.logical());
    let viewport_clip = ScreenClip::new(
        LogicalScreenPosition::new(0.0, 0.0),
        LogicalScreenVector::new(layout.width, layout.height),
    )?;
    for (kind, mut visual) in &mut panels {
        let (rect, color) = panel_style(*kind, &layout, &state);
        visual.set_geometry(rect.position(), rect.size())?;
        visual.set_corner_radius(match kind {
            Panel::Button(_) => super::layout::BUTTON_RADIUS,
            // Engine 0.4.1 rejects the radius-1 tessellation of 2px-wide bars
            // during native projection. Square indicators avoid that failure.
            Panel::Focus(_) => 0.0,
            Panel::Modal => 16.0,
            _ => 0.0,
        })?;
        visual.set_clip(if !layout.usable() || color.alpha() <= 0.001 {
            ScreenClip::Empty
        } else {
            viewport_clip
        });
        visual.set_color(if layout.usable() {
            color
        } else {
            Color::TRANSPARENT
        })?;
    }
    for (kind, mut visual) in &mut labels {
        refresh_label(*kind, &mut visual, &layout, &state)?;
        let hidden = visual.tint().alpha() <= 0.001;
        visual.set_clip(if hidden {
            ScreenClip::Empty
        } else if matches!(kind, Label::Button(Target::ProjectName)) {
            let rect = layout.target(Target::ProjectName, state.overlay);
            ScreenClip::new(
                LogicalScreenPosition::new(rect.x + 16.0, rect.y + 4.0),
                LogicalScreenVector::new(
                    (rect.width - 32.0).max(1.0),
                    (rect.height - 8.0).max(1.0),
                ),
            )?
        } else {
            viewport_clip
        });
        if !layout.usable() && !matches!(kind, Label::MinimumTitle | Label::MinimumDetail) {
            visual.set_tint(Color::TRANSPARENT)?;
        }
    }
    for (kind, mut visual) in &mut pictures {
        let (rect, alpha) = match kind {
            Picture::Halo => {
                let rect = layout.backdrop(state.domains_blend);
                let visibility = if layout.compact {
                    state.domains_blend
                } else {
                    1.0
                };
                let preview_weight: f32 = state.domain_mix.iter().sum();
                let weight = 1.0 - state.domains_blend + state.domains_blend * preview_weight;
                (
                    rect,
                    visibility
                        * weight
                        * (1.0 - state.domains_blend * 0.7)
                        * (1.0 - state.scale_blend),
                )
            }
            Picture::Social(link) => {
                let target = Target::Social(*link);
                let slot = layout.target(target, state.overlay);
                let size = 24.0 + state.emphasis[target.index()] * 2.0;
                (
                    Rect::new(
                        slot.x + (slot.width - size) * 0.5,
                        slot.y + (slot.height - size) * 0.5,
                        size,
                        size,
                    ),
                    (0.7 + 0.3 * state.emphasis[target.index()]) * (1.0 - state.domains_blend),
                )
            }
        };
        visual.set_geometry(rect.position(), rect.size())?;
        visual.set_clip(if !layout.usable() || alpha <= 0.001 {
            ScreenClip::Empty
        } else {
            viewport_clip
        });
        let tint = if matches!(kind, Picture::Halo) {
            theme::mixed_domain(std::array::from_fn(|index| {
                state.home_mix[index] * (1.0 - state.domains_blend)
                    + state.domain_mix[index] * state.domains_blend
            }))
        } else {
            Color::WHITE
        };
        visual.set_tint(tint.with_alpha(if layout.usable() { alpha } else { 0.0 }))?;
    }
    Ok(())
}
