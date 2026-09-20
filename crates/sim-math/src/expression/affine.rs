//! Conservative structural linearity over selected variables in meval's RPN.
//! Never infer linearity from a few numeric probes (e.g. sin(I) is not linear).
use super::Expression;
use meval::tokenizer::{Operation, Token};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Affine {
    pub offset: f64,
    pub weights: BTreeMap<String, f64>,
}
impl Affine {
    fn scalar(value: f64) -> Self {
        Self {
            offset: value,
            weights: BTreeMap::new(),
        }
    }
    fn scaled(mut self, scale: f64) -> Self {
        self.offset *= scale;
        for value in self.weights.values_mut() {
            *value *= scale;
        }
        self
    }
    fn sum(mut self, other: Self) -> Self {
        self.offset += other.offset;
        for (name, value) in other.weights {
            *self.weights.entry(name).or_default() += value;
        }
        self
    }
    fn valid(mut self) -> Option<Self> {
        self.weights.retain(|_, v| *v != 0.0);
        (self.offset.is_finite() && self.weights.values().all(|v| v.is_finite())).then_some(self)
    }
}
impl Expression {
    pub(crate) fn affine(&self, names: &BTreeSet<&str>) -> Option<Affine> {
        let mut stack: Vec<Affine> = vec![];
        for token in self.compiled.iter() {
            let value = match token {
                Token::Number(n) => Affine::scalar(*n),
                Token::Var(name) if names.contains(name.as_str()) => Affine {
                    offset: 0.0,
                    weights: BTreeMap::from([(name.clone(), 1.0)]),
                },
                Token::Var(name) => Affine::scalar(match name.as_str() {
                    "pi" => std::f64::consts::PI,
                    "e" => std::f64::consts::E,
                    "tau" => std::f64::consts::TAU,
                    _ => *self.bindings.get(name)?,
                }),
                Token::Unary(op) => match op {
                    Operation::Minus => stack.pop()?.scaled(-1.0),
                    Operation::Plus => stack.pop()?,
                    _ => return None,
                },
                Token::Binary(op) => {
                    let right = stack.pop()?;
                    let left = stack.pop()?;
                    match op {
                        Operation::Plus => left.sum(right),
                        Operation::Minus => left.sum(right.scaled(-1.0)),
                        Operation::Times if right.weights.is_empty() => left.scaled(right.offset),
                        Operation::Times if left.weights.is_empty() => right.scaled(left.offset),
                        Operation::Div if right.weights.is_empty() && right.offset != 0.0 => {
                            left.scaled(1.0 / right.offset)
                        }
                        Operation::Pow if right.weights.is_empty() && left.weights.is_empty() => {
                            Affine::scalar(left.offset.powf(right.offset))
                        }
                        Operation::Pow if right.weights.is_empty() && right.offset == 1.0 => left,
                        _ => return None,
                    }
                }
                Token::Func(name, Some(count)) => {
                    let args = stack.split_off(stack.len().checked_sub(*count)?);
                    if args.iter().any(|a| !a.weights.is_empty()) {
                        return None;
                    }
                    Affine::scalar(
                        crate::functions::find(name)?
                            .evaluate(&args.iter().map(|a| a.offset).collect::<Vec<_>>()),
                    )
                }
                _ => return None,
            };
            stack.push(value.valid()?);
        }
        if stack.len() == 1 { stack.pop() } else { None }
    }
}
