//! Equal-status expression rows, structured notation, and contextual popovers.
use super::{
    assets::Fonts,
    drawing::*,
    graph_style,
    layout::{Layout, Rect, Target},
    state::MathState,
};
use sim_logic::prelude::*;
pub(super) fn draw(out: &mut Scene, state: &MathState, layout: &Layout, fonts: &Fonts) {
    let full = Rect::new(0.0, 0.0, layout.width, layout.height);
    out.panel(
        Rect::new(0.0, 66.0, layout.sidebar, layout.keypad_top - 66.0),
        PANEL,
        3.0,
    );
    out.panel(
        Rect::new(0.0, 66.0, layout.sidebar, 46.0),
        Color::rgb8(21, 28, 39),
        4.0,
    );
    out.line(
        [layout.sidebar, 66.0],
        [layout.sidebar, layout.keypad_top],
        MUTED.with_alpha(0.35),
        1.0,
        full,
        4.0,
    );
    for (i, row) in layout.rows.iter().enumerate() {
        out.text(
            (i + 1).to_string(),
            [8.0, row.y + 17.0],
            2,
            MUTED,
            layout.list,
        );
        let center = [20.0, row.y + 41.0];
        let style = state.document.styles[i];
        let visible = state.document.visible[i];
        let populated = !state.document.fields[i].rows[0].is_empty();
        out.dot(
            center,
            12.0,
            if visible && populated {
                style.tint()
            } else {
                MUTED.with_alpha(0.20)
            },
            layout.list,
            5.0,
        );
        if visible && populated {
            let mut previous = None;
            for j in 0..12 {
                let x = j as f32 / 11.0;
                let p = [
                    center[0] - 9.0 + x * 18.0,
                    center[1] - (x * std::f32::consts::TAU).sin() * 4.0,
                ];
                if let Some(a) = previous {
                    out.line(a, p, INK, 1.6, layout.list, 6.0);
                }
                previous = Some(p);
            }
        }
        out.line(
            [0.0, row.y + row.h],
            [layout.sidebar, row.y + row.h],
            MUTED.with_alpha(0.22),
            1.0,
            layout.list,
            4.0,
        );
        if let Some(result) = state
            .plot
            .as_ref()
            .and_then(|p| p.rows.get(i))
            .filter(|_| !state.stale)
        {
            if state.spatial.blend > 0.0
                && let Some(hint) =
                    super::row_interpretation::xy_curve_hint(&state.document.fields[i], result)
            {
                // Reuse the ordinary row footer so entering 3D never changes
                // expression height, caret placement or sidebar scroll.
                out.text(
                    hint,
                    [52.0, row.y + row.h - 8.0],
                    2,
                    MUTED.with_alpha(state.spatial.blend as f32),
                    layout.list,
                );
            }
            if let Some(picture) = &result.integral_plot {
                let baseline =
                    row.y + row.h - super::layout::parameter_extra(state, i, layout.sidebar) - 34.0;
                let x = if picture.bands.is_empty() {
                    52.0
                } else {
                    out.text("+", [52.0, baseline], 2, style.tint(), layout.list);
                    out.text(
                        "-",
                        [69.0, baseline],
                        2,
                        super::integral_view::negative_color(style),
                        layout.list,
                    );
                    86.0
                };
                out.text(picture.note, [x, baseline], 2, MUTED, layout.list);
            }
            if let Some(value) = result.scalar
                && state.parameters.get(i).is_none_or(Option::is_none)
            {
                let label = format!("= {}", number(value));
                let width = fonts.width(1, &label);
                out.text(
                    label,
                    [layout.sidebar - 42.0 - width, row.y + row.h - 9.0],
                    1,
                    style.tint(),
                    layout.list,
                );
            }
            if let Some(message) = &result.diagnostic {
                out.text(
                    message,
                    [
                        52.0,
                        row.y + row.h
                            - super::layout::parameter_extra(state, i, layout.sidebar)
                            - 8.0,
                    ],
                    2,
                    Color::rgb8(238, 175, 118),
                    layout.list,
                );
            }
        }
    }
    if let Some(draft) = layout.draft
        && let Some(clip) = draft.intersection(layout.list)
    {
        let hover = state.motion.hover(Target::Draft);
        out.panel(clip, ACCENT.with_alpha(0.025 + hover * 0.055), 4.0);
        out.text(
            (state.document.fields.len() + 1).to_string(),
            [8.0, draft.y + 17.0],
            2,
            MUTED,
            layout.list,
        );
        out.text(
            "Click to add an expression",
            [57.0, draft.y + 35.0],
            2,
            MUTED.with_alpha(0.75 + hover * 0.25),
            layout.list,
        );
    }
    for (index, rect) in layout.fields.iter().copied().enumerate() {
        let Some(clip) = rect.intersection(layout.list) else {
            continue;
        };
        let origin = [
            rect.x + 12.0 - state.offsets[index][0],
            rect.y + 8.0 - state.offsets[index][1],
        ];
        if state.focus == Some(index) {
            out.panel(clip, Color::rgb8(26, 32, 46), 4.0);
            out.line(
                [1.0, rect.y],
                [1.0, rect.y + rect.h],
                ACCENT,
                2.0,
                layout.list,
                5.0,
            );
        }
        let formula = &state.layouts[index];
        if state.document.fields[index].rows[0].is_empty() {
            out.text(
                "Type an expression...",
                [rect.x + 16.0, origin[1]],
                2,
                MUTED.with_alpha(0.75),
                clip,
            );
        }
        if state.document.fields[index].select_all && state.focus == Some(index) {
            out.panel(clip, ACCENT.with_alpha(0.16), 4.5);
        } else if state.focus == Some(index)
            && let Some((row, start, end)) = state.document.fields[index].selection()
            && let (Some(a), Some(b)) = (
                formula.stop(super::formula::Caret { row, index: start }),
                formula.stop(super::formula::Caret { row, index: end }),
            )
        {
            let (top, bottom) = if row == 0 {
                (-formula.above, formula.below)
            } else {
                (a.y - a.height, a.y + a.height * 0.3)
            };
            if let Some(selection) = Rect::new(
                origin[0] + a.x,
                origin[1] + top,
                (b.x - a.x).max(2.0),
                bottom - top,
            )
            .intersection(clip)
            {
                out.panel(selection, ACCENT.with_alpha(0.22), 4.5);
            }
        }
        for glyph in &formula.glyphs {
            let x = origin[0] + glyph.x;
            if x + 50.0 < clip.x {
                continue;
            }
            out.text(
                &glyph.text,
                [x, origin[1] + glyph.y],
                glyph.style,
                INK,
                clip,
            );
        }
        for rule in &formula.rules {
            out.line(
                [origin[0] + rule.from[0], origin[1] + rule.from[1]],
                [origin[0] + rule.to[0], origin[1] + rule.to[1]],
                INK,
                1.5,
                clip,
                6.0,
            );
        }
        if state.focus == Some(index)
            && state.blink < 0.6
            && let Some(stop) = formula.stop(state.document.fields[index].caret)
        {
            out.line(
                [origin[0] + stop.x, origin[1] + stop.y - stop.height * 0.8],
                [origin[0] + stop.x, origin[1] + stop.y + stop.height * 0.2],
                ACCENT,
                1.5,
                clip,
                7.0,
            );
        }
    }

    if state.keypad {
        out.panel(
            Rect::new(
                0.0,
                layout.keypad_top,
                layout.width,
                layout.height - layout.keypad_top - 28.0,
            ),
            Color::rgb8(19, 26, 37),
            4.75,
        );
        out.line(
            [0.0, layout.keypad_top],
            [layout.width, layout.keypad_top],
            MUTED.with_alpha(0.4),
            1.0,
            full,
            5.0,
        );
    }
    for (target, rect, caption) in &layout.controls {
        if state.confirm_back || matches!(target, Target::Visible(_)) {
            continue;
        }
        let Some(clip) = rect.intersection(if Layout::in_list(*target) {
            layout.list
        } else {
            full
        }) else {
            continue;
        };
        let active = state.hovered == Some(*target)
            || matches!(target,Target::Tool(i) if *i==state.tool)
            || (*target == Target::Functions && state.functions);
        if let Target::Slider(i) = target
            && let Some(p) = state.parameters[*i]
        {
            let center = rect.y + rect.h * 0.5;
            let t = ((p.value - p.range.low) / (p.range.high - p.range.low)).clamp(0.0, 1.0) as f32;
            out.line(
                [rect.x, center],
                [rect.x + rect.w, center],
                MUTED.with_alpha(0.45),
                3.0,
                clip,
                5.0,
            );
            out.line(
                [rect.x, center],
                [rect.x + rect.w * t, center],
                ACCENT,
                3.0,
                clip,
                5.1,
            );
            out.dot([rect.x + rect.w * t, center], 6.0, ACCENT, layout.list, 6.0);
            continue;
        }
        let range_caption;
        let caption = if let Target::ParameterRange(i, upper) = target {
            range_caption = if let Some((r, u, text)) = &state.range_edit
                && r == i
                && u == upper
            {
                format!("{text}|")
            } else if let Some(p) = state.parameters[*i] {
                number(if *upper { p.range.high } else { p.range.low })
            } else {
                String::new()
            };
            range_caption.as_str()
        } else {
            caption
        };
        let caption = if let Target::Calculate(i) = target {
            if state
                .plot
                .as_ref()
                .and_then(|p| p.rows.get(*i))
                .is_some_and(|r| r.pending)
            {
                "Cancel"
            } else {
                caption
            }
        } else {
            caption
        };
        let key_opacity = layout.control_opacity(*target);
        out.button(
            clip,
            (if *target == Target::Enter {
                Color::rgb8(68, 63, 125)
            } else {
                let amount = if matches!(target,Target::Tool(i) if *i==state.tool)
                    || (*target == Target::Functions && state.functions)
                {
                    1.0
                } else {
                    state.motion.hover(*target)
                };
                Color::rgb8(
                    (26.0 + 22.0 * amount) as u8,
                    (34.0 + 13.0 * amount) as u8,
                    (47.0 + 22.0 * amount) as u8,
                )
            })
            .with_alpha(key_opacity),
            5.0,
        );
        if *target == Target::Spatial {
            spatial_toggle(out, *rect, clip, fonts, state.spatial.blend as f32);
            continue;
        }
        button_text(
            out,
            *rect,
            clip,
            caption,
            fonts,
            if rect.y >= layout.keypad_top { 1 } else { 2 },
            (if active { ACCENT } else { INK }).with_alpha(key_opacity),
        );
    }
    if layout.max_scroll > 0.0 {
        let thumb = (layout.list.h * layout.list.h / (layout.list.h + layout.max_scroll)).max(20.0);
        let start = layout.list.y
            + (layout.list.h - thumb) * state.sidebar_scroll.min(layout.max_scroll)
                / layout.max_scroll;
        out.line(
            [layout.sidebar - 3.0, start],
            [layout.sidebar - 3.0, start + thumb],
            MUTED.with_alpha(0.5),
            2.0,
            layout.list,
            7.0,
        );
    }
    if let Some(popup) = layout.popup {
        let text_start = out.texts.len();
        let panel_start = out.panels.len();
        let line_start = out.lines.len();
        out.button(popup, Color::rgb8(26, 33, 45), 10.0);
        out.text(
            if state.functions {
                "FUNCTIONS"
            } else {
                "LINE APPEARANCE"
            },
            [popup.x + 14.0, popup.y + 27.0],
            2,
            INK,
            popup,
        );
        for (name, p) in &layout.headings {
            out.text(*name, *p, 2, MUTED, layout.popup_clip);
        }
        if let Some(i) = state.style_popup {
            let style = state.document.styles[i];
            out.text(
                format!("Width  {}", style.width),
                [popup.x + 14.0, popup.y + 109.0],
                1,
                INK,
                popup,
            );
            out.text(
                format!("Opacity  {:.0}%", style.opacity * 100.0),
                [popup.x + 14.0, popup.y + 149.0],
                1,
                INK,
                popup,
            );
        }
        for (target, r, caption) in &layout.popup_controls {
            let clipping = if *target == Target::ClosePopup {
                popup
            } else {
                layout.popup_clip
            };
            let Some(clip) = r.intersection(clipping) else {
                continue;
            };
            let active = state.hovered == Some(*target)
                || state.style_popup.is_some_and(|i| match target {
                    Target::Color(c) => state.document.styles[i].color == *c,
                    Target::Pattern(p) => state.document.styles[i].pattern == *p,
                    _ => false,
                });
            let color = if let Target::Color(c) = target {
                graph_style::color(*c)
            } else if active {
                Color::rgb8(63, 60, 87)
            } else {
                Color::rgb8(39, 47, 61)
            };
            out.button(clip, color, 11.0);
            button_text(out, *r, clip, caption, fonts, 1, INK);
            if active && matches!(target, Target::Color(_)) {
                out.text("*", [r.x + 13.0, r.y + 26.0], 1, INK, clip);
            }
        }
        for t in &mut out.texts[text_start..] {
            t.depth = 12.0;
        }
        if state.functions && layout.catalog_max > 0.0 {
            let r = layout.popup_clip;
            let thumb = (r.h * r.h / (r.h + layout.catalog_max)).max(20.0);
            let y = r.y
                + (r.h - thumb) * state.catalog_scroll.min(layout.catalog_max) / layout.catalog_max;
            out.line(
                [popup.x + popup.w - 4.0, y],
                [popup.x + popup.w - 4.0, y + thumb],
                MUTED,
                2.0,
                popup,
                12.0,
            );
        }
        let alpha = state.motion.popup;
        for panel in &mut out.panels[panel_start..] {
            panel.color = panel.color.with_alpha(panel.color.alpha() * alpha);
        }
        for text in &mut out.texts[text_start..] {
            text.color = text.color.with_alpha(text.color.alpha() * alpha);
        }
        for line in &mut out.lines[line_start..] {
            line.color = line.color.with_alpha(line.color.alpha() * alpha);
        }
    }
    if let Some(Target::Function(name)) = state.hovered
        && let Some(function) = sim_math::functions::find(name)
    {
        out.panel(
            Rect::new(0.0, layout.height - 28.0, layout.width, 28.0),
            PANEL,
            14.0,
        );
        let start = out.texts.len();
        out.text(
            format!(
                "{}({})   -   {}",
                function.name, function.signature, function.description
            ),
            [16.0, layout.height - 9.0],
            2,
            INK,
            full,
        );
        for t in &mut out.texts[start..] {
            t.depth = 15.0;
        }
    }
}
fn spatial_toggle(out: &mut Scene, rect: Rect, clip: Rect, fonts: &Fonts, blend: f32) {
    let label = "2D / 3D";
    let start = rect.x + (rect.w - fonts.width(2, label)) * 0.5;
    let first_width = fonts.width(2, "2D");
    let second = start + fonts.width(2, "2D / ");
    let baseline = rect.y + rect.h * 0.5 + 4.0;
    out.text(
        "2D",
        [start, baseline],
        2,
        INK.with_alpha(1.0 - blend * 0.45),
        clip,
    );
    out.text(" / ", [start + first_width, baseline], 2, MUTED, clip);
    out.text(
        "3D",
        [second, baseline],
        2,
        INK.with_alpha(0.55 + blend * 0.45),
        clip,
    );
    let center = start + first_width * 0.5 + (second - start) * blend;
    out.line(
        [center - 10.0, rect.y + rect.h - 5.0],
        [center + 10.0, rect.y + rect.h - 5.0],
        ACCENT,
        2.0,
        clip,
        6.0,
    );
}
fn number(value: f64) -> String {
    if value.abs() >= 1e10 || (value != 0.0 && value.abs() < 1e-7) {
        format!("{value:.9e}")
    } else {
        format!("{value:.10}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}
fn button_text(
    out: &mut Scene,
    rect: Rect,
    clip: Rect,
    text: &str,
    fonts: &Fonts,
    style: usize,
    color: Color,
) {
    let style = if style < 2 && fonts.width(style, text) > rect.w - 10.0 {
        style + 1
    } else {
        style
    };
    let x = rect.x + (rect.w - fonts.width(style, text)).max(6.0) * 0.5;
    out.text(text, [x, rect.y + rect.h * 0.5 + 6.0], style, color, clip);
}
