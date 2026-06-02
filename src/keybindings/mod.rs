pub mod action;
pub mod builtin;
pub mod key_sequence;
pub mod preset;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::canvas::actions::CanvasAction;
use crate::canvas::modes::AppMode;

pub use action::CanvasKeyAction;
pub use builtin::{
    builtin_emacs_preset, builtin_helix_preset, builtin_vim_preset,
    default_builtin_action_bindings, default_emacs_action_bindings,
    default_helix_action_bindings, default_vim_action_bindings, emacs_preset_toml,
    helix_preset_toml, vim_preset_toml, BuiltinCanvasKeybindingPreset,
};
pub use key_sequence::{
    parse_binding, try_parse_binding, try_parse_key, KeyStroke, ParseKeyError,
};
pub use preset::{
    CanvasKeybindingPreset, CanvasKeybindingPresetBinding, CanvasKeybindingPresetError,
    CanvasKeybindingPresetIssue, CanvasKeybindingPresetSection,
};

#[derive(Clone, Debug)]
struct Binding {
    action: CanvasKeyAction,
    sequence: Vec<KeyStroke>,
}

#[derive(Clone, Debug, Default)]
pub struct CanvasKeyBindings {
    ro: Vec<Binding>,
    edit: Vec<Binding>,
    hl: Vec<Binding>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasActionKeyBinding {
    pub mode: AppMode,
    pub action: CanvasAction,
    pub sequence: Vec<KeyStroke>,
}

pub type CanvasActionBinding = CanvasActionKeyBinding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEventOutcome {
    Consumed(Option<String>),
    Pending,
    NotMatched,
    /// Reached top boundary while navigating upward.
    ExitTop,
    /// Reached bottom boundary while navigating downward.
    ExitBottom,
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
        self.sequence
            .push(key_sequence::normalize_stroke(stroke));
        self.last_key_time = now;
    }

    pub fn sequence(&self) -> &[KeyStroke] {
        &self.sequence
    }
}

impl CanvasKeyBindings {
    pub fn vim_defaults() -> Self {
        Self::from_preset(&builtin_vim_preset())
    }

    pub fn helix_defaults() -> Self {
        Self::from_preset(&builtin_helix_preset())
    }

    pub fn emacs_defaults() -> Self {
        Self::from_preset(&builtin_emacs_preset())
    }

    pub fn from_preset(preset: &CanvasKeybindingPreset) -> Self {
        Self::try_from_preset(preset).expect("canvas keybinding preset was validated")
    }

    pub fn try_from_preset(
        preset: &CanvasKeybindingPreset,
    ) -> Result<Self, CanvasKeybindingPresetError> {
        preset.validate()?;

        let mut bindings = Self::default();
        for section in preset.sections() {
            let section_bindings = section
                .bindings
                .iter()
                .flat_map(|binding| {
                    binding.keys.iter().map(|key| {
                        try_parse_binding(key).map(|sequence| Binding {
                            action: binding.action.clone(),
                            sequence,
                        })
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .expect("canvas keybinding preset was validated");

            match section.mode {
                AppMode::ReadOnly => bindings.ro.extend(section_bindings),
                AppMode::Edit => bindings.edit.extend(section_bindings),
                AppMode::Highlight => bindings.hl.extend(section_bindings),
                _ => {}
            }
        }
        Ok(bindings)
    }

    pub fn from_mode_maps(
        read_only: &HashMap<String, Vec<String>>,
        edit: &HashMap<String, Vec<String>>,
        highlight: &HashMap<String, Vec<String>>,
    ) -> Self {
        let mut bindings = Self::default();
        bindings.ro = collect_bindings(read_only);
        bindings.edit = collect_bindings(edit);
        bindings.hl = collect_bindings(highlight);
        bindings
    }

    pub fn try_from_mode_maps(
        read_only: &HashMap<String, Vec<String>>,
        edit: &HashMap<String, Vec<String>>,
        highlight: &HashMap<String, Vec<String>>,
    ) -> Result<Self, CanvasKeybindingPresetError> {
        let preset = CanvasKeybindingPreset::from_mode_maps(read_only, edit, highlight)?;
        Self::try_from_preset(&preset)
    }

    pub fn lookup_action(
        &self,
        mode: AppMode,
        seq: &[KeyStroke],
    ) -> (Option<&CanvasKeyAction>, bool) {
        let bindings = match mode {
            AppMode::ReadOnly => &self.ro,
            AppMode::Edit => &self.edit,
            AppMode::Highlight => &self.hl,
            _ => return (None, false),
        };

        if seq.is_empty() {
            return (None, false);
        }

        for binding in bindings {
            if binding.sequence == seq {
                return (Some(&binding.action), false);
            }
        }

        for binding in bindings {
            if seq.len() < binding.sequence.len() && binding.sequence.starts_with(seq) {
                return (None, true);
            }
        }

        (None, false)
    }

    pub fn lookup(&self, mode: AppMode, seq: &[KeyStroke]) -> (Option<&str>, bool) {
        let (action, is_prefix) = self.lookup_action(mode, seq);
        (action.map(|action| action.as_str()), is_prefix)
    }
}

fn collect_bindings(mode_map: &HashMap<String, Vec<String>>) -> Vec<Binding> {
    let mut out = Vec::new();
    for (action, list) in mode_map {
        for binding_str in list {
            if let Some(sequence) = parse_binding(binding_str) {
                out.push(Binding {
                    action: CanvasKeyAction::from_name(action),
                    sequence,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn strict_mode_maps_support_named_undo_and_redo() {
        let mut read_only = HashMap::new();
        read_only.insert("undo".to_string(), vec!["u".to_string()]);
        read_only.insert("redo".to_string(), vec!["ctrl+r".to_string()]);

        let keybindings = CanvasKeyBindings::try_from_mode_maps(
            &read_only,
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();

        let undo = [KeyStroke {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::empty(),
        }];
        let redo = [KeyStroke {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
        }];

        assert_eq!(
            keybindings.lookup_action(AppMode::ReadOnly, &undo).0,
            Some(&CanvasKeyAction::Undo)
        );
        assert_eq!(
            keybindings.lookup_action(AppMode::ReadOnly, &redo).0,
            Some(&CanvasKeyAction::Redo)
        );
    }
}
