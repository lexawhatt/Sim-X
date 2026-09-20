//! Numeric and alphabet pages of the same dock, with shared hit geometry.
use super::layout::{Rect, Target};
pub(super) fn numeric(controls: &mut Vec<(Target, Rect, &'static str)>, x: f32, top: f32) {
    let symbols = [
        (Target::Type('x'), "x"),
        (Target::Type('y'), "y"),
        (Target::Square, "a^2"),
        (Target::Power, "a^b"),
        (Target::Type('π'), "pi"),
        (Target::Type('('), "("),
        (Target::Type(')'), ")"),
        (Target::Type('|'), "|a|"),
        (Target::Type('<'), "<"),
        (Target::Type('>'), ">"),
        (Target::NthRoot, "n-root"),
        (Target::Root, "sqrt"),
        (Target::Type(','), ","),
        (Target::Type('≤'), "≤"),
        (Target::Type('≥'), "≥"),
        (Target::Alphabet, "ABC"),
        (Target::Integral, "Integral"),
        (Target::Derivative, "d/dx"),
        (Target::Paste, "Paste"),
    ];
    let numbers = [
        (Target::Type('7'), "7"),
        (Target::Type('8'), "8"),
        (Target::Type('9'), "9"),
        (Target::Fraction, "/"),
        (Target::Type('4'), "4"),
        (Target::Type('5'), "5"),
        (Target::Type('6'), "6"),
        (Target::Type('*'), "*"),
        (Target::Type('1'), "1"),
        (Target::Type('2'), "2"),
        (Target::Type('3'), "3"),
        (Target::Type('-'), "-"),
        (Target::Type('0'), "0"),
        (Target::Type('.'), "."),
        (Target::Type('='), "="),
        (Target::Type('+'), "+"),
    ];
    for (bank, keys) in [symbols.as_slice(), numbers.as_slice()]
        .into_iter()
        .enumerate()
    {
        for (i, (target, label)) in keys.iter().copied().enumerate() {
            // Names should use the same readable key font as symbols. Give
            // Integral its measured breathing room instead of shrinking it.
            let (offset, width, row) = if bank == 0 && i >= 15 {
                let (offset, width) =
                    [(0.0, 52.0), (58.0, 82.0), (146.0, 58.0), (210.0, 64.0)][i - 15];
                (offset, width, 3)
            } else if bank == 0 && i >= 10 {
                let (offset, width) = [
                    (0.0, 60.0),
                    (66.0, 50.0),
                    (122.0, 36.0),
                    (164.0, 50.0),
                    (220.0, 54.0),
                ][i - 10];
                (offset, width, 2)
            } else if bank == 0 {
                ((i % 5) as f32 * 56.0, 50.0, i / 5)
            } else {
                ((i % 4) as f32 * 70.0, 64.0, i / 4)
            };
            controls.push((
                target,
                Rect::new(
                    x + bank as f32 * 302.0 + offset,
                    top + 12.0 + row as f32 * 40.0,
                    width,
                    34.0,
                ),
                label,
            ));
        }
    }
    let x = x + 604.0;
    controls.extend([
        (Target::Left, Rect::new(x, top + 52.0, 66.0, 34.0), "Left"),
        (
            Target::Right,
            Rect::new(x + 72.0, top + 52.0, 66.0, 34.0),
            "Right",
        ),
        (
            Target::Backspace,
            Rect::new(x, top + 92.0, 138.0, 34.0),
            "Delete",
        ),
        (
            Target::Enter,
            Rect::new(x, top + 132.0, 138.0, 34.0),
            "Enter",
        ),
    ]);
}
pub(super) fn alphabet(controls: &mut Vec<(Target, Rect, &'static str)>, x: f32, top: f32) {
    for (row, letters) in ["qwertyuiop", "asdfghjkl", "zxcvbnm"]
        .into_iter()
        .enumerate()
    {
        let offset = (10 - letters.len()) as f32 * 35.0;
        for (column, (index, ch)) in letters.char_indices().enumerate() {
            controls.push((
                Target::Type(ch),
                Rect::new(
                    x + offset + column as f32 * 70.0,
                    top + 12.0 + row as f32 * 40.0,
                    64.0,
                    34.0,
                ),
                &letters[index..index + 1],
            ));
        }
    }
    for (i, (target, label)) in [
        (Target::Alphabet, "123"),
        (Target::Copy, "Copy"),
        (Target::Cut, "Cut"),
        (Target::Paste, "Paste"),
        (Target::Left, "Left"),
        (Target::Right, "Right"),
        (Target::Backspace, "Delete"),
        (Target::Enter, "Enter"),
    ]
    .into_iter()
    .enumerate()
    {
        controls.push((
            target,
            Rect::new(x + i as f32 * 93.0, top + 132.0, 86.0, 34.0),
            label,
        ));
    }
    // The shared Functions anchor occupies this position on the numeric page.
    // Move it to the right of the letters without obscuring any key.
    if let Some((_, rect, _)) = controls
        .iter_mut()
        .find(|(t, _, _)| *t == Target::Functions)
    {
        *rect = Rect::new(x + 604.0, top + 92.0, 138.0, 34.0);
    }
}
