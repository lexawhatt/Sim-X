//! Retained native visuals. Capacity grows on demand; it is not a document quota.
use super::{
    assets::Fonts,
    drawing::*,
    graph_view,
    layout::{Layout, Rect},
    state::MathState,
};
use sim_logic::prelude::*;

#[derive(Component)]
pub(super) struct Slot(pub usize);
#[derive(Resource)]
pub(super) struct Pool {
    counts: [usize; 4],
}
// Warm capacity avoids a blank startup frame. Expansion is scheduled, not denied.
const WARM: [usize; 4] = [400, 160, 256, 32];
pub(super) fn spawn(world: &mut WorldBuilder, fonts: &Fonts) -> LogicResult {
    world.spawn(ActiveCamera2d::centered(1.0)?)?;
    world.insert_resource(WorldBackground::new(BG)?)?;
    world.insert_resource(Pool { counts: WARM })?;
    for i in 0..WARM[0] {
        world.spawn((Slot(i), empty_line()?))?;
    }
    for i in 0..WARM[1] {
        world.spawn((Slot(i), empty_text(&fonts.faces[1])?))?;
    }
    for i in 0..WARM[2] {
        world.spawn((Slot(i), empty_panel()?))?;
    }
    for i in 0..WARM[3] {
        world.spawn((Slot(i), empty_dot()?))?;
    }
    Ok(())
}
fn empty_line() -> LogicResult<ScreenLineVisual> {
    let mut v = ScreenLineVisual::new(
        LogicalScreenPosition::new(0.0, 0.0),
        LogicalScreenPosition::new(1.0, 0.0),
        1.0,
        INK,
    )?;
    v.set_clip(ScreenClip::Empty);
    Ok(v)
}
fn empty_text(font: &TextFont) -> LogicResult<ScreenTextVisual> {
    let mut v = ScreenTextVisual::new(font.clone(), "", LogicalScreenPosition::new(0.0, 0.0))?;
    v.set_clip(ScreenClip::Empty);
    Ok(v)
}
fn empty_panel() -> LogicResult<ScreenRectangleVisual> {
    let mut v = ScreenRectangleVisual::new(
        LogicalScreenPosition::new(0.0, 0.0),
        LogicalScreenVector::new(1.0, 1.0),
        PANEL,
    )?;
    v.set_clip(ScreenClip::Empty);
    Ok(v)
}
fn empty_dot() -> LogicResult<ScreenCircleVisual> {
    let mut v = ScreenCircleVisual::new(LogicalScreenPosition::new(0.0, 0.0), 1.0, INK)?;
    v.set_clip(ScreenClip::Empty);
    Ok(v)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn refresh(
    state: Option<Res<MathState>>,
    fonts: Option<Res<Fonts>>,
    pool: Option<ResMut<Pool>>,
    viewport: FrameViewport,
    mut commands: Commands,
    mut lines: Query<(&Slot, &mut ScreenLineVisual)>,
    mut texts: Query<(&Slot, &mut ScreenTextVisual)>,
    mut panels: Query<(&Slot, &mut ScreenRectangleVisual)>,
    mut dots: Query<(&Slot, &mut ScreenCircleVisual)>,
) -> LogicResult {
    let (Some(state), Some(fonts), Some(mut pool)) = (state, fonts, pool) else {
        return Ok(());
    };
    let layout = Layout::for_state(viewport.logical(), &state);
    let scene = scene(&state, &layout, &fonts);
    let needed = [
        scene.lines.len(),
        scene.texts.len(),
        scene.panels.len(),
        scene.dots.len(),
    ];
    let mut allowance = 96;
    for (kind, needed) in needed.into_iter().enumerate() {
        while pool.counts[kind] < needed && allowance > 0 {
            let id = pool.counts[kind];
            match kind {
                0 => {
                    commands.spawn((Slot(id), empty_line()?))?;
                }
                1 => {
                    commands.spawn((Slot(id), empty_text(&fonts.faces[1])?))?;
                }
                2 => {
                    commands.spawn((Slot(id), empty_panel()?))?;
                }
                _ => {
                    commands.spawn((Slot(id), empty_dot()?))?;
                }
            }
            pool.counts[kind] += 1;
            allowance -= 1;
        }
    }
    for (slot, mut visual) in &mut lines {
        visual.set_clip(ScreenClip::Empty);
        if let Some(line) = scene.lines.get(slot.0) {
            *visual = ScreenLineVisual::new(
                position(line.from),
                position(line.to),
                line.width,
                line.color,
            )?;
            visual.set_draw_order_depth(line.depth)?;
            visual.set_clip(line.clip.clip()?);
        }
    }
    for (slot, mut visual) in &mut texts {
        visual.set_clip(ScreenClip::Empty);
        if let Some(text) = scene.texts.get(slot.0) {
            visual.set_font(fonts.faces[text.style].clone())?;
            visual.set_text(&text.text)?;
            visual.set_position(position(text.position))?;
            visual.set_tint(text.color)?;
            visual.set_draw_order_depth(text.depth)?;
            visual.set_clip(text.clip.clip()?);
        }
    }
    for (slot, mut visual) in &mut panels {
        visual.set_clip(ScreenClip::Empty);
        if let Some(panel) = scene.panels.get(slot.0) {
            visual.set_geometry(
                LogicalScreenPosition::new(panel.rect.x, panel.rect.y),
                LogicalScreenVector::new(panel.rect.w, panel.rect.h),
            )?;
            visual.set_color(panel.color)?;
            visual.set_corner_radius(panel.radius)?;
            visual.set_draw_order_depth(panel.depth)?;
            visual.set_clip(panel.rect.clip()?);
        }
    }
    for (slot, mut visual) in &mut dots {
        visual.set_clip(ScreenClip::Empty);
        if let Some(dot) = scene.dots.get(slot.0) {
            visual.set_geometry(position(dot.center), dot.radius)?;
            visual.set_color(dot.color)?;
            visual.set_draw_order_depth(dot.depth)?;
            visual.set_clip(dot.clip.clip()?);
        }
    }
    Ok(())
}
fn position(p: [f32; 2]) -> LogicalScreenPosition {
    LogicalScreenPosition::new(p[0], p[1])
}

pub(super) fn scene(state: &MathState, layout: &Layout, fonts: &Fonts) -> Scene {
    let mut out = Scene::default();
    let full = Rect::new(0.0, 0.0, layout.width, layout.height);
    if !layout.usable() {
        out.text(
            "Enlarge the window to at least 900 x 640 to edit mathematics.",
            [24.0, 60.0],
            1,
            INK,
            full,
        );
        return out;
    }
    out.panel(Rect::new(0.0, 0.0, layout.width, 66.0), PANEL, 3.0);
    out.text("Sim;Math", [22.0, 40.0], 0, ACCENT, full);
    out.text("Untitled workspace", [190.0, 28.0], 1, INK, full);
    for (label, opacity) in [
        (
            "EUCLIDEAN / 2D   -   SESSION ONLY",
            1.0 - state.spatial.blend as f32,
        ),
        (
            "EUCLIDEAN / 3D WIREFRAME - SESSION ONLY",
            state.spatial.blend as f32,
        ),
    ] {
        if opacity > 0.0 {
            out.text(label, [190.0, 49.0], 2, MUTED.with_alpha(opacity), full);
        }
    }
    super::sidebar_view::draw(&mut out, state, layout, fonts);
    // Warm visual slots serve notation first. Growing graph capacity must not
    // temporarily erase an integral sign or fraction bar in the sidebar.
    graph_view::draw(&mut out, state, layout, fonts);
    out.text(
        state
            .notice
            .as_deref()
            .or_else(|| super::entry_hint::for_state(state))
            .unwrap_or(&state.status),
        [16.0, layout.height - 10.0],
        2,
        MUTED,
        full,
    );
    for (label, opacity) in [
        ("MMB pan / Wheel zoom", 1.0 - state.spatial.blend as f32),
        (
            "Drag to orbit / Wheel zoom / Home reset",
            state.spatial.blend as f32,
        ),
    ] {
        if opacity > 0.0 {
            out.text(
                label,
                [
                    layout.canvas.x + 16.0,
                    layout.canvas.y + layout.canvas.h - 16.0,
                ],
                2,
                MUTED.with_alpha(opacity),
                layout.canvas,
            );
        }
    }
    if state.confirm_back {
        let start_text = out.texts.len();
        out.panel(full, BG.with_alpha(0.93), 10.0);
        let rect = Rect::new(
            layout.width * 0.5 - 230.0,
            layout.height * 0.5 - 80.0,
            460.0,
            174.0,
        );
        out.panel(rect, PANEL, 11.0);
        out.text(
            "Leave this unsaved workspace?",
            [rect.x + 24.0, rect.y + 36.0],
            1,
            INK,
            full,
        );
        out.text(
            "The current document exists only in memory.",
            [rect.x + 24.0, rect.y + 65.0],
            2,
            MUTED,
            full,
        );
        for (_, r, caption) in &layout.controls {
            out.panel(*r, Color::rgb8(49, 48, 70), 12.0);
            out.text(*caption, [r.x + 10.0, r.y + 25.0], 2, INK, full);
        }
        for text in &mut out.texts[start_text..] {
            text.depth = 13.0;
        }
    }
    out
}
