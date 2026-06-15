// src/canvas/actions/movement/word.rs
//! Word-based cursor movement with vim-like semantics

#[derive(PartialEq, Copy, Clone)]
enum CharType {
    Whitespace,
    Alphanumeric,
    Punctuation,
}

fn get_char_type(c: char) -> CharType {
    if c.is_whitespace() {
        CharType::Whitespace
    } else if c.is_alphanumeric() {
        CharType::Alphanumeric
    } else {
        CharType::Punctuation
    }
}

fn last_non_whitespace_index(text: &str) -> Option<usize> {
    let len = text.chars().count();
    for (rev_idx, ch) in text.chars().rev().enumerate() {
        if !ch.is_whitespace() {
            return Some(len - 1 - rev_idx);
        }
    }
    None
}

/// Find the start of the next word from the current position
pub fn find_next_word_start(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 {
        return 0;
    }
    let current_pos = current_pos.min(len);

    if current_pos == len {
        return current_pos;
    }

    let mut pos = current_pos;
    let initial_type = match text.chars().nth(pos) {
        Some(ch) => get_char_type(ch),
        None => return pos,
    };
    let mut skipping_whitespace = false;

    for ch in text.chars().skip(current_pos) {
        let ch_type = get_char_type(ch);
        if !skipping_whitespace {
            if ch_type == initial_type {
                pos += 1;
                continue;
            }
            if ch_type == CharType::Whitespace {
                skipping_whitespace = true;
                pos += 1;
                continue;
            }
            break;
        } else if ch_type == CharType::Whitespace {
            pos += 1;
        } else {
            break;
        }
    }

    pos
}

/// Find the end of the current or next word
pub fn find_word_end(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 {
        return 0;
    }

    let mut pos = current_pos.min(len - 1);
    let current_type = match text.chars().nth(pos) {
        Some(ch) => get_char_type(ch),
        None => return len.saturating_sub(1),
    };

    if current_type != CharType::Whitespace {
        for ch in text.chars().skip(pos) {
            if get_char_type(ch) == current_type {
                pos += 1;
            } else {
                break;
            }
        }
        return pos.saturating_sub(1);
    }

    pos = find_next_word_start(text, pos);
    if pos >= len {
        return len.saturating_sub(1);
    }

    let word_type = match text.chars().nth(pos) {
        Some(ch) => get_char_type(ch),
        None => return len.saturating_sub(1),
    };
    for ch in text.chars().skip(pos) {
        if get_char_type(ch) == word_type {
            pos += 1;
        } else {
            break;
        }
    }

    pos.saturating_sub(1).min(len.saturating_sub(1))
}

/// Find the start of the previous word
pub fn find_prev_word_start(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 || current_pos == 0 {
        return 0;
    }

    let start = current_pos.min(len).saturating_sub(1);
    let mut pos = 0usize;
    let mut ch_at_pos = '\0';
    let mut found = false;

    for (rev_idx, ch) in text.chars().rev().enumerate() {
        let idx = len - 1 - rev_idx;
        if idx > start {
            continue;
        }
        if idx > 0 && ch.is_whitespace() {
            continue;
        }
        pos = idx;
        ch_at_pos = ch;
        found = true;
        break;
    }

    if !found {
        return 0;
    }

    if !ch_at_pos.is_whitespace() {
        let word_type = get_char_type(ch_at_pos);
        for (rev_idx, prev_ch) in text.chars().rev().enumerate() {
            let idx = len - 1 - rev_idx;
            if idx >= pos {
                continue;
            }
            if get_char_type(prev_ch) == word_type {
                pos = idx;
            } else {
                break;
            }
        }
    }

    pos
}

/// Find the end of the previous word (vim `ge`)
pub fn find_prev_word_end(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 || current_pos == 0 {
        return 0;
    }

    let mut in_word = false;
    let mut current_word_type: Option<CharType> = None;
    let mut last_end_before_current = 0usize;
    let mut found = false;

    for (i, ch) in text.chars().enumerate() {
        let char_type = get_char_type(ch);

        match char_type {
            CharType::Whitespace => {
                if in_word {
                    let end = i - 1;
                    if end < current_pos {
                        last_end_before_current = end;
                        found = true;
                    }
                    in_word = false;
                    current_word_type = None;
                }
            }
            _ => {
                if !in_word || current_word_type != Some(char_type) {
                    if in_word {
                        let end = i - 1;
                        if end < current_pos {
                            last_end_before_current = end;
                            found = true;
                        }
                    }
                    in_word = true;
                    current_word_type = Some(char_type);
                }
            }
        }
    }

    if in_word {
        let end = len - 1;
        if end < current_pos {
            last_end_before_current = end;
            found = true;
        }
    }

    if found { last_end_before_current } else { 0 }
}

/// Find the start of the next big_word (whitespace-separated)
pub fn find_next_big_word_start(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 || current_pos >= len {
        return len;
    }

    let mut pos = current_pos;
    let mut skipping_whitespace = false;

    for ch in text.chars().skip(current_pos) {
        if !skipping_whitespace {
            if !ch.is_whitespace() {
                pos += 1;
            } else {
                skipping_whitespace = true;
                pos += 1;
            }
        } else if ch.is_whitespace() {
            pos += 1;
        } else {
            break;
        }
    }

    pos
}

