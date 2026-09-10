use ::sim_engine::{
    Camera2d, Color, FramePassOptions, LogicalPixels, LogicalScreenPosition, LogicalScreenVector,
    LogicalViewport, LogicalViewportRegion, Scene as WorldScene, SceneBudget, ScreenScene,
    ShapeStyle, Vec2,
};

use crate::presentation::ui::geometry::{Point, UiRect};

/// Maximum retained work for one Sim;X streaming UI scene.
///
/// The current pixel font is command-heavy, so the budget deliberately leaves
/// headroom for a complete 1080p workspace while still making construction
/// cost and upload memory finite.
pub(super) const UI_SCENE_BUDGET: SceneBudget = SceneBudget::new(
    16_384,
    32_768,
    1_000_000,
    4 * 1024 * 1024,
    8 * 1024 * 1024,
    96 * 1024 * 1024,
    16_384,
);

const SCIENTIFIC_SCENE_BUDGET: SceneBudget = SceneBudget::new(
    8_192,
    32_768,
    500_000,
    2 * 1024 * 1024,
    4 * 1024 * 1024,
    48 * 1024 * 1024,
    8_192,
);

const OVERLAY_SCENE_BUDGET: SceneBudget = SceneBudget::new(
    2_048,
    4_096,
    128_000,
    512 * 1024,
    1024 * 1024,
    16 * 1024 * 1024,
    2_048,
);

const HEADS_UP_SCENE_BUDGET: SceneBudget = SceneBudget::new(
    4_096,
    8_192,
    256_000,
    1024 * 1024,
    2 * 1024 * 1024,
    24 * 1024 * 1024,
    4_096,
);

pub(super) struct ScientificScene {
    pub(super) scene: WorldScene,
    pub(super) camera: Camera2d,
    pub(super) options: FramePassOptions,
}

pub(super) struct BuiltFrame {
    pub(super) screen: ScreenScene,
    pub(super) scientific: Option<ScientificScene>,
    pub(super) heads_up: Option<ScreenScene>,
    pub(super) overlay: Option<ScreenScene>,
}

#[cfg(test)]
impl BuiltFrame {
    pub(super) fn command_count(&self) -> usize {
        self.screen.command_count()
            + self
                .scientific
                .as_ref()
                .map_or(0, |scientific| scientific.scene.command_count())
            + self.heads_up.as_ref().map_or(0, ScreenScene::command_count)
            + self.overlay.as_ref().map_or(0, ScreenScene::command_count)
    }

    pub(super) const fn budget(&self) -> Option<SceneBudget> {
        self.screen.budget()
    }
}

struct ScientificSceneBuilder {
    scene: WorldScene,
    camera: Camera2d,
    viewport: LogicalViewportRegion,
    canvas: UiRect,
}

/// Fallible adapter-local builder for logical-screen geometry.
///
/// Drawing helpers keep their lightweight call sites, while the first engine
/// rejection is retained and returned by [`Self::finish`]. Required visuals
/// therefore cannot disappear behind the boolean convenience API.
pub(super) struct UiSceneBuilder {
    scene: Option<ScreenScene>,
    scientific: Option<ScientificSceneBuilder>,
    heads_up: Option<ScreenScene>,
    draw_into_heads_up: bool,
    first_error: Option<String>,
}

impl UiSceneBuilder {
    pub(super) fn new(background: Color) -> Self {
        Self::with_budget(background, UI_SCENE_BUDGET)
    }

    pub(super) fn new_overlay(background: Color) -> Self {
        Self::with_budget(background, OVERLAY_SCENE_BUDGET)
    }

    fn with_budget(background: Color, budget: SceneBudget) -> Self {
        match ScreenScene::with_budget(background, budget) {
            Ok(scene) => Self {
                scene: Some(scene),
                scientific: None,
                heads_up: None,
                draw_into_heads_up: false,
                first_error: None,
            },
            Err(error) => Self {
                scene: None,
                scientific: None,
                heads_up: None,
                draw_into_heads_up: false,
                first_error: Some(format!("could not create bounded UI scene: {error}")),
            },
        }
    }

