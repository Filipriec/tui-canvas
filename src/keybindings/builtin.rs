use super::preset::CanvasKeybindingPreset;
use super::{try_parse_binding, CanvasActionKeyBinding};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinCanvasKeybindingPreset {
    Vim,
}

impl BuiltinCanvasKeybindingPreset {
    pub fn name(&self) -> &str {
        match self {
            Self::Vim => "vim",
        }
    }

    pub fn toml(&self) -> &str {
        match self {
            Self::Vim => include_str!("presets/vim.toml"),
        }
    }

    pub fn preset(self) -> CanvasKeybindingPreset {
        builtin_preset(self.name(), self.toml())
    }
}

pub fn vim_preset_toml() -> &'static str {
    BuiltinCanvasKeybindingPreset::Vim.toml()
}

pub fn builtin_vim_preset() -> CanvasKeybindingPreset {
    BuiltinCanvasKeybindingPreset::Vim.preset()
}

pub fn default_vim_action_bindings() -> Vec<CanvasActionKeyBinding> {
    let mut bindings = Vec::new();
    for section in builtin_vim_preset().sections() {
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
    fn parses_builtin_vim_preset() {
        let preset = CanvasKeybindingPreset::from_toml(vim_preset_toml()).unwrap();

        assert_eq!(preset.sections().len(), 3);
        assert!(preset
            .sections()
            .iter()
            .any(|section| section.mode == AppMode::ReadOnly
                && section.bindings.iter().any(|binding| binding.action == CanvasKeyAction::Undo
                    && binding.keys == vec!["u".to_string()])));
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
}
