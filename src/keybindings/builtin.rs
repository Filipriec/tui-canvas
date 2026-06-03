use super::preset::CanvasKeybindingPreset;
use super::{try_parse_binding, CanvasActionKeyBinding};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinCanvasKeybindingPreset {
    Vim,
    Helix,
    Emacs,
}

impl BuiltinCanvasKeybindingPreset {
    pub fn name(&self) -> &str {
        match self {
            Self::Vim => "vim",
            Self::Helix => "helix",
            Self::Emacs => "emacs",
        }
    }

    pub fn toml(&self) -> &str {
        match self {
            Self::Vim => include_str!("presets/vim.toml"),
            Self::Helix => include_str!("presets/helix.toml"),
            Self::Emacs => include_str!("presets/emacs.toml"),
        }
    }

    pub fn preset(self) -> CanvasKeybindingPreset {
        builtin_preset(self.name(), self.toml())
    }
}

pub fn vim_preset_toml() -> &'static str {
    BuiltinCanvasKeybindingPreset::Vim.toml()
}

pub fn helix_preset_toml() -> &'static str {
    BuiltinCanvasKeybindingPreset::Helix.toml()
}

pub fn emacs_preset_toml() -> &'static str {
    BuiltinCanvasKeybindingPreset::Emacs.toml()
}

pub fn builtin_vim_preset() -> CanvasKeybindingPreset {
    BuiltinCanvasKeybindingPreset::Vim.preset()
}

pub fn builtin_helix_preset() -> CanvasKeybindingPreset {
    BuiltinCanvasKeybindingPreset::Helix.preset()
}

pub fn builtin_emacs_preset() -> CanvasKeybindingPreset {
    BuiltinCanvasKeybindingPreset::Emacs.preset()
}

pub fn default_builtin_action_bindings(
    preset: BuiltinCanvasKeybindingPreset,
) -> Vec<CanvasActionKeyBinding> {
    action_bindings_from_preset(preset.preset())
}

pub fn default_vim_action_bindings() -> Vec<CanvasActionKeyBinding> {
    default_builtin_action_bindings(BuiltinCanvasKeybindingPreset::Vim)
}

pub fn default_helix_action_bindings() -> Vec<CanvasActionKeyBinding> {
    default_builtin_action_bindings(BuiltinCanvasKeybindingPreset::Helix)
}

pub fn default_emacs_action_bindings() -> Vec<CanvasActionKeyBinding> {
    default_builtin_action_bindings(BuiltinCanvasKeybindingPreset::Emacs)
}

fn action_bindings_from_preset(preset: CanvasKeybindingPreset) -> Vec<CanvasActionKeyBinding> {
    let mut bindings = Vec::new();
    for section in preset.sections() {
        for binding in &section.bindings {
            let Some(action) = binding.action.to_canvas_action() else {
                continue;
            };
            for key in &binding.keys {
                let sequence =
                    try_parse_binding(key).expect("built-in canvas vim keybinding was validated");
                bindings.push(CanvasActionKeyBinding {
                    mode: section.mode,
                    action: action.clone(),
                    sequence,
                });
            }
        }
    }
    bindings
}

fn builtin_preset(name: &str, source: &str) -> CanvasKeybindingPreset {
    CanvasKeybindingPreset::from_toml(source)
        .unwrap_or_else(|err| panic!("invalid built-in canvas {name} keybinding preset: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::actions::CanvasAction;
    use crate::canvas::modes::AppMode;
    use crate::keybindings::{CanvasKeyAction, CanvasKeyBindings, KeyStroke};
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn parses_builtin_presets() {
        for preset in [
            BuiltinCanvasKeybindingPreset::Vim,
            BuiltinCanvasKeybindingPreset::Helix,
            BuiltinCanvasKeybindingPreset::Emacs,
        ] {
            let parsed = CanvasKeybindingPreset::from_toml(preset.toml()).unwrap();
            assert_eq!(parsed.sections().len(), 3);
        }
    }

    #[test]
    fn vim_defaults_maps_undo_and_redo() {
        let keybindings = CanvasKeyBindings::vim_defaults();
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

    #[test]
    fn action_bindings_are_derived_from_builtin_preset() {
        let bindings = default_vim_action_bindings();

        assert!(bindings.iter().any(|binding| {
            binding.mode == AppMode::ReadOnly
                && binding.action == CanvasAction::Undo
                && binding.sequence
                    == vec![KeyStroke {
                        code: KeyCode::Char('u'),
                        modifiers: KeyModifiers::empty(),
                    }]
        }));
        assert!(bindings.iter().any(|binding| {
            binding.mode == AppMode::ReadOnly
                && binding.action == CanvasAction::Redo
                && binding.sequence
                    == vec![KeyStroke {
                        code: KeyCode::Char('r'),
                        modifiers: KeyModifiers::CONTROL,
                    }]
        }));
    }

    #[test]
    fn non_vim_defaults_are_available() {
        let helix = default_helix_action_bindings();
        let emacs = default_emacs_action_bindings();

        assert!(helix
            .iter()
            .any(|binding| binding.mode == AppMode::ReadOnly
                && binding.action == CanvasAction::Undo));
        assert!(emacs
            .iter()
            .any(|binding| binding.mode == AppMode::Edit
                && binding.action == CanvasAction::DeleteForward));
    }

    #[test]
    fn builtin_presets_include_edit_mode_suggestion_bindings() {
        for bindings in [
            default_vim_action_bindings(),
            default_helix_action_bindings(),
            default_emacs_action_bindings(),
        ] {
            for action in [
                CanvasAction::TriggerSuggestions,
                CanvasAction::SuggestionDown,
                CanvasAction::SuggestionUp,
                CanvasAction::SelectSuggestion,
            ] {
                assert!(
                    bindings
                        .iter()
                        .any(|binding| binding.mode == AppMode::Edit
                            && binding.action == action),
                    "missing edit mode binding for {action:?}"
                );
            }
        }
    }
}
