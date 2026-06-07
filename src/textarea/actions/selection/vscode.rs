//! Modeless, VSCode-style selection.
//!
//! Unlike the modal paradigms, VSCode selection lives *inside* edit mode: the
//! caret keeps inserting, and a `Characterwise` selection anchor is tracked
//! alongside it. The renderer already highlights from `selection_state()`
//! (independent of `AppMode`), so simply maintaining the anchor here is enough
//! to draw the selection. Ranges are half-open `[start, end)` to match VSCode
//! (and ordinary text-editor) semantics, in contrast to the inclusive vim/helix
//! selections.

#[cfg(feature = "keybindings")]
use crate::{
    canvas::state::SelectionState,
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn vscode_selection_active(&self) -> bool {
        !matches!(self.selection_state(), SelectionState::None)
    }

    /// Anchor a selection at the caret if one isn't active yet, then run a caret
    /// movement so the selection grows to the caret's new position.
    pub(crate) fn vscode_extend<F: FnOnce(&mut Self)>(&mut self, mv: F) {
        if !self.vscode_selection_active() {
            let anchor = (self.current_field(), self.cursor_position());
            self.core.ui_state.selection = SelectionState::Characterwise { anchor };
        }
        mv(self);
    }

    pub(crate) fn vscode_clear_selection(&mut self) {
        self.core.ui_state.selection = SelectionState::None;
    }

    /// Select the entire buffer (`Ctrl+A`).
    pub(crate) fn vscode_select_all(&mut self) {
        self.core.ui_state.selection = SelectionState::Characterwise { anchor: (0, 0) };
        let _ = self.move_last_line();
        self.move_line_end();
    }

    /// Ordered, half-open endpoints of the active selection, or `None` when
    /// there is no (non-empty) selection.
    fn vscode_region_endpoints(&self) -> Option<((usize, usize), (usize, usize))> {
        let SelectionState::Characterwise { anchor } = *self.selection_state() else {
            return None;
        };
        let cursor = (self.current_field(), self.cursor_position());
        if anchor == cursor {
            return None;
        }
        Some((anchor.min(cursor), anchor.max(cursor)))
    }

    /// Copy the active selection into the yank register (and OS clipboard).
    /// Returns `false` when there is no non-empty selection.
    pub(crate) fn vscode_copy_selection(&mut self) -> bool {
        let Some((start, end)) = self.vscode_region_endpoints() else {
            return false;
        };
        let lines = self.core.data_provider().capture_content();
        if start.0 >= lines.len() || end.0 >= lines.len() {
            return false;
        }

        let yanked: Vec<String> = if start.0 == end.0 {
            vec![lines[start.0]
                .chars()
                .skip(start.1)
                .take(end.1.saturating_sub(start.1))
                .collect()]
        } else {
            let mut yanked = Vec::new();
            yanked.push(lines[start.0].chars().skip(start.1).collect());
            for line in &lines[start.0 + 1..end.0] {
                yanked.push(line.clone());
            }
            yanked.push(lines[end.0].chars().take(end.1).collect());
            yanked
        };

        self.core
            .behavior_state
            .yank_mut()
            .set_text_register(yanked);
        true
    }

    /// Delete the active selection (without yanking) and clear it. Returns
    /// `true` when something was deleted.
    pub(crate) fn vscode_delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.vscode_region_endpoints() else {
            self.vscode_clear_selection();
            return false;
        };
        let mut content = self.core.data_provider().capture_content();
        if start.0 >= content.len() || end.0 >= content.len() {
            self.vscode_clear_selection();
            return false;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Delete);

        if start.0 == end.0 {
            let new_line: String = content[start.0]
                .chars()
                .enumerate()
                .filter_map(|(idx, ch)| (idx < start.1 || idx >= end.1).then_some(ch))
                .collect();
            content[start.0] = new_line;
        } else {
            let first: String = content[start.0].chars().take(start.1).collect();
            let last: String = content[end.0].chars().skip(end.1).collect();
            content[start.0] = format!("{first}{last}");
            content.drain(start.0 + 1..=end.0);
        }

        self.core.data_provider_mut().restore_content(&content);
        let _ = self.transition_to_field(start.0);
        self.set_cursor_position(start.1);
        self.vscode_clear_selection();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        true
    }
}
