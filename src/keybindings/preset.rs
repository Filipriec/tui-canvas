use std::collections::HashMap;
use std::fmt;

use toml::Value;

use crate::canvas::modes::AppMode;

use super::{display_binding, try_parse_binding, CanvasKeyAction, KeyStroke, ParseKeyError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasKeybindingPreset {
    sections: Vec<CanvasKeybindingPresetSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasKeybindingPresetSection {
    pub name: String,
    pub mode: AppMode,
    pub bindings: Vec<CanvasKeybindingPresetBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasKeybindingPresetBinding {
    pub action: CanvasKeyAction,
    pub keys: Vec<String>,
}

#[derive(Debug)]
pub enum CanvasKeybindingPresetError {
    Toml(toml::de::Error),
    Issues(Vec<CanvasKeybindingPresetIssue>),
    UnknownSection {
        section: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasKeybindingPresetIssue {
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
        source: ParseKeyError,
    },
    DuplicateBinding {
        section: String,
        mode: AppMode,
        binding: String,
        first_action: CanvasKeyAction,
        second_action: CanvasKeyAction,
    },
    BindingConflict {
        section: String,
        mode: AppMode,
        binding: String,
        action: CanvasKeyAction,
        existing_binding: String,
        existing_action: CanvasKeyAction,
        kind: CanvasKeybindingConflictKind,
    },
    UnsupportedMode {
        mode: AppMode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasKeybindingConflictKind {
    Exact,
    PrefixOf,
    ExtensionOf,
}

impl fmt::Display for CanvasKeybindingPresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(err) => write!(f, "invalid TOML: {err}"),
            Self::Issues(issues) => {
                write!(f, "{} canvas keybinding preset issue(s)", issues.len())?;
                for issue in issues {
                    write!(f, "; {issue}")?;
                }
                Ok(())
            }
            Self::UnknownSection { section } => {
                write!(f, "unknown canvas keybinding section {section:?}")
            }
        }
    }
}

impl fmt::Display for CanvasKeybindingPresetIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotTable => write!(f, "canvas keybinding preset must be a TOML table"),
            Self::SectionNotTable { section } => {
                write!(f, "canvas keybinding section {section:?} must be a table")
            }
            Self::ModeNotString { section } => {
                write!(f, "canvas keybinding section {section:?} has a non-string mode")
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
                source,
            } => {
                write!(
                    f,
                    "invalid binding {binding:?} for {} in section {section:?}: {source}",
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
            Self::BindingConflict {
                section,
                mode,
                binding,
                action,
                existing_binding,
                existing_action,
                kind,
            } => {
                let relationship = match kind {
                    CanvasKeybindingConflictKind::Exact => "is already bound as",
                    CanvasKeybindingConflictKind::PrefixOf => "is a prefix of",
                    CanvasKeybindingConflictKind::ExtensionOf => "extends",
                };
                write!(
                    f,
                    "binding {binding:?} for {} in mode {mode:?}, section {section:?} {relationship} {existing_binding:?} for {}",
                    action.as_str(),
                    existing_action.as_str()
                )
            }
            Self::UnsupportedMode { mode } => {
                write!(f, "canvas keybindings do not support runtime storage for mode {mode:?}")
            }
        }
    }
}

impl std::error::Error for CanvasKeybindingPresetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Toml(err) => Some(err),
            _ => None,
        }
    }
}

