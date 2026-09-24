//! Expression-row lifecycle and cross-row caret navigation.
use crate::math_editor::{
    formula::{Caret, Formula, layout::FormulaLayout},
    scene::style,
    state::MathState,
};

impl MathState {
    pub(in crate::math_editor) fn focus_expression(&mut self, index: usize, at_end: bool) {
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

    pub(in crate::math_editor) fn insert_expression(&mut self, index: usize) {
        let before = self.document.clone();
        self.insert_formula(index, Formula::default());
        self.focus_expression(index, false);
        self.remember(before);
    }

    /// Insert a row and its aligned presentation state. The caller commits the
    /// whole operation once, so a batch never publishes a partial document.
    pub(super) fn insert_formula(&mut self, index: usize, formula: Formula) {
        self.document.fields.insert(index, formula);
        self.document.visible.insert(index, true);
        self.document.styles.insert(
            index,
            style::GraphStyle::new(self.document.fields.len() - 1),
        );
        self.calculate.insert(index, false);
        self.layouts.insert(index, FormulaLayout::default());
        self.layout_dirty.insert(index, true);
        self.offsets.insert(index, [0.0; 2]);
    }

    pub(super) fn paste_expressions(&mut self, row: usize, formulas: Vec<Formula>) {
        let count = formulas.len();
        let current = &self.document.fields[row];
        let replace = current.rows[0].is_empty()
            || current.selection() == Some((0, 0, current.rows[0].len()));
        let before = self.document.clone();
        let start = row + usize::from(!replace);
        for (offset, formula) in formulas.into_iter().enumerate() {
            let index = start + offset;
            if replace && offset == 0 {
                self.document.fields[index] = formula;
                self.calculate[index] = false;
            } else {
                self.insert_formula(index, formula);
            }
        }
        self.editing.reset();
        self.focus_expression(start + count - 1, true);
        self.remember(before);
        self.notice = Some(format!("Pasted {count} expressions. Ctrl+Z to undo."));
    }

    pub(in crate::math_editor) fn focus_draft(&mut self) {
        let last = self.document.fields.len() - 1;
        if self.document.fields[last].rows[0].is_empty() {
            self.focus_expression(last, false);
        } else {
            self.insert_expression(last + 1);
        }
    }

    pub(in crate::math_editor) fn next_expression(&mut self) {
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

    pub(in crate::math_editor) fn remove_expression(&mut self, index: usize, previous: bool) {
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

    pub(in crate::math_editor) fn erase_empty_expression(&mut self) -> bool {
        let Some(index) = self.focus else {
            return false;
        };
        if !self.document.fields[index].rows[0].is_empty() {
            return false;
        }
        self.remove_expression(index, true);
        true
    }

    pub(in crate::math_editor) fn adjacent_expression(&mut self, up: bool) {
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

pub(in crate::math_editor) mod hints {
    //! Contextual editing hints describe the actual structural slot at the caret.
    use crate::math_editor::{formula::Atom, state::MathState};

    pub(in crate::math_editor) fn for_state(state: &MathState) -> Option<&'static str> {
        if state.functions
            || state.style_popup.is_some()
            || state.confirm_back
            || state.range_edit.is_some()
        {
            return None;
        }
        let formula = state.document.fields.get(state.focus?)?;
        if formula.selection().is_some() {
            return Some(
                "Selection: type to replace; root, power, fraction or function to wrap. Ctrl+C/X/V.",
            );
        }
        let row = formula.caret.row;
        if let Some((parent, index)) = formula.parent(row) {
            return Some(match &formula.rows[parent][index] {
                Atom::IndexedRoot(degree, _) if *degree == row => {
                    "Root degree: 3 for a cube root. Tab or comma moves to the value under the root."
                }
                Atom::IndexedRoot(..) | Atom::Root(_) => {
                    "Value under the root. Right arrow at the end, Space or ) exits the root."
                }
                Atom::Fraction(numerator, _) if *numerator == row => {
                    "Numerator. Down or Tab moves to the denominator."
                }
                Atom::Fraction(..) => {
                    "Denominator. Right arrow at the end or Space returns to the main expression."
                }
                Atom::Power(_, exponent) if *exponent == row => {
                    "Exponent. Right arrow at the end or Space returns to the main expression."
                }
                Atom::Integral(lower, _, _) if *lower == row => {
                    "Lower integration bound. Tab moves to the upper bound."
                }
                Atom::Integral(_, upper, _) if *upper == row => {
                    "Upper integration bound. Tab moves to the integrand."
                }
                Atom::Integral(..) => {
                    "Integrand. Enter calculates; Shift+Enter starts the next expression."
                }
                _ => {
                    "Tab moves between formula slots. Right arrow at the end or ) exits this group."
                }
            });
        }
        if formula.rows[0].is_empty() {
            Some("Type an expression, e.g. y=x^2 or root(3,x). Click below a row to start another.")
        } else if formula.is_integral() {
            Some(
                "Enter calculates; Shift+Enter starts the next expression. Ctrl+C/X/V copies, cuts or pastes.",
            )
        } else {
            Some(
                "Enter starts the next expression. Up/Down changes rows; Ctrl+C/X/V copies, cuts or pastes.",
            )
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::math_editor::formula::Formula;
        #[test]
        fn indexed_root_hints_follow_degree_body_and_exit() {
            let mut state = MathState::new().unwrap();
            state.document.fields[0].template('n');
            assert!(for_state(&state).unwrap().starts_with("Root degree"));
            for ch in "3,".chars() {
                state.document.fields[0].type_char(ch);
            }
            assert!(for_state(&state).unwrap().starts_with("Value under"));
            for ch in "x)".chars() {
                state.document.fields[0].type_char(ch);
            }
            assert!(for_state(&state).unwrap().starts_with("Enter starts"));
            state.document.fields[0] = Formula::typed("int(x)");
            assert!(for_state(&state).unwrap().starts_with("Enter calculates"));
            state.functions = true;
            assert!(for_state(&state).is_none());
        }
    }
}
