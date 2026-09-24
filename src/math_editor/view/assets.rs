use sim_logic::prelude::*;

#[derive(Resource, Clone)]
pub(in crate::math_editor) struct Fonts {
    pub faces: [TextFont; 3],
    pub widths: [[f32; 128]; 3],
    prefixes: Vec<(String, [f32; 3])>,
}
impl Fonts {
    pub fn new(faces: [TextFont; 3]) -> LogicResult<Self> {
        let mut widths = [[0.0; 128]; 3];
        for (style, font) in faces.iter().enumerate() {
            let mut session = font.shaping_session()?;
            for code in 32_u8..127 {
                let visual = ScreenTextVisual::new_with_session(
                    &mut session,
                    &char::from(code).to_string(),
                    LogicalScreenPosition::new(0.0, 0.0),
                )?;
                widths[style][usize::from(code)] = visual.metrics().advance();
            }
        }
        let mut prefixes = vec![];
        let names: Vec<String> = sim_math::functions::FUNCTIONS
            .iter()
            .map(|f| format!("{}(", f.name))
            .chain(["tg(", "ctg(", "arcsin(", "arccos(", "arctan(", "π", " dx"].map(str::to_owned))
            .collect();
        for prefix in names {
            let mut sizes = [0.0; 3];
            for (style, font) in faces.iter().enumerate() {
                sizes[style] = ScreenTextVisual::new(
                    font.clone(),
                    &prefix,
                    LogicalScreenPosition::new(0.0, 0.0),
                )?
                .metrics()
                .advance();
            }
            prefixes.push((prefix, sizes));
        }
        Ok(Self {
            faces,
            widths,
            prefixes,
        })
    }
    pub fn width(&self, style: usize, text: &str) -> f32 {
        if let Some((_, sizes)) = self.prefixes.iter().find(|(prefix, _)| prefix == text) {
            sizes[style]
        } else {
            text.chars()
                .map(|c| {
                    if c.is_ascii() {
                        self.widths[style][c as usize]
                    } else {
                        self.widths[style][usize::from(b'?')]
                    }
                })
                .sum()
        }
    }
}
