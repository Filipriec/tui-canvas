// src/editor/paste.rs
//! Shared paste engine. Like the selection engine, the only difference between a
//! text area and a text form is structural and is driven by
//! [`EditorCore::row_policy`]:
//!
//! - [`crate::RowPolicy::Dynamic`] — pasted rows are inserted and multi-line
//!   text splits the current row into new rows.
//! - [`crate::RowPolicy::Fixed`] — pasted content is written into the existing
//!   slots in place (clamped to the row count); nothing shifts and no rows are
//!   created.
//!
//! Paradigm-specific concerns (where to paste, mode/selection cleanup) stay in
//! the product wrappers; only the buffer transform lives here.

#![cfg(feature = "keybindings")]

use crate::editor::features::history::EditKind;
use crate::editor::EditorCore;
use crate::{DataProvider, RowPolicy};

impl<D: DataProvider> EditorCore<D> {
    /// Paste whole rows from the line register, repeated `count` times. Leaves
    /// the cursor at the start of the first pasted row.
    pub(crate) fn paste_register_lines_core(
        &mut self,
        after: bool,
        count: usize,
        lines: Vec<String>,
    ) {
        if lines.is_empty() {
            return;
        }
        let repeat = count.max(1);
        let mut insert = Vec::with_capacity(lines.len().saturating_mul(repeat));
        for _ in 0..repeat {
            insert.extend(lines.iter().cloned());
        }
        self.record_checkpoint(EditKind::Other);

        match self.row_policy() {
            RowPolicy::Dynamic => {
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
            RowPolicy::Fixed => {
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
                for (offset, line) in insert.into_iter().enumerate() {
                    let field = start.saturating_add(offset);
                    if field >= field_count {
                        break;
                    }
                    self.data_provider_mut().set_field_value(field, line);
                }
                let _ = self.transition_to_field(start);
                self.move_line_start();
            }
        }
    }

    /// Insert register text at `(field, col)`. Dynamic splits on `\n` into new
    /// rows; Fixed distributes the lines across existing slots in place. Returns
    /// the `(field, col)` the cursor should land on.
    pub(crate) fn insert_register_text_core(
        &mut self,
        field: usize,
        col: usize,
        text: &str,
    ) -> (usize, usize) {
        self.record_checkpoint(EditKind::Other);

        match self.row_policy() {
            RowPolicy::Dynamic => {
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
            RowPolicy::Fixed => {
                let field_count = self.data_provider().field_count();
                if field_count == 0 {
                    return (field, col);
                }
                let field = field.min(field_count - 1);
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

                // Distribute across following slots without creating rows.
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
    }
}

/// Join register lines with `\n` and repeat the result `count` times.
pub(crate) fn repeated_text(lines: &[String], count: usize) -> String {
    let repeat = count.max(1);
    let text = lines.join("\n");
    let mut pasted = String::new();
    for i in 0..repeat {
        if i > 0 {
            pasted.push('\n');
        }
        pasted.push_str(&text);
    }
    pasted
}
