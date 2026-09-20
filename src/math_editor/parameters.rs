//! Literal scalar definitions remain the source of truth. Sliders edit those
//! rows; their finite, user-editable ranges are controls, not document limits.
use super::{
    formula::Formula,
    state::{Document, MathState},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Range {
    pub low: f64,
    pub high: f64,
}
#[derive(Clone, Copy)]
pub(super) struct Parameter {
    pub name: char,
    pub value: f64,
    pub range: Range,
}
pub(super) struct Playback {
    pub row: usize,
    pub phase: f64,
    pub before: Document,
}

impl MathState {
    pub(super) fn scan_parameters(&mut self) {
        let sources: Vec<_> = self
            .document
            .fields
            .iter()
            .map(|f| f.source().unwrap_or_default())
            .collect();
        let mut counts = BTreeMap::new();
        for source in &sources {
            if let Some((name, _)) = sim_math::statement::definition(source) {
                *counts.entry(name).or_insert(0) += 1;
            }
        }
        self.parameters = sources
            .iter()
            .map(|source| {
                let (name, value) = sim_math::statement::definition(source)?;
                if counts.get(&name) != Some(&1) {
                    return None;
                }
                let value: f64 = value.parse().ok()?;
                if !value.is_finite() {
                    return None;
                }
                let name = name.chars().next()?;
                let extent = value.abs().clamp(10.0, f64::MAX / 4.0);
                let range = self
                    .document
                    .parameter_ranges
                    .get(&name)
                    .copied()
                    .unwrap_or(Range {
                        low: -extent,
                        high: extent,
                    });
                Some(Parameter { name, value, range })
            })
            .collect();
        self.missing_parameters = sources
            .iter()
            .map(|source| {
                let mut missing = BTreeSet::new();
                for part in source.split(['=', '<', '>', '\u{2264}', '\u{2265}']) {
                    if let Ok(tokens) = meval::tokenizer::tokenize(part) {
                        for token in tokens {
                            if let meval::tokenizer::Token::Var(name) = token
                                && name.len() == 1
                                && name.as_bytes()[0].is_ascii_alphabetic()
                                && !matches!(name.as_str(), "x" | "y" | "z" | "e")
                                && !counts.contains_key(&name)
                                && let Some(name) = name.chars().next()
                            {
                                missing.insert(name);
                            }
                        }
                    }
                }
                missing.into_iter().collect()
            })
            .collect();
    }
    pub(super) fn add_parameter(&mut self, name: char) {
        self.stop_parameter_motion();
        let Ok(formula) = Formula::from_text(&format!("{name}=1")) else {
            return;
        };
        let before = self.document.clone();
        self.document.fields.push(formula);
        self.document.visible.push(true);
        self.document
            .styles
            .push(super::graph_style::GraphStyle::new(
                self.document.fields.len() - 1,
            ));
        self.focus = Some(self.document.fields.len() - 1);
        self.remember(before);
    }
    pub(super) fn set_parameter(&mut self, row: usize, value: f64) {
        let Some(parameter) = self.parameters.get(row).copied().flatten() else {
            return;
        };
        if value == parameter.value || !value.is_finite() {
            return;
        }
        if let Ok(formula) = Formula::from_text(&format!("{}={value}", parameter.name)) {
            self.document.fields[row] = formula;
            self.edit_epoch = self.edit_epoch.wrapping_add(1);
            self.parameters[row] = Some(Parameter { value, ..parameter });
            self.layout_dirty[row] = true;
            self.dirty = true;
            self.stale = true;
            self.area_progress.fill(1.0);
        }
    }
    pub(super) fn move_slider(&mut self, row: usize, x: f32, track: super::layout::Rect) {
        let Some(p) = self.parameters.get(row).copied().flatten() else {
            return;
        };
        let t = f64::from(((x - track.x) / track.w).clamp(0.0, 1.0));
        self.set_parameter(row, p.range.low * (1.0 - t) + p.range.high * t);
    }
    pub(super) fn toggle_parameter(&mut self, row: usize) {
        if self.playback.as_ref().is_some_and(|p| p.row == row) {
            self.stop_parameter_motion();
            return;
        }
        self.stop_parameter_motion();
        if let Some(p) = self.parameters.get(row).copied().flatten() {
            let t = ((p.value - p.range.low) / (p.range.high - p.range.low)).clamp(0.0, 1.0);
            self.playback = Some(Playback {
                row,
                phase: (2.0 * t - 1.0).asin(),
                before: self.document.clone(),
            });
            self.focus = None;
        }
    }
    pub(super) fn advance_parameters(&mut self, dt: f64) {
        // Backpressure: finish the current plot before issuing another value.
        // No fabricated intermediate function/measurement is displayed.
        if self.busy || self.dirty || self.confirm_back {
            return;
        }
        if let Some(playback) = self.playback.as_mut() {
            playback.phase = (playback.phase + dt * std::f64::consts::TAU / 8.0)
                .rem_euclid(std::f64::consts::TAU);
            let row = playback.row;
            let t = (playback.phase.sin() + 1.0) * 0.5;
            if let Some(p) = self.parameters[row] {
                self.set_parameter(row, p.range.low * (1.0 - t) + p.range.high * t);
            }
        }
    }
    pub(super) fn stop_parameter_motion(&mut self) {
        let before = self
            .parameter_drag
            .take()
            .map(|(_, d)| d)
            .or_else(|| self.playback.take().map(|p| p.before));
        if let Some(before) = before
            && before != self.document
        {
            self.undo.push(before);
            self.redo.clear();
        }
    }
    pub(super) fn commit_range(&mut self) {
        let Some((row, upper, text)) = self.range_edit.take() else {
            return;
        };
        let Some(p) = self.parameters.get(row).copied().flatten() else {
            return;
        };
        let value = text.parse::<f64>().ok().filter(|v| v.is_finite());
        let Some(value) = value else {
            self.notice = Some("Range endpoint must be a finite number.".into());
            return;
        };
        let range = if upper {
            Range {
                high: value,
                ..p.range
            }
        } else {
            Range {
                low: value,
                ..p.range
            }
        };
        if range.low >= range.high || !(range.high - range.low).is_finite() {
            self.notice =
                Some("Slider range needs finite low < high with a representable span.".into());
            return;
        }
        let before = self.document.clone();
        self.document.parameter_ranges.insert(p.name, range);
        self.remember(before);
    }
}
