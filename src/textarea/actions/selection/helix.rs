use crate::{
    canvas::{modes::AppMode, state::SelectionState},
    textarea::{TextAreaDataProvider, TextAreaState},
};

/// Which Helix word motion a selection step performs. These mirror Helix's
/// `WordMotionTarget` variants and drive the faithful port in [`helix_word`].
#[derive(Clone, Copy)]
enum HelixWordTarget {
    NextWordStart,
    NextWordEnd,
    PrevWordStart,
    PrevWordEnd,
    NextLongWordStart,
    NextLongWordEnd,
    PrevLongWordStart,
    PrevLongWordEnd,
}

impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// Apply a Helix word motion `count` times, replacing the primary selection
    /// with the moved-over range (Helix normal mode), or simply moving when not
    /// in normal mode.
    ///
    /// In normal mode each step is computed with a faithful port of Helix's
    /// `word_move`/`range_to_target` over the current field. This keeps the
    /// selection in sync with the block cursor across repeated motions (the
    /// previous `±1` patch moved the *real* cursor onto the trailing boundary
    /// char, so a second `w` could not advance). When a step reaches the field
    /// boundary we fall back to the cross-field vim motion.
    fn select_word_motion_helix(
        &mut self,
        count: usize,
        target: HelixWordTarget,
        mut motion: impl FnMut(&mut Self),
    ) {
        if self.mode() != AppMode::Nor {
            for _ in 0..count.max(1) {
                motion(self);
            }
            return;
        }

        for _ in 0..count.max(1) {
            self.select_word_motion_step_helix(target, &mut motion);
        }
    }

    fn select_word_motion_step_helix(
        &mut self,
        target: HelixWordTarget,
        motion: &mut impl FnMut(&mut Self),
    ) {
        let field = self.current_field();
        let cursor = self.cursor_position();

        // Recover the current selection's anchor within this field. If the
        // anchor lives in another field (a previous cross-field motion) we
        // collapse to the cursor and let the motion re-anchor here.
        let anchor_char = match self.selection_state() {
            SelectionState::Characterwise { anchor: (af, ac) } if *af == field => *ac,
            _ => cursor,
        };

        let chars: Vec<char> = self.current_text().chars().collect();
        let len = chars.len();
        if len == 0 {
            // Empty field: nothing to select; defer to the cross-field motion.
            let from = (field, cursor);
            motion(self);
            self.editor.ui_state.selection = SelectionState::Characterwise { anchor: from };
            return;
        }

        // Map our inclusive (anchor_char, cursor) selection to Helix's
        // (anchor, head) gap range. `head` is exclusive: it sits just past the
        // block-cursor char for a forward selection, and on the block-cursor
        // char for a backward one.
        let input = if cursor >= anchor_char {
            HelixRange {
                anchor: anchor_char,
                head: cursor + 1,
            }
        } else {
            HelixRange {
                anchor: anchor_char + 1,
                head: cursor,
            }
        };

        let result = helix_word_move(&chars, input, target);

        // Convert Helix's exclusive-head range back to our inclusive endpoints.
        let (new_anchor, new_cursor) = if result.anchor < result.head {
            (result.anchor, result.head - 1)
        } else if result.head < result.anchor {
            (result.anchor - 1, result.head)
        } else {
            let c = result.head.min(len - 1);
            (c, c)
        };

        let made_progress = new_cursor != cursor || new_anchor != anchor_char;
        if !made_progress {
            // At the field boundary: cross fields with the vim motion, keeping
            // the original position as the selection anchor.
            let from = (field, cursor);
            motion(self);
            if (self.current_field(), self.cursor_position()) != from {
                self.editor.ui_state.selection = SelectionState::Characterwise { anchor: from };
            }
            return;
        }

        self.editor.ui_state.set_cursor(new_cursor, len, false);
        self.editor.ui_state.selection = SelectionState::Characterwise {
            anchor: (field, new_anchor),
        };
    }

    pub(crate) fn select_next_word_helix(&mut self, count: usize) {
        self.select_word_motion_helix(count, HelixWordTarget::NextWordStart, Self::move_word_next);
    }

    pub(crate) fn select_prev_word_helix(&mut self, count: usize) {
        self.select_word_motion_helix(count, HelixWordTarget::PrevWordStart, Self::move_word_prev);
    }

    pub(crate) fn select_word_end_helix(&mut self, count: usize) {
        self.select_word_motion_helix(count, HelixWordTarget::NextWordEnd, Self::move_word_end);
    }

    pub(crate) fn select_word_end_prev_helix(&mut self, count: usize) {
        self.select_word_motion_helix(
            count,
            HelixWordTarget::PrevWordEnd,
            Self::move_word_end_prev,
        );
    }

    pub(crate) fn select_next_big_word_helix(&mut self, count: usize) {
        self.select_word_motion_helix(
            count,
            HelixWordTarget::NextLongWordStart,
            Self::move_big_word_next,
        );
    }

    pub(crate) fn select_prev_big_word_helix(&mut self, count: usize) {
        self.select_word_motion_helix(
            count,
            HelixWordTarget::PrevLongWordStart,
            Self::move_big_word_prev,
        );
    }

    pub(crate) fn select_big_word_end_helix(&mut self, count: usize) {
        self.select_word_motion_helix(
            count,
            HelixWordTarget::NextLongWordEnd,
            Self::move_big_word_end,
        );
    }

    pub(crate) fn select_big_word_end_prev_helix(&mut self, count: usize) {
        self.select_word_motion_helix(
            count,
            HelixWordTarget::PrevLongWordEnd,
            Self::move_big_word_end_prev,
        );
    }

    pub(crate) fn delete_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once(yank) {
                break;
            }
        }
        if self.mode() == AppMode::Nor {
            self.ensure_helix_primary_selection();
        }
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
        self.yank_selection();
        if self.mode() == AppMode::Sel {
            self.exit_highlight_mode_helix();
        }
    }

    pub(crate) fn collapse_selection_helix(&mut self) {
        self.collapse_helix_selection_to_cursor();
    }

    pub(crate) fn extend_line_below_helix(&mut self) {
        let current = self.current_field();
        match self.selection_state().clone() {
            SelectionState::Linewise { anchor_field } if anchor_field == current => {
                let next = current.saturating_add(1);
                if next < self.editor.data_provider().field_count() {
                    let _ = self.transition_to_field(next);
                    self.ui_state.current_mode = AppMode::Nor;
                    self.ui_state.selection = SelectionState::Linewise { anchor_field };
                }
            }
            _ => {
                self.ui_state.current_mode = AppMode::Nor;
                self.ui_state.selection = SelectionState::Linewise { anchor_field: current };
            }
        }
    }

    pub(crate) fn extend_to_line_bounds_helix(&mut self) {
        let current = self.current_field();
        self.ui_state.current_mode = AppMode::Nor;
        self.ui_state.selection = SelectionState::Linewise { anchor_field: current };
    }
}

