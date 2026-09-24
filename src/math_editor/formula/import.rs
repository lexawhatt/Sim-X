//! Plain-text clipboard import via meval's iterative parser. Import builds a
//! candidate first: unsupported notation never partially overwrites a formula.
use crate::math_editor::formula::{Atom, Caret, Formula};
use meval::tokenizer::{Operation, Token};

struct Term {
    atoms: Vec<Atom>,
    precedence: u8,
}
impl Formula {
    pub fn from_text(text: &str) -> Result<Self, String> {
        let mut formula = Self::default();
        if text.lines().filter(|line| !line.trim().is_empty()).count() > 1 {
            return Err("Paste one expression at a time; multiple rows are not merged.".into());
        }
        let (text, mut numbers) = normalize(text)?;
        if text.is_empty() {
            return Ok(formula);
        }
        for (relation, part) in relation_parts(&text)? {
            if let Some(relation) = relation {
                formula.rows[0].push(Atom::Symbol(relation));
            }
            let tokens = meval::tokenizer::tokenize(part).map_err(|e| e.to_string())?;
            let rpn = meval::shunting_yard::to_rpn(&tokens).map_err(|e| e.to_string())?;
            let mut stack: Vec<Term> = vec![];
            for token in rpn {
                let term = match token {
                    Token::Number(n) if n.is_finite() => Term {
                        atoms: formula
                            .number_literal(numbers.pop_front().ok_or("Missing numeric spelling")?),
                        precedence: 4,
                    },
                    Token::Var(name) => Term {
                        atoms: if name == "pi" {
                            vec![Atom::Symbol('π')]
                        } else {
                            name.chars().map(Atom::Symbol).collect()
                        },
                        precedence: 4,
                    },
                    Token::Unary(Operation::Plus) => stack.pop().ok_or("Missing operand")?,
                    Token::Unary(Operation::Minus) => {
                        let child = stack.pop().ok_or("Missing operand")?;
                        let mut atoms = vec![Atom::Symbol('-')];
                        atoms.extend(formula.group_below(child, 3));
                        Term {
                            atoms,
                            precedence: 3,
                        }
                    }
                    Token::Binary(op) => {
                        let right = stack.pop().ok_or("Missing right operand")?;
                        let left = stack.pop().ok_or("Missing left operand")?;
                        match op {
                            Operation::Div | Operation::Pow => {
                                let base = if op == Operation::Pow {
                                    formula.group_below(left, 4)
                                } else {
                                    left.atoms
                                };
                                let a = formula.row(base);
                                let b = formula.row(right.atoms);
                                Term {
                                    atoms: vec![if op == Operation::Div {
                                        Atom::Fraction(a, b)
                                    } else {
                                        Atom::Power(a, b)
                                    }],
                                    precedence: 4,
                                }
                            }
                            Operation::Plus | Operation::Minus | Operation::Times => {
                                let precedence = if op == Operation::Times { 2 } else { 1 };
                                let mut atoms = formula.group_below(left, precedence);
                                atoms.push(Atom::Symbol(match op {
                                    Operation::Plus => '+',
                                    Operation::Minus => '-',
                                    _ => '*',
                                }));
                                atoms.extend(formula.group_below(
                                    right,
                                    precedence + u8::from(op == Operation::Minus),
                                ));
                                Term { atoms, precedence }
                            }
                            _ => return Err("Use mod(x,y) instead of a remainder operator.".into()),
                        }
                    }
                    Token::Func(name, Some(n)) if stack.len() >= n => {
                        let args: Vec<_> = stack.drain(stack.len() - n..).collect();
                        let slots: Vec<_> =
                            args.into_iter().map(|t| formula.row(t.atoms)).collect();
                        let atom = match (name.as_str(), slots.as_slice()) {
                            ("integral" | "int", [a, b, c]) => Atom::Integral(*a, *b, *c),
                            ("derivative", [a]) => Atom::Derivative(*a),
                            ("sqrt", [a]) => Atom::Root(*a),
                            ("root" | "nthroot", [degree, body]) => {
                                Atom::IndexedRoot(*degree, *body)
                            }
                            ("cbrt", [body]) => {
                                let degree = formula.row(vec![Atom::Symbol('3')]);
                                Atom::IndexedRoot(degree, *body)
                            }
                            _ if sim_math::functions::find(&name).is_some() => {
                                let mut atoms = vec![];
                                for (i, slot) in slots.into_iter().enumerate() {
                                    if i > 0 {
                                        atoms.push(Atom::Symbol(','));
                                    }
                                    atoms.extend(std::mem::take(&mut formula.rows[slot]));
                                }
                                let child = formula.row(atoms);
                                Atom::Function(name, child)
                            }
                            _ => return Err(format!("Unsupported function or arguments: {name}")),
                        };
                        Term {
                            atoms: vec![atom],
                            precedence: 4,
                        }
                    }
                    _ => return Err("Unsupported clipboard notation.".into()),
                };
                stack.push(term);
            }
            if stack.len() != 1 {
                return Err("Incomplete clipboard expression.".into());
            }
            if let Some(term) = stack.pop() {
                formula.rows[0].extend(term.atoms);
            }
        }
        formula.caret = Caret {
            row: 0,
            index: formula.rows[0].len(),
        };
        Ok(formula)
    }
    fn group_below(&mut self, term: Term, precedence: u8) -> Vec<Atom> {
        if term.precedence < precedence {
            let row = self.row(term.atoms);
            vec![Atom::Group(row)]
        } else {
            term.atoms
        }
    }
    fn number_literal(&mut self, text: String) -> Vec<Atom> {
        if let Some((mantissa, exponent)) = text.split_once(['e', 'E']) {
            let base = self.row(vec![Atom::Symbol('1'), Atom::Symbol('0')]);
            let exponent = self.row(exponent.chars().map(Atom::Symbol).collect());
            let mut atoms: Vec<_> = mantissa.chars().map(Atom::Symbol).collect();
            atoms.push(Atom::Symbol('*'));
            atoms.push(Atom::Power(base, exponent));
            // Keep scientific notation one operand for enclosing powers.
            let group = self.row(atoms);
            vec![Atom::Group(group)]
        } else {
            text.chars().map(Atom::Symbol).collect()
        }
    }
}

