use std::collections::HashMap;
use std::fmt;

use toml::Value;

use crate::canvas::modes::AppMode;

use super::{parse_binding_to_sequence, CanvasKeyAction, KeyStroke};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasKeymapPreset {
    sections: Vec<CanvasKeymapPresetSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasKeymapPresetSection {
    pub name: String,
    pub mode: AppMode,
    pub bindings: Vec<CanvasKeymapPresetBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasKeymapPresetBinding {
    pub action: CanvasKeyAction,
    pub keys: Vec<String>,
}

#[derive(Debug)]
pub enum CanvasKeymapPresetError {
    Toml(toml::de::Error),
    Issues(Vec<CanvasKeymapPresetIssue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasKeymapPresetIssue {
    RootNotTable,
    SectionNotTable {
        section: String,
    },
    ModeNotString {
        section: String,
    },
    UnknownMode {
        section: String,
        mode: String,
    },
    UnknownAction {
        section: String,
        action: String,
    },
    BindingsNotStringList {
        section: String,
        action: String,
    },
    EmptyBindings {
        section: String,
        action: String,
    },
    InvalidBinding {
        section: String,
        action: CanvasKeyAction,
        binding: String,
    },
    DuplicateBinding {
        section: String,
        mode: AppMode,
        binding: String,
        first_action: CanvasKeyAction,
        second_action: CanvasKeyAction,
    },
}

impl fmt::Display for CanvasKeymapPresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(err) => write!(f, "invalid TOML: {err}"),
            Self::Issues(issues) => {
                write!(f, "{} canvas keymap preset issue(s)", issues.len())?;
                for issue in issues {
                    write!(f, "; {issue}")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for CanvasKeymapPresetIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotTable => write!(f, "canvas keymap preset must be a TOML table"),
            Self::SectionNotTable { section } => {
                write!(f, "canvas keymap section {section:?} must be a table")
            }
            Self::ModeNotString { section } => {
                write!(f, "canvas keymap section {section:?} has a non-string mode")
            }
            Self::UnknownMode { section, mode } => {
                write!(f, "unknown canvas mode {mode:?} in section {section:?}")
            }
            Self::UnknownAction { section, action } => {
                write!(f, "unknown canvas key action {action:?} in section {section:?}")
            }
            Self::BindingsNotStringList { section, action } => {
                write!(
                    f,
                    "bindings for action {action:?} in section {section:?} must be a string or string list"
                )
            }
            Self::EmptyBindings { section, action } => {
                write!(f, "action {action:?} in section {section:?} has no bindings")
            }
            Self::InvalidBinding {
                section,
                action,
                binding,
            } => {
                write!(
                    f,
                    "invalid binding {binding:?} for {} in section {section:?}",
                    action.as_str()
                )
            }
            Self::DuplicateBinding {
                section,
                mode,
                binding,
                first_action,
                second_action,
            } => {
                write!(
                    f,
                    "binding {binding:?} in mode {mode:?}, section {section:?} is assigned to both {} and {}",
                    first_action.as_str(),
                    second_action.as_str()
                )
            }
        }
    }
}

impl std::error::Error for CanvasKeymapPresetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Toml(err) => Some(err),
            Self::Issues(_) => None,
        }
    }
}

impl CanvasKeymapPreset {
    pub fn from_toml(source: &str) -> Result<Self, CanvasKeymapPresetError> {
        let value = source.parse::<Value>().map_err(CanvasKeymapPresetError::Toml)?;
        let Some(table) = value.as_table() else {
            return Err(CanvasKeymapPresetError::Issues(vec![
                CanvasKeymapPresetIssue::RootNotTable,
            ]));
        };

        let mut sections = Vec::with_capacity(table.len());
        let mut issues = Vec::new();
        for (section_name, section_value) in table {
            let Some(section) = section_value.as_table() else {
                issues.push(CanvasKeymapPresetIssue::SectionNotTable {
                    section: section_name.clone(),
                });
                continue;
            };

            let mode_name = match section.get("mode") {
                Some(value) => value.as_str().unwrap_or_else(|| {
                    issues.push(CanvasKeymapPresetIssue::ModeNotString {
                        section: section_name.clone(),
                    });
                    section_name.as_str()
                }),
                None => section_name.as_str(),
            };
            let Some(mode) = app_mode_from_name(mode_name) else {
                issues.push(CanvasKeymapPresetIssue::UnknownMode {
                    section: section_name.clone(),
                    mode: mode_name.to_string(),
                });
                continue;
            };

            let mut bindings = Vec::new();
            for (action_name, bindings_value) in section {
                if action_name == "mode" {
                    continue;
                }

                let action = CanvasKeyAction::from_name(action_name);
                if matches!(action, CanvasKeyAction::Unknown(_)) {
                    issues.push(CanvasKeymapPresetIssue::UnknownAction {
                        section: section_name.clone(),
                        action: action_name.clone(),
                    });
                    continue;
                }

                let Some(keys) =
                    parse_string_list(section_name, action_name, bindings_value, &mut issues)
                else {
                    continue;
                };
                if keys.is_empty() {
                    issues.push(CanvasKeymapPresetIssue::EmptyBindings {
                        section: section_name.clone(),
                        action: action_name.clone(),
                    });
                    continue;
                }

                bindings.push(CanvasKeymapPresetBinding { action, keys });
            }

            sections.push(CanvasKeymapPresetSection {
                name: section_name.clone(),
                mode,
                bindings,
            });
        }

        let preset = Self { sections };
        issues.extend(preset.validation_issues());
        if issues.is_empty() {
            Ok(preset)
        } else {
            Err(CanvasKeymapPresetError::Issues(issues))
        }
    }

