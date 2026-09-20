use sim_logic::prelude::Color;

// Linear values corresponding to the palette's sRGB swatches. Color::rgb8
// performs this conversion at runtime and is not a const constructor.
pub(crate) const BACKGROUND: Color = Color::rgb(0.003677, 0.005182, 0.008023); // #0c1016
pub(crate) const PANEL: Color = Color::rgb(0.006512, 0.009721, 0.015209); // #131921
pub(crate) const ACCENT: Color = Color::rgb(0.258183, 0.723055, 0.508881); // #8bddbd
pub(crate) const INK: Color = Color::rgb(0.822786, 0.871367, 0.863157); // #eaf0ef
pub(crate) const MUTED: Color = Color::rgb(0.270498, 0.341914, 0.401978); // #8e9eaa
pub(crate) const HINT: Color = Color::rgb(0.417885, 0.485150, 0.558340); // #adbac5
pub(crate) const LINE: Color = Color::rgb(0.024158, 0.042311, 0.056128); // #2b3a43

pub(crate) fn button(emphasis: f32, pressed: bool) -> Color {
    let factor = if pressed { 0.8 } else { emphasis };
    Color::rgb8(
        (24.0 + 18.0 * factor) as u8,
        (32.0 + 39.0 * factor) as u8,
        (41.0 + 34.0 * factor) as u8,
    )
}

pub(crate) fn domain(domain: super::state::Domain) -> Color {
    use super::state::Domain;
    match domain {
        Domain::Phys => ACCENT,
        Domain::Math => Color::rgb8(180, 178, 250),
        Domain::Chem => Color::rgb8(230, 188, 121),
        Domain::Biol => Color::rgb8(233, 146, 185),
    }
}

pub(crate) fn mixed_domain(weights: [f32; 4]) -> Color {
    let total: f32 = weights.iter().sum();
    if total <= 0.0 {
        return ACCENT;
    }
    let mut channels = [0.0; 3];
    for item in super::state::Domain::ALL {
        let color = domain(item);
        let weight = weights[item.index()] / total;
        channels[0] += color.red() * weight;
        channels[1] += color.green() * weight;
        channels[2] += color.blue() * weight;
    }
    Color::rgb(
        channels[0].min(1.0),
        channels[1].min(1.0),
        channels[2].min(1.0),
    )
}