fn normalize(text: &str) -> Result<(String, std::collections::VecDeque<String>), String> {
    let text = text
        .replace('π', "pi")
        .replace('−', "-")
        .replace(['×', '·'], "*")
        .replace('÷', "/")
        .replace('≤', "<=")
        .replace('≥', ">=");
    let chars: Vec<_> = text.chars().collect();
    let mut tokens: Vec<String> = vec![];
    let mut numbers = std::collections::VecDeque::new();
    let mut absolute = 0_usize;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && matches!(chars[i], 'e' | 'E') {
                let mut end = i + 1;
                if end < chars.len() && matches!(chars[end], '+' | '-') {
                    end += 1;
                }
                let digits = end;
                while end < chars.len() && chars[end].is_ascii_digit() {
                    end += 1;
                }
                if end > digits {
                    i = end;
                }
            }
            let spelling: String = chars[start..i].iter().collect();
            numbers.push_back(spelling.clone());
            tokens.push(if spelling.starts_with('.') {
                format!("0{spelling}")
            } else {
                spelling
            });
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            let mut name: String = chars[start..i]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();
            if name == "atan" && chars.get(i) == Some(&'2') {
                name.push('2');
                i += 1;
            }
            if sim_math::functions::find(&name).is_some()
                || matches!(name.as_str(), "pi" | "integral" | "int" | "derivative")
            {
                tokens.push(name);
            } else {
                tokens.extend(name.chars().map(|c| c.to_string()));
            }
        } else if c == '|' {
            let complete = tokens.last().is_some_and(|token| {
                token == ")"
                    || token.starts_with('.')
                    || token
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
            });
            if absolute > 0 && complete {
                tokens.push(")".into());
                absolute -= 1;
            } else {
                tokens.push("abs".into());
                tokens.push("(".into());
                absolute += 1;
            }
            i += 1;
        } else if "+-*/^(),=<>".contains(c) {
            tokens.push(c.to_string());
            i += 1;
        } else {
            return Err(format!(
                "Unsupported pasted character: {c}. Use plain math, not LaTeX."
            ));
        }
    }
    if absolute != 0 {
        return Err("Close every absolute-value bar before pasting.".into());
    }
    let mut out = String::new();
    let mut previous_value = false;
    for (index, token) in tokens.iter().enumerate() {
        let function = sim_math::functions::find(token).is_some()
            || matches!(token.as_str(), "int" | "integral" | "derivative");
        if function && tokens.get(index + 1).is_none_or(|next| next != "(") {
            return Err(format!(
                "Use {token}(...) in pasted text; put a power after its argument."
            ));
        }
        let value = token.as_bytes()[0].is_ascii_alphanumeric() || token.starts_with('.');
        if previous_value && (value || token == "(") {
            out.push('*');
        }
        previous_value = token == ")" || (value && !function);
        out.push_str(token);
    }
    Ok((out, numbers))
}

/// Comparisons are document relations, not scalar meval operators. Parse each
/// side independently so an inclusive sign is never mistaken for an equation.
fn relation_parts(text: &str) -> Result<Vec<(Option<char>, &str)>, String> {
    let bytes = text.as_bytes();
    let mut relation = None;
    let mut depth = 0_i64;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'<' | b'>' | b'=' => {
                if relation.is_some() {
                    return Err("Chained comparisons are not supported yet.".into());
                }
                if depth != 0 {
                    return Err("Put the comparison outside functions and parentheses.".into());
                }
                let start = i;
                let mut sign = char::from(bytes[i]);
                if sign != '=' && bytes.get(i + 1) == Some(&b'=') {
                    i += 1;
                    sign = if sign == '<' { '≤' } else { '≥' };
                }
                relation = Some((start, i + 1, sign));
            }
            _ => {}
        }
        i += 1;
    }
    Ok(if let Some((start, end, sign)) = relation {
        vec![(None, &text[..start]), (Some(sign), &text[end..])]
    } else {
        vec![(None, text)]
    })
}
