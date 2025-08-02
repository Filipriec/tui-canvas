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


#[cfg(feature = "gui")]
#[derive(Debug, Clone, Default)]
pub struct DefaultCanvasTheme;

#[cfg(feature = "gui")]
impl CanvasTheme for DefaultCanvasTheme {
    fn bg(&self) -> Color {
        Color::Black
    }
    fn fg(&self) -> Color {
        Color::White
    }
    fn border(&self) -> Color {
        Color::DarkGray
    }
    fn accent(&self) -> Color {
        Color::Cyan
    }
    fn secondary(&self) -> Color {
        Color::Gray
    }
    fn highlight(&self) -> Color {
        Color::Yellow
    }
    fn highlight_bg(&self) -> Color {
        Color::Blue
    }
    fn warning(&self) -> Color {
        Color::Red
    }
}
