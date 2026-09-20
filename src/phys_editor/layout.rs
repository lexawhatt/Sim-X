//! Logical-pixel geometry shared by authoring controls and input routing.

use sim_logic::prelude::*;

use super::state::{Control, EditorState, Mode, Tool};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rect {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl Rect {
    pub(crate) const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub(crate) fn position(self) -> LogicalScreenPosition {
        LogicalScreenPosition::new(self.x, self.y)
    }

    pub(crate) fn size(self) -> LogicalScreenVector {
        LogicalScreenVector::new(self.width, self.height)
    }

    pub(crate) fn contains(self, point: LogicalScreenPosition) -> bool {
        let point = point.to_vec2();
        point.x() >= self.x
            && point.x() < self.x + self.width
            && point.y() >= self.y
            && point.y() < self.y + self.height
    }
}

pub(crate) struct Layout {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) canvas: Rect,
    pub(crate) inspector: Rect,
    pub(crate) palette: Rect,
    pub(crate) modal: Rect,
    pub(crate) environment_open: bool,
    mode: Mode,
}

impl Layout {
    pub(crate) fn new(viewport: LogicalViewport, mode: Mode) -> Self {
        let width = viewport.width();
        let height = viewport.height();
        let editor = mode == Mode::Editor;
        let sidebar = if editor { 272.0 } else { 0.0 };
        let bottom = if editor { 190.0 } else { 94.0 };
        Self {
            width,
            height,
            mode,
            environment_open: false,
            modal: Rect::new((width - 440.0) * 0.5, (height - 400.0) * 0.5, 440.0, 400.0),
            canvas: Rect::new(
                16.0,
                132.0,
                (width - 32.0 - sidebar).max(1.0),
                (height - 132.0 - bottom).max(1.0),
            ),
            inspector: Rect::new(
                (width - 272.0).max(0.0),
                132.0,
                256.0,
                (height - 164.0).max(1.0),
            ),
            palette: Rect::new(
                16.0,
                (height - 178.0).max(0.0),
                (width - 304.0).max(1.0),
                146.0,
            ),
        }
    }

    pub(crate) fn usable(&self) -> bool {
        self.width >= 900.0 && self.height >= 600.0
    }

    pub(crate) fn for_state(viewport: LogicalViewport, state: &EditorState) -> Self {
        let mut layout = Self::new(viewport, state.mode);
        layout.environment_open = state.environment_open;
        layout
    }

    pub(crate) fn control(&self, control: Control) -> Option<Rect> {
        if !self.usable() {
            return None;
        }
        if self.environment_open {
            let panel = self.modal;
            let (row, right) = match control {
                Control::GravityLeft => (0, false),
                Control::GravityRight => (0, true),
                Control::GravityDown => (1, false),
                Control::GravityUp => (1, true),
                Control::DragDown => (2, false),
                Control::DragUp => (2, true),
                Control::EarthGravity => {
                    return Some(Rect::new(panel.x + 24.0, panel.y + 248.0, 140.0, 34.0));
                }
                Control::ZeroGravity => {
                    return Some(Rect::new(panel.x + 176.0, panel.y + 248.0, 150.0, 34.0));
                }
                Control::CloseEnvironment => {
                    return Some(Rect::new(panel.x + 24.0, panel.y + 342.0, 392.0, 34.0));
                }
                _ => return None,
            };
            return Some(Rect::new(
                panel.x + if right { 366.0 } else { 324.0 },
                panel.y + 96.0 + row as f32 * 54.0,
                32.0,
                32.0,
            ));
        }
        if control == Control::Environment {
            return Some(Rect::new(self.width - 350.0, 22.0, 134.0, 42.0));
        }
        if self.mode == Mode::Preview {
            return match control {
                Control::Back => Some(Rect::new(self.width - 208.0, 22.0, 192.0, 42.0)),
                Control::Home => Some(Rect::new(16.0, 88.0, 72.0, 32.0)),
                Control::Pause => Some(Rect::new(16.0, self.height - 76.0, 110.0, 32.0)),
                Control::Slow => Some(Rect::new(134.0, self.height - 76.0, 76.0, 32.0)),
                Control::Normal => Some(Rect::new(218.0, self.height - 76.0, 76.0, 32.0)),
                Control::Fast => Some(Rect::new(302.0, self.height - 76.0, 76.0, 32.0)),
                _ => None,
            };
        }
        let rect = match control {
            Control::Run => Rect::new(self.width - 208.0, 22.0, 192.0, 42.0),
            Control::Back | Control::Pause | Control::Slow | Control::Normal | Control::Fast => {
                return None;
            }
            Control::Tool(Tool::Rod)
            | Control::Tool(Tool::Spring)
            | Control::Pendulum
            | Control::SpringPair => {
                return None;
            }
            Control::Tool(tool) => {
                let index = match tool {
                    Tool::Select => 0,
                    Tool::Build => 1,
                    Tool::Erase => 2,
                    Tool::Rod | Tool::Spring => return None,
                };
                Rect::new(16.0 + index as f32 * 114.0, 88.0, 106.0, 32.0)
            }
            Control::Undo => Rect::new(374.0, 88.0, 64.0, 32.0),
            Control::Redo => Rect::new(446.0, 88.0, 64.0, 32.0),
            Control::Snap => Rect::new(526.0, 88.0, 72.0, 32.0),
            Control::Home => Rect::new(606.0, 88.0, 72.0, 32.0),
            Control::Palette(_) => return None,
            Control::MassDown
            | Control::MassUp
            | Control::SizeDown
            | Control::SizeUp
            | Control::HeightDown
            | Control::HeightUp
            | Control::RestitutionDown
            | Control::RestitutionUp
            | Control::RotateLeft
            | Control::RotateRight => {
                let (row, right) = match control {
                    Control::MassDown => (0, false),
                    Control::MassUp => (0, true),
                    Control::SizeDown => (1, false),
                    Control::SizeUp => (1, true),
                    Control::HeightDown => (2, false),
                    Control::HeightUp => (2, true),
                    Control::RestitutionDown => (3, false),
                    Control::RestitutionUp => (3, true),
                    Control::RotateLeft => (4, false),
                    _ => (4, true),
                };
                let (x, width) = if row == 4 {
                    (
                        self.inspector.x + 114.0 + if right { 64.0 } else { 0.0 },
                        62.0,
                    )
                } else {
                    (
                        self.inspector.x + 156.0 + if right { 38.0 } else { 0.0 },
                        30.0,
                    )
                };
                Rect::new(x, self.inspector.y + 180.0 + row as f32 * 36.0, width, 28.0)
            }
            Control::Fixed => Rect::new(
                self.inspector.x + 16.0,
                self.inspector.y + 140.0,
                224.0,
                28.0,
            ),
            Control::Duplicate => Rect::new(
                self.inspector.x + 16.0,
                self.inspector.y + 364.0,
                108.0,
                34.0,
            ),
            Control::Delete => Rect::new(
                self.inspector.x + 132.0,
                self.inspector.y + 364.0,
                108.0,
                34.0,
            ),
            _ => return None,
        };
        Some(rect)
    }

