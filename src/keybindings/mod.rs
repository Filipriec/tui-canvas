pub mod action;
pub mod builtin;
pub mod key_sequence;
pub mod preset;
pub mod profile;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::canvas::actions::CanvasAction;
use crate::canvas::modes::AppMode;
#[cfg(feature = "keybindings")]
use crate::editor::behavior::KeybindingParadigm;
use crate::keybindings::key_sequence::normalize_binding;
use crate::keybindings::preset::conflict_kind;

pub use action::CanvasKeyAction;
pub use builtin::{
    builtin_emacs_preset, builtin_helix_preset, builtin_vim_preset,
    default_builtin_action_bindings, default_emacs_action_bindings,
    default_helix_action_bindings, default_vim_action_bindings, emacs_preset_toml,
    helix_preset_toml, vim_preset_toml, BuiltinCanvasKeybindingPreset,
};
pub use key_sequence::{
    display_binding, parse_binding, try_parse_binding, try_parse_key, KeyStroke, ParseKeyError,
};
pub use preset::{
    CanvasKeybindingPreset, CanvasKeybindingPresetBinding, CanvasKeybindingPresetError,
    CanvasKeybindingPresetIssue, CanvasKeybindingPresetSection, CanvasKeybindingConflictKind,
};
pub use profile::CanvasKeybindingProfile;

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
    pub(crate) paradigm: Option<KeybindingParadigm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasActionKeyBinding {
    pub mode: AppMode,
    pub action: CanvasAction,
    pub sequence: Vec<KeyStroke>,
}

