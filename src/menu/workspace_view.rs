//! Retained captions for explicit Physics scales and the project entry flow.

use sim_logic::prelude::*;

use super::{
    assets::MenuAssets,
    layout::Layout,
    state::{MenuState, Overlay, PhysicsScale, Target},
    theme,
};

pub(crate) const TEXT_COUNT: usize = 13;

#[derive(Component, Clone, Copy)]
pub(crate) enum WorkspaceCaption {
    ScaleEyebrow,
    ScaleHeading,
    ScaleDescription,
    ScaleStatus,
    ScaleNumber(PhysicsScale),
    Unavailable(PhysicsScale),
    ProjectBreadcrumb,
    ProjectHeading,
    ProjectDescription,
    ProjectStatus,
}

pub(crate) fn spawn(world: &mut WorldBuilder, assets: &MenuAssets) -> LogicResult {
    use WorkspaceCaption::*;
    for caption in [
        ScaleEyebrow,
        ScaleHeading,
        ScaleDescription,
        ScaleStatus,
        ScaleNumber(PhysicsScale::Micro),
        ScaleNumber(PhysicsScale::Macro),
        ScaleNumber(PhysicsScale::Astra),
        Unavailable(PhysicsScale::Micro),
        Unavailable(PhysicsScale::Astra),
        ProjectBreadcrumb,
        ProjectHeading,
        ProjectDescription,
        ProjectStatus,
    ] {
        let font = if matches!(caption, ScaleHeading | ProjectHeading) {
            &assets.heading
        } else {
            &assets.small
        };
        let mut visual =
            ScreenTextVisual::new(font.clone(), "", LogicalScreenPosition::new(0.0, 0.0))?;
        visual.set_draw_order_depth(13.0)?;
        visual.set_clip(ScreenClip::Empty);
        world.spawn((caption, visual))?;
    }
    Ok(())
}

pub(crate) fn refresh(
    viewport: FrameViewport,
    state: Option<Res<MenuState>>,
    mut captions: Query<(&WorkspaceCaption, &mut ScreenTextVisual)>,
) -> LogicResult {
    let Some(state) = state else {
        return Ok(());
    };
    let layout = Layout::new(viewport.logical());
    let center = layout.width * 0.5;
    let top = layout.domain_top();
    let clip = ScreenClip::new(
        LogicalScreenPosition::new(16.0, 0.0),
        LogicalScreenVector::new((layout.width - 32.0).max(1.0), layout.height),
    )?;
    for (caption, mut visual) in &mut captions {
        use WorkspaceCaption::*;
        let (text, x, y, tint, opacity, alignment) = match caption {
            ScaleEyebrow => (
                "SIM;X / PHYS",
                center,
                top - 103.0,
                theme::ACCENT,
                state.picker_mix[1],
                TextAlignment::Center,
            ),
            ScaleHeading => (
                "Phys;X",
                center,
                top - 60.0,
                theme::INK,
                state.picker_mix[1],
                TextAlignment::Center,
            ),
            ScaleDescription => (
                state.preview_scale().map_or(
                    "Choose your scale of exploration.",
                    PhysicsScale::description,
                ),
                center,
                top - 26.0,
                theme::MUTED,
                state.picker_mix[1],
                TextAlignment::Center,
            ),
            ScaleStatus => (
                if state.project_error.is_empty() {
                    "Macro is ready. Micro and Astra are previews."
                } else {
                    state.project_error
                },
                center,
                top + 310.0,
                theme::MUTED,
                state.picker_mix[1],
                TextAlignment::Center,
            ),
            ScaleNumber(scale) => {
                let row = layout.target(Target::Scale(*scale), Overlay::PhysicsScales);
                (
                    match scale {
                        PhysicsScale::Micro => "01",
                        PhysicsScale::Macro => "02",
                        PhysicsScale::Astra => "03",
                    },
                    row.x + 30.0,
                    row.y + 35.0,
                    theme::ACCENT,
                    state.picker_mix[1],
                    TextAlignment::Center,
                )
            }
            Unavailable(scale) => {
                let row = layout.target(Target::Scale(*scale), Overlay::PhysicsScales);
                (
                    "Soon",
                    row.x + row.width - 20.0,
                    row.y + 35.0,
                    theme::MUTED,
                    state.picker_mix[1],
                    TextAlignment::Right,
                )
            }
            ProjectBreadcrumb => (
                "SIM;X / PHYS / MACRO",
                center,
                top - 103.0,
                theme::ACCENT,
                state.picker_mix[2],
                TextAlignment::Center,
            ),
            ProjectHeading => (
                "Projects",
                center,
                top - 60.0,
                theme::INK,
                state.picker_mix[2],
                TextAlignment::Center,
            ),
            ProjectDescription => (
                "Start with an empty physical scene.",
                center,
                top + 4.0,
                theme::INK,
                state.picker_mix[2],
                TextAlignment::Center,
            ),
            ProjectStatus => (
                "This prototype keeps projects in memory only.",
                center,
                top + 255.0,
                theme::MUTED,
                state.picker_mix[2],
                TextAlignment::Center,
            ),
        };
        visual.set_text(text)?;
        visual.set_alignment(alignment)?;
        visual.set_position(LogicalScreenPosition::new(x, y))?;
        visual.set_tint(tint.scale_alpha(opacity))?;
        visual.set_clip(if layout.usable() && opacity > 0.001 {
            clip
        } else {
            ScreenClip::Empty
        });
    }
    Ok(())
}
