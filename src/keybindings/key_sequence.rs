use std::fmt;

use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyStroke {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseKeyError {
    Empty,
    UnknownKey(String),
}

impl fmt::Display for ParseKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty key binding"),
            Self::UnknownKey(token) => write!(f, "unrecognised key token: {token:?}"),
        }
    }
}

impl std::error::Error for ParseKeyError {}

pub fn parse_binding(binding: &str) -> Option<Vec<KeyStroke>> {
    try_parse_binding(binding).ok()
}

pub fn try_parse_binding(binding: &str) -> Result<Vec<KeyStroke>, ParseKeyError> {
    let mut sequence = Vec::new();
    for token in binding.split_whitespace() {
        sequence.extend(try_parse_binding_token(token)?);
    }
    if sequence.is_empty() {
        return Err(ParseKeyError::Empty);
    }
    Ok(sequence)
}

pub fn try_parse_key(token: &str) -> Result<KeyStroke, ParseKeyError> {
    try_parse_chord(token)
}

impl KeyStroke {
    pub fn display_string(&self) -> String {
        let stroke = normalize_stroke(self.clone());
        let mut out = String::new();
        if stroke.modifiers.contains(KeyModifiers::CONTROL) {
            out.push_str("Ctrl+");
        }
        if stroke.modifiers.contains(KeyModifiers::ALT) {
            out.push_str("Alt+");
        }
        if stroke.modifiers.contains(KeyModifiers::SHIFT) {
            out.push_str("Shift+");
        }
        if stroke.modifiers.contains(KeyModifiers::SUPER) {
            out.push_str("Super+");
        }

        match stroke.code {
            KeyCode::Char(' ') => out.push_str("Space"),
            KeyCode::Char(ch) => out.push(ch),
            KeyCode::Enter => out.push_str("Enter"),
            KeyCode::Tab => out.push_str("Tab"),
            KeyCode::BackTab => out.push_str("BackTab"),
            KeyCode::Backspace => out.push_str("Backspace"),
            KeyCode::Esc => out.push_str("Esc"),
            KeyCode::Up => out.push_str("Up"),
            KeyCode::Down => out.push_str("Down"),
            KeyCode::Left => out.push_str("Left"),
            KeyCode::Right => out.push_str("Right"),
            KeyCode::Home => out.push_str("Home"),
            KeyCode::End => out.push_str("End"),
            KeyCode::PageUp => out.push_str("PageUp"),
            KeyCode::PageDown => out.push_str("PageDown"),
            KeyCode::Delete => out.push_str("Delete"),
            KeyCode::Insert => out.push_str("Insert"),
            KeyCode::F(number) => out.push_str(&format!("F{number}")),
            other => out.push_str(&format!("{other:?}")),
        }

        out
    }
}

pub fn display_binding(sequence: &[KeyStroke]) -> String {
    sequence
        .iter()
        .map(KeyStroke::display_string)
        .collect::<Vec<_>>()
        .join(" ")
}

fn try_parse_binding_token(token: &str) -> Result<Vec<KeyStroke>, ParseKeyError> {
    let token = token.trim();
    if token.is_empty() {
        return Err(ParseKeyError::Empty);
    }

    let parts = token.split('+').collect::<Vec<_>>();
    if parts.len() > 1 {
        if is_modifier_sequence(&parts) {
            return try_parse_chord(token).map(|stroke| vec![stroke]);
        }

        return parts
            .into_iter()
            .map(|part| try_parse_chord(part).map(|stroke| vec![stroke]))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| items.into_iter().flatten().collect())
            .map_err(|_| ParseKeyError::UnknownKey(token.to_string()));
    }

    try_parse_compact_or_key(token)
}

fn try_parse_compact_or_key(token: &str) -> Result<Vec<KeyStroke>, ParseKeyError> {
    match try_parse_chord(token) {
        Ok(stroke) => Ok(vec![stroke]),
        Err(ParseKeyError::UnknownKey(_)) if token.chars().count() > 1 => Ok(token
            .chars()
            .map(|ch| KeyStroke {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::empty(),
            })
            .collect()),
        Err(err) => Err(err),
    }
}

fn try_parse_chord(token: &str) -> Result<KeyStroke, ParseKeyError> {
    let original = token.trim();
    if original.is_empty() {
        return Err(ParseKeyError::Empty);
    }

    let mut modifiers = KeyModifiers::empty();
    let mut key = original;
    loop {
        let Some((prefix, rest)) = key.split_once('+') else {
            break;
        };
        match prefix.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "c" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "meta" | "m" => modifiers |= KeyModifiers::ALT,
            "shift" | "s" => modifiers |= KeyModifiers::SHIFT,
            "super" | "cmd" => modifiers |= KeyModifiers::SUPER,
            _ => break,
        }
        key = rest;
    }

    let code =
        parse_key_code(key).ok_or_else(|| ParseKeyError::UnknownKey(original.to_string()))?;
    Ok(normalize_stroke(KeyStroke { code, modifiers }))
}