/// Find the start of the previous big_word (whitespace-separated)
pub fn find_prev_big_word_start(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 || current_pos == 0 {
        return 0;
    }

    let start = current_pos.min(len).saturating_sub(1);
    let mut pos = 0usize;
    let mut found = false;

    for (rev_idx, ch) in text.chars().rev().enumerate() {
        let idx = len - 1 - rev_idx;
        if idx > start {
            continue;
        }
        if idx > 0 && ch.is_whitespace() {
            continue;
        }
        pos = idx;
        found = true;
        break;
    }

    if !found {
        return 0;
    }

    for (rev_idx, prev_ch) in text.chars().rev().enumerate() {
        let idx = len - 1 - rev_idx;
        if idx >= pos {
            continue;
        }
        if !prev_ch.is_whitespace() {
            pos = idx;
        } else {
            break;
        }
    }

    pos
}

/// Find the end of the current/next big_word (whitespace-separated)
pub fn find_big_word_end(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 {
        return 0;
    }

    let mut pos = current_pos;
    let mut in_word = false;

    for ch in text.chars().skip(current_pos) {
        if !in_word {
            if ch.is_whitespace() {
                pos += 1;
            } else {
                in_word = true;
                pos += 1;
            }
        } else if ch.is_whitespace() {
            break;
        } else {
            pos += 1;
        }
    }

    if !in_word {
        return len;
    }

    pos.saturating_sub(1)
}

/// Find the end of the previous big_word (whitespace-separated)
pub fn find_prev_big_word_end(text: &str, current_pos: usize) -> usize {
    let len = text.chars().count();
    if len == 0 || current_pos == 0 {
        return 0;
    }

    let start = current_pos.min(len).saturating_sub(1);
    let mut word_start_idx = 0usize;
    let mut ch_at_pos = '\0';
    let mut found = false;

    for (rev_idx, ch) in text.chars().rev().enumerate() {
        let idx = len - 1 - rev_idx;
        if idx > start {
            continue;
        }
        if idx > 0 && ch.is_whitespace() {
            continue;
        }
        word_start_idx = idx;
        ch_at_pos = ch;
        found = true;
        break;
    }

    if !found {
        return 0;
    }

    if word_start_idx == 0 && ch_at_pos.is_whitespace() {
        return 0;
    }

    for (rev_idx, prev_ch) in text.chars().rev().enumerate() {
        let idx = len - 1 - rev_idx;
        if idx >= word_start_idx {
            continue;
        }
        if !prev_ch.is_whitespace() {
            word_start_idx = idx;
        } else {
            break;
        }
    }

    let mut pos = word_start_idx;
    for ch in text.chars().skip(word_start_idx) {
        if !ch.is_whitespace() {
            pos += 1;
        } else {
            break;
        }
    }

    pos.saturating_sub(1)
}

/// Find the start of the last word in a field (for cross-field 'b' movement)
pub fn find_last_word_start_in_field(text: &str) -> usize {
    let len = text.chars().count();
    let mut pos = 0usize;
    let mut ch_at_pos = '\0';
    let mut found = false;

    for (rev_idx, ch) in text.chars().rev().enumerate() {
        if ch.is_whitespace() {
            continue;
        }
        pos = len - 1 - rev_idx;
        ch_at_pos = ch;
        found = true;
        break;
    }

    if !found {
        return 0;
    }
    let char_type = get_char_type(ch_at_pos);

    for (rev_idx, prev_char) in text.chars().rev().enumerate() {
        let idx = len - 1 - rev_idx;
        if idx >= pos {
            continue;
        }
        if get_char_type(prev_char) == char_type {
            pos = idx;
        } else {
            break;
        }
    }

    pos
}

/// Find the end of the last word in a field (for cross-field 'ge' movement)
pub fn find_last_word_end_in_field(text: &str) -> usize {
    last_non_whitespace_index(text).unwrap_or(0)
}

/// Find the start of the last big_word in a field (for cross-field 'B' movement)
pub fn find_last_big_word_start_in_field(text: &str) -> usize {
    let len = text.chars().count();
    let mut pos = 0usize;
    let mut found = false;

    for (rev_idx, ch) in text.chars().rev().enumerate() {
        if ch.is_whitespace() {
            continue;
        }
        pos = len - 1 - rev_idx;
        found = true;
        break;
    }

    if !found {
        return 0;
    }

    for (rev_idx, prev_char) in text.chars().rev().enumerate() {
        let idx = len - 1 - rev_idx;
        if idx >= pos {
            continue;
        }
        if prev_char.is_whitespace() {
            break;
        }
        pos = idx;
    }

    pos
}

/// Find the end of the last big_word in a field (for cross-field 'gE' movement)
pub fn find_last_big_word_end_in_field(text: &str) -> usize {
    last_non_whitespace_index(text).unwrap_or(0)
}
