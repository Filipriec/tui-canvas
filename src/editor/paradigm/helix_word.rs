use crate::{
    canvas::{modes::AppMode, state::SelectionState},
    editor::EditorCore,
    DataProvider,
};

/// Which Helix word motion a selection step performs. These mirror Helix's
/// `WordMotionTarget` variants and drive the faithful Helix word-motion port.
#[derive(Clone, Copy)]
pub(crate) enum HelixWordTarget {
    NextWordStart,
    NextWordEnd,
    PrevWordStart,
    PrevWordEnd,
    NextLongWordStart,
    NextLongWordEnd,
    PrevLongWordStart,
    PrevLongWordEnd,
}

impl<D: DataProvider> EditorCore<D> {
    /// Apply a Helix word motion `count` times.
    ///
    /// Each step runs a faithful port of Helix's `word_move`/`range_to_target`
    /// over the whole document flattened with `\n` between fields, then maps the
    /// result back to a `(field, char)` position. Running over the flattened
    /// buffer means line breaks are handled exactly like Helix (a `w` at the end
    /// of a line crosses into the next line's first word), instead of the old
    /// per-field fallback that re-anchored onto the previous line.
    ///
    /// Normal mode replaces the whole selection with the moved-over range;
    /// select/extend mode (`v`) keeps the anchor pinned and only advances the
    /// head. Both share the same head computation so the two modes stay
    /// consistent.
    pub(crate) fn select_word_motion_helix(&mut self, count: usize, target: HelixWordTarget) {
        let extend = self.mode() != AppMode::Nor;
        for _ in 0..count.max(1) {
            self.select_word_motion_step_helix(target, extend);
        }
    }

    fn select_word_motion_step_helix(&mut self, target: HelixWordTarget, extend: bool) {
        let field_count = self.data_provider().field_count();
        if field_count == 0 {
            return;
        }

        // Flatten the document into one char buffer with `\n` between fields,
        // recording where each field starts so positions can be mapped back.
        let mut chars: Vec<char> = Vec::new();
        let mut field_starts: Vec<usize> = Vec::with_capacity(field_count);
        for f in 0..field_count {
            field_starts.push(chars.len());
            chars.extend(self.data_provider().field_value(f).chars());
            if f + 1 < field_count {
                chars.push('\n');
            }
        }
        let len = chars.len();
        if len == 0 {
            return;
        }

        let field = self.current_field();
        let cursor = self.cursor_position();
        let to_flat = |pos: (usize, usize)| field_starts[pos.0] + pos.1;

        let flat_cursor = to_flat((field, cursor));
        let pinned_anchor = match self.selection_state() {
            SelectionState::Characterwise { anchor } => *anchor,
            _ => (field, cursor),
        };
        let flat_anchor = to_flat(pinned_anchor);

        // Map our inclusive (anchor, cursor) selection to Helix's (anchor, head)
        // gap range. `head` is exclusive: just past the block-cursor char for a
        // forward selection, on the block-cursor char for a backward one.
        let input = if flat_cursor >= flat_anchor {
            HelixRange {
                anchor: flat_anchor,
                head: flat_cursor + 1,
            }
        } else {
            HelixRange {
                anchor: flat_anchor + 1,
                head: flat_cursor,
            }
        };

        let result = helix_word_move(&chars, input, target);

        // Convert Helix's exclusive-head range back to inclusive endpoints.
        let (motion_anchor, new_cursor) = if result.anchor < result.head {
            (result.anchor, result.head - 1)
        } else if result.head < result.anchor {
            (result.anchor - 1, result.head)
        } else {
            let c = result.head.min(len - 1);
            (c, c)
        };

        // Nothing moved (already at the document edge).
        if new_cursor == flat_cursor && (extend || motion_anchor == flat_anchor) {
            return;
        }

        let (cur_field, cur_char) = flat_to_field(&chars, &field_starts, new_cursor);
        let _ = self.transition_to_field(cur_field);
        let field_len = self.current_text().chars().count();
        self.ui_state.set_cursor(cur_char, field_len, false);

        // Extend keeps the pinned anchor; replace adopts the motion's anchor.
        let anchor = if extend {
            pinned_anchor
        } else {
            flat_to_field(&chars, &field_starts, motion_anchor)
        };
        self.ui_state.selection = SelectionState::Characterwise { anchor };
    }
}

/// A Helix selection range expressed as gap indices, where `head` is exclusive
/// of the block-cursor character. Mirrors `helix_core::selection::Range`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct HelixRange {
    anchor: usize,
    head: usize,
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

impl HelixWordTarget {
    fn is_prev(self) -> bool {
        matches!(
            self,
            HelixWordTarget::PrevWordStart
                | HelixWordTarget::PrevWordEnd
                | HelixWordTarget::PrevLongWordStart
                | HelixWordTarget::PrevLongWordEnd
        )
    }

    fn is_long(self) -> bool {
        matches!(
            self,
            HelixWordTarget::NextLongWordStart
                | HelixWordTarget::NextLongWordEnd
                | HelixWordTarget::PrevLongWordStart
                | HelixWordTarget::PrevLongWordEnd
        )
    }

    /// Whether the target boundary is reached "after" the previous char (word
    /// starts) rather than "before" the next char (word ends).
    fn stops_at_word_start(self) -> bool {
        matches!(
            self,
            HelixWordTarget::NextWordStart
                | HelixWordTarget::PrevWordEnd
                | HelixWordTarget::NextLongWordStart
                | HelixWordTarget::PrevLongWordEnd
        )
    }
}

#[derive(PartialEq, Clone, Copy)]
enum CharClass {
    Eol,
    Whitespace,
    Word,
    Punctuation,
}

