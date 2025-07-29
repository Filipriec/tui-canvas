// canvas/src/gui/theme.rs

#[cfg(feature = "gui")]
use ratatui::style::Color;

/// Theme trait that must be implemented by applications using the canvas GUI
#[cfg(feature = "gui")]
pub trait CanvasTheme {
    fn bg(&self) -> Color;
    fn fg(&self) -> Color;
    fn border(&self) -> Color;
    fn accent(&self) -> Color;
    fn secondary(&self) -> Color;
    fn highlight(&self) -> Color;
    fn highlight_bg(&self) -> Color;
    fn warning(&self) -> Color;
}
