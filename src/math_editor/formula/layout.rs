//! A single layout supplies glyph placement, rules, and mouse/keyboard caret stops.
//! Traversal is iterative: nesting is not limited by the Rust call stack.
use crate::math_editor::formula::{Atom, Caret, Formula, ProductBoundary};

#[derive(Clone, Copy, Default)]
struct Size {
    width: f32,
    above: f32,
    below: f32,
}

#[derive(Clone)]
pub(in crate::math_editor) struct Glyph {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub style: usize,
}
#[derive(Clone, Copy)]
pub(in crate::math_editor) struct Rule {
    pub from: [f32; 2],
    pub to: [f32; 2],
}
#[derive(Clone, Copy)]
pub(in crate::math_editor) struct Stop {
    pub caret: Caret,
    pub x: f32,
    pub y: f32,
    pub height: f32,
}
#[derive(Clone, Default)]
pub(in crate::math_editor) struct FormulaLayout {
    pub glyphs: Vec<Glyph>,
    pub rules: Vec<Rule>,
    pub stops: Vec<Stop>,
    pub width: f32,
    pub above: f32,
    pub below: f32,
}

pub(in crate::math_editor) const HEIGHTS: [f32; 3] = [26.0, 18.0, 14.0];

impl FormulaLayout {
    pub fn build(formula: &Formula, measure: impl Fn(usize, &str) -> f32) -> Self {
        let mut styles = vec![0; formula.rows.len()];
        let mut pending = vec![0];
        while let Some(row) = pending.pop() {
            for atom in &formula.rows[row] {
                for child in atom.children() {
                    let script = matches!(atom,Atom::Power(_,exponent) if child==*exponent)
                        || matches!(atom,Atom::IndexedRoot(degree,_) if child==*degree)
                        || matches!(atom,Atom::Integral(lower,upper,_) if child==*lower || child==*upper);
                    styles[child] = (styles[row] + usize::from(script)).min(2);
                    pending.push(child);
                }
            }
        }
        let mut sizes = vec![Size::default(); formula.rows.len()];
        let mut atom_sizes: Vec<Vec<Size>> = vec![vec![]; formula.rows.len()];
        for row in formula.postorder() {
            let style = styles[row];
            let h = HEIGHTS[style];
            let mut total = Size {
                width: 0.0,
                above: h * 0.8,
                below: h * 0.25,
            };
            for (index, atom) in formula.rows[row].iter().enumerate() {
                let size = match atom {
                    Atom::Symbol(c) => Size {
                        width: measure(style, &c.to_string()) + 1.0,
                        ..total_base(h)
                    },
                    Atom::Function(name, a) if name == "abs" => Size {
                        width: sizes[*a].width + 16.0,
                        ..sizes[*a]
                    },
                    Atom::Group(a) | Atom::Function(_, a) => {
                        let prefix = match atom {
                            Atom::Function(name, _) => format!("{name}("),
                            _ => "(".into(),
                        };
                        Size {
                            width: measure(style, &prefix)
                                + sizes[*a].width
                                + measure(style, ")")
                                + 2.0,
                            ..sizes[*a]
                        }
                    }
                    Atom::Root(a) => Size {
                        width: sizes[*a].width + 17.0,
                        above: sizes[*a].above + 5.0,
                        below: sizes[*a].below,
                    },
                    Atom::IndexedRoot(degree, body) => {
                        let lift = sizes[*body].above * 0.55 + sizes[*degree].below;
                        Size {
                            width: sizes[*degree].width + sizes[*body].width + 20.0,
                            above: (sizes[*body].above + 5.0).max(lift + sizes[*degree].above),
                            below: sizes[*body].below,
                        }
                    }
                    Atom::Fraction(a, b) => Size {
                        width: sizes[*a].width.max(sizes[*b].width) + 12.0,
                        above: sizes[*a].above + sizes[*a].below + 9.0,
                        below: sizes[*b].above + sizes[*b].below + 5.0,
                    },
                    Atom::Power(a, b) => {
                        let lift = sizes[*a].above * 0.75 + sizes[*b].below;
                        Size {
                            width: sizes[*a].width + sizes[*b].width + 2.0,
                            above: sizes[*a].above.max(lift + sizes[*b].above),
                            below: sizes[*a].below,
                        }
                    }
                    Atom::Integral(lower, upper, body) => Size {
                        width: 32.0
                            + sizes[*lower].width.max(sizes[*upper].width)
                            + sizes[*body].width
                            + measure(style, " dx")
                            + 8.0,
                        above: sizes[*body].above.max(sizes[*upper].above + 24.0),
                        below: sizes[*body].below.max(sizes[*lower].below + 24.0),
                    },
                    Atom::Derivative(body) => Size {
                        width: sizes[*body].width + 44.0,
                        above: sizes[*body].above.max(32.0),
                        below: sizes[*body].below.max(18.0),
                    },
                };
                if formula.product_before(row, index) == ProductBoundary::Dot {
                    total.width += measure(style, "·") + h * 0.3;
                }
                total.width += size.width;
                total.above = total.above.max(size.above);
                total.below = total.below.max(size.below);
                atom_sizes[row].push(size);
            }
            if formula.rows[row].is_empty() && row != 0 {
                total.width = measure(style, "?") + 8.0;
            }
            sizes[row] = total;
        }
        let mut out = Self {
            width: sizes[0].width,
            above: sizes[0].above,
            below: sizes[0].below,
            ..Self::default()
        };
        let mut pending = vec![(0, 0.0, 0.0)];
        while let Some((row, mut x, y)) = pending.pop() {
            let style = styles[row];
            let h = HEIGHTS[style];
            out.stops.push(Stop {
                caret: Caret { row, index: 0 },
                x,
                y,
                height: h,
            });
            if formula.rows[row].is_empty() && row != 0 {
                out.glyphs.push(Glyph {
                    text: "?".into(),
                    x: x + 3.0,
                    y,
                    style,
                });
            }
            for (index, atom) in formula.rows[row].iter().enumerate() {
                let size = atom_sizes[row][index];
                if formula.product_before(row, index) == ProductBoundary::Dot {
                    out.glyphs.push(Glyph {
                        text: "·".into(),
                        x: x + h * 0.15,
                        y,
                        style,
                    });
                    x += measure(style, "·") + h * 0.3;
                    // Keep the existing boundary directly before its operand.
                    // The separator is not another editable character.
                    if let Some(stop) = out.stops.last_mut() {
                        stop.x = x;
                    }
                }
                match atom {
                    Atom::Symbol(c) => out.glyphs.push(Glyph {
                        text: c.to_string(),
                        x,
                        y,
                        style,
                    }),
                    Atom::Function(name, a) if name == "abs" => {
                        for dx in [3.0, size.width - 3.0] {
                            out.rules.push(Rule {
                                from: [x + dx, y - sizes[*a].above],
                                to: [x + dx, y + sizes[*a].below],
                            });
                        }
                        pending.push((*a, x + 8.0, y));
                    }
                    Atom::Group(a) | Atom::Function(_, a) => {
                        let prefix = match atom {
                            Atom::Function(name, _) => format!("{name}("),
                            _ => "(".into(),
                        };
                        let width = measure(style, &prefix);
                        out.glyphs.push(Glyph {
                            text: prefix,
                            x,
                            y,
                            style,
                        });
                        pending.push((*a, x + width, y));
                        out.glyphs.push(Glyph {
                            text: ")".into(),
                            x: x + width + sizes[*a].width,
                            y,
                            style,
                        });
                    }
                    Atom::Root(a) | Atom::IndexedRoot(_, a) => {
                        let offset = if let Atom::IndexedRoot(degree, _) = atom {
                            pending.push((
                                *degree,
                                x,
                                y - sizes[*a].above * 0.55 - sizes[*degree].below,
                            ));
                            sizes[*degree].width + 3.0
                        } else {
                            0.0
                        };
                        let radical = x + offset;
                        let top = y - sizes[*a].above - 3.0;
                        for (from, to) in [
                            ([radical, y - 5.0], [radical + 4.0, y - 8.0]),
                            ([radical + 4.0, y - 8.0], [radical + 8.0, y + 2.0]),
                            ([radical + 8.0, y + 2.0], [radical + 13.0, top]),
                            ([radical + 13.0, top], [x + size.width, top]),
                        ] {
                            out.rules.push(Rule { from, to });
                        }
                        pending.push((*a, radical + 16.0, y));
                    }
                    Atom::Fraction(a, b) => {
                        let bar = y - 3.0;
                        out.rules.push(Rule {
                            from: [x + 2.0, bar],
                            to: [x + size.width - 2.0, bar],
                        });
                        pending.push((
                            *a,
                            x + (size.width - sizes[*a].width) * 0.5,
                            bar - 5.0 - sizes[*a].below,
                        ));
                        pending.push((
                            *b,
                            x + (size.width - sizes[*b].width) * 0.5,
                            bar + 5.0 + sizes[*b].above,
                        ));
                    }
                    Atom::Power(a, b) => {
                        pending.push((*a, x, y));
                        pending.push((
                            *b,
                            x + sizes[*a].width + 1.0,
                            y - sizes[*a].above * 0.75 - sizes[*b].below,
                        ));
                    }
                    Atom::Integral(lower, upper, body) => {
                        let bounds = sizes[*lower].width.max(sizes[*upper].width);
                        pending.push((*lower, x + 26.0, y + 25.0));
                        pending.push((*upper, x + 26.0, y - 25.0));
                        pending.push((*body, x + 32.0 + bounds, y));
                        out.glyphs.push(Glyph {
                            text: " dx".into(),
                            x: x + 32.0 + bounds + sizes[*body].width,
                            y,
                            style,
                        });
                        // Hooked integral path, matching the height of its limits.
                        let controls = [
                            [[23.0, -31.0], [8.0, -41.0], [17.0, -9.0], [12.0, 1.0]],
                            [[12.0, 1.0], [7.0, 18.0], [12.0, 37.0], [0.0, 29.0]],
                        ];
                        for c in controls {
                            let mut previous = None;
                            for i in 0..16 {
                                let t = i as f32 / 15.0;
                                let s = 1.0 - t;
                                let w = [s * s * s, 3.0 * s * s * t, 3.0 * s * t * t, t * t * t];
                                let p = [
                                    x + (0..4).map(|j| c[j][0] * w[j]).sum::<f32>(),
                                    y + (0..4).map(|j| c[j][1] * w[j]).sum::<f32>(),
                                ];
                                if let Some(from) = previous {
                                    out.rules.push(Rule { from, to: p });
                                }
                                previous = Some(p);
                            }
                        }
                    }
                    Atom::Derivative(body) => {
                        out.glyphs.push(Glyph {
                            text: "d".into(),
                            x: x + 11.0,
                            y: y - 14.0,
                            style: 1,
                        });
                        out.glyphs.push(Glyph {
                            text: "dx".into(),
                            x: x + 6.0,
                            y: y + 15.0,
                            style: 1,
                        });
                        out.rules.push(Rule {
                            from: [x + 2.0, y - 6.0],
                            to: [x + 33.0, y - 6.0],
                        });
                        pending.push((*body, x + 44.0, y));
                    }
                }
                x += size.width;
                out.stops.push(Stop {
                    caret: Caret {
                        row,
                        index: index + 1,
                    },
                    x,
                    y,
                    height: h,
                });
            }
        }
        out
    }
    pub fn stop(&self, caret: Caret) -> Option<Stop> {
        self.stops.iter().find(|s| s.caret == caret).copied()
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<Caret> {
        self.stops
            .iter()
            .min_by(|a, b| {
                let distance =
                    |s: &Stop| (s.x - x).powi(2) + (s.y - s.height * 0.35 - y).powi(2) * 2.0;
                distance(a).total_cmp(&distance(b))
            })
            .map(|s| s.caret)
    }
    pub fn vertical(&self, caret: Caret, up: bool) -> Option<Caret> {
        let origin = self.stop(caret)?;
        self.stops
            .iter()
            .filter(|s| {
                if up {
                    s.y < origin.y - 2.0
                } else {
                    s.y > origin.y + 2.0
                }
            })
            .min_by(|a, b| {
                let score = |s: &Stop| (s.x - origin.x).abs() + (s.y - origin.y).abs() * 0.5;
                score(a).total_cmp(&score(b))
            })
            .map(|s| s.caret)
    }
}
fn total_base(h: f32) -> Size {
    Size {
        width: 0.0,
        above: h * 0.8,
        below: h * 0.25,
    }
}
