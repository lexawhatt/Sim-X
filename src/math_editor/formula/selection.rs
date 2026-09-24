//! Structural selection. Crossing a slot boundary selects its enclosing atom;
//! copy/cut never produces half of a fraction or a dangling child reference.
use crate::math_editor::formula::{Atom, Caret, Formula};

impl Formula {
    pub fn selection(&self) -> Option<(usize, usize, usize)> {
        if self.select_all {
            return Some((0, 0, self.rows[0].len()));
        }
        let anchor = self.anchor?;
        if anchor == self.caret {
            return None;
        }
        if anchor.row == self.caret.row {
            return Some((
                anchor.row,
                anchor.index.min(self.caret.index),
                anchor.index.max(self.caret.index),
            ));
        }
        let mut left = vec![(anchor.row, anchor.index, anchor.index)];
        let mut child = anchor.row;
        while let Some((row, index)) = self.parent(child) {
            left.push((row, index, index + 1));
            child = row;
        }
        let mut right = (self.caret.row, self.caret.index, self.caret.index);
        loop {
            if let Some(&(row, start, end)) = left.iter().find(|p| p.0 == right.0) {
                return Some((row, start.min(right.1), end.max(right.2)));
            }
            let (row, index) = self.parent(right.0)?;
            right = (row, index, index + 1);
        }
    }
    pub fn move_horizontal(&mut self, right: bool, extend: bool) {
        let anchor = self.anchor.unwrap_or(self.caret);
        if !extend && let Some((row, start, end)) = self.selection() {
            self.caret = Caret {
                row,
                index: if right { end } else { start },
            };
            self.select_all = false;
            self.anchor = None;
            return;
        }
        self.horizontal(right);
        self.anchor = extend.then_some(anchor);
    }
    pub fn fragment(&self) -> Self {
        let (row, start, end) = self.selection().unwrap_or((0, 0, self.rows[0].len()));
        let mut result = self.clone();
        result.rows[0] = self.rows[row][start..end].to_vec();
        // A compact copy avoids stale parents from the source's other rows.
        let mut compact = Self::default();
        compact.splice_fragment(&result);
        compact
    }
    pub fn splice_fragment(&mut self, fragment: &Self) {
        if self.selection().is_some() {
            self.erase(true);
        }
        let mut mapping = vec![None; fragment.rows.len()];
        // Postorder contains reachable rows only; all child mappings exist first.
        for row in fragment.postorder() {
            let mut atoms = fragment.rows[row].clone();
            for atom in &mut atoms {
                let remap = |r: &mut usize| {
                    *r = mapping[*r].unwrap_or(0);
                };
                match atom {
                    Atom::Symbol(_) => {}
                    Atom::Group(a) | Atom::Function(_, a) | Atom::Root(a) | Atom::Derivative(a) => {
                        remap(a)
                    }
                    Atom::IndexedRoot(a, b) | Atom::Fraction(a, b) | Atom::Power(a, b) => {
                        remap(a);
                        remap(b);
                    }
                    Atom::Integral(a, b, c) => {
                        remap(a);
                        remap(b);
                        remap(c);
                    }
                }
            }
            if row == 0 {
                let n = atoms.len();
                self.rows[self.caret.row].splice(self.caret.index..self.caret.index, atoms);
                self.caret.index += n;
            } else {
                mapping[row] = Some(self.row(atoms));
            }
        }
        self.anchor = None;
        self.select_all = false;
    }
}
