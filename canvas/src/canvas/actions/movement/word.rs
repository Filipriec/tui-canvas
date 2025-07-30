// src/canvas/actions/movement/word.rs

#[derive(PartialEq)]
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

/// Find the start of the next word from the current position
pub fn find_next_word_start(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }
    let current_pos = current_pos.min(chars.len());

    if current_pos == chars.len() {
        return current_pos;
    }

    let mut pos = current_pos;
    let initial_type = get_char_type(chars[pos]);

    // Skip current word/token
    while pos < chars.len() && get_char_type(chars[pos]) == initial_type {
        pos += 1;
    }

    // Skip whitespace
    while pos < chars.len() && get_char_type(chars[pos]) == CharType::Whitespace {
        pos += 1;
    }

    pos
}

/// Find the end of the current or next word
pub fn find_word_end(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return 0;
    }

    let mut pos = current_pos.min(len - 1);
    let current_type = get_char_type(chars[pos]);
    
    // If we're not on whitespace, move to end of current word
    if current_type != CharType::Whitespace {
        while pos < len && get_char_type(chars[pos]) == current_type {
            pos += 1;
        }
        return pos.saturating_sub(1);
    }

    // If we're on whitespace, find next word and go to its end
    pos = find_next_word_start(text, pos);
    if pos >= len {
        return len.saturating_sub(1);
    }

    let word_type = get_char_type(chars[pos]);
    while pos < len && get_char_type(chars[pos]) == word_type {
        pos += 1;
    }

    pos.saturating_sub(1).min(len.saturating_sub(1))
}

/// Find the start of the previous word
pub fn find_prev_word_start(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || current_pos == 0 {
        return 0;
    }

    let mut pos = current_pos.saturating_sub(1);

    // Skip whitespace backwards
    while pos > 0 && get_char_type(chars[pos]) == CharType::Whitespace {
        pos -= 1;
    }

    // Move to start of word
    if get_char_type(chars[pos]) != CharType::Whitespace {
        let word_type = get_char_type(chars[pos]);
        while pos > 0 && get_char_type(chars[pos - 1]) == word_type {
            pos -= 1;
        }
    }

    if pos == 0 && get_char_type(chars[0]) == CharType::Whitespace {
        0
    } else {
        pos
    }
}

/// Find the end of the previous word
pub fn find_prev_word_end(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || current_pos == 0 {
        return 0;
    }

    let mut pos = current_pos.saturating_sub(1);

    // Skip whitespace backwards
    while pos > 0 && get_char_type(chars[pos]) == CharType::Whitespace {
        pos -= 1;
    }

    if pos == 0 && get_char_type(chars[0]) == CharType::Whitespace {
        return 0;
    }
    if pos == 0 && get_char_type(chars[0]) != CharType::Whitespace {
        return 0;
    }

    let word_type = get_char_type(chars[pos]);
    while pos > 0 && get_char_type(chars[pos - 1]) == word_type {
        pos -= 1;
    }

    // Skip whitespace before this word
    while pos > 0 && get_char_type(chars[pos - 1]) == CharType::Whitespace {
        pos -= 1;
    }

    if pos > 0 {
        pos - 1
    } else {
        0
    }
}
