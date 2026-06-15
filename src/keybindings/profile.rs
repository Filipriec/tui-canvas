use std::collections::HashSet;

use toml::Value;

use crate::canvas::modes::AppMode;

use super::{
    BuiltinCanvasKeybindingPreset, CanvasKeyAction, CanvasKeyBindingEntry, CanvasKeyBindings,
    CanvasKeybindingPresetError, CanvasKeybindingPresetIssue, display_binding,
};

#[derive(Clone, Debug)]
pub struct CanvasKeybindingProfile {
    preset: BuiltinCanvasKeybindingPreset,
    defaults: CanvasKeyBindings,
    current: CanvasKeyBindings,
}

struct OverrideSection {
    mode: AppMode,
    bindings: Vec<(CanvasKeyAction, Vec<String>)>,
}

impl CanvasKeybindingProfile {
    pub fn new(preset: BuiltinCanvasKeybindingPreset) -> Self {
        let defaults = CanvasKeyBindings::from_builtin_preset(preset);
        Self {
            preset,
            current: defaults.clone(),
            defaults,
        }
    }

    pub fn with_overrides_toml(
        preset: BuiltinCanvasKeybindingPreset,
        source: &str,
    ) -> Result<Self, CanvasKeybindingPresetError> {
        let mut profile = Self::new(preset);
        profile.apply_overrides_toml(source)?;
        Ok(profile)
    }

    pub fn preset(&self) -> BuiltinCanvasKeybindingPreset {
        self.preset
    }

    pub fn defaults(&self) -> &CanvasKeyBindings {
        &self.defaults
    }

    pub fn current(&self) -> &CanvasKeyBindings {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut CanvasKeyBindings {
        &mut self.current
    }

    pub fn entries(&self) -> Vec<CanvasKeyBindingEntry> {
        self.current.entries()
    }

    pub fn remap_action(
        &mut self,
        mode: AppMode,
        action: CanvasKeyAction,
        bindings: Vec<String>,
    ) -> Result<(), CanvasKeybindingPresetError> {
        self.current.remap_action(mode, action, bindings)
    }

    pub fn apply_overrides_toml(
        &mut self,
        source: &str,
    ) -> Result<(), CanvasKeybindingPresetError> {
        for section in parse_override_toml(source)? {
            for (action, bindings) in section.bindings {
                self.current.remap_action(section.mode, action, bindings)?;
            }
        }
        Ok(())
    }

    pub fn overrides_toml(&self) -> String {
        let mut out = String::new();
        for mode in [AppMode::Nor, AppMode::Ins, AppMode::Sel] {
            let mut lines = Vec::new();
            let mut actions = self.actions_for_mode(mode);
            actions.sort_by(|left, right| left.as_str().cmp(right.as_str()));
            actions.dedup();

            for action in actions {
                let default = binding_set(self.defaults.bindings_for(mode, &action));
                let current = binding_set(self.current.bindings_for(mode, &action));
                if default == current {
                    continue;
                }

                let mut display = current
                    .iter()
                    .map(|sequence| display_binding(sequence))
                    .collect::<Vec<_>>();
                display.sort();
                let value = if display.is_empty() {
                    "[]".to_string()
                } else {
                    format!(
                        "[{}]",
                        display
                            .into_iter()
                            .map(|binding| format!("{:?}", binding))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                lines.push(format!("{} = {}", action.as_str(), value));
            }

            if lines.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("[{}]\n", mode_section_name(mode)));
            for line in lines {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }

    fn actions_for_mode(&self, mode: AppMode) -> Vec<CanvasKeyAction> {
        self.defaults
            .entries()
            .into_iter()
            .chain(self.current.entries())
            .filter(|entry| entry.mode == mode)
            .map(|entry| entry.action)
            .collect()
    }
}

fn parse_override_toml(source: &str) -> Result<Vec<OverrideSection>, CanvasKeybindingPresetError> {
    if source.trim().is_empty() {
        return Ok(Vec::new());
    }

    let value = toml::from_str::<Value>(source).map_err(CanvasKeybindingPresetError::Toml)?;
    let Some(table) = value.as_table() else {
        return Err(CanvasKeybindingPresetError::Issues(vec![
            CanvasKeybindingPresetIssue::RootNotTable,
        ]));
    };

    let mut sections = Vec::new();
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

        let Ok(mode) = mode_name.parse::<AppMode>() else {
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
                parse_override_string_list(section_name, action_name, bindings_value, &mut issues)
            else {
                continue;
            };
            bindings.push((action, keys));
        }

        sections.push(OverrideSection { mode, bindings });
    }

    if issues.is_empty() {
        Ok(sections)
    } else {
        Err(CanvasKeybindingPresetError::Issues(issues))
    }
}

fn parse_override_string_list(
    section: &str,
    action: &str,
    value: &Value,
    issues: &mut Vec<CanvasKeybindingPresetIssue>,
) -> Option<Vec<String>> {
    if let Some(text) = value.as_str() {
        return Some(vec![text.to_string()]);
    }
    let Some(values) = value.as_array() else {
        issues.push(CanvasKeybindingPresetIssue::BindingsNotStringList {
            section: section.to_string(),
            action: action.to_string(),
        });
        return None;
    };

    let mut out = Vec::new();
    for value in values {
        let Some(text) = value.as_str() else {
            issues.push(CanvasKeybindingPresetIssue::BindingsNotStringList {
                section: section.to_string(),
                action: action.to_string(),
            });
            return None;
        };
        out.push(text.to_string());
    }
    Some(out)
}

fn binding_set(bindings: Vec<Vec<super::KeyStroke>>) -> HashSet<Vec<super::KeyStroke>> {
    bindings.into_iter().collect()
}

fn mode_section_name(mode: AppMode) -> &'static str {
    mode.as_str()
}
