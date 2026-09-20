//! Local editing cadence and undo transactions, not synthetic host input.
use super::{
    formula::{Atom, Caret},
    input,
    state::{Document, MathState},
};
use sim_logic::prelude::PhysicalKeyCode;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Insert,
    Backspace,
    Delete,
}

struct EditGroup {
    kind: EditKind,
    field: usize,
    caret: Caret,
    age: f32,
}

#[derive(Default)]
pub(super) struct EditingSession {
    held: Option<(PhysicalKeyCode, f32)>,
    blocked_until_release: Option<PhysicalKeyCode>,
    context: Option<(EditKind, usize)>,
    group: Option<EditGroup>,
}

impl EditingSession {
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Only contiguous single-symbol edits share history. Templates, selections,
    /// clipboard, row operations and explicit navigation remain undo boundaries.
    pub(super) fn coalesce(
        &mut self,
        before: &Document,
        after: &Document,
        focus: Option<usize>,
    ) -> bool {
        let context = self.context.filter(|(_, field)| {
            focus == Some(*field) && before.fields.len() == after.fields.len()
        });
        let Some((kind, field)) = context else {
            self.group = None;
            return false;
        };
        let a = &before.fields[field];
        let b = &after.fields[field];
        if a.selection().is_some()
            || a.rows.len() != b.rows.len()
            || a.caret.row != b.caret.row
            || !single_symbol_edit(
                &a.rows[a.caret.row],
                &b.rows[b.caret.row],
                kind,
                a.caret.index,
            )
        {
            self.group = None;
            return false;
        }
        let merge = self.group.as_ref().is_some_and(|group| {
            group.kind == kind && group.field == field && group.caret == a.caret && group.age < 0.7
        });
        self.group = Some(EditGroup {
            kind,
            field,
            caret: b.caret,
            age: 0.0,
        });
        merge
    }
}

fn single_symbol_edit(before: &[Atom], after: &[Atom], kind: EditKind, caret: usize) -> bool {
    let (long, short, index) = match kind {
        EditKind::Insert => (after, before, caret),
        EditKind::Backspace => {
            let Some(index) = caret.checked_sub(1) else {
                return false;
            };
            (before, after, index)
        }
        EditKind::Delete => (before, after, caret),
    };
    long.len() == short.len() + 1
        && matches!(long.get(index), Some(Atom::Symbol(_)))
        && index <= short.len()
        && long[..index] == short[..index]
        && long[index + 1..] == short[index..]
}

impl MathState {
    pub(super) fn editor_key(&mut self, key: PhysicalKeyCode) {
        use PhysicalKeyCode::*;
        let ctrl = self.controls.iter().any(|v| *v);
        let shift = self.shifts.iter().any(|v| *v);
        let kind = if ctrl || self.confirm_back || self.range_edit.is_some() {
            None
        } else {
            match key {
                Backspace => Some(EditKind::Backspace),
                Delete => Some(EditKind::Delete),
                _ => input::character(key, shift)
                    .filter(|ch| ch.is_ascii_alphanumeric() || ".+-*=,".contains(*ch))
                    .map(|_| EditKind::Insert),
            }
        };
        self.editing.context = kind.zip(self.focus);
        if self.editing.context.is_none() {
            self.editing.group = None;
        }
        let before_row = (self.focus, self.document.fields.len());
        input::key_press(self, key);
        if matches!(key, Backspace | Delete)
            && before_row != (self.focus, self.document.fields.len())
        {
            // A hold may finish the current row, but must not eat an unrelated
            // expression after row removal moves focus. Require a fresh press.
            self.editing.held = None;
            self.editing.blocked_until_release = Some(key);
        }
        self.editing.context = None;
    }

    /// Releases always cancel their held key. Modifier/shortcut/modal changes
    /// cannot transform an old hold into an edit of a new document or field.
    pub(super) fn observe_editor_key(&mut self, key: PhysicalKeyCode, down: bool) {
        use PhysicalKeyCode::*;
        if !down {
            if self.editing.held.is_some_and(|(held, _)| held == key) {
                self.editing.held = None;
            }
            if self.editing.blocked_until_release == Some(key) {
                self.editing.blocked_until_release = None;
            }
            return;
        }
        let eligible = self.editing.blocked_until_release != Some(key)
            && !self.confirm_back
            && !self.functions
            && self.style_popup.is_none()
            && !self.controls.iter().any(|v| *v)
            && (self.focus.is_some() || self.range_edit.is_some())
            && (matches!(
                key,
                Backspace | Delete | ArrowLeft | ArrowRight | ArrowUp | ArrowDown
            ) || input::character(key, self.shifts.iter().any(|v| *v))
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '.'));
        self.editing.held = eligible.then_some((key, 0.45));
    }

    pub(super) fn advance_editing(&mut self, dt: f32) -> bool {
        if let Some(group) = &mut self.editing.group {
            group.age += dt;
        }
        if self.confirm_back || self.functions || self.style_popup.is_some() || self.leaving {
            self.editing.held = None;
            return false;
        }
        let Some((key, remaining)) = &mut self.editing.held else {
            return false;
        };
        *remaining -= dt;
        if *remaining > 0.0 {
            return false;
        }
        let key = *key;
        // A delayed frame must not replay a burst of destructive editing actions.
        // At normal frame rates the cadence is 25 Hz; no backlog is accumulated.
        *remaining = remaining.rem_euclid(0.04);
        self.editor_key(key);
        true
    }
}
