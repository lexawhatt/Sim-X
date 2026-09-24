use crate::math_editor::{
    compute::{Plot, Request, Worker},
    formula::{self, Formula, layout::FormulaLayout},
    interaction::{clipboard, editing, parameters},
    scene::{self, style},
    view::{
        layout::{Camera, Layout, Target},
        motion,
    },
};
use sim_logic::prelude::*;
use sim_math::geometry::{Construction, Geometry, Point, PointId, Shape};

#[derive(Clone, PartialEq)]
pub(super) struct Document {
    pub(super) fields: Vec<Formula>,
    pub(super) visible: Vec<bool>,
    pub(super) styles: Vec<style::GraphStyle>,
    pub(super) geometry: Geometry,
    pub(super) parameter_ranges: std::collections::BTreeMap<char, parameters::Range>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            fields: vec![Formula::default()],
            visible: vec![true],
            styles: vec![style::GraphStyle::new(0)],
            geometry: Geometry::default(),
            parameter_ranges: Default::default(),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Drag {
    pub(super) id: PointId,
    pub(super) candidate: Point,
}
#[derive(Resource)]
pub(crate) struct MathState {
    pub(super) document: Document,
    pub(super) undo: Vec<Document>,
    pub(super) redo: Vec<Document>,
    pub(super) focus: Option<usize>,
    pub(super) editing: editing::EditingSession,
    pub(super) layouts: Vec<FormulaLayout>,
    pub(super) layout_dirty: Vec<bool>,
    pub(super) offsets: Vec<[f32; 2]>,
    pub(super) formula_drag: Option<(usize, formula::Caret, [f32; 2])>,
    pub(super) parameters: Vec<Option<parameters::Parameter>>,
    pub(super) missing_parameters: Vec<Vec<char>>,
    pub(super) parameter_drag: Option<(usize, Document)>,
    pub(super) playback: Option<parameters::Playback>,
    pub(super) range_edit: Option<(usize, bool, String)>,
    pub(super) sidebar_scroll: f32,
    pub(super) keypad: bool,
    pub(super) alphabet: bool,
    pub(crate) clipboard_pending: Option<clipboard::ClipboardRequest>,
    pub(super) clipboard_local: Option<(String, Formula)>,
    pub(super) notice: Option<String>,
    pub(super) functions: bool,
    pub(super) catalog_scroll: f32,
    pub(super) style_popup: Option<usize>,
    pub(super) icon_hold: Option<(usize, f32, [f32; 2])>,
    pub(super) suppress_release: bool,
    pub(super) camera: Camera,
    pub(super) camera_target: Option<Camera>,
    pub(super) spatial: scene::Spatial,
    pub(super) worker: Worker,
    pub(super) plot: Option<Plot>,
    pub(super) stale: bool,
    pub(super) dirty: bool,
    pub(super) edit_epoch: u64,
    pub(super) calculate: Vec<bool>,
    pub(super) busy: bool,
    pub(super) status: String,
    pub(super) area_progress: Vec<f32>,
    pub(super) pointer: PointerButton<Target>,
    pub(super) shifts: [bool; 2],
    pub(super) controls: [bool; 2],
    pub(super) viewport: Option<LogicalViewport>,
    pub(super) hovered: Option<Target>,
    pub(super) blink: f32,
    pub(super) motion: motion::Motion,
    pub(super) tool: usize,
    pub(super) link: Option<PointId>,
    pub(super) drag: Option<Drag>,
    pub(super) canvas_pressed: bool,
    pub(super) pan: Option<([f32; 2], Camera)>,
    pub(super) confirm_back: bool,
    pub(super) leaving: bool,
    pub(super) fullscreen: bool,
    pub(super) reduced_motion: bool,
}
impl MathState {
    pub(super) fn new() -> std::io::Result<Self> {
        Ok(Self {
            document: Document::default(),
            undo: vec![],
            redo: vec![],
            focus: Some(0),
            editing: Default::default(),
            layouts: vec![FormulaLayout::default()],
            layout_dirty: vec![true],
            offsets: vec![[0.0; 2]],
            formula_drag: None,
            parameters: vec![None],
            missing_parameters: vec![vec![]],
            parameter_drag: None,
            playback: None,
            range_edit: None,
            sidebar_scroll: 0.0,
            keypad: false,
            alphabet: false,
            clipboard_pending: None,
            clipboard_local: None,
            notice: None,
            functions: false,
            catalog_scroll: 0.0,
            style_popup: None,
            icon_hold: None,
            suppress_release: false,
            camera: Camera::default(),
            camera_target: None,
            spatial: Default::default(),
            worker: Worker::new()?,
            plot: None,
            stale: true,
            dirty: true,
            edit_epoch: 0,
            calculate: vec![false],
            busy: false,
            status: "Type normally. Use arrows to leave a fraction or exponent.".into(),
            area_progress: vec![0.0],
            pointer: PointerButton::new(MouseButton::Left),
            shifts: [false; 2],
            controls: [false; 2],
            viewport: None,
            hovered: None,
            blink: 0.0,
            motion: Default::default(),
            tool: 0,
            link: None,
            drag: None,
            canvas_pressed: false,
            pan: None,
            confirm_back: false,
            leaving: false,
            fullscreen: true,
            reduced_motion: false,
        })
    }
    pub(super) fn remember(&mut self, before: Document) {
        if before != self.document {
            if !self.editing.coalesce(&before, &self.document, self.focus) {
                self.undo.push(before);
            }
            self.redo.clear();
            self.changed();
        }
    }
    pub(super) fn changed(&mut self) {
        self.notice = None;
        self.edit_epoch = self.edit_epoch.wrapping_add(1);
        self.scan_parameters();
        self.layouts
            .resize_with(self.document.fields.len(), FormulaLayout::default);
        self.offsets.resize(self.document.fields.len(), [0.0; 2]);
        self.layout_dirty.resize(self.document.fields.len(), true);
        self.layout_dirty.fill(true);
        self.calculate.resize(self.document.fields.len(), false);
        self.style_popup = None;
        self.icon_hold = None;
        if self
            .plot
            .as_ref()
            .is_some_and(|p| p.rows.len() != self.document.fields.len())
        {
            self.plot = None;
        }
        self.worker.cancel();
        self.dirty = true;
        self.stale = true;
        self.area_progress.resize(self.document.fields.len(), 0.0);
        self.area_progress.fill(0.0);
        self.blink = 0.0;
    }
    pub(super) fn history(&mut self, redo: bool) {
        self.editing.reset();
        self.stop_parameter_motion();
        let candidate = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if let Some(document) = candidate {
            let old = std::mem::replace(&mut self.document, document);
            if redo {
                self.undo.push(old)
            } else {
                self.redo.push(old)
            }
            self.link = None;
            self.drag = None;
            self.offsets = vec![[0.0; 2]; self.document.fields.len()];
            self.focus = self.focus.map(|i| i.min(self.document.fields.len() - 1));
            // Calculate is an explicit action, not a row index carried across
            // structural undo/redo into a different expression.
            self.calculate = vec![false; self.document.fields.len()];
            self.changed();
        }
    }
    pub(super) fn cancel_gesture(&mut self) {
        self.editing.reset();
        self.stop_parameter_motion();
        self.range_edit = None;
        self.formula_drag = None;
        self.pointer.cancel();
        self.icon_hold = None;
        self.suppress_release = false;
        self.drag = None;
        self.canvas_pressed = false;
        self.pan = None;
        self.spatial.drag = None;
    }
    pub(super) fn submit(&mut self, layout: &Layout) {
        if !self.dirty || !layout.usable() {
            return;
        }
        self.dirty = false;
        self.worker.cancel();
        self.busy = false;
        let sources = self
            .document
            .fields
            .iter()
            .map(|f| {
                if f.rows[0].is_empty() {
                    Ok(String::new())
                } else {
                    f.source().map_err(str::to_owned)
                }
            })
            .collect();
        let sample_camera = self.camera_target.unwrap_or(self.camera);
        let Some(left) = sample_camera.unproject(
            [layout.canvas.x, layout.canvas.y + layout.canvas.h],
            layout.canvas,
        ) else {
            return;
        };
        let Some(right) = sample_camera.unproject(
            [layout.canvas.x + layout.canvas.w, layout.canvas.y],
            layout.canvas,
        ) else {
            return;
        };
        self.worker.submit(Request {
            sources,
            calculate: self.calculate.clone(),
            lower: left,
            upper: right,
            pixels: [layout.canvas.w, layout.canvas.h],
            spatial: self.spatial.visible(),
        });
        self.busy = true;
        self.status = "Updating expressions...".into();
    }
    pub(super) fn poll(&mut self) {
        if let Some(plot) = self.worker.poll() {
            self.busy = plot.rows.iter().any(|r| r.pending);
            self.stale = false;
            self.status = if self.busy {
                "Calculating... Click Calculate again to cancel."
            } else {
                "Click a color to hide. Hold it for appearance. Radians."
            }
            .into();
            self.plot = Some(plot);
        }
    }
    pub(super) fn resolved(&self) -> Vec<Option<Point>> {
        let expression = self
            .plot
            .as_ref()
            .and_then(|plot| plot.rows.iter().find_map(|row| row.expression.as_ref()));
        let evaluate = |x| expression.and_then(|f| f.evaluate_at(x, 0.0).ok());
        if let Some(drag) = self.drag {
            let mut geometry = self.document.geometry.clone();
            let _ = geometry.move_free(drag.id, drag.candidate);
            geometry.resolve_with(evaluate)
        } else {
            self.document.geometry.resolve_with(evaluate)
        }
    }
    pub(super) fn pick(&self, position: Point) -> Option<PointId> {
        let threshold = 10.0 / self.camera.scale;
        self.resolved().iter().enumerate().rev().find_map(|(i, p)| {
            p.filter(|p| (p.x - position.x).hypot(p.y - position.y) <= threshold)
                .and_then(|_| self.document.geometry.point_id(i))
        })
    }
    pub(super) fn begin_canvas(&mut self, p: Point) {
        self.focus = None;
        self.canvas_pressed = true;
        if self.tool == 0
            && let Some(id) = self.pick(p)
            && matches!(
                self.document.geometry.points()[id.index()],
                Construction::Free(_)
            )
        {
            self.drag = Some(Drag { id, candidate: p });
        }
    }
    pub(super) fn finish_canvas(&mut self, p: Point) {
        if !self.canvas_pressed {
            return;
        }
        self.canvas_pressed = false;
        let before = self.document.clone();
        let result = if let Some(drag) = self.drag.take() {
            self.document.geometry.move_free(drag.id, p)
        } else {
            match self.tool {
                1 => self.document.geometry.add_free(p).map(|_| ()),
                2 => self.document.geometry.add_on_function(p.x).map(|_| ()),
                3..=5 => {
                    if let Some(id) = self.pick(p) {
                        if let Some(first) = self.link.take() {
                            match self.tool {
                                3 => self.document.geometry.add_shape(Shape::Segment(first, id)),
                                4 => self.document.geometry.add_midpoint(first, id).map(|_| ()),
                                _ => self.document.geometry.add_shape(Shape::Circle(first, id)),
                            }
                        } else {
                            self.link = Some(id);
                            Ok(())
                        }
                    } else {
                        self.status = "Choose an existing point.".into();
                        Ok(())
                    }
                }
                _ => Ok(()),
            }
        };
        if let Err(error) = result {
            self.status = error.to_string();
        }
        // Geometry-only changes do not re-integrate an unchanged function.
        if before != self.document {
            self.undo.push(before);
            self.redo.clear();
        }
    }
}