pub type CanvasActionBinding = CanvasActionKeyBinding;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasKeyBindingEntry {
    pub mode: AppMode,
    pub action: CanvasKeyAction,
    pub sequence: Vec<KeyStroke>,
}

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
        Self::from_builtin_preset(BuiltinCanvasKeybindingPreset::Vim)
    }

    pub fn helix_defaults() -> Self {
        Self::from_builtin_preset(BuiltinCanvasKeybindingPreset::Helix)
    }

    pub fn emacs_defaults() -> Self {
        Self::from_builtin_preset(BuiltinCanvasKeybindingPreset::Emacs)
    }

    pub fn from_builtin_preset(preset: BuiltinCanvasKeybindingPreset) -> Self {
        let mut bindings = Self::from_preset(&preset.preset());
        bindings.paradigm = Some(match preset {
            BuiltinCanvasKeybindingPreset::Vim => KeybindingParadigm::Vim,
            BuiltinCanvasKeybindingPreset::Helix => KeybindingParadigm::Helix,
            BuiltinCanvasKeybindingPreset::Emacs => KeybindingParadigm::Emacs,
        });
        bindings
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
                AppMode::Nor => bindings.ro.extend(section_bindings),
                AppMode::Ins => bindings.edit.extend(section_bindings),
                AppMode::Sel => bindings.hl.extend(section_bindings),
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
            AppMode::Nor => &self.ro,
            AppMode::Ins => &self.edit,
            AppMode::Sel => &self.hl,
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

    pub fn entries(&self) -> Vec<CanvasKeyBindingEntry> {
        let mut entries = Vec::new();
        for (mode, bindings) in [
            (AppMode::Nor, &self.ro),
            (AppMode::Ins, &self.edit),
            (AppMode::Sel, &self.hl),
        ] {
            for binding in bindings {
                entries.push(CanvasKeyBindingEntry {
                    mode,
                    action: binding.action.clone(),
                    sequence: binding.sequence.clone(),
                });
            }
        }
        entries
    }

    pub fn bindings_for(&self, mode: AppMode, action: &CanvasKeyAction) -> Vec<Vec<KeyStroke>> {
        let Some(bindings) = self.bindings_for_mode(mode) else {
            return Vec::new();
        };
        bindings
            .iter()
            .filter(|binding| &binding.action == action)
            .map(|binding| binding.sequence.clone())
            .collect()
    }

    pub fn bind(
        &mut self,
        mode: AppMode,
        action: CanvasKeyAction,
        binding: &str,
    ) -> Result<(), CanvasKeybindingPresetError> {
        let sequence = parse_runtime_binding(mode, &action, binding)?;
        self.ensure_available(mode, &action, &sequence)?;
        self.bindings_for_mode_mut(mode)?
            .push(Binding { action, sequence });
        Ok(())
    }

    pub fn unbind(
        &mut self,
        mode: AppMode,
        binding: &str,
    ) -> Result<Option<CanvasKeyAction>, CanvasKeybindingPresetError> {
        let sequence = try_parse_binding(binding).map_err(|source| {
            CanvasKeybindingPresetError::Issues(vec![CanvasKeybindingPresetIssue::InvalidBinding {
                section: mode_section_name(mode).to_string(),
                action: CanvasKeyAction::Unknown("<unbind>".to_string()),
                binding: binding.to_string(),
                source,
            }])
        })?;
        let sequence = normalize_binding(&sequence);
        let bindings = self.bindings_for_mode_mut(mode)?;
        let Some(index) = bindings
            .iter()
            .position(|existing| existing.sequence == sequence)
        else {
            return Ok(None);
        };
        Ok(Some(bindings.remove(index).action))
    }

    pub fn unbind_action(
        &mut self,
        mode: AppMode,
        action: &CanvasKeyAction,
    ) -> Result<usize, CanvasKeybindingPresetError> {
        let bindings = self.bindings_for_mode_mut(mode)?;
        let before = bindings.len();
        bindings.retain(|binding| &binding.action != action);
        Ok(before - bindings.len())
    }

    pub fn remap_action(
        &mut self,
        mode: AppMode,
        action: CanvasKeyAction,
        bindings: Vec<String>,
    ) -> Result<(), CanvasKeybindingPresetError> {
        let mut sequences = Vec::new();
        let mut seen = HashSet::new();
        for binding in &bindings {
            let sequence = parse_runtime_binding(mode, &action, binding)?;
            if !seen.insert(sequence.clone()) {
                return Err(CanvasKeybindingPresetError::Issues(vec![
                    CanvasKeybindingPresetIssue::BindingConflict {
                        section: mode_section_name(mode).to_string(),
                        mode,
                        binding: display_binding(&sequence),
                        action: action.clone(),
                        existing_binding: display_binding(&sequence),
                        existing_action: action.clone(),
                        kind: CanvasKeybindingConflictKind::Exact,
                    },
                ]));
            }
            sequences.push(sequence);
        }

        for left in 0..sequences.len() {
            for right in (left + 1)..sequences.len() {
                let Some(kind) = conflict_kind(&sequences[left], &sequences[right]) else {
                    continue;
                };
                return Err(CanvasKeybindingPresetError::Issues(vec![
                    CanvasKeybindingPresetIssue::BindingConflict {
                        section: mode_section_name(mode).to_string(),
                        mode,
                        binding: display_binding(&sequences[left]),
                        action: action.clone(),
                        existing_binding: display_binding(&sequences[right]),
                        existing_action: action.clone(),
                        kind,
                    },
                ]));
            }
        }

        let mut candidate = self.clone();
        candidate.unbind_action(mode, &action)?;
        for sequence in &sequences {
            candidate.ensure_available(mode, &action, sequence)?;
        }

        let target = self.bindings_for_mode_mut(mode)?;
        target.retain(|binding| binding.action != action);
        for sequence in sequences {
            target.push(Binding {
                action: action.clone(),
                sequence,
            });
        }
        Ok(())
    }

    fn bindings_for_mode(&self, mode: AppMode) -> Option<&Vec<Binding>> {
        match mode {
            AppMode::Nor => Some(&self.ro),
            AppMode::Ins => Some(&self.edit),
            AppMode::Sel => Some(&self.hl),
            AppMode::General | AppMode::Command => None,
        }
    }

    fn bindings_for_mode_mut(
        &mut self,
        mode: AppMode,
    ) -> Result<&mut Vec<Binding>, CanvasKeybindingPresetError> {
        match mode {
            AppMode::Nor => Ok(&mut self.ro),
            AppMode::Ins => Ok(&mut self.edit),
            AppMode::Sel => Ok(&mut self.hl),
            AppMode::General | AppMode::Command => Err(unsupported_mode_error(mode)),
        }
    }

    fn ensure_available(
        &self,
        mode: AppMode,
        action: &CanvasKeyAction,
        sequence: &[KeyStroke],
    ) -> Result<(), CanvasKeybindingPresetError> {
        let Some(bindings) = self.bindings_for_mode(mode) else {
            return Err(unsupported_mode_error(mode));
        };
        for existing in bindings {
            let Some(kind) = conflict_kind(sequence, &existing.sequence) else {
                continue;
            };
            return Err(CanvasKeybindingPresetError::Issues(vec![
                CanvasKeybindingPresetIssue::BindingConflict {
                    section: mode_section_name(mode).to_string(),
                    mode,
                    binding: display_binding(sequence),
                    action: action.clone(),
                    existing_binding: display_binding(&existing.sequence),
                    existing_action: existing.action.clone(),
                    kind,
                },
            ]));
        }
        Ok(())
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

fn parse_runtime_binding(
    mode: AppMode,
    action: &CanvasKeyAction,
    binding: &str,
) -> Result<Vec<KeyStroke>, CanvasKeybindingPresetError> {
    try_parse_binding(binding)
        .map(|sequence| normalize_binding(&sequence))
        .map_err(|source| {
            CanvasKeybindingPresetError::Issues(vec![CanvasKeybindingPresetIssue::InvalidBinding {
                section: mode_section_name(mode).to_string(),
                action: action.clone(),
                binding: binding.to_string(),
                source,
            }])
        })
}

fn unsupported_mode_error(mode: AppMode) -> CanvasKeybindingPresetError {
    CanvasKeybindingPresetError::Issues(vec![CanvasKeybindingPresetIssue::UnsupportedMode {
        mode,
    }])
}

fn mode_section_name(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Nor => "nor",
        AppMode::Ins => "ins",
        AppMode::Sel => "sel",
        AppMode::General => "general",
        AppMode::Command => "command",
    }
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
            keybindings.lookup_action(AppMode::Nor, &undo).0,
            Some(&CanvasKeyAction::Undo)
        );
        assert_eq!(
            keybindings.lookup_action(AppMode::Nor, &redo).0,
            Some(&CanvasKeyAction::Redo)
        );
    }

    #[test]
    fn profile_applies_partial_helix_overrides_and_keeps_unmentioned_defaults() {
        let profile = BuiltinCanvasKeybindingPreset::Helix
            .profile_with_overrides(
                r#"
[nor]
undo = ["z u"]
redo = ["z r"]
"#,
            )
            .unwrap();

        let current = profile.current();
        let old_undo = try_parse_binding("u").unwrap();
        let new_undo = try_parse_binding("z u").unwrap();
        let redo = try_parse_binding("z r").unwrap();
        let move_down = try_parse_binding("j").unwrap();

        assert_eq!(current.lookup_action(AppMode::Nor, &old_undo).0, None);
        assert_eq!(
            current.lookup_action(AppMode::Nor, &new_undo).0,
            Some(&CanvasKeyAction::Undo)
        );
        assert_eq!(
            current.lookup_action(AppMode::Nor, &redo).0,
            Some(&CanvasKeyAction::Redo)
        );
        assert_eq!(
            current.lookup_action(AppMode::Nor, &move_down).0,
            Some(&CanvasKeyAction::MoveDown)
        );
    }

    #[test]
    fn profile_serializes_only_normalized_differences() {
        let mut profile = CanvasKeybindingProfile::new(BuiltinCanvasKeybindingPreset::Helix);
        profile
            .remap_action(
                AppMode::Nor,
                CanvasKeyAction::Undo,
                vec!["Shift+z".to_string(), "z u".to_string()],
            )
            .unwrap();
        profile
            .remap_action(AppMode::Nor, CanvasKeyAction::Redo, Vec::new())
            .unwrap();

        let overrides = profile.overrides_toml();
        assert!(overrides.contains("[nor]"));
        assert!(overrides.contains("undo = [\"Z\", \"z u\"]"));
        assert!(overrides.contains("redo = []"));
        assert!(!overrides.contains("move_down"));

        let round_tripped = BuiltinCanvasKeybindingPreset::Helix
            .profile_with_overrides(&overrides)
            .unwrap();
        assert_eq!(round_tripped.overrides_toml(), overrides);
    }

    #[test]
    fn runtime_remap_reports_prefix_conflicts() {
        let mut bindings = CanvasKeyBindings::default();
        bindings
            .bind(AppMode::Nor, CanvasKeyAction::Undo, "z u")
            .unwrap();
        let err = bindings
            .bind(AppMode::Nor, CanvasKeyAction::Redo, "z")
            .unwrap_err();

        let CanvasKeybindingPresetError::Issues(issues) = err else {
            panic!("expected issues");
        };
        assert!(issues.iter().any(|issue| matches!(
            issue,
            CanvasKeybindingPresetIssue::BindingConflict {
                kind: CanvasKeybindingConflictKind::PrefixOf,
                ..
            }
        )));
    }

    #[test]
    fn runtime_remap_reports_extension_conflicts() {
        let mut bindings = CanvasKeyBindings::default();
        bindings
            .bind(AppMode::Nor, CanvasKeyAction::Undo, "z")
            .unwrap();
        let err = bindings
            .bind(AppMode::Nor, CanvasKeyAction::Redo, "z r")
            .unwrap_err();

        let CanvasKeybindingPresetError::Issues(issues) = err else {
            panic!("expected issues");
        };
        assert!(issues.iter().any(|issue| matches!(
            issue,
            CanvasKeybindingPresetIssue::BindingConflict {
                kind: CanvasKeybindingConflictKind::ExtensionOf,
                ..
            }
        )));
    }

    #[test]
    fn runtime_mutation_rejects_unsupported_modes() {
        let mut bindings = CanvasKeyBindings::default();
        let err = bindings
            .bind(AppMode::Command, CanvasKeyAction::Undo, "u")
            .unwrap_err();

        let CanvasKeybindingPresetError::Issues(issues) = err else {
            panic!("expected issues");
        };
        assert!(issues.iter().any(|issue| matches!(
            issue,
            CanvasKeybindingPresetIssue::UnsupportedMode {
                mode: AppMode::Command
            }
        )));
    }
}
