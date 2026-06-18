// src/canvas/theme.rs

#[cfg(feature = "gui")]
use ratatui::style::{Color, Modifier, Style};

/// Theme trait that must be implemented by applications using the canvas GUI.
///
/// Each method returns a [`Style`] that captures the full visual intent for a
/// specific UI role (foreground, background, modifiers). This allows Helix
/// theme scopes to be mapped directly without losing information.
#[cfg(feature = "gui")]
pub trait CanvasTheme {
    /// Background fill for the canvas surface and dropdown panels.
    fn background(&self) -> Style;
    /// Field labels rendered beside each input.
    fn label(&self) -> Style;
    /// Non-active input field text.
    fn input(&self) -> Style;
    /// Active (focused) input field text.
    fn input_active(&self) -> Style;
    /// Text selection highlight (characterwise and linewise).
    fn selection(&self) -> Style;
    /// Inline completion / ghost text shown after the cursor.
    fn completion(&self) -> Style;
    /// Non-selected suggestions dropdown items.
    fn suggestions(&self) -> Style;
    /// Selected / highlighted suggestion in the dropdown.
    fn suggestion_selected(&self) -> Style;
    /// Warning-style emphasis for validation errors, etc.
    ///
    /// Reserved for future use; not yet consumed by tui-canvas render
    /// code. Implementors should map this to a suitable Helix scope
    /// (e.g. `warning` / `diagnostic.warning`).
    fn warning(&self) -> Style;
    /// Inactive border style.
    ///
    /// Reserved for future use; not yet consumed by tui-canvas render
    /// code. Implementors should map this to a suitable Helix scope
    /// (e.g. `ui.window`).
    fn border(&self) -> Style;
    /// Active / focused border style.
    ///
    /// Reserved for future use; not yet consumed by tui-canvas render
    /// code. Implementors should map this to a suitable Helix scope
    /// (e.g. `ui.cursor.primary.normal` / `ui.text.focus`).
    fn border_active(&self) -> Style;
}

#[cfg(feature = "gui")]
#[derive(Debug, Clone, Default)]
pub struct DefaultCanvasTheme;

#[cfg(feature = "gui")]
impl CanvasTheme for DefaultCanvasTheme {
    fn background(&self) -> Style {
        Style::default().bg(Color::Black)
    }
    fn label(&self) -> Style {
        Style::default().fg(Color::White)
    }
    fn input(&self) -> Style {
        Style::default().fg(Color::White)
    }
    fn input_active(&self) -> Style {
        Style::default().fg(Color::White)
    }
    fn selection(&self) -> Style {
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    }
    fn completion(&self) -> Style {
        Style::default().fg(Color::DarkGray)
    }
    fn suggestions(&self) -> Style {
        Style::default().fg(Color::White).bg(Color::Black)
    }
    fn suggestion_selected(&self) -> Style {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    }
    fn warning(&self) -> Style {
        Style::default().fg(Color::Red)
    }
    fn border(&self) -> Style {
        Style::default().fg(Color::DarkGray)
    }
    fn border_active(&self) -> Style {
        Style::default().fg(Color::Cyan)
    }
}
