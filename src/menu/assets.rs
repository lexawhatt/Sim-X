use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg::{Options, Tree},
};
use sim_logic::prelude::*;

use super::input::MenuAction;

mod halo;

const UI_FONT: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-Medium.ttf");
const LOGO_FONT: &[u8] = include_bytes!("../../assets/fonts/SpaceGrotesk-Medium.ttf");

pub(crate) struct MenuAssets {
    pub(crate) small: TextFont,
    pub(crate) body: TextFont,
    pub(crate) heading: TextFont,
    pub(crate) logo: TextFont,
    pub(crate) icons: [ImageAssetId; 3],
    pub(crate) halo: ImageAssetId,
    pub(crate) physics: ImageAssetId,
}

impl MenuAssets {
    pub(crate) const fn font_bytes() -> usize {
        UI_FONT.len() + LOGO_FONT.len()
    }

    pub(crate) fn register(app: &mut Application<MenuAction>) -> LogicResult<Self> {
        let body = app.register_font(UI_FONT.to_vec(), TextSettings::new(18.0)?)?;
        // Sizes keep distinct atlas identities, but share one parsed UI face.
        let small = app.register_font_style(&body, TextSettings::new(14.0)?)?;
        let heading = app.register_font_style(&body, TextSettings::new(26.0)?)?;
        let logo = app.register_font(LOGO_FONT.to_vec(), TextSettings::new(88.0)?)?;
        Ok(Self {
            small,
            body,
            heading,
            logo,
            icons: [
                register_svg(app, include_bytes!("../../assets/icons/github.svg"), 96)?,
                register_svg(app, include_bytes!("../../assets/icons/youtube.svg"), 96)?,
                register_svg(app, include_bytes!("../../assets/icons/telegram.svg"), 96)?,
            ],
            halo: app.register_image_rgba8(halo::SIZE, halo::SIZE, &halo::pixels())?,
            physics: register_svg(
                app,
                include_bytes!("../../assets/illustrations/physics.svg"),
                768,
            )?,
        })
    }
}

fn register_svg(
    app: &mut Application<MenuAction>,
    bytes: &[u8],
    size: u32,
) -> LogicResult<ImageAssetId> {
    // Only trusted embedded SVGs enter this startup-only decoder. Aspect ratio
    // is preserved, and pixels are converted to Logic's straight-alpha contract.
    let tree = Tree::from_data(bytes, &Options::default())?;
    let mut pixmap = Pixmap::new(size, size).ok_or("cannot allocate menu image")?;
    let extent = tree.size();
    let scale = (size as f32 / extent.width()).min(size as f32 / extent.height());
    let transform = Transform::from_row(
        scale,
        0.0,
        0.0,
        scale,
        (size as f32 - extent.width() * scale) * 0.5,
        (size as f32 - extent.height() * scale) * 0.5,
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    for pixel in pixmap.data_mut().chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = (u32::from(*channel) * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
    Ok(app.register_image_rgba8(size, size, pixmap.data())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_sizes_share_one_ui_face_and_support_russian_labels() -> LogicResult {
        let mut config = AppConfig::default();
        config.set_text_limits(TextLimits::new(4, MenuAssets::font_bytes()));
        config.set_image_asset_limits(ImageAssetLimits::new(5, 768, 768, 5 * 1024 * 1024));
        let mut app = Application::<MenuAction>::new(config)?;
        let assets = MenuAssets::register(&mut app)?;
        assert!(assets.small.shares_face_with(&assets.body));
        assert!(assets.heading.shares_face_with(&assets.body));
        assert!(!assets.logo.shares_face_with(&assets.body));
        assert_ne!(assets.small, assets.body);
        assert_ne!(assets.heading, assets.body);
        for font in [&assets.small, &assets.body, &assets.heading] {
            let russian = ScreenTextVisual::new(
                font.clone(),
                "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ абвгдеёжзийклмнопрстуфхцчшщъыьэюя",
                LogicalScreenPosition::new(0.0, 0.0),
            )?;
            assert!(russian.metrics().advance() > 0.0);
        }
        let logo =
            ScreenTextVisual::new(assets.logo, "Sim;X", LogicalScreenPosition::new(0.0, 0.0))?;
        assert!(logo.metrics().advance() > 0.0);
        Ok(())
    }
}
