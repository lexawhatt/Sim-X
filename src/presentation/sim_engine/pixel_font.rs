use ::sim_engine::{Color, ShapeStyle};

use crate::presentation::ui::{
    geometry::{Point, UiRect},
    layout::UiLayout,
};

use super::scene_builder::UiSceneBuilder as Scene;

pub(super) fn draw(
    scene: &mut Scene,
    _layout: UiLayout,
    text: &str,
    origin: Point,
    pixel_size: f32,
    color: Color,
) {
    let mut cursor_x = origin.x;
    for character in text.chars() {
        let pattern = glyph_pattern(character.to_ascii_uppercase());
        for (row, bits) in pattern.into_iter().enumerate() {
            for column in 0..3 {
                if bits & (1 << (2 - column)) != 0 {
                    let min = Point::new(
                        cursor_x + column as f32 * pixel_size,
                        origin.y + row as f32 * pixel_size,
                    );
                    let pixel = UiRect::from_min_size(
                        min,
                        (pixel_size - 0.55).max(0.5),
                        (pixel_size - 0.55).max(0.5),
                    );
                    scene.rect(ui_rect(pixel), 0.0, ShapeStyle::filled(color));
                }
            }
        }
        cursor_x += pixel_size * 4.0;
    }
}

pub(super) fn draw_centered(
    scene: &mut Scene,
    layout: UiLayout,
    text: &str,
    center: Point,
    pixel_size: f32,
    color: Color,
) {
    let width = text_width(text, pixel_size);
    draw(
        scene,
        layout,
        text,
        Point::new(center.x - width * 0.5, center.y - pixel_size * 2.5),
        pixel_size,
        color,
    );
}

fn text_width(text: &str, pixel_size: f32) -> f32 {
    let glyph_count = text.chars().count();
    let columns = glyph_count
        .checked_sub(1)
        .map_or(0, |spacing_count| spacing_count * 4)
        + 3;
    columns as f32 * pixel_size
}

fn ui_rect(rect: UiRect) -> UiRect {
    rect
}

fn glyph_pattern(character: char) -> [u8; 5] {
    match character {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b111, 0b101, 0b101, 0b111, 0b001],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        ';' => [0b000, 0b010, 0b000, 0b010, 0b100],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '&' => [0b010, 0b101, 0b010, 0b101, 0b011],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        '=' => [0b000, 0b111, 0b000, 0b111, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '?' => [0b111, 0b001, 0b011, 0b000, 0b010],
        _ => [0; 5],
    }
}
