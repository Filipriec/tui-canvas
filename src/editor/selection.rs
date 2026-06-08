// src/editor/selection.rs
//! Selection-aware structural edits shared by every product.
//!
//! Deleting/yanking the active selection is identical for a text area and a
//! text form *except* for what happens to the row structure. That single
//! difference is driven by [`EditorCore::row_policy`]:
//!
//! - [`crate::RowPolicy::Dynamic`] — rows are removed and neighbours merged
//!   (classic text-area behavior).
//! - [`crate::RowPolicy::Fixed`] — the row count is preserved; the affected
//!   slots are cleared in place so later rows never shift upward.
//!
//! All edits go through the base-trait [`crate::DataProvider::capture_content`]/
//! [`crate::DataProvider::restore_content`] (or direct `set_field_value`), so
//! this engine stays generic over any provider without needing the rope-only
//! `TextAreaDataProvider` structural methods.

#![cfg(feature = "keybindings")]

use crate::canvas::state::SelectionState;
use crate::editor::features::history::EditKind;
use crate::editor::EditorCore;
use crate::{DataProvider, RowPolicy};

impl<D: DataProvider> EditorCore<D> {
    /// Delete the current selection once. Returns `true` if anything changed.
    pub(crate) fn delete_selection_once_core(&mut self, yank: bool) -> bool {
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => self.delete_linewise_core(anchor_field, yank),
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.current_field(), self.cursor_position());
                if anchor == cursor {
                    return self.delete_primary_character_core(yank);
                }
                self.delete_characterwise_core(anchor.min(cursor), anchor.max(cursor), yank)
            }
            SelectionState::None => self.delete_primary_character_core(yank),
        }
    }

    fn delete_linewise_core(&mut self, anchor_field: usize, yank: bool) -> bool {
        let current = self.current_field();
        let start = anchor_field.min(current);
        let field_count = self.data_provider().field_count();
        if field_count == 0 || start >= field_count {
            return false;
        }
        let end = anchor_field.max(current).min(field_count - 1);

        if yank {
            let lines: Vec<String> = (start..=end)
                .map(|i| self.data_provider().field_value(i).to_string())
                .collect();
            self.behavior_state.yank_mut().set_line_register(lines);
        }
        self.record_checkpoint(EditKind::Delete);

        let target = match self.row_policy() {
            RowPolicy::Fixed => {
                // Clear the slots in place; row count is preserved.
                for i in start..=end {
                    self.data_provider_mut().set_field_value(i, String::new());
                }
                start
            }
            RowPolicy::Dynamic => {
                let mut content = self.data_provider().capture_content();
                if content.len() <= 1 {
                    content = vec![String::new()];
                } else {
                    content.drain(start..=end.min(content.len() - 1));
                    if content.is_empty() {
                        content.push(String::new());
                    }
                }
                self.data_provider_mut().restore_content(&content);
                start.min(content.len().saturating_sub(1))
            }
        };

        let _ = self.transition_to_field(target);
        self.move_line_start();
        true
    }

    fn delete_characterwise_core(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        yank: bool,
    ) -> bool {
        let field_count = self.data_provider().field_count();
        if start.0 >= field_count || end.0 >= field_count {
            return false;
        }

        if yank {
            let lines = self.data_provider().capture_content();
            let yanked = Self::extract_characterwise_text_core(&lines, start, end);
            self.behavior_state.yank_mut().set_text_register(yanked);
        }
        self.record_checkpoint(EditKind::Delete);

        match self.row_policy() {
            RowPolicy::Fixed => {
                if start.0 == end.0 {
                    let kept = Self::remove_char_range(
                        self.data_provider().field_value(start.0),
                        start.1,
                        end.1,
                    );
                    self.data_provider_mut().set_field_value(start.0, kept);
                } else {
                    // Keep every row: trim the head row's tail, clear interior
                    // rows, trim the tail row's head. No merge.
                    let first: String = self
                        .data_provider()
                        .field_value(start.0)
                        .chars()
                        .take(start.1)
                        .collect();
                    self.data_provider_mut().set_field_value(start.0, first);
                    for i in start.0 + 1..end.0 {
                        self.data_provider_mut().set_field_value(i, String::new());
                    }
                    let last: String = self
                        .data_provider()
                        .field_value(end.0)
                        .chars()
                        .skip(end.1 + 1)
                        .collect();
                    self.data_provider_mut().set_field_value(end.0, last);
                }
            }
            RowPolicy::Dynamic => {
                let mut content = self.data_provider().capture_content();
                if start.0 == end.0 {
                    content[start.0] = Self::remove_char_range(&content[start.0], start.1, end.1);
                } else {
                    let first: String = content[start.0].chars().take(start.1).collect();
                    let last: String = content[end.0].chars().skip(end.1 + 1).collect();
                    content[start.0] = format!("{first}{last}");
                    if end.0 > start.0 {
                        content.drain(start.0 + 1..=end.0);
                    }
                }
                self.data_provider_mut().restore_content(&content);
            }
        }

        let _ = self.transition_to_field(start.0);
        self.set_cursor_position(start.1);
        true
    }

    /// Delete the single character under the cursor. At end-of-field, only
    /// `Dynamic` rows pull the next row up (join); `Fixed` rows do nothing.
    pub(crate) fn delete_primary_character_core(&mut self, yank: bool) -> bool {
        let line_idx = self.current_field();
        let col = self.cursor_position();
        let current = self.current_text().to_string();
        let line_len = current.chars().count();

        if col < line_len {
            if yank {
                let ch: String = current.chars().skip(col).take(1).collect();
                self.behavior_state.yank_mut().set_text_register(vec![ch]);
            }
            self.record_checkpoint(EditKind::Delete);
            let kept = Self::remove_char_range(&current, col, col);
            self.data_provider_mut().set_field_value(line_idx, kept);
            return true;
        }

        // End of field: join the next row only for dynamic-row buffers.
        if self.row_policy() == RowPolicy::Dynamic
            && line_idx + 1 < self.data_provider().field_count()
        {
            if yank {
                let text = self.data_provider().field_value(line_idx + 1).to_string();
                self.behavior_state.yank_mut().set_text_register(vec![text]);
            }
            self.record_checkpoint(EditKind::Delete);
            let mut content = self.data_provider().capture_content();
            if line_idx + 1 < content.len() {
                let next = content.remove(line_idx + 1);
                content[line_idx].push_str(&next);
                self.data_provider_mut().restore_content(&content);
                self.set_cursor_position(line_len);
                return true;
            }
        }
        false
    }

    /// Remove the inclusive `[from, to]` character range from `line`.
    fn remove_char_range(line: &str, from: usize, to: usize) -> String {
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
