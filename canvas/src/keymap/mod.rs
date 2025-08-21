// src/keymap/mod.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyModifiers};

use crate::canvas::modes::AppMode;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyStroke {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

#[derive(Clone, Debug)]
struct Binding {
    action: String,
    sequence: Vec<KeyStroke>,
}

#[derive(Clone, Debug, Default)]
pub struct CanvasKeyMap {
    ro: Vec<Binding>,
    edit: Vec<Binding>,
    hl: Vec<Binding>,
}

// FIXED: Removed Copy because Option<String> is not Copy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEventOutcome {
    Consumed(Option<String>),
    Pending,
    NotMatched,
}

#[derive(Debug, Clone)]
pub struct KeySequenceTracker {
    sequence: Vec<KeyStroke>,
    last_key_time: Instant,
    timeout: Duration,
}

impl KeySequenceTracker {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            sequence: Vec::new(),
            last_key_time: Instant::now(),
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    pub fn reset(&mut self) {
        self.sequence.clear();
        self.last_key_time = Instant::now();
    }

    pub fn add_key(&mut self, stroke: KeyStroke) {
        let now = Instant::now();
        if now.duration_since(self.last_key_time) > self.timeout {
            self.reset();
        }
        self.sequence.push(normalize_stroke(stroke));
        self.last_key_time = now;
    }

    pub fn sequence(&self) -> &[KeyStroke] {
        &self.sequence
    }
}

fn normalize_stroke(mut s: KeyStroke) -> KeyStroke {
    // Normalize Shift+Tab to BackTab
    let is_shift_tab =
        s.code == KeyCode::Tab && s.modifiers.contains(KeyModifiers::SHIFT);
    if is_shift_tab {
        s.code = KeyCode::BackTab;
        s.modifiers.remove(KeyModifiers::SHIFT);
        return s;
    }

    // Normalize Shift+char to uppercase char without SHIFT when possible
    if let KeyCode::Char(c) = s.code {
        if s.modifiers.contains(KeyModifiers::SHIFT) {
            let mut up = c;
            // Only letters transform meaningfully
            if c.is_ascii_alphabetic() {
                up = c.to_ascii_uppercase();
            }
            s.code = KeyCode::Char(up);
            s.modifiers.remove(KeyModifiers::SHIFT);
            return s;
        }
    }
    s
}

impl CanvasKeyMap {
    pub fn from_mode_maps(
        read_only: &HashMap<String, Vec<String>>,
        edit: &HashMap<String, Vec<String>>,
        highlight: &HashMap<String, Vec<String>>,
    ) -> Self {
        let mut km = Self::default();
        km.ro = collect_bindings(read_only);
        km.edit = collect_bindings(edit);
        km.hl = collect_bindings(highlight);
        km
    }

    pub fn lookup(
        &self,
        mode: AppMode,
        seq: &[KeyStroke],
    ) -> (Option<&str>, bool) {
        let bindings = match mode {
            AppMode::ReadOnly => &self.ro,
            AppMode::Edit => &self.edit,
            AppMode::Highlight => &self.hl,
            _ => return (None, false),
        };

        if seq.is_empty() {
            return (None, false);
        }

        // Exact match
        for b in bindings {
            if sequences_equal(&b.sequence, seq) {
                return (Some(b.action.as_str()), false);
            }
        }

        // Prefix match
        for b in bindings {
            if is_prefix(&b.sequence, seq) {
                return (None, true);
            }
        }

        (None, false)
    }
}

fn sequences_equal(a: &[KeyStroke], b: &[KeyStroke]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| strokes_equal(x, y))
}

fn strokes_equal(a: &KeyStroke, b: &KeyStroke) -> bool {
    // Both KeyStroke are already normalized
    a.code == b.code && a.modifiers == b.modifiers
}

fn is_prefix(binding: &[KeyStroke], seq: &[KeyStroke]) -> bool {
    if seq.len() >= binding.len() {
        return false;
    }
    binding
        .iter()
        .zip(seq.iter())
        .all(|(b, s)| strokes_equal(b, s))
}

fn collect_bindings(
    mode_map: &HashMap<String, Vec<String>>,
) -> Vec<Binding> {
    let mut out = Vec::new();
    for (action, list) in mode_map {
        for binding_str in list {
            if let Some(seq) = parse_binding_to_sequence(binding_str) {
                out.push(Binding {
                    action: action.to_string(),
                    sequence: seq,
                });
            }
        }
    }
    out
}

