// src/canvas/modes/manager.rs
//! App mode definitions used by the canvas editor.

use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Top-level application modes used by the canvas UI.
///
/// These modes control input handling, cursor behavior, and how the UI should
/// respond to user actions.
pub enum AppMode {
    /// For intro and admin screens
    General,
    /// Canvas insert mode (insertion/modification)
    Ins,
    /// Canvas selection mode (visual selection)
    Sel,
    /// Canvas normal mode (navigation)
    Nor,
    /// Command mode overlay (for commands)
    Command,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseAppModeError {
    name: String,
}

impl ParseAppModeError {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ParseAppModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown app mode {:?}", self.name)
    }
}

impl std::error::Error for ParseAppModeError {}

impl AppMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AppMode::Nor => "nor",
            AppMode::Ins => "ins",
            AppMode::Sel => "sel",
            AppMode::Command => "command",
            AppMode::General => "general",
        }
    }
}

impl FromStr for AppMode {
    type Err = ParseAppModeError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "nor" | "normal" | "read_only" | "readonly" => Ok(AppMode::Nor),
            "ins" | "insert" | "edit" => Ok(AppMode::Ins),
            "sel" | "select" | "selection" | "highlight" => Ok(AppMode::Sel),
            "general" => Ok(AppMode::General),
            "command" => Ok(AppMode::Command),
            _ => Err(ParseAppModeError {
                name: name.to_string(),
            }),
        }
    }
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for AppMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for AppMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = <String as serde::Deserialize>::deserialize(deserializer)?;
        name.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct ModeConfig {
        mode: AppMode,
    }

    #[test]
    fn app_mode_names_round_trip_through_traits_and_serde() {
        assert_eq!("normal".parse::<AppMode>(), Ok(AppMode::Nor));
        assert_eq!("selection".parse::<AppMode>(), Ok(AppMode::Sel));
        assert_eq!(AppMode::Ins.to_string(), "ins");
        assert_eq!(
            toml::from_str::<ModeConfig>("mode = \"read_only\"")
                .unwrap()
                .mode,
            AppMode::Nor
        );
        assert_eq!(
            toml::to_string(&ModeConfig {
                mode: AppMode::Command,
            })
            .unwrap(),
            "mode = \"command\"\n"
        );
    }
}
