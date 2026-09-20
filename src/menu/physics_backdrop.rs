//! Retained flat Physics motifs, using the approved atom's contour language.
//! Coordinates, stroke hierarchy and glow share one centered frame.

mod geometry;
#[cfg(test)]
mod tests;

use super::{
    assets::MenuAssets,
    layout::{Layout, Rect},
    state::{MenuState, Overlay, PhysicsScale},
    theme,
};
use geometry::{DOTS, Dot, Mechanism, PATHS, Path};
use sim_logic::prelude::*;

pub(crate) const LINE_COUNT: usize = {
    let mut count = 0;
    let mut index = 0;
    while index < PATHS.len() {
        count += PATHS[index].1 as usize;
        index += 1;
    }
    count
};
pub(crate) const CIRCLE_COUNT: usize = DOTS.len() * 2;
pub(crate) const IMAGE_COUNT: usize = 2;
pub(crate) const ENTITY_COUNT: usize = LINE_COUNT + CIRCLE_COUNT + IMAGE_COUNT;
const VISIBLE_ALPHA: f32 = 0.002;

#[derive(Component, Clone, Copy)]
pub(crate) struct PhysicsBackdropLine {
    path: Path,
    segment: u16,
    segments: u16,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct PhysicsBackdropDot {
    dot: Dot,
    glow: bool,
}

#[derive(Component, Clone, Copy)]
pub(crate) enum PhysicsBackdropImage {
    Atom,
    Halo,
}

#[derive(Clone, Copy)]
struct Frame {
    placement: Rect,
    opacity: f32,
    color: Color,
}

fn scale_color(scale: PhysicsScale) -> Color {
    match scale {
        PhysicsScale::Micro => theme::ACCENT,
        PhysicsScale::Macro => Color::rgb8(229, 188, 127),
        PhysicsScale::Astra => Color::rgb8(124, 187, 237),
    }
}

fn frames(layout: &Layout, state: &MenuState) -> [Frame; 3] {
    let base = layout.backdrop(1.0);
    let enabled = layout.usable() && state.overlay == Overlay::PhysicsScales;
    PhysicsScale::ALL.map(|scale| {
        let weight = state.scale_mix[scale.index()].clamp(0.0, 1.0);
        let pulse = if !state.reduced_motion && state.preview_scale() == Some(scale) {
            4.0 * weight * (1.0 - weight)
        } else {
            0.0
        };
        let grow = 0.025 * pulse;
        Frame {
            placement: Rect::new(
                base.x - base.width * grow * 0.5,
                base.y - base.height * grow * 0.5,
                base.width * (1.0 + grow),
                base.height * (1.0 + grow),
            ),
            opacity: if enabled {
                weight * state.scale_blend * 0.48
            } else {
                0.0
            },
            color: scale_color(scale),
        }
    })
}

fn mechanism_frame(mut frame: Frame, part: Option<Mechanism>, state: &MenuState) -> Frame {
    if let Some(part) = part {
        let (current, fade) = Mechanism::current(state.macro_seconds);
        frame.opacity *= if part != current {
            0.0
        } else if state.reduced_motion {
            1.0
        } else {
            fade
        };
    }
    frame
}

fn screen_point(point: Vec2, placement: Rect) -> LogicalScreenPosition {
    LogicalScreenPosition::new(
        placement.x + (point.x() + 0.5) * placement.width,
        placement.y + (point.y() + 0.5) * placement.height,
    )
}

fn line_visual(
    part: PhysicsBackdropLine,
    frame: Frame,
    phase: f32,
) -> LogicResult<ScreenLineVisual> {
    let denominator = f32::from(part.segments);
    let from = screen_point(
        part.path
            .point(f32::from(part.segment) / denominator, phase),
        frame.placement,
    );
    let to = screen_point(
        part.path
            .point(f32::from(part.segment + 1) / denominator, phase),
        frame.placement,
    );
    let delta = to.to_vec2() - from.to_vec2();
    let collapsed = delta.x().hypot(delta.y()) < 0.5;
    let alpha = (frame.opacity * part.path.opacity()).clamp(0.0, 1.0);
    let (from, to) = if collapsed {
        (
            LogicalScreenPosition::new(0.0, 0.0),
            LogicalScreenPosition::new(1.0, 0.0),
        )
    } else {
        (from, to)
    };
    let mut visual = ScreenLineVisual::new(from, to, 1.15, frame.color.with_alpha(alpha))?;
    visual.set_draw_order_depth(-4.8)?;
    if collapsed || alpha < VISIBLE_ALPHA {
        visual.set_clip(ScreenClip::Empty);
    }
    Ok(visual)
}

fn circle_visual(
    part: PhysicsBackdropDot,
    frame: Frame,
    phase: f32,
) -> LogicResult<ScreenCircleVisual> {
    let dot = part.dot;
    let (point, radius) = dot.geometry(phase);
    let outline = dot.is_outline();
    // Large bodies are hollow diagrams, not opaque globe/metal illustrations.
    let alpha = if outline {
        0.0
    } else if part.glow {
        0.06
    } else {
        0.90
    };
    let mut visual = ScreenCircleVisual::new(
        screen_point(point, frame.placement),
        (radius * frame.placement.width * if part.glow { 2.3 } else { 1.0 }).max(0.5),
        frame.color.with_alpha(frame.opacity * alpha),
    )?;
    if outline && !part.glow {
        // Use only public Logic API until it re-exports the Stroke constructor.
        let stroke = ScreenLineVisual::new(
            LogicalScreenPosition::new(0.0, 0.0),
            LogicalScreenPosition::new(1.0, 0.0),
            1.15,
            frame.color.with_alpha(frame.opacity * 0.42),
        )?
        .stroke();
        visual.set_stroke(Some(stroke))?;
    }
    visual.set_draw_order_depth(if part.glow { -5.0 } else { -4.5 })?;
    if frame.opacity < VISIBLE_ALPHA || (outline && part.glow) {
        visual.set_clip(ScreenClip::Empty);
    }
    Ok(visual)
}

pub(crate) fn spawn(world: &mut WorldBuilder, assets: &MenuAssets) -> LogicResult {
    let layout = Layout::new(LogicalViewport::new(1280.0, 800.0)?);
    let hidden = Frame {
        placement: layout.backdrop(1.0),
        opacity: 0.0,
        color: theme::ACCENT,
    };
    for (part, asset) in [
        (PhysicsBackdropImage::Atom, assets.physics),
        (PhysicsBackdropImage::Halo, assets.halo),
    ] {
        let mut visual =
            ScreenImageVisual::new(asset, hidden.placement.position(), hidden.placement.size())?;
        visual.set_filter(ImageFilter::Linear);
        visual.set_draw_order_depth(if matches!(part, PhysicsBackdropImage::Halo) {
            -6.0
        } else {
            -5.0
        })?;
        visual.set_clip(ScreenClip::Empty);
        world.spawn((part, visual))?;
    }
    for (path, segments) in PATHS {
        for segment in 0..segments {
            let part = PhysicsBackdropLine {
                path,
                segment,
                segments,
            };
            world.spawn((part, line_visual(part, hidden, 0.0)?))?;
        }
    }
    for dot in DOTS {
        for glow in [true, false] {
            let part = PhysicsBackdropDot { dot, glow };
            world.spawn((part, circle_visual(part, hidden, 0.0)?))?;
        }
    }
    Ok(())
}

pub(crate) fn refresh(
    viewport: FrameViewport,
    state: Option<Res<MenuState>>,
    mut lines: Query<(&PhysicsBackdropLine, &mut ScreenLineVisual)>,
    mut circles: Query<(&PhysicsBackdropDot, &mut ScreenCircleVisual)>,
    mut images: Query<(&PhysicsBackdropImage, &mut ScreenImageVisual)>,
) -> LogicResult {
    let Some(state) = state else {
        return Ok(());
    };
    let frames = frames(&Layout::new(viewport.logical()), &state);
    for (part, mut visual) in &mut lines {
        let frame = mechanism_frame(
            frames[part.path.scale().index()],
            part.path.mechanism(),
            &state,
        );
        if frame.opacity * part.path.opacity() < VISIBLE_ALPHA {
            visual.set_clip(ScreenClip::Empty);
        } else {
            *visual = line_visual(*part, frame, state.phase)?;
        }
    }
    for (part, mut visual) in &mut circles {
        let frame = mechanism_frame(
            frames[part.dot.scale().index()],
            part.dot.mechanism(),
            &state,
        );
        if frame.opacity < VISIBLE_ALPHA {
            visual.set_clip(ScreenClip::Empty);
        } else {
            *visual = circle_visual(*part, frame, state.phase)?;
        }
    }
    for (part, mut visual) in &mut images {
        let (frame, color, alpha, angle) = match part {
            PhysicsBackdropImage::Atom => {
                let frame = frames[PhysicsScale::Micro.index()];
                (frame, Color::WHITE, frame.opacity, state.phase * 0.1)
            }
            PhysicsBackdropImage::Halo => {
                let total = frames.iter().map(|frame| frame.opacity).sum::<f32>();
                let mut channels = [0.0; 3];
                for frame in frames {
                    channels[0] += frame.color.red() * frame.opacity;
                    channels[1] += frame.color.green() * frame.opacity;
                    channels[2] += frame.color.blue() * frame.opacity;
                }
                let divisor = total.max(1e-6);
                let color = Color::rgb(
                    channels[0] / divisor,
                    channels[1] / divisor,
                    channels[2] / divisor,
                );
                // The bundled Gaussian already has 8% peak alpha. Apply the
                // same modest glow to every scale, with no hard filled disk.
                (frames[0], color, total * 0.55, 0.0)
            }
        };
        if alpha < VISIBLE_ALPHA {
            visual.set_clip(ScreenClip::Empty);
            continue;
        }
        visual.set_geometry(frame.placement.position(), frame.placement.size())?;
        visual.set_rotation(angle)?;
        visual.set_tint(color.with_alpha(alpha.clamp(0.0, 1.0)))?;
        visual.set_clip(ScreenClip::Unclipped);
    }
    Ok(())
}