fn parse_binding_to_sequence(input: &str) -> Option<Vec<KeyStroke>> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }

    let has_space = s.contains(' ');
    let has_plus = s.contains('+');

    if has_space {
        let mut seq = Vec::new();
        for part in s.split_whitespace() {
            if let Some(mut strokes) = parse_part_to_sequence(part) {
                seq.append(&mut strokes);
            } else {
                return None;
            }
        }
        return Some(seq);
    }

    if has_plus {
        if contains_modifier_token(s) {
            if let Some(k) = parse_chord_with_modifiers(s) {
                return Some(vec![k]);
            }
            return None;
        } else {
            let mut seq = Vec::new();
            for t in s.split('+') {
                if let Some(mut strokes) = parse_part_to_sequence(t) {
                    seq.append(&mut strokes);
                } else {
                    return None;
                }
            }
            return Some(seq);
        }
    }

    if is_compound_key(s) {
        if let Some(k) = parse_simple_key(s) {
            return Some(vec![k]);
        }
        return None;
    }

    if s.len() > 1 {
        let mut seq = Vec::new();
        for ch in s.chars() {
            seq.push(KeyStroke {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::empty(),
            });
        }
        return Some(seq);
    }

    if let Some(k) = parse_simple_key(s) {
        return Some(vec![k]);
    }
    None
}

fn parse_part_to_sequence(part: &str) -> Option<Vec<KeyStroke>> {
    let p = part.trim();
    if p.is_empty() {
        return None;
    }

    if p.contains('+') && contains_modifier_token(p) {
        if let Some(k) = parse_chord_with_modifiers(p) {
            return Some(vec![k]);
        }
        return None;
    }

    if is_compound_key(p) {
        if let Some(k) = parse_simple_key(p) {
            return Some(vec![k]);
        }
        return None;
    }

    if p.len() > 1 {
        let mut seq = Vec::new();
        for ch in p.chars() {
            seq.push(KeyStroke {
                code: KeyCode::Char(ch),
                modifiers: KeyModifiers::empty(),
            });
        }
        return Some(seq);
    }

    parse_simple_key(p).map(|k| vec![k])
}

fn contains_modifier_token(s: &str) -> bool {
    let low = s.to_lowercase();
    low.contains("ctrl") || low.contains("shift") || low.contains("alt") || 
    low.contains("super") || low.contains("cmd") || low.contains("meta")
}

fn parse_chord_with_modifiers(s: &str) -> Option<KeyStroke> {
    let mut mods = KeyModifiers::empty();
    let mut key: Option<KeyCode> = None;

    for comp in s.split('+') {
        match comp.to_lowercase().as_str() {
            "ctrl" => mods |= KeyModifiers::CONTROL,
            "shift" => mods |= KeyModifiers::SHIFT,
            "alt" => mods |= KeyModifiers::ALT,
            "super" | "cmd" => mods |= KeyModifiers::SUPER,
            "meta" => mods |= KeyModifiers::META,
            other => {
                key = string_to_keycode(other);
            }
        }
    }

    key.map(|k| normalize_stroke(KeyStroke { code: k, modifiers: mods }))
}

fn is_compound_key(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(),
        "left" | "right" | "up" | "down" | "esc" | "enter" | "backspace" | 
        "delete" | "tab" | "home" | "end" | "$" | "0"
    )
}

fn parse_simple_key(s: &str) -> Option<KeyStroke> {
    if let Some(kc) = string_to_keycode(&s.to_lowercase()) {
        return Some(KeyStroke { code: kc, modifiers: KeyModifiers::empty() });
    }

    if s.chars().count() == 1 {
        let ch = s.chars().next().unwrap();
        return Some(KeyStroke { code: KeyCode::Char(ch), modifiers: KeyModifiers::empty() });
    }

    None
}

fn string_to_keycode(s: &str) -> Option<KeyCode> {
    Some(match s {
        "left" => KeyCode::Left,
        "right" => KeyCode::Right, 
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "esc" => KeyCode::Esc,
        "enter" => KeyCode::Enter,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "tab" => KeyCode::Tab,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "$" => KeyCode::Char('$'),
        "0" => KeyCode::Char('0'),
        _ => return None,
    })
}