    pub fn sections(&self) -> &[CanvasKeymapPresetSection] {
        &self.sections
    }

    pub fn validation_issues(&self) -> Vec<CanvasKeymapPresetIssue> {
        let mut issues = Vec::new();
        let mut seen: HashMap<(String, Vec<KeyStroke>), (String, CanvasKeyAction, String)> =
            HashMap::new();
        for section in &self.sections {
            let mode_key = app_mode_name(section.mode).to_string();
            for binding in &section.bindings {
                for key in &binding.keys {
                    let Some(sequence) = parse_binding_to_sequence(key) else {
                        issues.push(CanvasKeymapPresetIssue::InvalidBinding {
                            section: section.name.clone(),
                            action: binding.action.clone(),
                            binding: key.clone(),
                        });
                        continue;
                    };
                    let previous = seen.insert(
                        (mode_key.clone(), sequence.clone()),
                        (section.name.clone(), binding.action.clone(), key.clone()),
                    );
                    if let Some((first_section, first_action, first_key)) = previous {
                        if first_action != binding.action {
                            issues.push(CanvasKeymapPresetIssue::DuplicateBinding {
                                section: section.name.clone(),
                                mode: section.mode,
                                binding: key.clone(),
                                first_action: first_action.clone(),
                                second_action: binding.action.clone(),
                            });
                        }
                        seen.insert(
                            (mode_key.clone(), sequence),
                            (first_section, first_action, first_key),
                        );
                    }
                }
            }
        }
        issues
    }
}

pub fn vim_preset_toml() -> &'static str {
    include_str!("presets/vim.toml")
}

pub fn builtin_vim_preset() -> CanvasKeymapPreset {
    CanvasKeymapPreset::from_toml(vim_preset_toml())
        .expect("built-in canvas vim keymap preset must be valid")
}

fn parse_string_list(
    section: &str,
    action: &str,
    value: &Value,
    issues: &mut Vec<CanvasKeymapPresetIssue>,
) -> Option<Vec<String>> {
    if let Some(single) = value.as_str() {
        return Some(vec![single.to_string()]);
    }

    let Some(items) = value.as_array() else {
        issues.push(CanvasKeymapPresetIssue::BindingsNotStringList {
            section: section.to_string(),
            action: action.to_string(),
        });
        return None;
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(binding) = item.as_str() else {
            issues.push(CanvasKeymapPresetIssue::BindingsNotStringList {
                section: section.to_string(),
                action: action.to_string(),
            });
            return None;
        };
        out.push(binding.to_string());
    }
    Some(out)
}

fn app_mode_from_name(name: &str) -> Option<AppMode> {
    match name {
        "read_only" | "normal" => Some(AppMode::ReadOnly),
        "edit" | "insert" => Some(AppMode::Edit),
        "highlight" | "select" => Some(AppMode::Highlight),
        "command" => Some(AppMode::Command),
        "general" => Some(AppMode::General),
        _ => None,
    }
}

fn app_mode_name(mode: AppMode) -> &'static str {
    match mode {
        AppMode::ReadOnly => "read_only",
        AppMode::Edit => "edit",
        AppMode::Highlight => "highlight",
        AppMode::Command => "command",
        AppMode::General => "general",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn parses_builtin_vim_preset() {
        let preset = CanvasKeymapPreset::from_toml(vim_preset_toml()).unwrap();

        assert_eq!(preset.sections().len(), 3);
        assert!(preset
            .sections()
            .iter()
            .any(|section| section.mode == AppMode::ReadOnly
                && section.bindings.iter().any(|binding| binding.action == CanvasKeyAction::Undo
                    && binding.keys == vec!["u".to_string()])));
    }

    #[test]
    fn validates_preset_issues() {
        let err = CanvasKeymapPreset::from_toml(
            r#"
            [normal]
            unknown_action = ["x"]
            move_left = ["h"]
            move_right = ["h"]
            move_up = 1
            move_down = []
            move_word_next = ["ctrl+notakey"]
            "#,
        )
        .unwrap_err();

        let CanvasKeymapPresetError::Issues(issues) = err else {
            panic!("expected validation issues");
        };
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeymapPresetIssue::UnknownAction { .. })));
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeymapPresetIssue::BindingsNotStringList { .. })));
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeymapPresetIssue::EmptyBindings { .. })));
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeymapPresetIssue::InvalidBinding { .. })));
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeymapPresetIssue::DuplicateBinding { .. })));
    }

    #[test]
    fn vim_defaults_maps_undo_and_redo() {
        let keymap = crate::keymap::CanvasKeyMap::vim_defaults();
        let undo = [KeyStroke {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::empty(),
        }];
        let redo = [KeyStroke {
            code: KeyCode::Char('r'),
            modifiers: KeyModifiers::CONTROL,
        }];

        assert_eq!(
            keymap.lookup_action(AppMode::ReadOnly, &undo).0,
            Some(&CanvasKeyAction::Undo)
        );
        assert_eq!(
            keymap.lookup_action(AppMode::ReadOnly, &redo).0,
            Some(&CanvasKeyAction::Redo)
        );
    }
}
