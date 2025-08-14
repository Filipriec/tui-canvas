// src/canvas/actions/movement/word.rs
// Replace the entire file with this corrected version:

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

/// Find the end of the previous word (CORRECTED VERSION for vim's ge command)
pub fn find_prev_word_end(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || current_pos == 0 {
        return 0;
    }

    // Find all word end positions using boundary detection
    let mut word_ends = Vec::new();
    let mut in_word = false;
    let mut current_word_type: Option<CharType> = None;

    for (i, &ch) in chars.iter().enumerate() {
        let char_type = get_char_type(ch);
        
        match char_type {
            CharType::Whitespace => {
                if in_word {
                    // End of a word
                    word_ends.push(i - 1);
                    in_word = false;
                    current_word_type = None;
                }
            }
            _ => {
                if !in_word || current_word_type != Some(char_type) {
                    // Start of a new word (or word type change)
                    if in_word {
                        // End the previous word first
                        word_ends.push(i - 1);
                    }
                    in_word = true;
                    current_word_type = Some(char_type);
                }
            }
        }
    }

    // Add the final word end if text doesn't end with whitespace
    if in_word && !chars.is_empty() {
        word_ends.push(chars.len() - 1);
    }

    // Find the largest word end position that's before current_pos
    for &end_pos in word_ends.iter().rev() {
        if end_pos < current_pos {
            return end_pos;
        }
    }

    0
}

/// Find the start of the next WORD (whitespace-separated)
pub fn find_next_WORD_start(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || current_pos >= chars.len() {
        return text.chars().count();
    }

    let mut pos = current_pos;

    // If we're on non-whitespace, skip to end of current WORD
    while pos < chars.len() && !chars[pos].is_whitespace() {
        pos += 1;
    }

    // Skip whitespace to find start of next WORD
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }

    pos
}

/// Find the start of the previous WORD (whitespace-separated)
pub fn find_prev_WORD_start(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || current_pos == 0 {
        return 0;
    }

    let mut pos = current_pos.saturating_sub(1);

    // Skip whitespace backwards
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // Find start of current WORD by going back while non-whitespace
    while pos > 0 && !chars[pos - 1].is_whitespace() {
        pos -= 1;
    }

    pos
}

/// Find the end of the current/next WORD (whitespace-separated)
pub fn find_WORD_end(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    let mut pos = current_pos;

    // If we're on whitespace, skip to start of next WORD
    while pos < chars.len() && chars[pos].is_whitespace() {
        pos += 1;
    }

    // If we reached end, return it
    if pos >= chars.len() {
        return chars.len();
    }

    // Find end of current WORD (last non-whitespace char)
    while pos < chars.len() && !chars[pos].is_whitespace() {
        pos += 1;
    }

    // Return position of last character in WORD
    pos.saturating_sub(1)
}

/// Find the end of the previous WORD (whitespace-separated)
pub fn find_prev_WORD_end(text: &str, current_pos: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() || current_pos == 0 {
        return 0;
    }

    let mut pos = current_pos.saturating_sub(1);

    // Skip whitespace backwards
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // If we hit start of text and it's whitespace, return 0
    if pos == 0 && chars[0].is_whitespace() {
        return 0;
    }

    // Skip back to start of current WORD, then forward to end
    while pos > 0 && !chars[pos - 1].is_whitespace() {
        pos -= 1;
    }

    // Now find end of this WORD
    while pos < chars.len() && !chars[pos].is_whitespace() {
        pos += 1;
    }

    // Return position of last character in WORD
    pos.saturating_sub(1)
}

// ============================================================================
// FIELD BOUNDARY HELPER FUNCTIONS (for cross-field movement)
// ============================================================================

/// Find the start of the last word in a field (for cross-field 'b' movement)
pub fn find_last_word_start_in_field(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    let mut pos = chars.len().saturating_sub(1);

    // Skip trailing whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // If the whole field is whitespace, return 0
    if pos == 0 && chars[0].is_whitespace() {
        return 0;
    }

    // Now we're on a non-whitespace character
    // Find the start of this word by going backwards while chars are the same type
    let char_type = if chars[pos].is_alphanumeric() { "alnum" } else { "punct" };

    while pos > 0 {
        let prev_char = chars[pos - 1];
        let prev_type = if prev_char.is_alphanumeric() {
            "alnum"
        } else if prev_char.is_whitespace() {
            "space"
        } else {
            "punct"
        };

        // Stop if we hit whitespace or different word type
        if prev_type == "space" || prev_type != char_type {
            break;
        }
        pos -= 1;
    }

    pos
}

/// Find the end of the last word in a field (for cross-field 'ge' movement)
pub fn find_last_word_end_in_field(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    // Start from the end and find the last non-whitespace character
    let mut pos = chars.len() - 1;
    
    // Skip trailing whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // If the whole field is whitespace, return 0
    if chars[pos].is_whitespace() {
        return 0;
    }

    // We're now at the end of the last word
    pos
}

/// Find the start of the last WORD in a field (for cross-field 'B' movement)
pub fn find_last_WORD_start_in_field(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    let mut pos = chars.len().saturating_sub(1);

    // Skip trailing whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // If the whole field is whitespace, return 0
    if pos == 0 && chars[0].is_whitespace() {
        return 0;
    }

    // Now we're on a non-whitespace character
    // Find the start of this WORD by going backwards while chars are non-whitespace
    while pos > 0 {
        let prev_char = chars[pos - 1];

        // Stop if we hit whitespace (WORD boundary)
        if prev_char.is_whitespace() {
            break;
        }
        pos -= 1;
    }

    pos
}

/// Find the end of the last WORD in a field (for cross-field 'gE' movement)
pub fn find_last_WORD_end_in_field(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return 0;
    }

    let mut pos = chars.len().saturating_sub(1);

    // Skip trailing whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // If the whole field is whitespace, return 0
    if pos == 0 && chars[0].is_whitespace() {
        return 0;
    }

    // We're now at the end of the last WORD
    pos
}
