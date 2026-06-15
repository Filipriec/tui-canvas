use crate::{
    canvas::{modes::AppMode, state::SelectionState},
    editor::{features::history::EditKind, paradigm::helix_word::HelixWordTarget},
    textarea::{TextAreaDataProvider, TextAreaState},
};

/// Case transformation applied to a selection by `~`, `` ` ``, and `` Alt-` ``.
#[derive(Clone, Copy)]
pub(crate) enum HelixCase {
    Toggle,
    Lower,
    Upper,
}

/// A Helix command waiting for the next character the user types.
#[derive(Clone, Copy)]
pub(crate) enum HelixPending {
    /// `f`/`F`/`t`/`T`: jump to (or up to) the typed char on the current line.
    Find { till: bool, forward: bool },
    /// `r`: replace every character of the selection with the typed char.
    Replace,
    /// `ms`: wrap the selection in the pair for the typed char.
    SurroundAdd,
    /// `md`: delete the surrounding pair for the typed char.
    SurroundDelete,
    /// `mr`: waiting for the existing pair char to replace.
    SurroundReplaceFrom,
    /// `mr`: existing pair captured; waiting for the replacement pair char.
    SurroundReplaceTo { from: char },
}

/// Map a typed char to its surround pair (open, close). Bracket-likes mirror;
/// everything else surrounds with the literal char on both sides.
fn surround_pair(ch: char) -> (char, char) {
    match ch {
        '(' | ')' => ('(', ')'),
        '[' | ']' => ('[', ']'),
        '{' | '}' => ('{', '}'),
        '<' | '>' => ('<', '>'),
        other => (other, other),
    }
}