/// A Helix selection range expressed as gap indices, where `head` is exclusive
/// of the block-cursor character. Mirrors `helix_core::selection::Range`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct HelixRange {
    anchor: usize,
    head: usize,
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
    Whitespace,
    Word,
    Punctuation,
}

fn classify(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Whitespace
    } else if c.is_alphanumeric() {
        CharClass::Word
    } else {
        CharClass::Punctuation
    }
}

fn reached_target(target: HelixWordTarget, prev: char, next: char) -> bool {
    let boundary = if target.is_long() {
        prev.is_whitespace() != next.is_whitespace()
    } else {
        classify(prev) != classify(next)
    };
    if !boundary {
        return false;
    }
    if target.stops_at_word_start() {
        !next.is_whitespace()
    } else {
        !prev.is_whitespace()
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

/// Port of Helix's `range_to_target`, specialized to a single field (no line
/// endings). Walks from `origin.head` toward `target`, returning the resulting
/// `(anchor, head)` gap range.
fn range_to_target(chars: &[char], target: HelixWordTarget, origin: HelixRange) -> HelixRange {
    let is_prev = target.is_prev();
    let mut cursor = CharCursor::new(chars, origin.head);
    if is_prev {
        cursor.reverse();
    }

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
        if is_prev {
            head = head.saturating_sub(1);
        } else {
            head += 1;
        }
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