    pub(super) fn rect(&mut self, rect: UiRect, corner_radius: f32, style: ShapeStyle) {
        if self.first_error.is_some() {
            return;
        }
        let scene = if self.draw_into_heads_up {
            self.heads_up.as_mut()
        } else {
            self.scene.as_mut()
        };
        let Some(scene) = scene else {
            self.first_error = Some("UI scene storage is unavailable".to_owned());
            return;
        };
        let min = logical_position(rect.min);
        let size = logical_vector(rect.width(), rect.height());
        let result = if corner_radius == 0.0 {
            scene
                .try_square_rect(min, size, style)
                .map_err(|error| error.to_string())
        } else {
            logical_pixels(corner_radius).and_then(|radius| {
                scene
                    .try_rect(min, size, radius, style)
                    .map_err(|error| error.to_string())
            })
        };
        self.record(result);
    }

    /// Routes subsequent screen-space commands above the scientific world pass.
    pub(super) fn start_heads_up_overlay(&mut self) {
        if self.first_error.is_some() || self.draw_into_heads_up {
            return;
        }
        match ScreenScene::with_budget(Color::TRANSPARENT, HEADS_UP_SCENE_BUDGET) {
            Ok(scene) => {
                self.heads_up = Some(scene);
                self.draw_into_heads_up = true;
            }
            Err(error) => {
                self.first_error =
                    Some(format!("could not create bounded heads-up scene: {error}"));
            }
        }
    }

    pub(super) fn start_scientific_canvas(&mut self, canvas: UiRect, camera: Camera2d) {
        if self.first_error.is_some() {
            return;
        }
        if self.scientific.is_some() {
            self.first_error = Some("a UI frame may declare only one scientific canvas".to_owned());
            return;
        }
        let result = (|| {
            let viewport = LogicalViewport::new(canvas.width(), canvas.height())
                .map_err(|error| error.to_string())?;
            let viewport = LogicalViewportRegion::new(
                LogicalScreenPosition::new(canvas.min.x, canvas.min.y),
                viewport,
            )
            .map_err(|error| error.to_string())?;
            let mut scene =
                WorldScene::with_budget(Color::rgba8(0, 0, 0, 0), SCIENTIFIC_SCENE_BUDGET)
                    .map_err(|error| error.to_string())?;
            append_adaptive_grid(&mut scene, camera, viewport.viewport())?;
            Ok(ScientificSceneBuilder {
                scene,
                camera,
                viewport,
                canvas,
            })
        })();
        match result {
            Ok(scientific) => self.scientific = Some(scientific),
            Err(error) => self.first_error = Some(error),
        }
    }

    pub(super) fn scientific_circle(&mut self, center: Point, radius: f32, style: ShapeStyle) {
        if self.first_error.is_some() {
            return;
        }
        let Some(scientific) = self.scientific.as_mut() else {
            self.first_error = Some("scientific geometry was emitted before its canvas".to_owned());
            return;
        };
        let center = match scientific_world_point(scientific.camera, scientific.canvas, center) {
            Ok(center) => center,
            Err(error) => {
                self.first_error = Some(error);
                return;
            }
        };
        let result = scientific
            .scene
            .try_circle(center, radius / scientific.camera.zoom(), style)
            .map_err(|error| error.to_string());
        self.record(result);
    }

    /// Adds a circle whose center and radius are already expressed in world units.
    pub(super) fn scientific_world_circle(
        &mut self,
        center: Point,
        radius: f32,
        style: ShapeStyle,
    ) {
        if self.first_error.is_some() {
            return;
        }
        let Some(scientific) = self.scientific.as_mut() else {
            self.first_error = Some("scientific geometry was emitted before its canvas".to_owned());
            return;
        };
        let result = scientific
            .scene
            .try_circle(Vec2::new(center.x, center.y), radius, style)
            .map_err(|error| error.to_string());
        self.record(result);
    }

