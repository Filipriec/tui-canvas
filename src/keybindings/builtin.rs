use super::preset::CanvasKeybindingPreset;
use super::{CanvasKeyBindings, CanvasKeybindingPresetError, CanvasKeybindingProfile};
use super::{try_parse_binding, CanvasActionKeyBinding};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinCanvasKeybindingPreset {
    Vim,
    Helix,
    Emacs,
    Vscode,
}

impl BuiltinCanvasKeybindingPreset {
    pub fn name(&self) -> &str {
        match self {
            Self::Vim => "vim",
            Self::Helix => "helix",
            Self::Emacs => "emacs",
            Self::Vscode => "vscode",
        }
    }

    pub fn toml(&self) -> &str {
        match self {
            Self::Vim => include_str!("presets/vim.toml"),
            Self::Helix => include_str!("presets/helix.toml"),
            Self::Emacs => include_str!("presets/emacs.toml"),
            Self::Vscode => include_str!("presets/vscode.toml"),
        }
    }

    pub fn preset(self) -> CanvasKeybindingPreset {
        builtin_preset(self.name(), self.toml())
    }

    pub fn profile(self) -> CanvasKeybindingProfile {
        CanvasKeybindingProfile::new(self)
    }

    pub fn profile_with_overrides(
        self,
        source: &str,
    ) -> Result<CanvasKeybindingProfile, CanvasKeybindingPresetError> {
        CanvasKeybindingProfile::with_overrides_toml(self, source)
    }

    pub fn keybindings_with_overrides(
        self,
        source: &str,
    ) -> Result<CanvasKeyBindings, CanvasKeybindingPresetError> {
        Ok(self.profile_with_overrides(source)?.current().clone())
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

pub fn vscode_preset_toml() -> &'static str {
    BuiltinCanvasKeybindingPreset::Vscode.toml()
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

pub fn builtin_vscode_preset() -> CanvasKeybindingPreset {
    BuiltinCanvasKeybindingPreset::Vscode.preset()
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

pub fn default_vscode_action_bindings() -> Vec<CanvasActionKeyBinding> {
    default_builtin_action_bindings(BuiltinCanvasKeybindingPreset::Vscode)
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
            BuiltinCanvasKeybindingPreset::Vscode,
        ] {
            let parsed = CanvasKeybindingPreset::from_toml(preset.toml()).unwrap();
            assert_eq!(parsed.sections().len(), 3);
        }
    }

    #[test]
    fn vscode_defaults_map_modeless_edit_bindings() {
        let keybindings = CanvasKeyBindings::vscode_defaults();

        // Undo/redo on the VSCode chords, in the always-active insert mode.
        let undo = [KeyStroke {
            code: KeyCode::Char('z'),
            modifiers: KeyModifiers::CONTROL,
        }];
        let redo = [KeyStroke {
            code: KeyCode::Char('y'),
            modifiers: KeyModifiers::CONTROL,
        }];
        assert_eq!(
            keybindings.lookup_action(AppMode::Ins, &undo).0,
            Some(&CanvasKeyAction::Undo)
        );
        assert_eq!(
            keybindings.lookup_action(AppMode::Ins, &redo).0,
            Some(&CanvasKeyAction::Redo)
        );

        // Word-wise delete (Ctrl+Backspace) and word motion (Ctrl+Right).
        let del_word = [KeyStroke {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::CONTROL,
        }];
        let word_next = [KeyStroke {
            code: KeyCode::Right,
            modifiers: KeyModifiers::CONTROL,
        }];
        assert_eq!(
            keybindings.lookup_action(AppMode::Ins, &del_word).0,
            Some(&CanvasKeyAction::DeleteWordBackward)
        );
        assert_eq!(
            keybindings.lookup_action(AppMode::Ins, &word_next).0,
            Some(&CanvasKeyAction::MoveWordNext)
        );
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
            keybindings.lookup_action(AppMode::Nor, &undo).0,
            Some(&CanvasKeyAction::Undo)
        );
        assert_eq!(
            keybindings.lookup_action(AppMode::Nor, &redo).0,
            Some(&CanvasKeyAction::Redo)
        );
    }

