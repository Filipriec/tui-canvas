// src/editor/selection.rs
//! Universal selection primitives — used by both row models with no knowledge of
//! which product or paradigm is calling. The operations that actually differ by
//! row model live in [`crate::editor::rows`].

#![cfg(feature = "keybindings")]

use crate::DataProvider;
use crate::canvas::state::SelectionState;
use crate::editor::EditorCore;

impl<D: DataProvider> EditorCore<D> {
    /// Copy the active selection into the yank register without modifying the
    /// buffer.
    pub(crate) fn yank_selection_core(&mut self) {
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
                let current = self.current_field();
                let start = anchor_field.min(current);
                let end = anchor_field.max(current);
                let lines = self.data_provider().capture_content();
                if start < lines.len() {
                    self.behavior_state
                        .yank_mut()
                        .set_line_register(lines[start..=end.min(lines.len() - 1)].to_vec());
                }
            }
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.current_field(), self.cursor_position());
                let start = anchor.min(cursor);
                let end = anchor.max(cursor);
                let lines = self.data_provider().capture_content();
                if start.0 >= lines.len() || end.0 >= lines.len() {
                    return;
                }
                let yanked = Self::extract_characterwise_text_core(&lines, start, end);
                if yanked.iter().all(|text| text.is_empty()) {
                    return;
                }
                self.behavior_state.yank_mut().set_text_register(yanked);
            }
            SelectionState::None => {}
        }
    }

    /// Remove the inclusive `[from, to]` character range from `line`.
    pub(crate) fn remove_char_range(line: &str, from: usize, to: usize) -> String {
        line.chars()
            .enumerate()
            .filter_map(|(idx, ch)| {
                if idx < from || idx > to {
                    Some(ch)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract the text covered by a characterwise selection across `lines`.
    pub(crate) fn extract_characterwise_text_core(
        lines: &[String],
        start: (usize, usize),
        end: (usize, usize),
    ) -> Vec<String> {
        if start.0 == end.0 {
            let text: String = lines[start.0]
                .chars()
                .skip(start.1)
                .take(end.1.saturating_sub(start.1) + 1)
                .collect();
            return vec![text];
        }

        let mut yanked = Vec::new();
        let first: String = lines[start.0].chars().skip(start.1).collect();
        yanked.push(first);
        for line in &lines[start.0 + 1..end.0] {
            yanked.push(line.clone());
        }
        let last: String = lines[end.0].chars().take(end.1 + 1).collect();
        yanked.push(last);
        yanked
    }
}
