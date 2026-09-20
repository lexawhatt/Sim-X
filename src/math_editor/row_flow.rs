//! Expression-row lifecycle and cross-row caret navigation.
use super::{
    formula::{Caret, Formula},
    formula_layout::FormulaLayout,
    state::MathState,
};

impl MathState {
    pub(super) fn focus_expression(&mut self, index: usize, at_end: bool) {
        self.focus = Some(index);
        self.document.fields[index].caret = Caret {
            row: 0,
            index: if at_end {
                self.document.fields[index].rows[0].len()
            } else {
                0
            },
        };
        self.document.fields[index].anchor = None;
        self.document.fields[index].select_all = false;
        self.blink = 0.0;
    }

    pub(super) fn insert_expression(&mut self, index: usize) {
        let before = self.document.clone();
        self.document.fields.insert(index, Formula::default());
        self.document.visible.insert(index, true);
        self.document.styles.insert(
            index,
            super::graph_style::GraphStyle::new(self.document.fields.len() - 1),
        );
        self.calculate.insert(index, false);
        self.layouts.insert(index, FormulaLayout::default());
        self.layout_dirty.insert(index, true);
        self.offsets.insert(index, [0.0; 2]);
        self.focus_expression(index, false);
        self.remember(before);
    }

    pub(super) fn focus_draft(&mut self) {
        let last = self.document.fields.len() - 1;
        if self.document.fields[last].rows[0].is_empty() {
            self.focus_expression(last, false);
        } else {
            self.insert_expression(last + 1);
        }
    }

    pub(super) fn next_expression(&mut self) {
        let Some(index) = self.focus else {
            self.focus_draft();
            return;
        };
        // Enter on an empty draft does not manufacture more empty work.
        if self.document.fields[index].rows[0].is_empty() {
            self.focus_expression(index, false);
        } else if self
            .document
            .fields
            .get(index + 1)
            .is_some_and(|f| f.rows[0].is_empty())
        {
            self.focus_expression(index + 1, false);
        } else {
            self.insert_expression(index + 1);
        }
    }

    pub(super) fn remove_expression(&mut self, index: usize, previous: bool) {
        let before = self.document.clone();
        if self.document.fields.len() == 1 {
            self.document.fields[0] = Formula::default();
            self.calculate[0] = false;
            self.focus_expression(0, false);
        } else {
            self.document.fields.remove(index);
            self.document.visible.remove(index);
            self.document.styles.remove(index);
            self.calculate.remove(index);
            self.layouts.remove(index);
            self.layout_dirty.remove(index);
            self.offsets.remove(index);
            let focus = if previous {
                index.saturating_sub(1)
            } else {
                index.min(self.document.fields.len() - 1)
            };
            self.focus_expression(focus, previous && index > 0);
        }
        self.remember(before);
    }

    pub(super) fn erase_empty_expression(&mut self) -> bool {
        let Some(index) = self.focus else {
            return false;
        };
        if !self.document.fields[index].rows[0].is_empty() {
            return false;
        }
        self.remove_expression(index, true);
        true
    }

    pub(super) fn adjacent_expression(&mut self, up: bool) {
        let Some(index) = self.focus else {
            return;
        };
        let next = if up {
            index.checked_sub(1)
        } else {
            (index + 1 < self.document.fields.len()).then_some(index + 1)
        };
        let Some(next) = next else {
            return;
        };
        let x = self.layouts[index]
            .stop(self.document.fields[index].caret)
            .map_or(0.0, |s| s.x);
        // Preserve the perceived horizontal column, but never enter an unrelated
        // fraction or exponent just because its glyph happens to be close.
        let caret = self.layouts[next]
            .stops
            .iter()
            .filter(|s| s.caret.row == 0)
            .min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs()))
            .map(|s| s.caret);
        self.focus_expression(next, false);
        if let Some(caret) = caret {
            self.document.fields[next].caret = caret;
        }
    }
}