    pub(super) fn scientific_line(&mut self, from: Point, to: Point, width: f32, color: Color) {
        if self.first_error.is_some() {
            return;
        }
        let Some(scientific) = self.scientific.as_mut() else {
            self.first_error = Some("scientific geometry was emitted before its canvas".to_owned());
            return;
        };
        let (from, to) = match (
            scientific_world_point(scientific.camera, scientific.canvas, from),
            scientific_world_point(scientific.camera, scientific.canvas, to),
        ) {
            (Ok(from), Ok(to)) => (from, to),
            (Err(error), _) | (_, Err(error)) => {
                self.first_error = Some(error);
                return;
            }
        };
        let result = scientific
            .scene
            .try_line(from, to, width, color)
            .map_err(|error| error.to_string());
        self.record(result);
    }

    /// Adds a fixed-width line between points already expressed in world units.
    pub(super) fn scientific_world_line(
        &mut self,
        from: Point,
        to: Point,
        width: f32,
        color: Color,
    ) {
        if self.first_error.is_some() {
            return;
        }
        let Some(scientific) = self.scientific.as_mut() else {
            self.first_error = Some("scientific geometry was emitted before its canvas".to_owned());
            return;
        };
        let result = scientific
            .scene
            .try_line(
                Vec2::new(from.x, from.y),
                Vec2::new(to.x, to.y),
                width,
                color,
            )
            .map_err(|error| error.to_string());
        self.record(result);
    }

    #[cfg(test)]
    pub(super) fn command_count(&self) -> usize {
        self.scene.as_ref().map_or(0, ScreenScene::command_count)
            + self
                .scientific
                .as_ref()
                .map_or(0, |scientific| scientific.scene.command_count())
            + self.heads_up.as_ref().map_or(0, ScreenScene::command_count)
    }

    pub(super) fn finish(self) -> Result<BuiltFrame, String> {
        match (self.scene, self.first_error) {
            (_, Some(error)) => Err(format!("required UI visual was rejected: {error}")),
            (Some(screen), None) => Ok(BuiltFrame {
                screen,
                scientific: self.scientific.map(|scientific| ScientificScene {
                    scene: scientific.scene,
                    camera: scientific.camera,
                    options: FramePassOptions::new(10).with_viewport(scientific.viewport),
                }),
                heads_up: self.heads_up,
                overlay: None,
            }),
            (None, None) => Err("UI scene construction ended without a scene".to_owned()),
        }
    }

    fn record(&mut self, result: Result<(), String>) {
        if let Err(error) = result {
            self.first_error = Some(error);
        }
    }
}

fn logical_position(point: Point) -> LogicalScreenPosition {
    LogicalScreenPosition::new(point.x, point.y)
}

fn logical_vector(width: f32, height: f32) -> LogicalScreenVector {
    LogicalScreenVector::new(width, height)
}

fn logical_pixels(value: f32) -> Result<LogicalPixels, String> {
    LogicalPixels::new(value).map_err(|error| error.to_string())
}

fn scientific_world_point(camera: Camera2d, canvas: UiRect, point: Point) -> Result<Vec2, String> {
    let viewport =
        LogicalViewport::new(canvas.width(), canvas.height()).map_err(|error| error.to_string())?;
    let local = LogicalScreenPosition::new(point.x - canvas.min.x, point.y - canvas.min.y);
    camera
        .screen_to_world(local, viewport)
        .map_err(|error| error.to_string())
}

