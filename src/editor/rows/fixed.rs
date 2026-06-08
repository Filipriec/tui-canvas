// src/editor/rows/fixed.rs
//! Fixed-row operations (text forms). The row count never changes: deleting a
//! selection clears the affected slots in place and pasting writes into existing
//! slots. Nothing here shifts later rows or creates new ones.

use crate::canvas::state::SelectionState;
use crate::editor::features::history::EditKind;
use crate::editor::EditorCore;
use crate::DataProvider;

impl<D: DataProvider> EditorCore<D> {
    /// Delete the active selection, clearing slots in place. Returns `true` if
    /// anything changed.
    pub(crate) fn delete_selection_once_fixed(&mut self, yank: bool) -> bool {
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
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

                for i in start..=end {
                    self.data_provider_mut().set_field_value(i, String::new());
                }
                let _ = self.transition_to_field(start);
                self.move_line_start();
                // Drop the now-stale anchor; the caller re-establishes a
                // collapsed selection at the cursor.
                self.ui_state.selection = SelectionState::None;
                true
            }
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.current_field(), self.cursor_position());
                if anchor == cursor {
                    return self.delete_primary_character_fixed(yank);
                }
                let start = anchor.min(cursor);
                let end = anchor.max(cursor);
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

                let _ = self.transition_to_field(start.0);
                self.set_cursor_position(start.1);
                self.ui_state.selection = SelectionState::None;
                true
            }
            SelectionState::None => self.delete_primary_character_fixed(yank),
        }
    }

    /// Delete the character under the cursor. At end-of-field this does nothing
    /// (a fixed row is never pulled up into another).
    pub(crate) fn delete_primary_character_fixed(&mut self, yank: bool) -> bool {
        let field = self.current_field();
        let col = self.cursor_position();
        let current = self.current_text().to_string();
        if col >= current.chars().count() {
            return false;
        }

        if yank {
            let ch: String = current.chars().skip(col).take(1).collect();
            self.behavior_state.yank_mut().set_text_register(vec![ch]);
        }
        self.record_checkpoint(EditKind::Delete);
        let kept = Self::remove_char_range(&current, col, col);
        self.data_provider_mut().set_field_value(field, kept);
        true
    }

    /// Paste whole rows from the line register into the existing slots, starting
    /// at the current (or next) field. Clamped to the row count; nothing shifts.
    pub(crate) fn paste_lines_fixed(&mut self, after: bool, count: usize, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }
        let field_count = self.data_provider().field_count();
        if field_count == 0 {
            return;
        }
        let current = self.current_field();
        let start = if after {
            current.saturating_add(1)
        } else {
            current
        };
        if start >= field_count {
            return;
        }

        self.record_checkpoint(EditKind::Other);
        let repeat = count.max(1);
        let mut offset = 0;
        for _ in 0..repeat {
            for line in &lines {
                let field = start.saturating_add(offset);
                if field >= field_count {
                    break;
                }
                self.data_provider_mut().set_field_value(field, line.clone());
                offset += 1;
            }
        }
        let _ = self.transition_to_field(start);
        self.move_line_start();
    }

    /// Insert register text at `(field, col)`, distributing extra lines across
    /// the following slots in place. Returns the cursor landing position.
    pub(crate) fn insert_text_fixed(
        &mut self,
        field: usize,
        col: usize,
        text: &str,
    ) -> (usize, usize) {
        let field_count = self.data_provider().field_count();
        if field_count == 0 {
            return (field, col);
        }
        let field = field.min(field_count - 1);

        self.record_checkpoint(EditKind::Other);
        let current = self.data_provider().field_value(field).to_string();
        let col = col.min(current.chars().count());
        let prefix: String = current.chars().take(col).collect();
        let suffix: String = current.chars().skip(col).collect();
        let parts: Vec<&str> = text.split('\n').collect();

        if parts.len() == 1 {
            self.data_provider_mut()
                .set_field_value(field, format!("{prefix}{}{suffix}", parts[0]));
            return (field, col.saturating_add(parts[0].chars().count()));
        }

        let available = field_count - field;
        let last_offset = parts.len().min(available).saturating_sub(1);
        self.data_provider_mut()
            .set_field_value(field, format!("{prefix}{}", parts[0]));

        let mut target = (field, col.saturating_add(parts[0].chars().count()));
        for (offset, part) in parts.iter().enumerate().skip(1) {
            let next_field = field.saturating_add(offset);
            if next_field >= field_count {
                break;
            }
            let value = if offset == last_offset {
                format!("{part}{suffix}")
            } else {
                (*part).to_string()
            };
            self.data_provider_mut().set_field_value(next_field, value);
            target = (next_field, part.chars().count());
        }

        // Only the first row fit: re-attach the suffix to it.
        if last_offset == 0 {
            self.data_provider_mut()
                .set_field_value(field, format!("{prefix}{}{suffix}", parts[0]));
        }
        target
    }
}
