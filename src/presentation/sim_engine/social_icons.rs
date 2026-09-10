use ::sim_engine::{
    Color, FrameComposer, FramePassOptions, Image2d, ImageBudget, ImageSampling, ImageTexelRect,
    LogicalScreenPosition, LogicalViewport, LogicalViewportRegion, WgpuRenderer,
};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg::{Options, Tree},
};

use crate::presentation::ui::{
    catalog::{Screen, SocialLink},
    geometry::{Point, UiRect},
    layout::UiLayout,
    state::UiState,
};

const ICON_SIZE: u32 = 32;
const ICON_COUNT: u32 = 3;
const ATLAS_WIDTH: u32 = ICON_SIZE * ICON_COUNT;
const ATLAS_HEIGHT: u32 = ICON_SIZE;
const ATLAS_BYTES: usize = ATLAS_WIDTH as usize * ATLAS_HEIGHT as usize * 4;

const GITHUB_SVG: &[u8] = include_bytes!("../../../assets/icons/github.svg");
const YOUTUBE_SVG: &[u8] = include_bytes!("../../../assets/icons/youtube.svg");
const TELEGRAM_SVG: &[u8] = include_bytes!("../../../assets/icons/telegram.svg");

/// Retained GPU atlas derived once from the three project-owned SVG assets.
pub(super) struct SocialIconAtlas {
    image: Image2d,
    sources: [ImageTexelRect; ICON_COUNT as usize],
}

impl SocialIconAtlas {
    pub(super) fn new(renderer: &WgpuRenderer) -> Result<Self, String> {
        let pixels = rasterize_atlas()?;
        let budget = ImageBudget::new(ATLAS_WIDTH, ATLAS_HEIGHT, ATLAS_BYTES)
            .map_err(|error| error.to_string())?;
        let image = renderer
            .create_image_rgba8(ATLAS_WIDTH, ATLAS_HEIGHT, pixels, budget)
            .map_err(|error| error.to_string())?;
        let sources = [atlas_region(0)?, atlas_region(1)?, atlas_region(2)?];
        Ok(Self { image, sources })
    }

    /// Restores the exact retained atlas after the renderer changes device.
    ///
    /// The replacement is committed only after Sim;Engine has recreated the
    /// texture from the atlas's retained CPU pixels and original budget.
    pub(super) fn restore(&mut self, renderer: &WgpuRenderer) -> Result<(), String> {
        let restored = renderer
            .restore_image(&self.image)
            .map_err(|error| error.to_string())?;
        self.image = restored;
        Ok(())
    }

    pub(super) fn add_to_frame<'frame>(
        &'frame self,
        frame: &mut FrameComposer<'frame>,
        layout: UiLayout,
        state: &UiState,
    ) -> Result<(), String> {
        if state.screen != Screen::MainMenu {
            return Ok(());
        }
        for ((link, button), hover) in SocialLink::ALL
            .into_iter()
            .zip(layout.social_link_buttons())
            .zip(state.animations.social_hover)
        {
            let amount = hover.clamp(0.0, 1.0);
            let size = 29.0 + amount * 2.0;
            let center = button.center();
            let slot = UiRect::from_min_size(
                Point::new(center.x - size * 0.5, center.y - size * 0.5),
                size,
                size,
            );
            let viewport = LogicalViewportRegion::new(
                LogicalScreenPosition::new(slot.min.x, slot.min.y),
                LogicalViewport::new(slot.width(), slot.height())
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            frame
                .draw_image(
                    &self.image,
                    Some(self.sources[link.index()]),
                    Color::WHITE.with_alpha(0.84 + amount * 0.16),
                    ImageSampling::Linear,
                    FramePassOptions::new(10).with_viewport(viewport),
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

fn atlas_region(index: u32) -> Result<ImageTexelRect, String> {
    ImageTexelRect::new(index * ICON_SIZE, 0, ICON_SIZE, ICON_SIZE)
        .map_err(|error| error.to_string())
}

fn rasterize_atlas() -> Result<Vec<u8>, String> {
    let mut atlas = Vec::new();
    atlas
        .try_reserve_exact(ATLAS_BYTES)
        .map_err(|_| format!("could not reserve {ATLAS_BYTES} bytes for social icon atlas"))?;
    atlas.resize(ATLAS_BYTES, 0);

    for (index, (name, svg)) in [
        ("GitHub", GITHUB_SVG),
        ("YouTube", YOUTUBE_SVG),
        ("Telegram", TELEGRAM_SVG),
    ]
    .into_iter()
    .enumerate()
    {
        let pixels = rasterize_svg(svg, name)?;
        for row in 0..ICON_SIZE as usize {
            let source_start = row * ICON_SIZE as usize * 4;
            let destination_start = (row * ATLAS_WIDTH as usize + index * ICON_SIZE as usize) * 4;
            atlas[destination_start..destination_start + ICON_SIZE as usize * 4]
                .copy_from_slice(&pixels[source_start..source_start + ICON_SIZE as usize * 4]);
        }
    }
    Ok(atlas)
}

fn rasterize_svg(svg: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let tree = Tree::from_data(svg, &Options::default())
        .map_err(|error| format!("could not parse {name} SVG: {error}"))?;
    if tree.size().width() != tree.size().height() {
        return Err(format!("{name} SVG must use a square canvas"));
    }
    let mut pixmap = Pixmap::new(ICON_SIZE, ICON_SIZE)
        .ok_or_else(|| format!("could not allocate {name} icon raster"))?;
    let scale = ICON_SIZE as f32 / tree.size().width();
    resvg::render(
        &tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(ICON_SIZE as usize * ICON_SIZE as usize * 4)
        .map_err(|_| format!("could not reserve {name} icon pixels"))?;
    for color in pixmap.pixels() {
        let color = color.demultiply();
        pixels.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    if pixels.chunks_exact(4).all(|pixel| pixel[3] == 0) {
        return Err(format!("{name} SVG rasterized to an empty icon"));
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::{ATLAS_BYTES, ICON_COUNT, ICON_SIZE, rasterize_atlas};

    #[test]
    fn all_embedded_social_icons_form_one_exact_bounded_atlas() {
        let atlas = rasterize_atlas().expect("valid embedded SVG assets");
        assert_eq!(atlas.len(), ATLAS_BYTES);
        for icon in 0..ICON_COUNT as usize {
            let has_visible_pixel = (0..ICON_SIZE as usize).any(|row| {
                let row_start = (row * ICON_COUNT as usize * ICON_SIZE as usize
                    + icon * ICON_SIZE as usize)
                    * 4;
                atlas[row_start..row_start + ICON_SIZE as usize * 4]
                    .chunks_exact(4)
                    .any(|pixel| pixel[3] != 0)
            });
            assert!(has_visible_pixel);
        }
    }
}
