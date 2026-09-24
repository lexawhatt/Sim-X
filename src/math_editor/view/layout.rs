//! Math-only logical-pixel layout and scientific camera mapping.
use crate::math_editor::{state, view::keyboard};
use sim_logic::prelude::*;
use sim_math::geometry::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::math_editor) struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, p: [f32; 2]) -> bool {
        p[0] >= self.x && p[0] < self.x + self.w && p[1] >= self.y && p[1] < self.y + self.h
    }
    pub fn clip(self) -> LogicResult<ScreenClip> {
        Ok(ScreenClip::new(
            LogicalScreenPosition::new(self.x, self.y),
            LogicalScreenVector::new(self.w, self.h),
        )?)
    }
    pub fn center(self) -> [f32; 2] {
        [self.x + self.w * 0.5, self.y + self.h * 0.5]
    }
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let w = (self.x + self.w).min(other.x + other.w) - x;
        let h = (self.y + self.h).min(other.y + other.h) - y;
        (w > 0.0 && h > 0.0).then_some(Self::new(x, y, w, h))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::math_editor) enum Target {
    Add,
    Draft,
    Remove(usize),
    Visible(usize),
    Keypad,
    Alphabet,
    Copy,
    Cut,
    Paste,
    Slider(usize),
    PlayParameter(usize),
    ParameterRange(usize, bool),
    CreateParameter(char),
    Functions,
    Type(char),
    Function(&'static str),
    Left,
    Right,
    Backspace,
    Enter,
    Square,
    ZoomIn,
    ZoomOut,
    Spatial,
    SpatialZ,
    Field(usize),
    Fraction,
    Power,
    Root,
    NthRoot,
    Integral,
    Derivative,
    Calculate(usize),
    Home,
    Tool(usize),
    Undo,
    Redo,
    Back,
    ConfirmBack,
    CancelBack,
    Color(usize),
    Pattern(usize),
    Thickness(bool),
    Opacity(bool),
    ClosePopup,
    TransitionGuard,
}
pub(in crate::math_editor) struct Layout {
    pub width: f32,
    pub height: f32,
    pub canvas: Rect,
    pub fields: Vec<Rect>,
    pub rows: Vec<Rect>,
    pub draft: Option<Rect>,
    pub list: Rect,
    pub sidebar: f32,
    pub keypad_top: f32,
    pub max_scroll: f32,
    pub controls: Vec<(Target, Rect, &'static str)>,
    pub popup: Option<Rect>,
    pub popup_clip: Rect,
    pub popup_controls: Vec<(Target, Rect, &'static str)>,
    pub headings: Vec<(&'static str, [f32; 2])>,
    pub catalog_max: f32,
    spatial_z_enabled: bool,
    geometry_tools_enabled: bool,
    spatial_blend: f32,
}
impl Layout {
    pub fn for_state(viewport: LogicalViewport, state: &state::MathState) -> Self {
        let width = viewport.width();
        let height = viewport.height();
        let sidebar = (width * 0.30).clamp(360.0, 470.0);
        let keypad_top = height - if state.keypad { 204.0 } else { 28.0 };
        let canvas = Rect::new(
            sidebar,
            66.0,
            (width - sidebar).max(1.0),
            (keypad_top - 66.0).max(1.0),
        );
        let list = Rect::new(0.0, 112.0, sidebar, (keypad_top - 156.0).max(1.0));
        let key_start = (width - 742.0) * 0.5;
        let mut function_button = if state.keypad {
            Rect::new(key_start + 604.0, keypad_top + 12.0, 138.0, 34.0)
        } else {
            Rect::new(120.0, keypad_top - 38.0, 110.0, 32.0)
        };
        let heights: Vec<f32> = state
            .document
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let formula = &state.layouts[i];
                let natural = (formula.above + formula.below + 24.0).max(58.0);
                // The viewport, not nesting depth, determines visible row height.
                let field = natural.min((list.h - 64.0).max(58.0));
                field
                    + if f.is_integral() { 88.0 } else { 30.0 }
                    + parameter_extra(state, i, sidebar)
            })
            .collect();
        let needs_draft = state
            .document
            .fields
            .last()
            .is_some_and(|f| !f.rows[0].is_empty());
        let content: f32 = heights.iter().sum::<f32>() + if needs_draft { 72.0 } else { 0.0 };
        let max_scroll = (content + 12.0 - list.h).max(0.0);
        let mut y = list.y - state.sidebar_scroll.clamp(0.0, max_scroll);
        let mut rows = vec![];
        let mut fields = vec![];
        let mut controls = vec![
            (
                Target::Spatial,
                Rect::new(width - 222.0, 18.0, 96.0, 36.0),
                "2D / 3D",
            ),
            (
                Target::Back,
                Rect::new(width - 116.0, 18.0, 96.0, 36.0),
                "Menu",
            ),
            (
                Target::Add,
                Rect::new(12.0, 74.0, 104.0, 30.0),
                "+ Expression",
            ),
            (Target::Undo, Rect::new(130.0, 74.0, 64.0, 30.0), "Undo"),
            (Target::Redo, Rect::new(202.0, 74.0, 64.0, 30.0), "Redo"),
            (
                Target::Home,
                Rect::new(width - 68.0, 192.0, 52.0, 32.0),
                "Home",
            ),
            (
                Target::ZoomIn,
                Rect::new(width - 52.0, 110.0, 36.0, 32.0),
                "+",
            ),
            (
                Target::ZoomOut,
                Rect::new(width - 52.0, 150.0, 36.0, 32.0),
                "-",
            ),
            (
                Target::Keypad,
                Rect::new(12.0, keypad_top - 38.0, 100.0, 32.0),
                if state.keypad {
                    "Hide keys"
                } else {
                    "Keyboard"
                },
            ),
            (Target::Functions, function_button, "Functions"),
        ];
        for (i, h) in heights.into_iter().enumerate() {
            rows.push(Rect::new(0.0, y, sidebar, h));
            let integral = state.document.fields[i].is_integral();
            fields.push(Rect::new(
                45.0,
                y + 8.0,
                sidebar - 84.0,
                h - if integral { 88.0 } else { 30.0 } - parameter_extra(state, i, sidebar),
            ));
            controls.push((Target::Visible(i), Rect::new(4.0, y + 25.0, 32.0, 32.0), ""));
            controls.push((
                Target::Remove(i),
                Rect::new(sidebar - 32.0, y + 8.0, 26.0, 26.0),
                "x",
            ));
            if integral {
                controls.push((
                    Target::Calculate(i),
                    Rect::new(
                        52.0,
                        y + h - parameter_extra(state, i, sidebar) - 81.0,
                        110.0,
                        27.0,
                    ),
                    "Calculate",
                ));
            }
            if state.parameters.get(i).is_some_and(Option::is_some) {
                let bottom = y + h - 42.0;
                controls.extend([
                    (
                        Target::PlayParameter(i),
                        Rect::new(5.0, bottom, 44.0, 28.0),
                        if state.playback.as_ref().is_some_and(|p| p.row == i) {
                            "Pause"
                        } else {
                            "Play"
                        },
                    ),
                    (
                        Target::ParameterRange(i, false),
                        Rect::new(54.0, bottom, 62.0, 28.0),
                        "",
                    ),
                    (
                        Target::Slider(i),
                        Rect::new(122.0, bottom, sidebar - 224.0, 28.0),
                        "",
                    ),
                    (
                        Target::ParameterRange(i, true),
                        Rect::new(sidebar - 94.0, bottom, 62.0, 28.0),
                        "",
                    ),
                ]);
            }
            let columns = ((sidebar - 62.0) / 78.0).floor() as usize;
            if let Some(missing) = state.missing_parameters.get(i) {
                for (j, name) in missing.iter().copied().enumerate() {
                    let label = [
                        "Add a", "Add b", "Add c", "Add d", "Add e", "Add f", "Add g", "Add h",
                        "Add i", "Add j", "Add k", "Add l", "Add m", "Add n", "Add o", "Add p",
                        "Add q", "Add r", "Add s", "Add t", "Add u", "Add v", "Add w", "Add x",
                        "Add y", "Add z",
                    ][name as usize - 'a' as usize];
                    controls.push((
                        Target::CreateParameter(name),
                        Rect::new(
                            52.0 + (j % columns) as f32 * 78.0,
                            y + h - parameter_extra(state, i, sidebar)
                                + (j / columns) as f32 * 32.0,
                            72.0,
                            26.0,
                        ),
                        label,
                    ));
                }
            }
            y += h;
        }
        let draft = needs_draft.then_some(Rect::new(0.0, y, sidebar, 72.0));
        for (i, name) in ["Move", "Point", "On f", "Segment", "Midpoint", "Circle"]
            .into_iter()
            .enumerate()
        {
            if state.spatial.blend < 1.0 {
                controls.push((
                    Target::Tool(i),
                    Rect::new(
                        canvas.x + 16.0 + i as f32 * 72.0,
                        78.0 - state.spatial.blend as f32 * 8.0,
                        66.0,
                        30.0,
                    ),
                    name,
                ));
            }
        }
        if state.keypad && state.alphabet {
            keyboard::alphabet(&mut controls, key_start, keypad_top);
        } else if state.keypad {
            if state.spatial.visible() {
                controls.push((
                    Target::SpatialZ,
                    Rect::new(
                        key_start - 70.0,
                        keypad_top + 12.0 + (1.0 - state.spatial.blend as f32) * 8.0,
                        64.0,
                        34.0,
                    ),
                    "z",
                ));
            }
            keyboard::numeric(&mut controls, key_start, keypad_top);
        }
        if let Some((_, rect, _)) = controls.iter().find(|(t, _, _)| *t == Target::Functions) {
            function_button = *rect;
        }
        let mut popup = None;
        let mut popup_controls = vec![];
        let mut headings = vec![];
        let mut popup_clip = list;
        let mut catalog_max = 0.0;
        if state.functions {
            let bottom = function_button.y - 10.0;
            let h = (bottom - 118.0).clamp(100.0, 520.0);
            let w = 340.0;
            let x = (function_button.x + function_button.w - w)
                .clamp(10.0, (width - w - 10.0).max(10.0));
            let r = Rect::new(x, bottom - h, w, h);
            popup = Some(r);
            popup_clip = Rect::new(r.x + 8.0, r.y + 44.0, r.w - 16.0, r.h - 52.0);
            popup_controls.push((
                Target::ClosePopup,
                Rect::new(r.x + r.w - 34.0, r.y + 8.0, 26.0, 26.0),
                "x",
            ));
            let cell = (popup_clip.w - 12.0) / 3.0;
            let mut cursor = 0.0;
            let mut group = "";
            let mut col = 0;
            for function in sim_math::functions::FUNCTIONS {
                if group != function.group {
                    if col > 0 {
                        cursor += 38.0;
                    }
                    cursor += 30.0;
                    headings.push((
                        function.group,
                        [
                            popup_clip.x + 3.0,
                            popup_clip.y + cursor - state.catalog_scroll,
                        ],
                    ));
                    cursor += 10.0;
                    group = function.group;
                    col = 0;
                }
                popup_controls.push((
                    Target::Function(function.name),
                    Rect::new(
                        popup_clip.x + col as f32 * cell,
                        popup_clip.y + cursor - state.catalog_scroll,
                        cell - 5.0,
                        32.0,
                    ),
                    function.name,
                ));
                col += 1;
                if col == 3 {
                    col = 0;
                    cursor += 38.0;
                }
            }
            if col > 0 {
                cursor += 38.0;
            }
            cursor += 30.0;
            headings.push((
                "Calculus",
                [
                    popup_clip.x + 3.0,
                    popup_clip.y + cursor - state.catalog_scroll,
                ],
            ));
            cursor += 10.0;
            popup_controls.push((
                Target::Integral,
                Rect::new(
                    popup_clip.x,
                    popup_clip.y + cursor - state.catalog_scroll,
                    cell - 5.0,
                    36.0,
                ),
                "Integral",
            ));
            popup_controls.push((
                Target::Derivative,
                Rect::new(
                    popup_clip.x + cell,
                    popup_clip.y + cursor - state.catalog_scroll,
                    cell - 5.0,
                    36.0,
                ),
                "d/dx",
            ));
            cursor += 48.0;
            catalog_max = (cursor - popup_clip.h).max(0.0);
        } else if let Some(i) = state.style_popup {
            let r = Rect::new(
                38.0,
                (rows[i].y + 20.0).clamp(118.0, (height - 320.0).max(118.0)),
                (sidebar - 52.0).min(310.0),
                236.0,
            );
            popup = Some(r);
            popup_clip = r;
            popup_controls.push((
                Target::ClosePopup,
                Rect::new(r.x + r.w - 34.0, r.y + 8.0, 26.0, 26.0),
                "x",
            ));
            for (index, label) in ["Solid", "Dashed", "Dotted"].into_iter().enumerate() {
                popup_controls.push((
                    Target::Pattern(index),
                    Rect::new(r.x + 12.0 + index as f32 * 90.0, r.y + 45.0, 84.0, 30.0),
                    label,
                ));
            }
            for (t, x, y, label) in [
                (Target::Thickness(false), 170.0, 87.0, "-"),
                (Target::Thickness(true), 212.0, 87.0, "+"),
                (Target::Opacity(false), 170.0, 127.0, "-"),
                (Target::Opacity(true), 212.0, 127.0, "+"),
            ] {
                popup_controls.push((t, Rect::new(r.x + x, r.y + y, 34.0, 30.0), label));
            }
            for index in 0..6 {
                popup_controls.push((
                    Target::Color(index),
                    Rect::new(r.x + 12.0 + index as f32 * 44.0, r.y + 181.0, 36.0, 36.0),
                    "",
                ));
            }
        }
        if state.confirm_back {
            popup = None;
            popup_controls.clear();
            controls = vec![
                (
                    Target::ConfirmBack,
                    Rect::new(width * 0.5 - 180.0, height * 0.5 + 26.0, 170.0, 38.0),
                    "Leave workspace",
                ),
                (
                    Target::CancelBack,
                    Rect::new(width * 0.5 + 10.0, height * 0.5 + 26.0, 170.0, 38.0),
                    "Keep working",
                ),
            ];
        }
        Self {
            width,
            height,
            canvas,
            fields,
            rows,
            draft,
            list,
            sidebar,
            keypad_top,
            max_scroll,
            controls,
            popup,
            popup_clip,
            popup_controls,
            headings,
            catalog_max,
            spatial_z_enabled: state.spatial.enabled && state.spatial.blend >= 0.6,
            geometry_tools_enabled: !state.spatial.enabled && state.spatial.blend == 0.0,
            spatial_blend: state.spatial.blend as f32,
        }
    }
    pub fn usable(&self) -> bool {
        self.width >= 900.0 && self.height >= 640.0
    }
    pub fn hit(&self, p: [f32; 2], confirm: bool) -> Option<Target> {
        if !self.usable() {
            return None;
        }
        if let Some(popup) = self.popup {
            return self
                .popup_controls
                .iter()
                .find(|(t, r, _)| {
                    r.contains(p)
                        && (matches!(t, Target::ClosePopup) || self.popup_clip.contains(p))
                })
                .map(|(t, _, _)| *t)
                .or_else(|| (!popup.contains(p)).then_some(Target::ClosePopup));
        }
        if !confirm {
            for (i, r) in self.fields.iter().enumerate() {
                if r.contains(p) && self.list.contains(p) {
                    return Some(Target::Field(i));
                }
            }
        }
        self.controls
            .iter()
            .find(|(t, r, _)| r.contains(p) && (!Self::in_list(*t) || self.list.contains(p)))
            // A fading control owns its rectangle even while disabled. Returning
            // no target here would turn the same press into a canvas gesture.
            .map(|(t, _, _)| {
                if self.control_enabled(*t) {
                    *t
                } else {
                    Target::TransitionGuard
                }
            })
            .or_else(|| {
                (!confirm
                    && self.list.contains(p)
                    && self.rows.last().is_none_or(|row| p[1] >= row.y + row.h))
                .then_some(Target::Draft)
            })
    }
    pub fn control_enabled(&self, target: Target) -> bool {
        match target {
            Target::Tool(_) => self.geometry_tools_enabled,
            Target::SpatialZ => self.spatial_z_enabled,
            Target::TransitionGuard => false,
            _ => true,
        }
    }
    pub fn control_opacity(&self, target: Target) -> f32 {
        match target {
            Target::Tool(_) => 1.0 - self.spatial_blend,
            Target::SpatialZ => self.spatial_blend,
            _ => 1.0,
        }
    }
    pub fn in_list(target: Target) -> bool {
        matches!(
            target,
            Target::Visible(_)
                | Target::Remove(_)
                | Target::Calculate(_)
                | Target::Slider(_)
                | Target::PlayParameter(_)
                | Target::ParameterRange(..)
                | Target::CreateParameter(_)
        )
    }
}
pub(in crate::math_editor) fn parameter_extra(
    state: &state::MathState,
    i: usize,
    sidebar: f32,
) -> f32 {
    if state.parameters.get(i).is_some_and(Option::is_some) {
        40.0
    } else {
        let columns = ((sidebar - 62.0) / 78.0).floor() as usize;
        state
            .missing_parameters
            .get(i)
            .map_or(0.0, |p| p.len().div_ceil(columns) as f32 * 32.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::math_editor) struct Camera {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 55.0,
        }
    }
}
impl Camera {
    pub fn project(self, p: Point, rect: Rect) -> Option<[f32; 2]> {
        let c = rect.center();
        let x = f64::from(c[0]) + (p.x - self.x) * self.scale;
        let y = f64::from(c[1]) - (p.y - self.y) * self.scale;
        (x.is_finite()
            && y.is_finite()
            && x.abs() < f64::from(f32::MAX)
            && y.abs() < f64::from(f32::MAX))
        .then_some([x as f32, y as f32])
    }
    pub fn unproject(self, p: [f32; 2], rect: Rect) -> Option<Point> {
        let c = rect.center();
        Point::new(
            self.x + f64::from(p[0] - c[0]) / self.scale,
            self.y - f64::from(p[1] - c[1]) / self.scale,
        )
        .ok()
    }
    pub fn zoom(&mut self, factor: f64, p: [f32; 2], rect: Rect) {
        let Some(before) = self.unproject(p, rect) else {
            return;
        };
        let scale = self.scale * factor;
        if !scale.is_finite() || scale <= 0.0 {
            return;
        }
        let c = rect.center();
        let x = before.x - f64::from(p[0] - c[0]) / scale;
        let y = before.y + f64::from(p[1] - c[1]) / scale;
        // Reject only numeric loss of coordinate resolution, not a zoom quota.
        if x.is_finite() && y.is_finite() && x + 1.0 / scale != x && y + 1.0 / scale != y {
            *self = Self { x, y, scale };
        }
    }
}