fn char_is_line_ending(c: char) -> bool {
    c == '\n' || c == '\r'
}

fn classify(c: char) -> CharClass {
    if char_is_line_ending(c) {
        CharClass::Eol
    } else if c.is_whitespace() {
        CharClass::Whitespace
    } else if c.is_alphanumeric() {
        CharClass::Word
    } else {
        CharClass::Punctuation
    }
}

/// Short-word boundary: every character class is distinct.
fn is_word_boundary(a: char, b: char) -> bool {
    classify(a) != classify(b)
}

/// Long-word ("WORD") boundary: word and punctuation are grouped together, so
/// only transitions to/from whitespace or end-of-line count.
fn is_long_word_boundary(a: char, b: char) -> bool {
    match (classify(a), classify(b)) {
        (CharClass::Word, CharClass::Punctuation) | (CharClass::Punctuation, CharClass::Word) => {
            false
        }
        (x, y) => x != y,
    }
}

fn reached_target(target: HelixWordTarget, prev: char, next: char) -> bool {
    let boundary = if target.is_long() {
        is_long_word_boundary(prev, next)
    } else {
        is_word_boundary(prev, next)
    };
    if !boundary {
        return false;
    }
    // A word start stops where the next char begins a word or a line ends; a
    // word end stops where the previous char ends one. `Eol` is its own class,
    // so it is never treated as plain whitespace here (this is what lets a
    // motion cross a line break instead of stalling on it).
    if target.stops_at_word_start() {
        classify(next) != CharClass::Whitespace
    } else {
        classify(prev) != CharClass::Whitespace
    }
}

/// Bidirectional char cursor over a slice positioned at a gap, mirroring the
/// semantics of `ropey`'s `chars_at` cursor used by Helix's `range_to_target`.
struct CharCursor<'a> {
    chars: &'a [char],
    pos: usize,
    reversed: bool,
}

impl<'a> CharCursor<'a> {
    fn new(chars: &'a [char], pos: usize) -> Self {
        Self {
            chars,
            pos,
            reversed: false,
        }
    }

    fn reverse(&mut self) {
        self.reversed = !self.reversed;
    }

    fn next(&mut self) -> Option<char> {
        if self.reversed {
            if self.pos == 0 {
                return None;
            }
            self.pos -= 1;
            self.chars.get(self.pos).copied()
        } else {
            let ch = self.chars.get(self.pos).copied();
            if ch.is_some() {
                self.pos += 1;
            }
            ch
        }
    }

    fn prev(&mut self) -> Option<char> {
        if self.reversed {
            let ch = self.chars.get(self.pos).copied();
            if ch.is_some() {
                self.pos += 1;
            }
            ch
        } else {
            if self.pos == 0 {
                return None;
            }
            self.pos -= 1;
            self.chars.get(self.pos).copied()
        }
    }
}

/// Port of Helix's `range_to_target`. Walks from `origin.head` toward `target`
/// over a buffer that may contain `\n` line separators, returning the resulting
/// `(anchor, head)` gap range.
fn range_to_target(chars: &[char], target: HelixWordTarget, origin: HelixRange) -> HelixRange {
    let is_prev = target.is_prev();
    let mut cursor = CharCursor::new(chars, origin.head);
    if is_prev {
        cursor.reverse();
    }

    let advance = |head: &mut usize| {
        if is_prev {
            *head = head.saturating_sub(1);
        } else {
            *head += 1;
        }
    };

    let mut anchor = origin.anchor;
    let mut head = origin.head;

    // Peek the character just behind the head without moving the cursor.
    let mut prev_ch = {
        let ch = cursor.prev();
        if ch.is_some() {
            cursor.next();
        }
        ch
    };

    // Skip over any line endings at the head, advancing past them. When the
    // head started on a line ending, re-anchor just past it so the resulting
    // selection lives on the destination line rather than spanning the break.
    while let Some(ch) = cursor.next() {
        if char_is_line_ending(ch) {
            prev_ch = Some(ch);
            advance(&mut head);
        } else {
            cursor.prev();
            break;
        }
    }
    if prev_ch.map(char_is_line_ending).unwrap_or(false) {
        anchor = head;
    }

    let head_start = head;
    while let Some(next_ch) = cursor.next() {
        if prev_ch.is_none() || reached_target(target, prev_ch.unwrap(), next_ch) {
            if head == head_start {
                // Boundary at the very first step: skip it, re-anchoring here.
                anchor = head;
            } else {
                break;
            }
        }
        prev_ch = Some(next_ch);
        advance(&mut head);
    }

    HelixRange { anchor, head }
}

/// Port of Helix's `word_move`: prepares the start range to honor block-cursor
/// semantics, then advances toward `target` once.
fn helix_word_move(chars: &[char], range: HelixRange, target: HelixWordTarget) -> HelixRange {
    let is_prev = target.is_prev();
    let len = chars.len();

    // Early-out when there is nowhere left to move.
    if (is_prev && range.head == 0) || (!is_prev && range.head == len) {
        return range;
    }

    let start_range = if is_prev {
        if range.anchor < range.head {
            HelixRange {
                anchor: range.head,
                head: range.head.saturating_sub(1),
            }
        } else {
            HelixRange {
                anchor: (range.head + 1).min(len),
                head: range.head,
            }
        }
    } else if range.anchor < range.head {
        HelixRange {
            anchor: range.head.saturating_sub(1),
            head: range.head,
        }
    } else {
        HelixRange {
            anchor: range.head,
            head: (range.head + 1).min(len),
        }
    };

    let next = range_to_target(chars, target, start_range);
    if next == start_range {
        range
    } else {
        next
    }
}
