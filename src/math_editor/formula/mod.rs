//! Arena-backed editable notation. Child slots, not LaTeX or hidden raw text,
//! define cursor navigation. No formula length or nesting quota.

mod import;
pub(super) mod layout;
mod selection;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Atom {
    Symbol(char),
    Group(usize),
    Function(String, usize),
    Root(usize),
    IndexedRoot(usize, usize),
    Fraction(usize, usize),
    Power(usize, usize),
    Integral(usize, usize, usize),
    Derivative(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ProductBoundary {
    None,
    Juxtapose,
    Dot,
}

#[cfg(test)]
mod tests;
impl Atom {
    pub(in crate::math_editor) fn children(&self) -> Vec<usize> {
        match self {
            Self::Symbol(_) => vec![],
            Self::Group(a) | Self::Root(a) | Self::Function(_, a) | Self::Derivative(a) => vec![*a],
            Self::IndexedRoot(a, b) | Self::Fraction(a, b) | Self::Power(a, b) => vec![*a, *b],
            Self::Integral(a, b, c) => vec![*a, *b, *c],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Caret {
    pub row: usize,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Formula {
    pub rows: Vec<Vec<Atom>>,
    pub caret: Caret,
    pub select_all: bool,
    pub anchor: Option<Caret>,
}

impl Default for Formula {
    fn default() -> Self {
        Self {
            rows: vec![vec![]],
            caret: Caret { row: 0, index: 0 },
            select_all: false,
            anchor: None,
        }
    }
}

fn function(word: &str) -> bool {
    sim_math::functions::find(word).is_some()
}

impl Formula {
    /// Classify live token boundaries once for both evaluation and display.
    /// A visual dot is spacing, not an editable atom or an extra caret stop.
    pub(in crate::math_editor) fn product_before(
        &self,
        row: usize,
        index: usize,
    ) -> ProductBoundary {
        let Some(previous) = index.checked_sub(1).and_then(|i| self.rows[row].get(i)) else {
            return ProductBoundary::None;
        };
        let Some(current) = self.rows[row].get(index) else {
            return ProductBoundary::None;
        };
        let value = |atom: &Atom| match atom {
            Atom::Symbol(c) => c.is_ascii_alphanumeric() || matches!(c, '.' | 'π'),
            _ => true,
        };
        if !value(previous) || !value(current) {
            return ProductBoundary::None;
        }
        if let (Atom::Symbol(left), Atom::Symbol(right)) = (previous, current) {
            if ((left.is_ascii_digit() || *left == '.')
                && (right.is_ascii_digit() || *right == '.'))
                || (*left == 'p' && *right == 'i')
            {
                return ProductBoundary::None;
            }
            if right.is_ascii_alphabetic() || *right == 'π' {
                return ProductBoundary::Juxtapose;
            }
        }
        ProductBoundary::Dot
    }
    pub fn is_integral(&self) -> bool {
        self.rows[0].iter().any(|a| matches!(a, Atom::Integral(..)))
    }
    #[cfg(test)]
    pub fn typed(text: &str) -> Self {
        let mut result = Self::default();
        for ch in text.chars() {
            result.type_char(ch);
        }
        result
    }
    pub(in crate::math_editor) fn row(&mut self, atoms: Vec<Atom>) -> usize {
        let id = self.rows.len();
        self.rows.push(atoms);
        id
    }
    fn clear_selection(&mut self) {
        if self.select_all {
            self.rows = vec![vec![]];
            self.caret = Caret { row: 0, index: 0 };
            self.select_all = false;
            self.anchor = None;
        } else if let Some((row, start, end)) = self.selection() {
            self.rows[row].drain(start..end);
            self.caret = Caret { row, index: start };
            self.anchor = None;
        }
    }
    fn insert(&mut self, atom: Atom) {
        self.rows[self.caret.row].insert(self.caret.index, atom);
        self.caret.index += 1;
    }
    fn take_operand(&mut self) -> Vec<Atom> {
        let row = &mut self.rows[self.caret.row];
        let end = self.caret.index;
        let mut start = end;
        if end > 0 {
            start -= 1;
            if let Atom::Symbol(c) = row[start] {
                if c.is_ascii_digit() || c == '.' {
                    while start > 0
                        && matches!(row[start-1],Atom::Symbol(ch) if ch.is_ascii_digit() || ch=='.')
                    {
                        start -= 1;
                    }
                } else if c.is_ascii_alphabetic() {
                    while start > 0
                        && matches!(row[start-1],Atom::Symbol(ch) if ch.is_ascii_alphabetic())
                    {
                        start -= 1;
                    }
                    let word: String = row[start..end]
                        .iter()
                        .filter_map(|a| {
                            if let Atom::Symbol(c) = a {
                                Some(c)
                            } else {
                                None
                            }
                        })
                        .collect();
                    if word != "pi" && !function(&word) {
                        start = end - 1;
                    }
                } else if c != 'π' {
                    start = end;
                }
            }
        }
        self.caret.index = start;
        row.drain(start..end).collect()
    }
    /// Function palette insertion wraps a selection; typing ordinary letters
    /// still replaces it. Both paths use the same live notation slots.
    pub fn insert_function(&mut self, name: &str) {
        let Some((row, start, end)) = self.selection() else {
            for ch in name.chars().chain(std::iter::once('(')) {
                self.type_char(ch);
            }
            return;
        };
        if matches!(name, "int" | "integral" | "derivative") {
            self.template(if name == "derivative" { 'd' } else { 'i' });
            return;
        }
        let operand: Vec<_> = self.rows[row].drain(start..end).collect();
        let end = operand.len();
        let body = self.row(operand);
        let atom = match name {
            "sqrt" => Atom::Root(body),
            "root" | "nthroot" => {
                let degree = self.row(vec![]);
                Atom::IndexedRoot(degree, body)
            }
            "cbrt" => {
                let degree = self.row(vec![Atom::Symbol('3')]);
                Atom::IndexedRoot(degree, body)
            }
            _ => Atom::Function(name.into(), body),
        };
        let caret = match atom {
            Atom::IndexedRoot(degree, _) if name != "cbrt" => Caret {
                row: degree,
                index: 0,
            },
            _ => Caret {
                row: body,
                index: end,
            },
        };
        self.select_all = false;
        self.anchor = None;
        self.caret = Caret { row, index: start };
        self.insert(atom);
        self.caret = caret;
    }
    pub fn template(&mut self, kind: char) {
        let selected = self.selection().map(|(row, start, end)| {
            self.caret = Caret { row, index: start };
            self.rows[row].drain(start..end).collect::<Vec<_>>()
        });
        self.select_all = false;
        self.anchor = None;
        if kind == 'i' {
            let operand = selected.unwrap_or_else(|| {
                self.rows[self.caret.row]
                    .drain(self.caret.index..)
                    .collect()
            });
            let lower = self.row(vec![Atom::Symbol('0')]);
            let upper = self.row(vec![Atom::Symbol('1')]);
            let body = self.row(operand);
            self.insert(Atom::Integral(lower, upper, body));
            self.caret = Caret {
                row: body,
                index: 0,
            };
            return;
        }
        let parent = self.caret.row;
        if kind == 'd' {
            let child = self.row(selected.unwrap_or_default());
            self.insert(Atom::Derivative(child));
            self.caret = Caret {
                row: child,
                index: 0,
            };
        } else if matches!(kind, 'r' | 'n') {
            let body = self.row(selected.unwrap_or_default());
            let child = if kind == 'n' {
                let degree = self.row(vec![]);
                self.insert(Atom::IndexedRoot(degree, body));
                degree
            } else {
                self.insert(Atom::Root(body));
                body
            };
            self.caret = Caret {
                row: child,
                index: 0,
            };
        } else {
            let operand = selected.unwrap_or_else(|| self.take_operand());
            let first = self.row(operand);
            let second = self.row(vec![]);
            self.insert(if kind == '/' {
                Atom::Fraction(first, second)
            } else {
                Atom::Power(first, second)
            });
            self.caret = Caret {
                row: second,
                index: 0,
            };
        }
        debug_assert_ne!(parent, self.caret.row);
    }
    pub fn type_char(&mut self, ch: char) {
        if matches!(ch, '^' | '/') {
            self.template(ch);
            return;
        }
        if ch == '('
            && let Some((row, start, end)) = self.selection()
        {
            let atoms: Vec<_> = self.rows[row].drain(start..end).collect();
            let index = atoms.len();
            let child = self.row(atoms);
            self.caret = Caret { row, index: start };
            self.insert(Atom::Group(child));
            self.caret = Caret { row: child, index };
            self.select_all = false;
            self.anchor = None;
            return;
        }
        self.clear_selection();
        if ch == '=' && self.caret.index > 0 {
            let previous = &mut self.rows[self.caret.row][self.caret.index - 1];
            match previous {
                Atom::Symbol('<') => {
                    *previous = Atom::Symbol('≤');
                    return;
                }
                Atom::Symbol('>') => {
                    *previous = Atom::Symbol('≥');
                    return;
                }
                _ => {}
            }
        }
        if ch == 'i'
            && self.caret.index > 0
            && matches!(
                self.rows[self.caret.row][self.caret.index - 1],
                Atom::Symbol('p')
            )
        {
            self.rows[self.caret.row][self.caret.index - 1] = Atom::Symbol('π');
            return;
        }
        match ch {
            '(' => {
                if self.powered_function() {
                    return;
                }
                let end = self.caret.index;
                let mut start = end;
                while start > 0
                    && matches!(self.rows[self.caret.row][start-1],Atom::Symbol(c) if c.is_ascii_alphanumeric())
                {
                    start -= 1;
                }
                let name: String = self.rows[self.caret.row][start..end]
                    .iter()
                    .filter_map(|atom| {
                        if let Atom::Symbol(c) = atom {
                            Some(c)
                        } else {
                            None
                        }
                    })
                    .collect();
                // Preserve implicit coefficients: 2sin(x), xatan2(y,2).
                // Prefer the longest registered suffix (asinh, not sinh).
                let offset = name
                    .char_indices()
                    .find_map(|(i, _)| {
                        let suffix = &name[i..];
                        (function(suffix) || matches!(suffix, "int" | "integral" | "derivative"))
                            .then_some(i)
                    })
                    .unwrap_or(0);
                let name = name[offset..].to_owned();
                start += offset;
                if matches!(name.as_str(), "int" | "integral" | "derivative") {
                    self.rows[self.caret.row].drain(start..end);
                    self.caret.index = start;
                    self.template(if name == "derivative" { 'd' } else { 'i' });
                    return;
                }
                let child = self.row(vec![]);
                let atom = if function(&name) {
                    self.rows[self.caret.row].drain(start..end);
                    self.caret.index = start;
                    if name == "sqrt" {
                        Atom::Root(child)
                    } else if matches!(name.as_str(), "root" | "nthroot") {
                        let body = self.row(vec![]);
                        Atom::IndexedRoot(child, body)
                    } else if name == "cbrt" {
                        let degree = self.row(vec![Atom::Symbol('3')]);
                        Atom::IndexedRoot(degree, child)
                    } else {
                        Atom::Function(name, child)
                    }
                } else {
                    Atom::Group(child)
                };
                self.insert(atom);
                self.caret = Caret {
                    row: child,
                    index: 0,
                };
            }
            ')' => {
                self.close_group();
            }
            ' ' => {
                self.exit_slot();
            }
            '+' | '-' | '*' | '=' | ',' | '<' | '>' | '≤' | '≥' => {
                if ch == ',' && self.advance_root_argument() {
                    return;
                }
                // Ordinary arithmetic resumes on the baseline after a completed
                // exponent. A leading minus still belongs to the exponent.
                if self.caret.index > 0 && self.parent(self.caret.row).is_some_and(|(r,i)|matches!(self.rows[r][i],Atom::Power(_,exponent) if exponent==self.caret.row)) { self.exit_slot(); }
                self.insert(Atom::Symbol(ch));
            }
            '|' => {
                if !self.close_absolute() {
                    let child = self.row(vec![]);
                    self.insert(Atom::Function("abs".into(), child));
                    self.caret = Caret {
                        row: child,
                        index: 0,
                    };
                }
            }
            _ if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '+' | '-' | '*' | 'π') => {
                self.insert(Atom::Symbol(ch.to_ascii_lowercase()))
            }
            _ => {}
        }
    }
    fn close_absolute(&mut self) -> bool {
        let mut child = self.caret.row;
        while let Some((row, index)) = self.parent(child) {
            match &self.rows[row][index] {
                Atom::Function(name, _) if name == "abs" => {
                    if child == self.caret.row && self.caret.index == 0 {
                        return false;
                    }
                    self.caret = Caret {
                        row,
                        index: index + 1,
                    };
                    return true;
                }
                Atom::Fraction(..) | Atom::Power(..) => child = row,
                _ => break,
            }
        }
        false
    }
    pub fn parent(&self, child: usize) -> Option<(usize, usize)> {
        // Arena deletions may leave unreachable rows. Only live notation can
        // own a caret; stale children must never win a parent lookup.
        let mut pending = vec![0];
        while let Some(row) = pending.pop() {
            for (index, atom) in self.rows[row].iter().enumerate() {
                let children = atom.children();
                if children.contains(&child) {
                    return Some((row, index));
                }
                pending.extend(children);
            }
        }
        None
    }
    fn advance_root_argument(&mut self) -> bool {
        let mut child = self.caret.row;
        while let Some((row, index)) = self.parent(child) {
            match self.rows[row][index] {
                Atom::IndexedRoot(degree, body) if degree == child => {
                    self.caret = Caret {
                        row: body,
                        index: 0,
                    };
                    return true;
                }
                Atom::Fraction(..) | Atom::Power(..) => child = row,
                _ => break,
            }
        }
        false
    }
    pub fn exit_slot(&mut self) {
        if let Some((row, index)) = self.parent(self.caret.row) {
            self.caret = Caret {
                row,
                index: index + 1,
            };
        }
    }
    fn close_group(&mut self) {
        while let Some((row, index)) = self.parent(self.caret.row) {
            let grouped = matches!(
                self.rows[row][index],
                Atom::Group(_)
                    | Atom::Function(_, _)
                    | Atom::Root(_)
                    | Atom::IndexedRoot(..)
                    | Atom::Integral(..)
                    | Atom::Derivative(_)
            );
            self.caret = Caret {
                row,
                index: index + 1,
            };
            if grouped {
                if self.parent(row).is_some_and(
                    |(r, i)| matches!(self.rows[r][i],Atom::Power(base,_) if base==row),
                ) {
                    self.exit_slot();
                }
                break;
            }
        }
    }
    /// Familiar sin^2(x) wraps the function call, not the argument. The same
    /// structure is produced by sin(x)^2; negative powers remain reciprocals.
    fn powered_function(&mut self) -> bool {
        let Some((row, index)) = self.parent(self.caret.row) else {
            return false;
        };
        let Atom::Power(base, exponent) = self.rows[row][index] else {
            return false;
        };
        if exponent != self.caret.row || self.rows[exponent].is_empty() {
            return false;
        }
        let name: String = self.rows[base]
            .iter()
            .filter_map(|a| {
                if let Atom::Symbol(c) = a {
                    Some(c)
                } else {
                    None
                }
            })
            .collect();
        if name.len() != self.rows[base].len()
            || !function(&name)
            || matches!(name.as_str(), "sqrt" | "root" | "nthroot" | "cbrt")
        {
            return false;
        }
        let argument = self.row(vec![]);
        let inverse = if self.rows[exponent] == [Atom::Symbol('-'), Atom::Symbol('1')] {
            match name.as_str() {
                "sin" => Some("asin"),
                "cos" => Some("acos"),
                "tan" | "tg" => Some("atan"),
                "sec" => Some("asec"),
                "csc" => Some("acsc"),
                "cot" => Some("acot"),
                _ => None,
            }
        } else {
            None
        };
        if let Some(name) = inverse {
            self.rows[row][index] = Atom::Function(name.into(), argument);
        } else {
            self.rows[base] = vec![Atom::Function(name, argument)];
        }
        self.caret = Caret {
            row: argument,
            index: 0,
        };
        true
    }
    pub fn next_slot(&mut self, backwards: bool) {
        let mut order = vec![];
        let mut stack = vec![0];
        while let Some(row) = stack.pop() {
            order.push(row);
            for atom in self.rows[row].iter().rev() {
                for c in atom.children().into_iter().rev() {
                    stack.push(c);
                }
            }
        }
        if let Some(index) = order.iter().position(|r| *r == self.caret.row) {
            let target = if backwards {
                (index + order.len() - 1) % order.len()
            } else {
                (index + 1) % order.len()
            };
            self.caret = Caret {
                row: order[target],
                index: self.rows[order[target]].len(),
            };
        }
        self.select_all = false;
        self.anchor = None;
    }
    pub fn horizontal(&mut self, right: bool) {
        self.select_all = false;
        if right {
            if let Some(atom) = self.rows[self.caret.row].get(self.caret.index) {
                if let Some(child) = atom.children().first() {
                    self.caret = Caret {
                        row: *child,
                        index: 0,
                    };
                } else {
                    self.caret.index += 1;
                }
            } else if let Some((row, index)) = self.parent(self.caret.row) {
                let children = self.rows[row][index].children();
                if let Some(next) = children
                    .windows(2)
                    .find(|pair| pair[0] == self.caret.row)
                    .map(|p| p[1])
                {
                    self.caret = Caret {
                        row: next,
                        index: 0,
                    };
                } else {
                    self.exit_slot();
                }
            }
        } else if self.caret.index > 0 {
            self.caret.index -= 1;
            if let Some(child) = self.rows[self.caret.row][self.caret.index]
                .children()
                .last()
            {
                self.caret = Caret {
                    row: *child,
                    index: self.rows[*child].len(),
                };
            }
        } else if let Some((row, index)) = self.parent(self.caret.row) {
            let children = self.rows[row][index].children();
            if let Some(previous) = children
                .windows(2)
                .find(|pair| pair[1] == self.caret.row)
                .map(|p| p[0])
            {
                self.caret = Caret {
                    row: previous,
                    index: self.rows[previous].len(),
                };
            } else {
                self.caret = Caret { row, index };
            }
        }
    }
    pub fn erase(&mut self, forward: bool) {
        if self.selection().is_some() {
            self.clear_selection();
            return;
        }
        let row = self.caret.row;
        if forward && self.caret.index < self.rows[row].len() {
            self.remove_wrapper_or_atom(row, self.caret.index);
        } else if !forward && self.caret.index > 0 {
            self.caret.index -= 1;
            self.remove_wrapper_or_atom(row, self.caret.index);
        } else if !forward && let Some((parent, index)) = self.parent(row) {
            let unwrap = match self.rows[parent][index] {
                Atom::Integral(_, _, body)
                | Atom::IndexedRoot(_, body)
                | Atom::Root(body)
                | Atom::Derivative(body) => body == row,
                Atom::Power(_, second) | Atom::Fraction(_, second) => {
                    second == row && self.rows[second].is_empty()
                }
                _ => false,
            };
            if unwrap {
                self.caret = Caret { row: parent, index };
                self.remove_wrapper_or_atom(parent, index);
            } else {
                self.horizontal(false);
            }
        } else if forward {
            self.exit_slot();
        }
    }
    fn remove_wrapper_or_atom(&mut self, row: usize, index: usize) {
        let removed = self.rows[row].remove(index);
        let body = match removed {
            Atom::Integral(_, _, body)
            | Atom::Derivative(body)
            | Atom::Root(body)
            | Atom::IndexedRoot(_, body) => Some(body),
            Atom::Power(first, second) | Atom::Fraction(first, second)
                if self.rows[second].is_empty() =>
            {
                Some(first)
            }
            _ => None,
        };
        if let Some(body) = body {
            // Move, do not clone: orphan arena rows must not retain duplicate
            // parent links to nested nodes after structural editing.
            let body = std::mem::take(&mut self.rows[body]);
            self.rows[row].splice(index..index, body);
        }
    }
    /// Iterative postorder supports deeply nested input without recursive Drop.
    pub fn postorder(&self) -> Vec<usize> {
        let mut stack = vec![(0, false)];
        let mut out = vec![];
        while let Some((r, visited)) = stack.pop() {
            if visited {
                out.push(r);
            } else {
                stack.push((r, true));
                for atom in &self.rows[r] {
                    for c in atom.children() {
                        stack.push((c, false));
                    }
                }
            }
        }
        out
    }
    pub fn source(&self) -> Result<String, &'static str> {
        let mut values = vec![String::new(); self.rows.len()];
        for row in self.postorder() {
            if self.rows[row].is_empty() {
                return Err("Fill the empty formula slot");
            }
            let mut out = String::new();
            let mut index = 0;
            while index < self.rows[row].len() {
                let product = self.product_before(row, index);
                let atom = &self.rows[row][index];
                let part = match atom {
                    Atom::Symbol(c) if c.is_ascii_digit() || *c == '.' => {
                        let mut s = String::new();
                        while let Some(Atom::Symbol(c)) = self.rows[row].get(index) {
                            if !c.is_ascii_digit() && *c != '.' {
                                break;
                            }
                            s.push(*c);
                            index += 1;
                        }
                        index -= 1;
                        if s.starts_with('.') {
                            s.insert(0, '0');
                        }
                        s
                    }
                    Atom::Symbol(c) if c.is_ascii_alphabetic() => {
                        if *c == 'p'
                            && matches!(self.rows[row].get(index + 1), Some(Atom::Symbol('i')))
                        {
                            index += 1;
                            "pi".into()
                        } else {
                            c.to_string()
                        }
                    }
                    Atom::Symbol('π') => "pi".into(),
                    Atom::Symbol('≤') => "<=".into(),
                    Atom::Symbol('≥') => ">=".into(),
                    Atom::Symbol(c) => c.to_string(),
                    Atom::Group(a) => format!("({})", values[*a]),
                    Atom::Function(name, a) => format!("{name}({})", values[*a]),
                    Atom::Root(a) => format!("sqrt({})", values[*a]),
                    Atom::IndexedRoot(a, b) => {
                        format!("root({},{})", values[*a], values[*b])
                    }
                    Atom::Fraction(a, b) => format!("(({})/({}))", values[*a], values[*b]),
                    Atom::Power(a, b) => format!("(({})^({}))", values[*a], values[*b]),
                    Atom::Integral(a, b, c) => {
                        format!("integral({},{},{})", values[*a], values[*b], values[*c])
                    }
                    Atom::Derivative(a) => format!("derivative({})", values[*a]),
                };
                if product != ProductBoundary::None {
                    out.push('*');
                }
                out.push_str(&part);
                index += 1;
            }
            values[row] = out;
        }
        Ok(std::mem::take(&mut values[0]))
    }
}