fn parse_key_code(key: &str) -> Option<KeyCode> {
    let lower = key.to_ascii_lowercase();
    Some(match lower.as_str() {
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" | "bs" => KeyCode::Backspace,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "page_up" => KeyCode::PageUp,
        "pagedown" | "page_down" => KeyCode::PageDown,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        text if text.starts_with('f') && text.len() > 1 => {
            let number = text[1..].parse().ok()?;
            KeyCode::F(number)
        }
        _ => {
            let mut chars = key.chars();
            let first = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(first)
        }
    })
}

fn is_modifier_sequence(parts: &[&str]) -> bool {
    parts
        .iter()
        .take(parts.len().saturating_sub(1))
        .all(|part| is_modifier(part))
}

fn is_modifier(part: &str) -> bool {
    matches!(
        part.to_ascii_lowercase().as_str(),
        "ctrl" | "control" | "c" | "alt" | "meta" | "m" | "shift" | "s" | "super" | "cmd"
    )
}

pub(crate) fn normalize_stroke(mut stroke: KeyStroke) -> KeyStroke {
    let is_shift_tab =
        stroke.code == KeyCode::Tab && stroke.modifiers.contains(KeyModifiers::SHIFT);
    if is_shift_tab || stroke.code == KeyCode::BackTab {
        stroke.code = KeyCode::BackTab;
        stroke.modifiers.remove(KeyModifiers::SHIFT);
        return stroke;
    }

    if let KeyCode::Char(ch) = stroke.code {
        if stroke.modifiers.contains(KeyModifiers::SHIFT) && ch.is_ascii_alphabetic() {
            stroke.code = KeyCode::Char(ch.to_ascii_uppercase());
            stroke.modifiers.remove(KeyModifiers::SHIFT);
        }
    }

    stroke
}

pub(crate) fn normalize_binding(sequence: &[KeyStroke]) -> Vec<KeyStroke> {
    sequence.iter().cloned().map(normalize_stroke).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(code: KeyCode, modifiers: KeyModifiers) -> KeyStroke {
        KeyStroke { code, modifiers }
    }

    #[test]
    fn parses_named_keys_aliases_and_modifiers() {
        assert_eq!(
            try_parse_binding("ctrl+r").unwrap(),
            vec![stroke(KeyCode::Char('r'), KeyModifiers::CONTROL)]
        );
        assert_eq!(
            try_parse_binding("shift+tab").unwrap(),
            vec![stroke(KeyCode::BackTab, KeyModifiers::empty())]
        );
        assert_eq!(
            try_parse_binding("escape return space f12 page_down ins").unwrap(),
            vec![
                stroke(KeyCode::Esc, KeyModifiers::empty()),
                stroke(KeyCode::Enter, KeyModifiers::empty()),
                stroke(KeyCode::Char(' '), KeyModifiers::empty()),
                stroke(KeyCode::F(12), KeyModifiers::empty()),
                stroke(KeyCode::PageDown, KeyModifiers::empty()),
                stroke(KeyCode::Insert, KeyModifiers::empty()),
            ]
        );
    }

    #[test]
    fn preserves_compact_vim_sequences_and_uppercase_chars() {
        assert_eq!(
            try_parse_binding("gg gE G").unwrap(),
            vec![
                stroke(KeyCode::Char('g'), KeyModifiers::empty()),
                stroke(KeyCode::Char('g'), KeyModifiers::empty()),
                stroke(KeyCode::Char('g'), KeyModifiers::empty()),
                stroke(KeyCode::Char('E'), KeyModifiers::empty()),
                stroke(KeyCode::Char('G'), KeyModifiers::empty()),
            ]
        );
    }

    #[test]
    fn reports_modifier_typos_as_whole_token() {
        assert_eq!(
            try_parse_binding("ctrl+shft+j"),
            Err(ParseKeyError::UnknownKey("ctrl+shft+j".to_string()))
        );
        assert_eq!(
            try_parse_binding("ctrl+notakey"),
            Err(ParseKeyError::UnknownKey("ctrl+notakey".to_string()))
        );
    }

    #[test]
    fn display_binding_round_trips_to_normalized_sequence() {
        let cases = vec![
            vec![stroke(KeyCode::Char('u'), KeyModifiers::empty())],
            vec![stroke(KeyCode::Char('r'), KeyModifiers::CONTROL)],
            vec![stroke(KeyCode::Char('a'), KeyModifiers::SHIFT)],
            vec![stroke(KeyCode::Tab, KeyModifiers::SHIFT)],
            vec![stroke(KeyCode::BackTab, KeyModifiers::SHIFT)],
            vec![stroke(KeyCode::Char(' '), KeyModifiers::empty())],
            vec![stroke(KeyCode::F(12), KeyModifiers::empty())],
            vec![
                stroke(KeyCode::Char('g'), KeyModifiers::empty()),
                stroke(KeyCode::Char('g'), KeyModifiers::empty()),
            ],
            vec![
                stroke(KeyCode::Char('z'), KeyModifiers::empty()),
                stroke(KeyCode::Char('u'), KeyModifiers::empty()),
            ],
            vec![stroke(KeyCode::PageDown, KeyModifiers::ALT)],
            vec![stroke(KeyCode::Insert, KeyModifiers::SUPER)],
        ];

        for case in cases {
            let displayed = display_binding(&case);
            assert_eq!(
                try_parse_binding(&displayed).unwrap(),
                normalize_binding(&case)
            );
        }
    }
}