impl CanvasKeybindingPreset {
    pub fn from_toml(source: &str) -> Result<Self, CanvasKeybindingPresetError> {
        let value = source.parse::<Value>().map_err(CanvasKeybindingPresetError::Toml)?;
        let Some(table) = value.as_table() else {
            return Err(CanvasKeybindingPresetError::Issues(vec![
                CanvasKeybindingPresetIssue::RootNotTable,
            ]));
        };

        let mut sections = Vec::with_capacity(table.len());
        let mut issues = Vec::new();
        for (section_name, section_value) in table {
            let Some(section) = section_value.as_table() else {
                issues.push(CanvasKeybindingPresetIssue::SectionNotTable {
                    section: section_name.clone(),
                });
                continue;
            };

            let mode_name = match section.get("mode") {
                Some(value) => value.as_str().unwrap_or_else(|| {
                    issues.push(CanvasKeybindingPresetIssue::ModeNotString {
                        section: section_name.clone(),
                    });
                    section_name.as_str()
                }),
                None => section_name.as_str(),
            };
            let Some(mode) = app_mode_from_name(mode_name) else {
                issues.push(CanvasKeybindingPresetIssue::UnknownMode {
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
                    issues.push(CanvasKeybindingPresetIssue::UnknownAction {
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
                    issues.push(CanvasKeybindingPresetIssue::EmptyBindings {
                        section: section_name.clone(),
                        action: action_name.clone(),
                    });
                    continue;
                }

                bindings.push(CanvasKeybindingPresetBinding { action, keys });
            }

            sections.push(CanvasKeybindingPresetSection {
                name: section_name.clone(),
                mode,
                bindings,
            });
        }

        Self::validated(sections, issues)
    }

    pub fn from_mode_maps(
        read_only: &HashMap<String, Vec<String>>,
        edit: &HashMap<String, Vec<String>>,
        highlight: &HashMap<String, Vec<String>>,
    ) -> Result<Self, CanvasKeybindingPresetError> {
        let mut sections = Vec::new();
        let mut issues = Vec::new();
        for section in [
            section_from_mode_map("nor", AppMode::Nor, read_only),
            section_from_mode_map("ins", AppMode::Ins, edit),
            section_from_mode_map("sel", AppMode::Sel, highlight),
        ] {
            match section {
                Ok(section) => sections.push(section),
                Err(CanvasKeybindingPresetError::Issues(section_issues)) => {
                    issues.extend(section_issues)
                }
                Err(err) => return Err(err),
            }
        }
        Self::validated(sections, issues)
    }

    pub fn sections(&self) -> &[CanvasKeybindingPresetSection] {
        &self.sections
    }

    pub fn section(&self, name: &str) -> Option<&CanvasKeybindingPresetSection> {
        self.sections.iter().find(|section| section.name == name)
    }

    pub fn validate(&self) -> Result<(), CanvasKeybindingPresetError> {
        let issues = self.validation_issues();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(CanvasKeybindingPresetError::Issues(issues))
        }
    }

    pub fn validation_issues(&self) -> Vec<CanvasKeybindingPresetIssue> {
        let mut issues = Vec::new();
        let mut seen: HashMap<(String, Vec<KeyStroke>), (String, CanvasKeyAction, String)> =
            HashMap::new();
        let mut previous: Vec<(String, AppMode, Vec<KeyStroke>, CanvasKeyAction, String)> =
            Vec::new();
        for section in &self.sections {
            let mode_key = app_mode_name(section.mode).to_string();
            for binding in &section.bindings {
                for key in &binding.keys {
                    let sequence = match try_parse_binding(key) {
                        Ok(sequence) => sequence,
                        Err(source) => {
                            issues.push(CanvasKeybindingPresetIssue::InvalidBinding {
                                section: section.name.clone(),
                                action: binding.action.clone(),
                                binding: key.clone(),
                                source,
                            });
                            continue;
                        }
                    };
                    let previous = seen.insert(
                        (mode_key.clone(), sequence.clone()),
                        (section.name.clone(), binding.action.clone(), key.clone()),
                    );
                    if let Some((first_section, first_action, first_key)) = previous {
                        if first_action != binding.action {
                            issues.push(CanvasKeybindingPresetIssue::DuplicateBinding {
                                section: section.name.clone(),
                                mode: section.mode,
                                binding: key.clone(),
                                first_action: first_action.clone(),
                                second_action: binding.action.clone(),
                            });
                        }
                        seen.insert(
                            (mode_key.clone(), sequence.clone()),
                            (first_section, first_action, first_key),
                        );
                    }
                    for (
                        _existing_section,
                        existing_mode,
                        existing_sequence,
                        existing_action,
                        existing_key,
                    ) in &previous
                    {
                        if *existing_mode != section.mode || *existing_action == binding.action {
                            continue;
                        }
                        let Some(kind) = conflict_kind(&sequence, existing_sequence) else {
                            continue;
                        };
                        issues.push(CanvasKeybindingPresetIssue::BindingConflict {
                            section: section.name.clone(),
                            mode: section.mode,
                            binding: display_binding(&sequence),
                            action: binding.action.clone(),
                            existing_binding: existing_key.clone(),
                            existing_action: existing_action.clone(),
                            kind,
                        });
                    }
                    previous.push((
                        section.name.clone(),
                        section.mode,
                        sequence,
                        binding.action.clone(),
                        key.clone(),
                    ));
                }
            }
        }
        issues
    }

    fn validated(
        sections: Vec<CanvasKeybindingPresetSection>,
        mut issues: Vec<CanvasKeybindingPresetIssue>,
    ) -> Result<Self, CanvasKeybindingPresetError> {
        let preset = Self { sections };
        issues.extend(preset.validation_issues());
        if issues.is_empty() {
            Ok(preset)
        } else {
            Err(CanvasKeybindingPresetError::Issues(issues))
        }
    }
}

pub(crate) fn conflict_kind(
    requested: &[KeyStroke],
    existing: &[KeyStroke],
) -> Option<CanvasKeybindingConflictKind> {
    if requested == existing {
        Some(CanvasKeybindingConflictKind::Exact)
    } else if existing.starts_with(requested) {
        Some(CanvasKeybindingConflictKind::PrefixOf)
    } else if requested.starts_with(existing) {
        Some(CanvasKeybindingConflictKind::ExtensionOf)
    } else {
        None
    }
}

impl CanvasKeybindingPresetSection {
    pub fn validate(&self) -> Result<(), CanvasKeybindingPresetError> {
        let issues = self.validation_issues();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(CanvasKeybindingPresetError::Issues(issues))
        }
    }

    pub fn validation_issues(&self) -> Vec<CanvasKeybindingPresetIssue> {
        let preset = CanvasKeybindingPreset {
            sections: vec![self.clone()],
        };
        preset.validation_issues()
    }
}

fn section_from_mode_map(
    name: &str,
    mode: AppMode,
    map: &HashMap<String, Vec<String>>,
) -> Result<CanvasKeybindingPresetSection, CanvasKeybindingPresetError> {
    let mut issues = Vec::new();
    let mut bindings = Vec::new();
    for (action_name, keys) in map {
        let action = CanvasKeyAction::from_name(action_name);
        if matches!(action, CanvasKeyAction::Unknown(_)) {
            issues.push(CanvasKeybindingPresetIssue::UnknownAction {
                section: name.to_string(),
                action: action_name.clone(),
            });
            continue;
        }
        if keys.is_empty() {
            issues.push(CanvasKeybindingPresetIssue::EmptyBindings {
                section: name.to_string(),
                action: action_name.clone(),
            });
            continue;
        }
        bindings.push(CanvasKeybindingPresetBinding {
            action,
            keys: keys.clone(),
        });
    }

    if issues.is_empty() {
        Ok(CanvasKeybindingPresetSection {
            name: name.to_string(),
            mode,
            bindings,
        })
    } else {
        Err(CanvasKeybindingPresetError::Issues(issues))
    }
}

fn parse_string_list(
    section: &str,
    action: &str,
    value: &Value,
    issues: &mut Vec<CanvasKeybindingPresetIssue>,
) -> Option<Vec<String>> {
    let Some(keys) = parse_string_list_value(value) else {
        issues.push(CanvasKeybindingPresetIssue::BindingsNotStringList {
            section: section.to_string(),
            action: action.to_string(),
        });
        return None;
    };
    Some(keys)
}

fn parse_string_list_value(value: &Value) -> Option<Vec<String>> {
    if let Some(single) = value.as_str() {
        return Some(vec![single.to_string()]);
    }

    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(ToString::to_string))
        .collect()
}

