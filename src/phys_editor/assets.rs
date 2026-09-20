//! Startup-only shared font registration; no editor-specific bitmap assets.

use sim_logic::prelude::*;

use super::input::EditorAction;

const FONT: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-Medium.ttf");

pub(crate) struct EditorAssets {
    pub(crate) small: TextFont,
    pub(crate) body: TextFont,
    pub(crate) heading: TextFont,
}

impl EditorAssets {
    pub(crate) fn register_shared(
        app: &mut Application<EditorAction>,
        face: &TextFont,
        small: &TextFont,
    ) -> LogicResult<Self> {
        Ok(Self {
            small: small.clone(),
            body: app.register_font_style(face, TextSettings::new(16.0)?)?,
            heading: app.register_font_style(face, TextSettings::new(22.0)?)?,
        })
    }

    pub(crate) const fn font_bytes() -> usize {
        FONT.len()
    }

    pub(crate) fn register(app: &mut Application<EditorAction>) -> LogicResult<Self> {
        let body = app.register_font(FONT.to_vec(), TextSettings::new(16.0)?)?;
        let small = app.register_font_style(&body, TextSettings::new(14.0)?)?;
        let heading = app.register_font_style(&body, TextSettings::new(22.0)?)?;
        Ok(Self {
            small,
            body,
            heading,
        })
    }
}
