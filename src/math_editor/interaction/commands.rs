//! User commands for the Math document and presentation controls.
use crate::math_editor::{
    interaction::clipboard,
    state::MathState,
    view::layout::{Layout, Target},
};
impl MathState {
    pub(in crate::math_editor) fn activate(&mut self, target: Target) {
        if self.range_edit.is_some() {
            match target {
                Target::Type(ch) => {
                    if let Some((_, _, text)) = self.range_edit.as_mut()
                        && (ch.is_ascii_digit() || "+-.e".contains(ch))
                    {
                        text.push(ch);
                    }
                    return;
                }
                Target::Backspace => {
                    if let Some((_, _, text)) = self.range_edit.as_mut() {
                        text.pop();
                    }
                    return;
                }
                Target::Enter => {
                    self.commit_range();
                    return;
                }
                _ => {}
            }
        }
        if !matches!(target, Target::Slider(_) | Target::PlayParameter(_)) {
            self.stop_parameter_motion();
        }
        match target {
            Target::TransitionGuard => {}
            Target::SpatialZ => {
                if self.spatial.enabled && self.spatial.blend >= 0.6 {
                    self.activate(Target::Type('z'));
                }
            }
            Target::Spatial => {
                self.cancel_gesture();
                self.spatial.enabled = !self.spatial.enabled;
                self.dirty = true;
                self.focus = None;
            }
            Target::Slider(_) => {}
            Target::PlayParameter(i) => self.toggle_parameter(i),
            Target::CreateParameter(name) => self.add_parameter(name),
            Target::ParameterRange(i, upper) => {
                if let Some(p) = self.parameters.get(i).copied().flatten() {
                    self.range_edit = Some((
                        i,
                        upper,
                        if upper { p.range.high } else { p.range.low }.to_string(),
                    ));
                    self.notice = Some(
                        "Edit range: type a number; Enter applies, Esc cancels. Ctrl+A clears."
                            .into(),
                    );
                }
            }
            Target::Alphabet => {
                self.alphabet = !self.alphabet;
                self.functions = false;
                self.pointer.cancel();
            }
            Target::Copy => self.request_clipboard(clipboard::ClipboardAction::Copy),
            Target::Cut => self.request_clipboard(clipboard::ClipboardAction::Cut),
            Target::Paste => self.request_clipboard(clipboard::ClipboardAction::Paste),
            Target::Add => {
                self.insert_expression(self.document.fields.len());
            }
            Target::Draft => self.focus_draft(),
            Target::Remove(index) => self.remove_expression(index, false),
            Target::Visible(index) => {
                self.document.visible[index] = !self.document.visible[index];
            }
            Target::Keypad => {
                self.keypad = !self.keypad;
                self.functions = false;
                self.dirty = true;
                self.pointer.cancel();
            }
            Target::Enter => {
                if !self.shifts.iter().any(|s| *s)
                    && let Some(i) = self
                        .focus
                        .filter(|i| self.document.fields[*i].is_integral())
                {
                    self.activate(Target::Calculate(i));
                } else {
                    self.next_expression();
                }
            }
            Target::Square => {
                let before = self.document.clone();
                let field = self.focus.unwrap_or(0);
                self.document.fields[field].template('^');
                self.document.fields[field].type_char('2');
                self.document.fields[field].exit_slot();
                self.focus = Some(field);
                self.remember(before);
            }
            Target::Functions => {
                self.functions = !self.functions;
                self.pointer.cancel();
            }
            Target::Type(character) => {
                let before = self.document.clone();
                let field = self.focus.unwrap_or(0);
                self.document.fields[field].type_char(character);
                self.focus = Some(field);
                self.remember(before);
            }
            Target::Function(name) => {
                self.functions = false;
                let before = self.document.clone();
                let field = self.focus.unwrap_or(0);
                self.document.fields[field].insert_function(name);
                self.focus = Some(field);
                self.remember(before);
            }
            Target::Left | Target::Right => {
                let field = self.focus.unwrap_or(0);
                self.document.fields[field]
                    .move_horizontal(target == Target::Right, self.shifts.iter().any(|s| *s));
                self.focus = Some(field);
            }
            Target::Backspace => {
                if self.erase_empty_expression() {
                    return;
                }
                let before = self.document.clone();
                let field = self.focus.unwrap_or(0);
                self.document.fields[field].erase(false);
                self.focus = Some(field);
                self.remember(before);
            }
            Target::ZoomIn | Target::ZoomOut => {
                if let Some(viewport) = self.viewport {
                    let layout = Layout::for_state(viewport, self);
                    self.zoom_view(
                        if target == Target::ZoomIn {
                            1.3
                        } else {
                            1.0 / 1.3
                        },
                        layout.canvas.center(),
                        layout.canvas,
                    );
                    self.dirty = true;
                }
            }
            Target::Field(index) => self.focus = Some(index),
            Target::Fraction
            | Target::Power
            | Target::Root
            | Target::NthRoot
            | Target::Integral
            | Target::Derivative => {
                self.functions = false;
                let before = self.document.clone();
                let field = self.focus.unwrap_or(0);
                self.document.fields[field].template(match target {
                    Target::Fraction => '/',
                    Target::Power => '^',
                    Target::Integral => 'i',
                    Target::Derivative => 'd',
                    Target::NthRoot => 'n',
                    _ => 'r',
                });
                self.focus = Some(field);
                self.remember(before);
            }
            Target::Calculate(index) => {
                let pending = self
                    .plot
                    .as_ref()
                    .and_then(|plot| plot.rows.get(index))
                    .is_some_and(|row| row.pending);
                let enabled = !(pending && self.calculate[index]);
                self.calculate[index] = enabled;
                self.worker.cancel();
                self.busy = false;
                self.dirty = true;
                if let Some(progress) = self.area_progress.get_mut(index) {
                    *progress = 0.0;
                }
                if let Some(row) = self.plot.as_mut().and_then(|p| p.rows.get_mut(index)) {
                    row.scalar = None;
                    row.integral = None;
                    row.segments.clear();
                    row.pending = enabled;
                    row.diagnostic = None;
                }
            }
            Target::Color(color) => {
                if let Some(i) = self.style_popup {
                    self.document.styles[i].color = color;
                }
            }
            Target::Pattern(pattern) => {
                if let Some(i) = self.style_popup {
                    self.document.styles[i].pattern = pattern;
                }
            }
            Target::Thickness(up) => {
                if let Some(i) = self.style_popup {
                    self.document.styles[i].width =
                        (self.document.styles[i].width + if up { 0.5 } else { -0.5 }).max(0.5);
                }
            }
            Target::Opacity(up) => {
                if let Some(i) = self.style_popup {
                    self.document.styles[i].opacity = (self.document.styles[i].opacity
                        + if up { 0.1 } else { -0.1 })
                    .clamp(0.0, 1.0);
                }
            }
            Target::ClosePopup => {
                self.style_popup = None;
                self.functions = false;
                self.pointer.cancel();
            }
            Target::Home => {
                self.reset_view();
            }
            Target::Tool(index) => {
                self.focus = None;
                self.tool = index;
                self.link = None;
                self.drag = None;
                self.status = match index {
                    0 => "Drag a free point; linked constructions follow.",
                    1 => "Click the graph to add a free point.",
                    2 => "Click to add a point linked to f(x).",
                    _ => "Click two existing points to construct the selected object.",
                }
                .into();
            }
            Target::Undo => self.history(false),
            Target::Redo => self.history(true),
            Target::Back => {
                self.confirm_back = true;
                self.focus = None;
                self.cancel_gesture();
            }
            Target::CancelBack => self.confirm_back = false,
            Target::ConfirmBack => {
                self.worker.cancel();
                self.leaving = true;
            }
        }
    }
}
