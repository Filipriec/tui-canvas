// src/editor/rows/dynamic.rs
//! Dynamic-row operations (text areas). Deleting a selection removes rows and
//! merges neighbours; pasting inserts new rows and multi-line text splits the
//! current row. All structural edits go through the base-trait
//! `capture_content`/`restore_content` so this stays generic over any provider.

use crate::canvas::state::SelectionState;
use crate::editor::features::history::EditKind;
use crate::editor::EditorCore;
use crate::DataProvider;

impl<D: DataProvider> EditorCore<D> {
    /// Delete the active selection, removing/merging rows. Returns `true` if
    /// anything changed.
    pub(crate) fn delete_selection_once_dynamic(&mut self, yank: bool) -> bool {
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
                let current = self.current_field();
                let start = anchor_field.min(current);
                let end = anchor_field.max(current);
                let lines = self.data_provider().capture_content();
                if start >= lines.len() {
                    return false;
                }

                if yank {
                    self.behavior_state
                        .yank_mut()
                        .set_line_register(lines[start..=end.min(lines.len() - 1)].to_vec());
                }
                self.record_checkpoint(EditKind::Delete);

                let mut content = lines;
                if content.len() <= 1 {
                    content = vec![String::new()];
                } else {
                    content.drain(start..=end.min(content.len() - 1));
                    if content.is_empty() {
                        content.push(String::new());
                    }
                }
                self.data_provider_mut().restore_content(&content);
                let target = start.min(content.len().saturating_sub(1));
                let _ = self.transition_to_field(target);
                self.move_line_start();
                // The deleted content is gone, so the old anchor is stale (it
                // can sit below the new cursor on an upward selection). Clear the
                // selection; the caller re-establishes a collapsed one at the
                // cursor.
                self.ui_state.selection = SelectionState::None;
                true
            }
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.current_field(), self.cursor_position());
                if anchor == cursor {
                    return self.delete_primary_character_dynamic(yank);
                }
                let start = anchor.min(cursor);
                let end = anchor.max(cursor);
                let lines = self.data_provider().capture_content();
                if start.0 >= lines.len() || end.0 >= lines.len() {
                    return false;
                }

                if yank {
                    let yanked = Self::extract_characterwise_text_core(&lines, start, end);
                    self.behavior_state.yank_mut().set_text_register(yanked);
                }
                self.record_checkpoint(EditKind::Delete);

                let mut content = lines;
                if start.0 == end.0 {
                    content[start.0] = Self::remove_char_range(&content[start.0], start.1, end.1);
                } else {
                    let first: String = content[start.0].chars().take(start.1).collect();
                    let last: String = content[end.0].chars().skip(end.1 + 1).collect();
                    content[start.0] = format!("{first}{last}");
                    content.drain(start.0 + 1..=end.0);
                }
                self.data_provider_mut().restore_content(&content);
                let _ = self.transition_to_field(start.0);
                self.set_cursor_position(start.1);
                self.ui_state.selection = SelectionState::None;
                true
            }
            SelectionState::None => self.delete_primary_character_dynamic(yank),
        }
    }

    /// Delete the character under the cursor. At end-of-field, pull the next row
    /// up (join), matching a text area's forward delete.
    pub(crate) fn delete_primary_character_dynamic(&mut self, yank: bool) -> bool {
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

        if line_idx + 1 < self.data_provider().field_count() {
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

    /// Paste whole rows from the line register as new rows. Leaves the cursor at
    /// the start of the first pasted row.
    pub(crate) fn paste_lines_dynamic(&mut self, after: bool, count: usize, lines: Vec<String>) {
        if lines.is_empty() {
            return;
        }
        self.record_checkpoint(EditKind::Other);

        let repeat = count.max(1);
        let mut insert = Vec::with_capacity(lines.len().saturating_mul(repeat));
        for _ in 0..repeat {
            insert.extend(lines.iter().cloned());
        }

        let mut content = self.data_provider().capture_content();
        let current = self.current_field().min(content.len().saturating_sub(1));
        let insert_at = if after {
            current.saturating_add(1).min(content.len())
        } else {
            current
        };
        content.splice(insert_at..insert_at, insert);
        self.data_provider_mut().restore_content(&content);
        let _ = self.transition_to_field(insert_at.min(content.len().saturating_sub(1)));
        self.move_line_start();
    }

    /// Insert register text at `(field, col)`, splitting on `\n` into new rows.
    /// Returns the cursor landing position.
    pub(crate) fn insert_text_dynamic(
        &mut self,
        field: usize,
        col: usize,
        text: &str,
    ) -> (usize, usize) {
        self.record_checkpoint(EditKind::Other);

        let mut content = self.data_provider().capture_content();
        if content.is_empty() {
            content.push(String::new());
        }
        let field = field.min(content.len().saturating_sub(1));
        let line = &content[field];
        let col = col.min(line.chars().count());
        let prefix: String = line.chars().take(col).collect();
        let suffix: String = line.chars().skip(col).collect();
        let parts: Vec<&str> = text.split('\n').collect();

        let target = if parts.len() == 1 {
            content[field] = format!("{prefix}{}{suffix}", parts[0]);
            (field, col.saturating_add(parts[0].chars().count()))
        } else {
            let mut replacement = Vec::with_capacity(parts.len());
            replacement.push(format!("{prefix}{}", parts[0]));
            for part in &parts[1..parts.len() - 1] {
                replacement.push((*part).to_string());
            }
            let last = parts[parts.len() - 1];
            replacement.push(format!("{last}{suffix}"));
            content.splice(field..=field, replacement);
            (field.saturating_add(parts.len() - 1), last.chars().count())
        };

        self.data_provider_mut().restore_content(&content);
        target
    }
}