    #[test]
    fn vim_defaults_include_visual_big_word_motions() {
        let keybindings = CanvasKeyBindings::vim_defaults();

        for (key, action) in [
            ('W', CanvasKeyAction::MoveBigWordNext),
            ('B', CanvasKeyAction::MoveBigWordPrev),
            ('E', CanvasKeyAction::MoveBigWordEnd),
        ] {
            let stroke = [KeyStroke {
                code: KeyCode::Char(key),
                modifiers: KeyModifiers::empty(),
            }];
            assert_eq!(
                keybindings.lookup_action(AppMode::Sel, &stroke).0,
                Some(&action)
            );
        }
    }

    #[test]
    fn helix_defaults_use_goto_mode_motions() {
        let keybindings = CanvasKeyBindings::helix_defaults();

        for (keys, action) in [
            ("gh", CanvasKeyAction::MoveLineStart),
            ("gl", CanvasKeyAction::MoveLineEnd),
            ("ge", CanvasKeyAction::MoveLastLine),
        ] {
            let sequence = keys
                .chars()
                .map(|key| KeyStroke {
                    code: KeyCode::Char(key),
                    modifiers: KeyModifiers::empty(),
                })
                .collect::<Vec<_>>();
            assert_eq!(
                keybindings.lookup_action(AppMode::Nor, &sequence).0,
                Some(&action)
            );
            assert_eq!(
                keybindings.lookup_action(AppMode::Sel, &sequence).0,
                Some(&action)
            );
        }
    }

    #[test]
    fn action_bindings_are_derived_from_builtin_preset() {
        let bindings = default_vim_action_bindings();

        assert!(bindings.iter().any(|binding| {
            binding.mode == AppMode::Nor
                && binding.action == CanvasAction::Undo
                && binding.sequence
                    == vec![KeyStroke {
                        code: KeyCode::Char('u'),
                        modifiers: KeyModifiers::empty(),
                    }]
        }));
        assert!(bindings.iter().any(|binding| {
            binding.mode == AppMode::Nor
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
            .any(|binding| binding.mode == AppMode::Nor
                && binding.action == CanvasAction::Undo));
        assert!(helix
            .iter()
            .any(|binding| binding.mode == AppMode::Nor
                && binding.action == CanvasAction::Redo));
        let delete = [KeyStroke {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::empty(),
        }];
        assert_eq!(
            CanvasKeyBindings::helix_defaults()
                .lookup_action(AppMode::Nor, &delete)
                .0,
            Some(&CanvasKeyAction::DeleteSelection)
        );
        assert!(emacs
            .iter()
            .any(|binding| binding.mode == AppMode::Ins
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
                CanvasAction::ExitSuggestions,
            ] {
                assert!(
                    bindings
                        .iter()
                        .any(|binding| binding.mode == AppMode::Ins
                            && binding.action == action),
                    "missing edit mode binding for {action:?}"
                );
            }
        }
    }

    #[test]
    fn builtin_presets_include_undo_redo_in_command_modes() {
        for bindings in [default_vim_action_bindings()] {
            for mode in [AppMode::Nor] {
                for action in [CanvasAction::Undo, CanvasAction::Redo] {
                    assert!(
                        bindings
                            .iter()
                            .any(|binding| binding.mode == mode && binding.action == action),
                        "missing {mode:?} binding for {action:?}"
                    );
                }
            }
        }

        for bindings in [default_helix_action_bindings(), default_emacs_action_bindings()] {
            for mode in [AppMode::Nor, AppMode::Sel] {
                for action in [CanvasAction::Undo, CanvasAction::Redo] {
                    assert!(
                        bindings
                            .iter()
                            .any(|binding| binding.mode == mode && binding.action == action),
                        "missing {mode:?} binding for {action:?}"
                    );
                }
            }
        }
    }
}