/// The most recent find, so `Alt-.` can repeat it.
#[derive(Clone, Copy)]
pub(crate) struct HelixFind {
    pub ch: char,
    pub till: bool,
    pub forward: bool,
}

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn select_next_word_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::NextWordStart);
    }

    pub(crate) fn select_prev_word_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::PrevWordStart);
    }

    pub(crate) fn select_word_end_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::NextWordEnd);
    }

    pub(crate) fn select_word_end_prev_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::PrevWordEnd);
    }

    pub(crate) fn select_next_big_word_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::NextLongWordStart);
    }

    pub(crate) fn select_prev_big_word_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::PrevLongWordStart);
    }

    pub(crate) fn select_big_word_end_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::NextLongWordEnd);
    }

    pub(crate) fn select_big_word_end_prev_helix(&mut self, count: usize) {
        self.core
            .select_word_motion_helix(count, HelixWordTarget::PrevLongWordEnd);
    }

    /// `n` — go to the next search match and select it.
    pub(crate) fn search_next_helix(&mut self, count: usize) {
        self.navigate_search_helix(true, count);
    }

    /// `N` — go to the previous search match and select it.
    pub(crate) fn search_prev_helix(&mut self, count: usize) {
        self.navigate_search_helix(false, count);
    }

    /// Move to the next/previous search match relative to the *active match*
    /// (falling back to the cursor), wrapping around, then select it. Using the
    /// active match as the reference is what makes `N` work: after a match is
    /// selected the cursor sits at the match's end, so a cursor-relative search
    /// would keep re-finding the current match.
    fn navigate_search_helix(&mut self, forward: bool, count: usize) {
        let matches = self.search_matches();
        if matches.is_empty() {
            return;
        }
        let reference = self
            .active_search_match()
            .map(|m| (m.line, m.start))
            .unwrap_or_else(|| (self.current_field(), self.cursor_position()));

        let mut idx = if forward {
            matches
                .iter()
                .position(|m| (m.line, m.start) > reference)
                .unwrap_or(0)
        } else {
            matches
                .iter()
                .rposition(|m| (m.line, m.start) < reference)
                .unwrap_or(matches.len() - 1)
        };
        for _ in 1..count.max(1) {
            idx = if forward {
                (idx + 1) % matches.len()
            } else {
                (idx + matches.len() - 1) % matches.len()
            };
        }

        self.active_search_match = Some(matches[idx]);
        self.select_active_search_match_helix();
    }

    /// Turn the active search match into the Helix primary selection: anchor on
    /// the first matched char, block cursor (head) on the last matched char.
    pub(crate) fn select_active_search_match_helix(&mut self) {
        let Some(m) = self.active_search_match() else {
            return;
        };
        if m.end <= m.start {
            return;
        }
        let _ = self.transition_to_field(m.line);
        let len = self.current_text().chars().count();
        let cursor = m.end.saturating_sub(1).min(len.saturating_sub(1));
        self.core.ui_state.set_cursor(cursor, len, false);
        self.core.ui_state.selection = SelectionState::Characterwise {
            anchor: (m.line, m.start),
        };
    }

    /// `%` — select the whole document.
    pub(crate) fn select_all_helix(&mut self) {
        let field_count = self.core.data_provider().field_count();
        if field_count == 0 {
            return;
        }
        let last = field_count - 1;
        let _ = self.transition_to_field(last);
        let len = self.current_text().chars().count();
        self.core
            .ui_state
            .set_cursor(len.saturating_sub(1), len, false);
        self.core.ui_state.selection = SelectionState::Characterwise { anchor: (0, 0) };
    }

    /// `Alt-;` — flip the selection so anchor and head swap places.
    pub(crate) fn flip_selection_helix(&mut self) {
        match self.selection_state().clone() {
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.current_field(), self.cursor_position());
                if anchor == cursor {
                    return;
                }
                let _ = self.transition_to_field(anchor.0);
                let len = self.current_text().chars().count();
                self.core.ui_state.set_cursor(anchor.1, len, false);
                self.core.ui_state.selection = SelectionState::Characterwise { anchor: cursor };
            }
            SelectionState::Linewise { anchor_field } => {
                let current = self.current_field();
                if anchor_field == current {
                    return;
                }
                let _ = self.transition_to_field(anchor_field);
                self.core.ui_state.selection = SelectionState::Linewise {
                    anchor_field: current,
                };
            }
            SelectionState::None => {}
        }
    }

    /// `~` / `` ` `` / `` Alt-` `` — change the case of the selected text in
    /// place, leaving the selection where it is.
    pub(crate) fn switch_case_selection_helix(&mut self, case: HelixCase) {
        self.map_selection_chars_helix(|c| match case {
            HelixCase::Lower => c.to_lowercase().next().unwrap_or(c),
            HelixCase::Upper => c.to_uppercase().next().unwrap_or(c),
            HelixCase::Toggle => {
                if c.is_uppercase() {
                    c.to_lowercase().next().unwrap_or(c)
                } else if c.is_lowercase() {
                    c.to_uppercase().next().unwrap_or(c)
                } else {
                    c
                }
            }
        });
    }

    /// Apply `map` to every character covered by the current selection, in
    /// place (1:1, so offsets and the selection are preserved).
    fn map_selection_chars_helix(&mut self, map: impl Fn(char) -> char) {
        let (start, end) = self.selection_endpoints();
        let mut lines = self.core.data_provider().capture_content();
        if start.0 >= lines.len() || end.0 >= lines.len() {
            return;
        }

        self.core.record_checkpoint(EditKind::Other);
        for field in start.0..=end.0 {
            let count = lines[field].chars().count();
            if count == 0 {
                continue;
            }
            let (col_start, col_end) = if start.0 == end.0 {
                (start.1, end.1)
            } else if field == start.0 {
                (start.1, count - 1)
            } else if field == end.0 {
                (0, end.1)
            } else {
                (0, count - 1)
            };
            let new_line: String = lines[field]
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i >= col_start && i <= col_end {
                        map(c)
                    } else {
                        c
                    }
                })
                .collect();
            lines[field] = new_line;
        }
        self.core.data_provider_mut().restore_content(&lines);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// `_` — shrink the selection so it excludes leading and trailing
    /// whitespace. A whitespace-only selection is left unchanged.
    pub(crate) fn trim_selection_helix(&mut self) {
        let SelectionState::Characterwise { anchor } = self.selection_state().clone() else {
            return;
        };
        let cursor = (self.current_field(), self.cursor_position());
        let forward = cursor >= anchor;
        let (start, end) = (anchor.min(cursor), anchor.max(cursor));

        // Only single-line selections are trimmed; multi-line trimming would
        // need to pick which line edges to keep.
        if start.0 != end.0 {
            return;
        }

        let line: Vec<char> = self.current_text_for_field(start.0);
        let mut new_start = start.1;
        let mut new_end = end.1.min(line.len().saturating_sub(1));
        while new_start <= new_end && line[new_start].is_whitespace() {
            new_start += 1;
        }
        while new_end > new_start && line[new_end].is_whitespace() {
            new_end -= 1;
        }
        if new_start > new_end || line[new_start].is_whitespace() {
            return; // all whitespace
        }

        let _ = self.transition_to_field(start.0);
        let len = self.current_text().chars().count();
        let (anchor_pos, cursor_pos) = if forward {
            ((start.0, new_start), new_end)
        } else {
            ((start.0, new_end), new_start)
        };
        self.core.ui_state.set_cursor(cursor_pos, len, false);
        self.core.ui_state.selection = SelectionState::Characterwise { anchor: anchor_pos };
    }

    /// `gs` — move to the first non-whitespace character of the current line,
    /// replacing the selection (normal) or extending it (select mode).
    pub(crate) fn goto_first_nonwhitespace_helix(&mut self) {
        let field = self.current_field();
        let line: Vec<char> = self.current_text_for_field(field);
        let target = line.iter().position(|c| !c.is_whitespace()).unwrap_or(0);
        let extend = self.mode() != AppMode::Nor;
        let len = line.len();
        self.core.ui_state.set_cursor(target, len, false);
        if !extend {
            self.core.ui_state.selection = SelectionState::Characterwise {
                anchor: (field, target),
            };
        }
    }

    fn current_text_for_field(&self, field: usize) -> Vec<char> {
        self.core
            .data_provider()
            .field_value(field)
            .chars()
            .collect()
    }

    /// `*` — use the current selection as the search pattern (single line),
    /// without moving. `n`/`N` then navigate the matches.
    pub(crate) fn search_selection_helix(&mut self) {
        let SelectionState::Characterwise { anchor } = self.selection_state().clone() else {
            return;
        };
        let cursor = (self.current_field(), self.cursor_position());
        let (start, end) = (anchor.min(cursor), anchor.max(cursor));
        if start.0 != end.0 {
            return;
        }
        let line = self.current_text_for_field(start.0);
        let pattern: String = line
            .iter()
            .skip(start.1)
            .take(end.1 + 1 - start.1)
            .collect();
        if !pattern.trim().is_empty() {
            self.set_search_query(pattern);
        }
    }

    /// `Alt-:` — ensure the selection faces forward (anchor <= head).
    pub(crate) fn ensure_selection_forward_helix(&mut self) {
        let SelectionState::Characterwise { anchor } = self.selection_state().clone() else {
            return;
        };
        let cursor = (self.current_field(), self.cursor_position());
        if cursor >= anchor {
            return;
        }
        // Cursor is before the anchor: swap so the head sits at the larger end.
        let _ = self.transition_to_field(anchor.0);
        let len = self.current_text().chars().count();
        self.core.ui_state.set_cursor(anchor.1, len, false);
        self.core.ui_state.selection = SelectionState::Characterwise { anchor: cursor };
    }

    /// `>` — indent every line touched by the selection by `INDENT` spaces.
    pub(crate) fn indent_selection_helix(&mut self, count: usize) {
        const INDENT: usize = 4;
        let width = INDENT * count.max(1);
        let (start, end) = self.selection_endpoints();
        let mut content = self.core.data_provider().capture_content();
        if end.0 >= content.len() {
            return;
        }
        self.core.record_checkpoint(EditKind::Other);
        let indent: String = " ".repeat(width);
        let mut deltas = vec![0isize; content.len()];
        for f in start.0..=end.0 {
            if content[f].is_empty() {
                continue;
            }
            content[f] = format!("{indent}{}", content[f]);
            deltas[f] = width as isize;
        }
        self.core.data_provider_mut().restore_content(&content);
        self.shift_selection_columns_helix(&deltas);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// `<` — remove up to `INDENT` leading spaces from each selected line.
    pub(crate) fn unindent_selection_helix(&mut self, count: usize) {
        const INDENT: usize = 4;
        let width = INDENT * count.max(1);
        let (start, end) = self.selection_endpoints();
        let mut content = self.core.data_provider().capture_content();
        if end.0 >= content.len() {
            return;
        }
        self.core.record_checkpoint(EditKind::Other);
        let mut deltas = vec![0isize; content.len()];
        for f in start.0..=end.0 {
            let leading = content[f]
                .chars()
                .take_while(|c| *c == ' ')
                .count()
                .min(width);
            if leading > 0 {
                content[f] = content[f].chars().skip(leading).collect();
                deltas[f] = -(leading as isize);
            }
        }
        self.core.data_provider_mut().restore_content(&content);
        self.shift_selection_columns_helix(&deltas);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// Shift the cursor and characterwise anchor columns by per-line `deltas`
    /// after an indent/unindent so the selection still covers the same text.
    fn shift_selection_columns_helix(&mut self, deltas: &[isize]) {
        let field = self.current_field();
        let cursor = self.cursor_position();
        let anchor = match self.selection_state() {
            SelectionState::Characterwise { anchor } => Some(*anchor),
            _ => None,
        };
        let shift = |f: usize, col: usize| -> usize {
            let d = deltas.get(f).copied().unwrap_or(0);
            (col as isize + d).max(0) as usize
        };
        let len = self.current_text().chars().count();
        self.core
            .ui_state
            .set_cursor(shift(field, cursor).min(len), len, false);
        if let Some(a) = anchor {
            self.core.ui_state.selection = SelectionState::Characterwise {
                anchor: (a.0, shift(a.0, a.1)),
            };
        }
    }

    /// `Ctrl-a` / `Ctrl-x` — change the integer at or after the cursor on the
    /// current line by `delta * count`, selecting the resulting number.
    pub(crate) fn change_number_helix(&mut self, delta: i64, count: usize) {
        let field = self.current_field();
        let line = self.current_text_for_field(field);
        let n = line.len();
        let cursor = self.cursor_position();

        let mut i = 0;
        while i < n {
            if !line[i].is_ascii_digit() {
                i += 1;
                continue;
            }
            let mut end = i;
            while end + 1 < n && line[end + 1].is_ascii_digit() {
                end += 1;
            }
            if end >= cursor {
                let neg = i > 0 && line[i - 1] == '-';
                let num_start = if neg { i - 1 } else { i };
                let text: String = line[num_start..=end].iter().collect();
                if let Ok(val) = text.parse::<i64>() {
                    let new_val = val.saturating_add(delta.saturating_mul(count.max(1) as i64));
                    let new_str = new_val.to_string();
                    let mut new_line: Vec<char> = Vec::with_capacity(n);
                    new_line.extend_from_slice(&line[..num_start]);
                    new_line.extend(new_str.chars());
                    new_line.extend_from_slice(&line[end + 1..]);
                    self.core.record_checkpoint(EditKind::Other);
                    self.core
                        .data_provider_mut()
                        .set_field_value(field, new_line.iter().collect());
                    let new_count = new_str.chars().count();
                    let new_end = num_start + new_count - 1;
                    let len = self.current_text().chars().count();
                    self.core.ui_state.set_cursor(new_end, len, false);
                    self.core.ui_state.selection = SelectionState::Characterwise {
                        anchor: (field, num_start),
                    };
                    #[cfg(feature = "gui")]
                    {
                        self.edited_this_frame = true;
                    }
                    return;
                }
            }
            i = end + 1;
        }
    }

    /// `Ctrl-w` (insert) — delete the word before the cursor.
    pub(crate) fn delete_word_backward_helix(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_word_start;
        let field = self.current_field();
        let text = self.core.data_provider().field_value(field).to_string();
        let cursor = self.cursor_position();
        if cursor == 0 {
            return;
        }
        let start = find_prev_word_start(&text, cursor).min(cursor);
        if start == cursor {
            return;
        }
        self.core.record_checkpoint(EditKind::Delete);
        let new_line: String = text
            .chars()
            .take(start)
            .chain(text.chars().skip(cursor))
            .collect();
        self.core
            .data_provider_mut()
            .set_field_value(field, new_line);
        let len = self.current_text().chars().count();
        self.core.ui_state.set_cursor(start, len, false);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// `Ctrl-u` (insert) — delete from the line start to the cursor.
    pub(crate) fn delete_to_line_start_helix(&mut self) {
        let field = self.current_field();
        let text = self.core.data_provider().field_value(field).to_string();
        let cursor = self.cursor_position();
        if cursor == 0 {
            return;
        }
        self.core.record_checkpoint(EditKind::Delete);
        let new_line: String = text.chars().skip(cursor).collect();
        self.core
            .data_provider_mut()
            .set_field_value(field, new_line);
        let len = self.current_text().chars().count();
        self.core.ui_state.set_cursor(0, len, false);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// `Alt-d` (insert) — delete the word after the cursor.
    pub(crate) fn delete_word_forward_helix(&mut self) {
        use crate::canvas::actions::movement::word::find_next_word_start;
        let field = self.current_field();
        let text = self.core.data_provider().field_value(field).to_string();
        let cursor = self.cursor_position();
        let count = text.chars().count();
        if cursor >= count {
            return;
        }
        let end = find_next_word_start(&text, cursor)
            .max(cursor + 1)
            .min(count);
        self.core.record_checkpoint(EditKind::Delete);
        let new_line: String = text
            .chars()
            .take(cursor)
            .chain(text.chars().skip(end))
            .collect();
        self.core
            .data_provider_mut()
            .set_field_value(field, new_line);
        let len = self.current_text().chars().count();
        self.core.ui_state.set_cursor(cursor.min(len), len, false);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// Arm a command that needs the next typed character (`f`/`t`/`r`/…).
    pub(crate) fn set_helix_pending(&mut self, pending: HelixPending) {
        self.helix_pending = Some(pending);
    }

    /// Resolve a pending command with the character the user just typed.
    pub(crate) fn resolve_helix_pending(&mut self, pending: HelixPending, ch: char) {
        match pending {
            HelixPending::Find { till, forward } => {
                self.helix_last_find = Some(HelixFind { ch, till, forward });
                self.find_char_helix(ch, till, forward);
            }
            HelixPending::Replace => self.replace_selection_with_char_helix(ch),
            HelixPending::SurroundAdd => self.surround_add_helix(ch),
            HelixPending::SurroundDelete => self.surround_delete_helix(ch),
            HelixPending::SurroundReplaceFrom => {
                // Capture the existing pair, then wait for the replacement.
                self.helix_pending = Some(HelixPending::SurroundReplaceTo { from: ch });
            }
            HelixPending::SurroundReplaceTo { from } => self.surround_replace_helix(from, ch),
        }
    }

    /// `Alt-.` — repeat the last find/till motion.
    pub(crate) fn repeat_last_find_helix(&mut self) {
        if let Some(find) = self.helix_last_find {
            self.find_char_helix(find.ch, find.till, find.forward);
        }
    }

    /// `f`/`F`/`t`/`T` — move to (or just before/after) `ch` on the current
    /// line, replacing the selection (normal) or extending it (select mode).
    pub(crate) fn find_char_helix(&mut self, ch: char, till: bool, forward: bool) {
        let field = self.current_field();
        let line = self.current_text_for_field(field);
        let len = line.len();
        let cursor = self.cursor_position();

        let found = if forward {
            ((cursor + 1)..len).find(|&i| line[i] == ch)
        } else {
            (0..cursor).rev().find(|&i| line[i] == ch)
        };
        let Some(hit) = found else {
            return;
        };
        let target = if till {
            if forward {
                hit.saturating_sub(1)
            } else {
                hit + 1
            }
        } else {
            hit
        };
        if target == cursor {
            return;
        }

        let extend = self.mode() != AppMode::Nor;
        let anchor = match self.selection_state() {
            SelectionState::Characterwise { anchor } if extend => *anchor,
            _ => (field, cursor),
        };
        self.core.ui_state.set_cursor(target, len, false);
        self.core.ui_state.selection = SelectionState::Characterwise { anchor };
    }

    /// `r` — replace every character of the selection with `ch`.
    pub(crate) fn replace_selection_with_char_helix(&mut self, ch: char) {
        self.map_selection_chars_helix(|_| ch);
    }

    /// `ms<char>` — wrap the selection in the pair for `ch`.
    pub(crate) fn surround_add_helix(&mut self, ch: char) {
        let (open, close) = surround_pair(ch);
        let (start, end) = self.selection_endpoints();
        let mut content = self.core.data_provider().capture_content();
        if start.0 >= content.len() || end.0 >= content.len() {
            return;
        }
        self.core.record_checkpoint(EditKind::Other);

        // Insert the closing char first (later position) so the opening insert
        // doesn't shift it.
        let mut end_line: Vec<char> = content[end.0].chars().collect();
        let close_at = (end.1 + 1).min(end_line.len());
        end_line.insert(close_at, close);
        content[end.0] = end_line.into_iter().collect();

        let mut start_line: Vec<char> = content[start.0].chars().collect();
        let open_at = start.1.min(start_line.len());
        start_line.insert(open_at, open);
        content[start.0] = start_line.into_iter().collect();

        self.core.data_provider_mut().restore_content(&content);

        // The opening char shifts everything at/after `start` on the start line.
        let bump = |pos: (usize, usize)| -> (usize, usize) {
            if pos.0 == start.0 && pos.1 >= start.1 {
                (pos.0, pos.1 + 1)
            } else {
                pos
            }
        };
        let cursor = bump((self.current_field(), self.cursor_position()));
        let anchor = match self.selection_state() {
            SelectionState::Characterwise { anchor } => bump(*anchor),
            _ => cursor,
        };
        let _ = self.transition_to_field(cursor.0);
        let len = self.current_text().chars().count();
        self.core.ui_state.set_cursor(cursor.1, len, false);
        self.core.ui_state.selection = SelectionState::Characterwise { anchor };
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// `md<char>` — remove the nearest surrounding pair for `ch` on the line.
    pub(crate) fn surround_delete_helix(&mut self, ch: char) {
        let (open, close) = surround_pair(ch);
        let field = self.current_field();
        let line = self.current_text_for_field(field);
        let cursor = self.cursor_position().min(line.len().saturating_sub(1));

        let Some((open_idx, close_idx)) = find_surround_pair(&line, cursor, open, close) else {
            return;
        };
        self.core.record_checkpoint(EditKind::Other);
        let mut new_line = line.clone();
        new_line.remove(close_idx);
        new_line.remove(open_idx);
        self.core
            .data_provider_mut()
            .set_field_value(field, new_line.into_iter().collect());

        // Removing the opening char (before the cursor) shifts the cursor left.
        let new_cursor = self.cursor_position().saturating_sub(1);
        let len = self.current_text().chars().count();
        self.core
            .ui_state
            .set_cursor(new_cursor.min(len), len, false);
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// `mr<from><to>` — replace the surrounding `from` pair with the `to` pair.
    pub(crate) fn surround_replace_helix(&mut self, from: char, to: char) {
        let (from_open, from_close) = surround_pair(from);
        let (to_open, to_close) = surround_pair(to);
        let field = self.current_field();
        let line = self.current_text_for_field(field);
        let cursor = self.cursor_position().min(line.len().saturating_sub(1));

        let Some((open_idx, close_idx)) = find_surround_pair(&line, cursor, from_open, from_close)
        else {
            return;
        };
        self.core.record_checkpoint(EditKind::Other);
        let mut new_line = line.clone();
        new_line[open_idx] = to_open;
        new_line[close_idx] = to_close;
        self.core
            .data_provider_mut()
            .set_field_value(field, new_line.into_iter().collect());
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    /// `mm` — jump to the bracket matching the one at the cursor, searching the
    /// whole document. Replaces the selection (normal) or extends it (select).
    pub(crate) fn match_brackets_helix(&mut self) {
        const PAIRS: &[(char, char)] = &[('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];

        let field_count = self.core.data_provider().field_count();
        if field_count == 0 {
            return;
        }
        let mut chars: Vec<char> = Vec::new();
        let mut field_starts: Vec<usize> = Vec::with_capacity(field_count);
        for f in 0..field_count {
            field_starts.push(chars.len());
            chars.extend(self.core.data_provider().field_value(f).chars());
            if f + 1 < field_count {
                chars.push('\n');
            }
        }
        let cursor_flat = field_starts[self.current_field()] + self.cursor_position();
        let Some(&here) = chars.get(cursor_flat) else {
            return;
        };

        let target = if let Some((_, close)) = PAIRS.iter().find(|(o, _)| *o == here) {
            // Open bracket: scan forward for the matching close.
            let mut depth = 0i32;
            let mut found = None;
            for (i, &c) in chars.iter().enumerate().skip(cursor_flat) {
                if c == here {
                    depth += 1;
                } else if c == *close {
                    depth -= 1;
                    if depth == 0 {
                        found = Some(i);
                        break;
                    }
                }
            }
            found
        } else if let Some((open, _)) = PAIRS.iter().find(|(_, c)| *c == here) {
            // Close bracket: scan backward for the matching open.
            let mut depth = 0i32;
            let mut found = None;
            for i in (0..=cursor_flat).rev() {
                let c = chars[i];
                if c == here {
                    depth += 1;
                } else if c == *open {
                    depth -= 1;
                    if depth == 0 {
                        found = Some(i);
                        break;
                    }
                }
            }
            found
        } else {
            None
        };

        let Some(target_flat) = target else {
            return;
        };
        let (tf, tc) = flat_to_field(&chars, &field_starts, target_flat);
        let extend = self.mode() != AppMode::Nor;
        let _ = self.transition_to_field(tf);
        let len = self.current_text().chars().count();
        self.core.ui_state.set_cursor(tc, len, false);
        if !extend {
            self.core.ui_state.selection = SelectionState::Characterwise { anchor: (tf, tc) };
        }
    }

    pub(crate) fn delete_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once(yank) {
                break;
            }
        }
        self.core.finish_helix_selection_edit();
    }

    pub(crate) fn change_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once(yank) {
                break;
            }
        }
        self.enter_edit_mode_helix();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
    }

    pub(crate) fn yank_primary_selection_helix(&mut self) {
        self.core.yank_primary_selection_helix();
    }

    pub(crate) fn extend_line_below_helix(&mut self) {
        self.core.extend_line_below_helix();
    }

    pub(crate) fn extend_to_line_bounds_helix(&mut self) {
        self.core.extend_to_line_bounds_helix();
    }
}

/// Find the nearest surrounding `(open, close)` pair around `cursor` on a line.
/// Returns the open/close indices, scanning left for the open and right for the
/// close. Handles same-char pairs (quotes) too.
fn find_surround_pair(
    line: &[char],
    cursor: usize,
    open: char,
    close: char,
) -> Option<(usize, usize)> {
    if line.is_empty() {
        return None;
    }
    let cursor = cursor.min(line.len() - 1);
    let open_idx = (0..=cursor).rev().find(|&i| line[i] == open)?;
    // For a same-char pair, the close must be strictly after the open.
    let close_start = if open == close {
        open_idx + 1
    } else {
        cursor.max(open_idx)
    };
    let close_idx = (close_start..line.len()).find(|&i| line[i] == close)?;
    if open_idx < close_idx {
        Some((open_idx, close_idx))
    } else {
        None
    }
}

/// Map a flat char index in the document buffer back to a `(field, char)`
/// position. An index that lands on a `\n` separator is clamped to the last
/// real char of that line (or 0 for an empty line).
fn flat_to_field(chars: &[char], field_starts: &[usize], flat: usize) -> (usize, usize) {
    let field_count = field_starts.len();
    for f in 0..field_count {
        let start = field_starts[f];
        // Field f's chars occupy [start, next_start - 1), with a `\n` at
        // next_start - 1 (except the last field, which runs to the buffer end).
        let field_end = if f + 1 < field_count {
            field_starts[f + 1].saturating_sub(1)
        } else {
            chars.len()
        };
        if flat < field_end {
            return (f, flat - start);
        }
        if flat == field_end {
            // On the `\n` separator: clamp to this line's last real char.
            return (f, (field_end - start).saturating_sub(1));
        }
    }
    let last = field_count - 1;
    let start = field_starts[last];
    (last, chars.len().saturating_sub(start).saturating_sub(1))
}