pub(crate) fn app_mode_from_name(name: &str) -> Option<AppMode> {
    match name {
        "nor" | "read_only" | "normal" => Some(AppMode::Nor),
        "ins" | "edit" | "insert" => Some(AppMode::Ins),
        "sel" | "highlight" | "select" => Some(AppMode::Sel),
        "command" => Some(AppMode::Command),
        "general" => Some(AppMode::General),
        _ => None,
    }
}

pub(crate) fn app_mode_name(mode: AppMode) -> &'static str {
    match mode {
        AppMode::Nor => "nor",
        AppMode::Ins => "ins",
        AppMode::Sel => "sel",
        AppMode::Command => "command",
        AppMode::General => "general",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_multiple_validation_issues_with_parse_source() {
        let err = CanvasKeybindingPreset::from_toml(
            r#"
            [normal]
            unknown_action = ["x"]
            move_left = ["shift+tab"]
            move_right = ["backtab"]
            move_up = 1
            move_down = []
            move_word_next = ["ctrl+notakey"]
            "#,
        )
        .unwrap_err();

        let CanvasKeybindingPresetError::Issues(issues) = err else {
            panic!("expected validation issues");
        };
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeybindingPresetIssue::UnknownAction { .. })));
        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                CanvasKeybindingPresetIssue::BindingsNotStringList { .. }
            )
        }));
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeybindingPresetIssue::EmptyBindings { .. })));
        assert!(issues.iter().any(|issue| {
            matches!(
                issue,
                CanvasKeybindingPresetIssue::InvalidBinding {
                    source: ParseKeyError::UnknownKey(_),
                    ..
                }
            )
        }));
        assert!(issues
            .iter()
            .any(|issue| matches!(issue, CanvasKeybindingPresetIssue::DuplicateBinding { .. })));
    }
}
