//! Opt-in, CPU-only layout evidence. Never opens or captures a desktop window.
use crate::math_editor::view::{drawing::Scene, layout::Rect};
use sim_logic::prelude::Color;
use std::fmt::Write;

pub(in crate::math_editor) fn export(scene: &Scene, rect: Rect) {
    let Ok(destination) = std::env::var("SIM_X_MATH_PREVIEW_SVG") else {
        return;
    };
    let color = |c: Color| {
        let srgb = |v: f32| {
            if v <= 0.0031308 {
                v * 12.92
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            }
        };
        format!(
            "rgb({:.0},{:.0},{:.0})",
            srgb(c.red()) * 255.0,
            srgb(c.green()) * 255.0,
            srgb(c.blue()) * 255.0
        )
    };
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}"><rect x="{}" y="{}" width="{}" height="{}" fill="#0b1016"/>"##,
        rect.w, rect.h, rect.x, rect.y, rect.w, rect.h, rect.x, rect.y, rect.w, rect.h
    );
    let mut layers = Vec::new();
    for panel in &scene.panels {
        layers.push((
            panel.depth,
            format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{}" opacity="{}"/>"#,
                panel.rect.x,
                panel.rect.y,
                panel.rect.w,
                panel.rect.h,
                panel.radius,
                color(panel.color),
                panel.color.alpha()
            ),
        ));
    }
    for line in &scene.lines {
        layers.push((line.depth, format!(
            r#"<path d="M{} {}L{} {}" fill="none" stroke="{}" stroke-width="{}" opacity="{}"/>"#,
            line.from[0],
            line.from[1],
            line.to[0],
            line.to[1],
            color(line.color),
            line.width,
            line.color.alpha()
        )));
    }
    for (id, text) in scene.texts.iter().enumerate() {
        clip(&mut svg, "text", id, text.clip);
        let escaped = text
            .text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        layers.push((text.depth, format!(r#"<text clip-path="url(#text{})" x="{}" y="{}" fill="{}" font-family="sans-serif" font-size="{}" opacity="{}">{}</text>"#,
            id,text.position[0],text.position[1],color(text.color),[26,18,14][text.style],text.color.alpha(),escaped)));
    }
    for (id, dot) in scene.dots.iter().enumerate() {
        clip(&mut svg, "dot", id, dot.clip);
        layers.push((dot.depth, format!(r#"<circle clip-path="url(#dot{})" cx="{}" cy="{}" r="{}" fill="{}" opacity="{}"/>"#,
            id,dot.center[0],dot.center[1],dot.radius,color(dot.color),dot.color.alpha())));
    }
    layers.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, element) in layers {
        svg.push_str(&element);
    }
    svg.push_str("</svg>");
    std::fs::write(destination, svg).unwrap();
}

fn clip(svg: &mut String, prefix: &str, id: usize, rect: Rect) {
    writeln!(svg, r#"<defs><clipPath id="{prefix}{id}"><rect x="{}" y="{}" width="{}" height="{}"/></clipPath></defs>"#,
        rect.x,rect.y,rect.w,rect.h).unwrap();
}
