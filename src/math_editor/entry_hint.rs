//! Contextual editing hints describe the actual structural slot at the caret.
use super::{formula::Atom, state::MathState};

pub(super) fn for_state(state: &MathState) -> Option<&'static str> {
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
            _ => "Tab moves between formula slots. Right arrow at the end or ) exits this group.",
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