fn append_adaptive_grid(
    scene: &mut WorldScene,
    camera: Camera2d,
    viewport: LogicalViewport,
) -> Result<(), String> {
    let top_left = camera
        .screen_to_world(LogicalScreenPosition::new(0.0, 0.0), viewport)
        .map_err(|error| error.to_string())?;
    let bottom_right = camera
        .screen_to_world(
            LogicalScreenPosition::new(viewport.width(), viewport.height()),
            viewport,
        )
        .map_err(|error| error.to_string())?;
    let minimum_x = top_left.x().min(bottom_right.x());
    let maximum_x = top_left.x().max(bottom_right.x());
    let minimum_y = top_left.y().min(bottom_right.y());
    let maximum_y = top_left.y().max(bottom_right.y());
    let spacing = adaptive_grid_spacing(camera.zoom())?;
    let color = Color::rgba8(112, 211, 255, 22);

    let first_x = (minimum_x / spacing).floor() * spacing;
    let mut x = first_x;
    let mut lines = 0_usize;
    while x <= maximum_x && lines < 128 {
        scene
            .try_line(Vec2::new(x, minimum_y), Vec2::new(x, maximum_y), 1.0, color)
            .map_err(|error| error.to_string())?;
        x += spacing;
        lines += 1;
    }

    let first_y = (minimum_y / spacing).floor() * spacing;
    let mut y = first_y;
    while y <= maximum_y && lines < 256 {
        scene
            .try_line(Vec2::new(minimum_x, y), Vec2::new(maximum_x, y), 1.0, color)
            .map_err(|error| error.to_string())?;
        y += spacing;
        lines += 1;
    }
    Ok(())
}

pub(super) fn adaptive_grid_spacing(zoom: f32) -> Result<f32, String> {
    if !zoom.is_finite() || zoom <= 0.0 {
        return Err("scientific camera zoom must be finite and positive".to_owned());
    }
    let desired_world = 64.0 / zoom;
    let decade = 10.0_f32.powf(desired_world.log10().floor());
    for multiple in [1.0_f32, 2.0, 5.0, 10.0] {
        let candidate = decade * multiple;
        if candidate >= desired_world && candidate.is_finite() {
            return Ok(candidate);
        }
    }
    Err("adaptive grid spacing is outside the finite display range".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        HEADS_UP_SCENE_BUDGET, OVERLAY_SCENE_BUDGET, SCIENTIFIC_SCENE_BUDGET, UI_SCENE_BUDGET,
        adaptive_grid_spacing,
    };
    use crate::presentation::sim_engine::UI_FRAME_BUDGET;

    #[test]
    fn adaptive_grid_uses_readable_one_two_five_decades() {
        assert_eq!(adaptive_grid_spacing(80.0).expect("spacing"), 1.0);
        assert!((adaptive_grid_spacing(800.0).expect("spacing") - 0.1).abs() < f32::EPSILON);
        assert_eq!(adaptive_grid_spacing(8.0).expect("spacing"), 10.0);
    }

    #[test]
    fn frame_budget_covers_every_independently_bounded_visual_layer() {
        let budgets = [
            UI_SCENE_BUDGET,
            SCIENTIFIC_SCENE_BUDGET,
            HEADS_UP_SCENE_BUDGET,
            OVERLAY_SCENE_BUDGET,
        ];
        assert!(UI_FRAME_BUDGET.max_passes() >= budgets.len());
        assert!(
            UI_FRAME_BUDGET.max_commands()
                >= budgets.iter().map(|budget| budget.max_commands()).sum()
        );
        assert!(
            UI_FRAME_BUDGET.max_vertices()
                >= budgets
                    .iter()
                    .map(|budget| budget.max_tessellated_vertices())
                    .sum()
        );
        assert!(
            UI_FRAME_BUDGET.max_upload_bytes()
                >= budgets.iter().map(|budget| budget.max_upload_bytes()).sum()
        );
        assert!(
            UI_FRAME_BUDGET.max_draw_calls()
                >= budgets.iter().map(|budget| budget.max_draw_batches()).sum()
        );
    }
}
