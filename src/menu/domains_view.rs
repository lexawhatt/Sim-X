//! The centered selector is navigation UI; its previews contain no solvers.

use sim_logic::prelude::*;

use super::{
    assets::MenuAssets,
    layout::Layout,
    state::{Domain, MenuState, Overlay, Target},
    theme,
};

#[derive(Component, Clone, Copy)]
pub(crate) enum DomainCaption {
    Eyebrow,
    Heading,
    Description,
    Status,
    Number(Domain),
}

pub(crate) fn spawn(world: &mut WorldBuilder, assets: &MenuAssets) -> LogicResult {
    for (caption, text, font) in [
        (DomainCaption::Eyebrow, "SIM;X / EXPLORE", &assets.small),
        (DomainCaption::Heading, "Choose a domain", &assets.heading),
        (DomainCaption::Description, "", &assets.small),
        (DomainCaption::Status, "", &assets.small),
        (DomainCaption::Number(Domain::Phys), "01", &assets.small),
        (DomainCaption::Number(Domain::Math), "02", &assets.small),
        (DomainCaption::Number(Domain::Chem), "03", &assets.small),
        (DomainCaption::Number(Domain::Biol), "04", &assets.small),
    ] {
        let mut visual =
            ScreenTextVisual::new(font.clone(), text, LogicalScreenPosition::new(0.0, 0.0))?;
        visual.set_tint(Color::TRANSPARENT)?;
        visual.set_alignment(TextAlignment::Center)?;
        visual.set_draw_order_depth(13.0)?;
        world.spawn((caption, visual))?;
    }
    Ok(())
}

pub(crate) fn refresh(
    viewport: FrameViewport,
    state: Option<Res<MenuState>>,
    mut captions: Query<(&DomainCaption, &mut ScreenTextVisual)>,
) -> LogicResult {
    let Some(state) = state else {
        return Ok(());
    };
    let layout = Layout::new(viewport.logical());
    let opacity = if layout.usable() {
        state.picker_mix[0]
    } else {
        0.0
    };
    let accent = state.preview_domain().map_or(theme::ACCENT, theme::domain);
    for (caption, mut visual) in &mut captions {
        let (x, y, tint) = match caption {
            DomainCaption::Eyebrow => (layout.width * 0.5, layout.domain_top() - 103.0, accent),
            DomainCaption::Heading => (layout.width * 0.5, layout.domain_top() - 60.0, theme::INK),
            DomainCaption::Description => {
                visual.set_text(
                    state
                        .preview_domain()
                        .map_or("Hover or focus a domain to explore.", Domain::description),
                )?;
                (layout.width * 0.5, layout.domain_top() - 26.0, theme::MUTED)
            }
            DomainCaption::Status => {
                visual.set_text(match state.selected_domain {
                    None => "Choose Physics to create a Macro project.",
                    Some(Domain::Phys) => "Choose a scale to begin.",
                    Some(Domain::Math) => "Math workspace is coming next.",
                    Some(Domain::Chem) => "Chemistry workspace is coming next.",
                    Some(Domain::Biol) => "Biology workspace is coming next.",
                })?;
                (
                    layout.width * 0.5,
                    layout.domain_top() + 344.0,
                    theme::MUTED,
                )
            }
            DomainCaption::Number(domain) => {
                let row = layout.target(Target::Domain(*domain), Overlay::Domains);
                (
                    row.x + 30.0,
                    row.y + 32.0,
                    theme::domain(*domain).with_alpha(0.7),
                )
            }
        };
        visual.set_position(LogicalScreenPosition::new(x, y))?;
        visual.set_tint(tint.scale_alpha(opacity))?;
        visual.set_clip(if opacity <= 0.001 {
            ScreenClip::Empty
        } else {
            ScreenClip::Unclipped
        });
    }
    Ok(())
}
