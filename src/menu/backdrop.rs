//! Retained decorative motifs animated entirely in logical screen coordinates.
//!
//! Home cycles through one domain illustration at a time. Opening Domains
//! moves the preview behind the picker and fades it until a domain is chosen. These
//! visual metaphors contain no scientific state or fixed-update dependency.

mod geometry;

use sim_logic::prelude::*;

use super::{
    assets::MenuAssets,
    layout::{Layout, Rect},
    state::{Domain, MenuState},
    theme,
};
use geometry::{Dot, Path};

pub(crate) const LINE_COUNT: usize = 395;
pub(crate) const CIRCLE_COUNT: usize = 57;
pub(crate) const ENTITY_COUNT: usize = LINE_COUNT + CIRCLE_COUNT + 1;
const VISIBLE_ALPHA: f32 = 0.002;

/// One immutable orbit image, rotated around the same center as its native dots.
#[derive(Component, Clone, Copy)]
pub(crate) struct BackdropImage;

#[derive(Component, Clone, Copy)]
pub(crate) struct BackdropLine {
    domain: Domain,
    path: Path,
    segment: u16,
    segments: u16,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct BackdropDot {
    domain: Domain,
    dot: Dot,
    glow: u8,
}

#[derive(Clone, Copy)]
struct MotifFrame {
    placement: Rect,
    opacity: f32,
    dot_scale: f32,
    color: Color,
}

impl MotifFrame {
    fn initial(layout: &Layout, domain: Domain) -> Self {
        Self {
            placement: layout.backdrop(0.0),
            opacity: if domain == Domain::Phys { 1.0 } else { 0.0 },
            dot_scale: 1.0,
            color: theme::domain(domain),
        }
    }
}

fn screen_point(point: Vec2, placement: Rect) -> LogicalScreenPosition {
    LogicalScreenPosition::new(
        placement.x + (point.x() + 0.5) * placement.width,
        placement.y + (point.y() + 0.5) * placement.height,
    )
}

fn line_visual(part: BackdropLine, frame: MotifFrame, phase: f32) -> LogicResult<ScreenLineVisual> {
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
    let alpha = (frame.opacity * part.path.opacity(phase)).min(1.0);
    let color = frame.color.with_alpha(alpha);
    // Omit decorative detail below half a logical pixel, including DNA rungs
    // which briefly collapse to a point as the helix turns.
    let delta = to.to_vec2() - from.to_vec2();
    let collapsed = delta.x().hypot(delta.y()) < 0.5;
    let (from, to) = if collapsed {
        (
            LogicalScreenPosition::new(0.0, 0.0),
            LogicalScreenPosition::new(1.0, 0.0),
        )
    } else {
        (from, to)
    };
    let mut visual = ScreenLineVisual::new(from, to, part.path.width(), color)?;
    visual.set_draw_order_depth(-5.0)?;
    if collapsed || alpha < VISIBLE_ALPHA {
        visual.set_clip(ScreenClip::Empty);
    }
    Ok(visual)
}

fn add_path(
    world: &mut WorldBuilder,
    layout: &Layout,
    domain: Domain,
    path: Path,
    segments: u16,
) -> LogicResult {
    let frame = MotifFrame::initial(layout, domain);
    for segment in 0..segments {
        let part = BackdropLine {
            domain,
            path,
            segment,
            segments,
        };
        world.spawn((part, line_visual(part, frame, 0.0)?))?;
    }
    Ok(())
}

fn glow_radius(glow: u8) -> f32 {
    match glow {
        0 => 1.0,
        1 => 2.5,
        _ => 4.5,
    }
}

fn glow_opacity(glow: u8) -> f32 {
    match glow {
        0 => 0.9,
        1 => 0.14,
        _ => 0.035,
    }
}

fn add_dot(
    world: &mut WorldBuilder,
    layout: &Layout,
    domain: Domain,
    dot: Dot,
    glows: u8,
) -> LogicResult {
    let frame = MotifFrame::initial(layout, domain);
    let center = screen_point(dot.point(0.0), frame.placement);
    // Outer discs precede the bright core, producing a bounded layered glow.
    for glow in (0..glows).rev() {
        let mut visual = ScreenCircleVisual::new(
            center,
            dot.radius() * glow_radius(glow),
            frame.color.with_alpha(frame.opacity * glow_opacity(glow)),
        )?;
        visual.set_draw_order_depth(-4.0 + f32::from(glows - glow) * 0.1)?;
        if frame.opacity < VISIBLE_ALPHA {
            visual.set_clip(ScreenClip::Empty);
        }
        world.spawn((BackdropDot { domain, dot, glow }, visual))?;
    }
    Ok(())
}

/// Allocates all four decorative motifs once, initially laid out for 1280 x 800.
pub(crate) fn spawn(world: &mut WorldBuilder, assets: &MenuAssets) -> LogicResult {
    let layout = Layout::new(LogicalViewport::new(1280.0, 800.0)?);
    let placement = layout.backdrop(0.0);
    let mut physics =
        ScreenImageVisual::new(assets.physics, placement.position(), placement.size())?;
    physics.set_filter(ImageFilter::Linear);
    physics.set_draw_order_depth(-5.0)?;
    world.spawn((BackdropImage, physics))?;
    for orbit in 0..3 {
        add_dot(world, &layout, Domain::Phys, Dot::Electron(orbit), 3)?;
    }
    add_dot(world, &layout, Domain::Phys, Dot::Nucleus, 3)?;

    // Stylized H, C, and O use familiar shell occupancy as visual identity only.
    for (atom, electron_count) in [(0, 1), (1, 6), (2, 8)] {
        for shell in 0..if atom == 0 { 1 } else { 2 } {
            add_path(
                world,
                &layout,
                Domain::Chem,
                Path::AtomShell { atom, shell },
                32,
            )?;
        }
        add_dot(world, &layout, Domain::Chem, Dot::AtomNucleus(atom), 2)?;
        for electron in 0..electron_count {
            add_dot(
                world,
                &layout,
                Domain::Chem,
                Dot::AtomElectron { atom, electron },
                2,
            )?;
        }
    }
    for bond in 0..2 {
        add_path(world, &layout, Domain::Chem, Path::ReactionBond(bond), 1)?;
    }

    add_path(world, &layout, Domain::Math, Path::Wave, 48)?;
    add_path(world, &layout, Domain::Math, Path::UnitCircle, 48)?;
    add_path(world, &layout, Domain::Math, Path::Polygon, 8)?;
    for axis in 0..2 {
        add_path(world, &layout, Domain::Math, Path::Axis(axis), 1)?;
    }
    add_dot(world, &layout, Domain::Math, Dot::WaveTracer, 3)?;

    for strand in 0..2 {
        add_path(world, &layout, Domain::Biol, Path::Helix(strand), 32)?;
    }
    for rung in 0..15 {
        add_path(world, &layout, Domain::Biol, Path::Rung(rung), 1)?;
    }
    add_path(world, &layout, Domain::Biol, Path::Cell, 48)?;
    add_dot(world, &layout, Domain::Biol, Dot::CellNucleus, 2)?;
    for index in 0..2 {
        add_dot(world, &layout, Domain::Biol, Dot::CellParticle(index), 2)?;
    }
    Ok(())
}

fn opacity(layout: &Layout, state: &MenuState, domain: Domain) -> f32 {
    if !layout.usable() || state.scale_blend > 0.999 {
        return 0.0;
    }
    let blend = state.domains_blend;
    let visible = if layout.compact { blend } else { 1.0 };
    let weight =
        state.home_mix[domain.index()] * (1.0 - blend) + state.domain_mix[domain.index()] * blend;
    visible * weight * (1.0 - blend * 0.68) * (1.0 - state.scale_blend)
}

fn pulse(state: &MenuState, domain: Domain) -> f32 {
    if state.reduced_motion || state.preview_domain() != Some(domain) {
        return 0.0;
    }
    (std::f32::consts::PI * state.preview_pulse).sin().max(0.0) * state.domains_blend
}

fn motif_frames(layout: &Layout, state: &MenuState) -> [MotifFrame; 4] {
    let base = layout.backdrop(state.domains_blend);
    Domain::ALL.map(|domain| {
        let wave = pulse(state, domain);
        let growth = wave * 0.035;
        MotifFrame {
            placement: Rect::new(
                base.x - base.width * growth * 0.5,
                base.y - base.height * growth * 0.5,
                base.width * (1.0 + growth),
                base.height * (1.0 + growth),
            ),
            opacity: opacity(layout, state, domain) * (1.0 + wave * 0.2),
            dot_scale: 1.0 + growth,
            color: theme::domain(domain),
        }
    })
}

/// Updates positions and fading directly in FrameUpdate, including during pause.
pub(crate) fn refresh(
    state: Option<Res<MenuState>>,
    viewport: FrameViewport,
    mut lines: Query<(&BackdropLine, &mut ScreenLineVisual)>,
    mut dots: Query<(&BackdropDot, &mut ScreenCircleVisual)>,
    mut images: Query<&mut ScreenImageVisual, With<BackdropImage>>,
) -> LogicResult {
    let Some(state) = state else {
        return Ok(());
    };
    let layout = Layout::new(viewport.logical());
    let frames = motif_frames(&layout, &state);
    let physics = frames[Domain::Phys.index()];
    for mut visual in &mut images {
        if physics.opacity < VISIBLE_ALPHA {
            visual.set_clip(ScreenClip::Empty);
            continue;
        }
        visual.set_geometry(physics.placement.position(), physics.placement.size())?;
        visual.set_rotation(state.phase * 0.1)?;
        visual.set_tint(Color::WHITE.with_alpha(physics.opacity.min(1.0)))?;
        visual.set_clip(ScreenClip::Unclipped);
    }
    for (part, mut line) in &mut lines {
        let frame = frames[part.domain.index()];
        if frame.opacity * part.path.opacity(state.phase) < VISIBLE_ALPHA {
            line.set_clip(ScreenClip::Empty);
            continue;
        }
        // ScreenLineVisual currently exposes no direct color setter. Replacing
        // its small CPU value updates geometry and color without allocating an
        // entity, asset, or renderer resource.
        *line = line_visual(*part, frame, state.phase)?;
    }
    for (part, mut visual) in &mut dots {
        let frame = frames[part.domain.index()];
        let alpha = frame.opacity * glow_opacity(part.glow);
        if alpha < VISIBLE_ALPHA {
            visual.set_clip(ScreenClip::Empty);
            continue;
        }
        visual.set_geometry(
            screen_point(part.dot.point(state.phase), frame.placement),
            part.dot.radius() * glow_radius(part.glow) * frame.dot_scale,
        )?;
        visual.set_color(frame.color.with_alpha(alpha.min(1.0)))?;
        visual.set_clip(ScreenClip::Unclipped);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_shows_current_carousel_symbol_and_idle_domains_have_no_symbol() -> LogicResult {
        let layout = Layout::new(LogicalViewport::new(1280.0, 800.0)?);
        let home = MenuState::default();
        let frames = motif_frames(&layout, &home);
        for domain in Domain::ALL {
            let frame = frames[domain.index()];
            assert_eq!(frame.placement.width, layout.illustration().width);
            assert_eq!(
                frame.opacity,
                if domain == Domain::Phys { 1.0 } else { 0.0 }
            );
        }
        let domains = MenuState {
            domains_blend: 1.0,
            domain_mix: [0.0; 4],
            ..Default::default()
        };
        for frame in motif_frames(&layout, &domains) {
            assert_eq!(frame.opacity, 0.0);
            assert_eq!(frame.placement.x, layout.backdrop(1.0).x);
            assert_eq!(frame.placement.y, layout.backdrop(1.0).y);
        }
        Ok(())
    }

    fn visible_frame(placement: Rect) -> MotifFrame {
        MotifFrame {
            placement,
            opacity: 1.0,
            dot_scale: 1.0,
            color: theme::domain(Domain::Biol),
        }
    }

    #[test]
    fn collapsed_decorative_line_is_hidden_without_invalid_endpoints() -> LogicResult {
        let part = BackdropLine {
            domain: Domain::Biol,
            path: Path::Rung(4),
            segment: 0,
            segments: 1,
        };
        let visual = line_visual(part, visible_frame(Rect::new(500.0, 300.0, 1.0, 1.0)), 0.0)?;
        assert_ne!(visual.start(), visual.end());
        assert_eq!(visual.clip(), ScreenClip::Empty);
        Ok(())
    }

    #[test]
    fn nearly_collapsed_dna_rung_uses_decorative_subpixel_culling() -> LogicResult {
        let part = BackdropLine {
            domain: Domain::Biol,
            path: Path::Rung(4),
            segment: 0,
            segments: 1,
        };
        let rect = Rect::new(500.0, 300.0, 200.0, 200.0);
        let phase = 0.005;
        assert_ne!(
            screen_point(part.path.point(0.0, phase), rect),
            screen_point(part.path.point(1.0, phase), rect)
        );
        assert_eq!(
            line_visual(part, visible_frame(rect), phase)?.clip(),
            ScreenClip::Empty
        );
        assert_eq!(
            line_visual(part, visible_frame(rect), 0.02)?.clip(),
            ScreenClip::Unclipped
        );
        Ok(())
    }
}
