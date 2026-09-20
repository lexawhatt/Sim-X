//! Clipboard intents are inert in headless worlds. The executable installs the
//! native adapter; completion is guarded against edits, focus changes and undo.
use super::{formula::Formula, state::MathState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardAction {
    Copy,
    Cut,
    Paste,
}
#[derive(Clone)]
pub(crate) struct ClipboardRequest {
    #[cfg(feature = "desktop")]
    pub token: std::sync::Arc<()>,
    pub action: ClipboardAction,
    pub row: usize,
    pub(super) original: Formula,
    pub(super) fragment: Option<Formula>,
    pub text: String,
    epoch: u64,
}
impl MathState {
    pub(crate) fn request_clipboard(&mut self, action: ClipboardAction) {
        if self.clipboard_pending.is_some() {
            return;
        }
        let Some(row) = self.focus else {
            return;
        };
        let original = self.document.fields[row].clone();
        let fragment = (action != ClipboardAction::Paste).then(|| original.fragment());
        let text = if let Some(fragment) = &fragment {
            match fragment.source() {
                Ok(text) => text,
                Err(error) => {
                    self.notice = Some(format!("Copy: {error}"));
                    return;
                }
            }
        } else {
            String::new()
        };
        self.clipboard_pending = Some(ClipboardRequest {
            epoch: self.edit_epoch,
            #[cfg(feature = "desktop")]
            token: std::sync::Arc::new(()),
            action,
            row,
            original,
            fragment,
            text,
        });
        self.notice = Some("Clipboard...".into());
    }
    pub(crate) fn complete_clipboard(&mut self, result: Result<String, String>) {
        let Some(request) = self.clipboard_pending.take() else {
            return;
        };
        let text = match result {
            Ok(text) => text,
            Err(error) => {
                self.notice = Some(format!("Clipboard: {error}"));
                return;
            }
        };
        if request.action != ClipboardAction::Paste {
            self.clipboard_local = request.fragment.map(|f| (request.text.clone(), f));
        }
        if request.action == ClipboardAction::Copy {
            self.notice = Some("Copied formula as plain math.".into());
            return;
        }
        if self.edit_epoch != request.epoch
            || self.focus != Some(request.row)
            || self.document.fields.get(request.row) != Some(&request.original)
            || self.confirm_back
        {
            self.notice = Some("Clipboard result ignored because the selection changed.".into());
            return;
        }
        let fragment = if request.action == ClipboardAction::Paste {
            if text.trim().is_empty() {
                self.notice = Some("Clipboard contains no expression.".into());
                return;
            }
            if let Some((cached, fragment)) = &self.clipboard_local
                && cached == &text
            {
                Some(fragment.clone())
            } else {
                match Formula::from_text(&text) {
                    Ok(fragment) => Some(fragment),
                    Err(error) => {
                        self.notice = Some(format!("Paste: {error}"));
                        return;
                    }
                }
            }
        } else {
            None
        };
        let before = self.document.clone();
        let formula = &mut self.document.fields[request.row];
        if let Some(fragment) = fragment {
            formula.splice_fragment(&fragment);
        } else {
            if formula.selection().is_none() {
                formula.select_all = true;
            }
            formula.erase(true);
        }
        self.remember(before);
        self.notice = Some(
            if request.action == ClipboardAction::Paste {
                "Pasted. Ctrl+Z to undo."
            } else {
                "Cut. Ctrl+Z to undo."
            }
            .into(),
        );
    }
}