    pub(crate) fn hit(&self, point: LogicalScreenPosition) -> Option<Control> {
        Control::ALL.into_iter().find(|control| {
            self.control(*control)
                .is_some_and(|rect| rounded_contains(rect, point))
        })
    }
}

fn rounded_contains(rect: Rect, point: LogicalScreenPosition) -> bool {
    if !rect.contains(point) {
        return false;
    }
    let point = point.to_vec2();
    let radius = 6.0_f32.min(rect.width * 0.5).min(rect.height * 0.5);
    let center_x = point
        .x()
        .clamp(rect.x + radius, rect.x + rect.width - radius);
    let center_y = point
        .y()
        .clamp(rect.y + radius, rect.y + rect.height - radius);
    (point.x() - center_x).hypot(point.y() - center_y) <= radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_share_picking_geometry_and_never_overlap_canvas() -> LogicResult {
        for (width, height) in [(900.0, 600.0), (1280.0, 800.0), (1920.0, 1080.0)] {
            for mode in [Mode::Editor, Mode::Preview] {
                let layout = Layout::new(LogicalViewport::new(width, height)?, mode);
                for control in Control::ALL {
                    if let Some(rect) = layout.control(control) {
                        let center = LogicalScreenPosition::new(
                            rect.x + rect.width * 0.5,
                            rect.y + rect.height * 0.5,
                        );
                        assert_eq!(layout.hit(center), Some(control));
                        assert!(!layout.canvas.contains(center));
                        assert!(rect.x >= 0.0 && rect.y >= 0.0);
                        assert!(rect.x + rect.width <= width && rect.y + rect.height <= height);
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn tiny_window_has_no_targets_and_retains_positive_geometry() -> LogicResult {
        let layout = Layout::new(LogicalViewport::new(1.0, 1.0)?, Mode::Editor);
        assert!(!layout.usable());
        assert!(layout.canvas.width > 0.0 && layout.canvas.height > 0.0);
        assert!(
            Control::ALL
                .into_iter()
                .all(|control| layout.control(control).is_none())
        );
        Ok(())
    }

    #[test]
    fn rounded_button_corner_is_not_an_active_target() -> LogicResult {
        let layout = Layout::new(LogicalViewport::new(1280.0, 800.0)?, Mode::Editor);
        let rect = layout.control(Control::Run).unwrap();
        assert_eq!(layout.hit(rect.position()), None);
        assert_eq!(
            layout.hit(LogicalScreenPosition::new(rect.x + 6.0, rect.y + 1.0)),
            Some(Control::Run)
        );
        Ok(())
    }
}
