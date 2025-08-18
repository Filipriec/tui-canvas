// src/suggestions/gui.rs
//! Suggestions dropdown GUI (not inline autocomplete) updated to work with FormEditor

#[cfg(feature = "gui")]
use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph},  // Removed Borders
    Frame,
};

#[cfg(feature = "gui")]
use crate::canvas::theme::CanvasTheme;
use crate::data_provider::{DataProvider, SuggestionItem};
use crate::editor::FormEditor;

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthStr;

/// Render suggestions dropdown for FormEditor - call this AFTER rendering canvas
#[cfg(feature = "gui")]
pub fn render_suggestions_dropdown<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    frame_area: Rect,
    input_rect: Rect,
    theme: &T,
    editor: &FormEditor<D>,
) {
    let ui_state = editor.ui_state();

    if !ui_state.is_suggestions_active() {
        return;
    }

    if ui_state.suggestions.is_loading {
        render_loading_indicator(f, frame_area, input_rect, theme);
    } else if !editor.suggestions().is_empty() {
        render_suggestions_dropdown_list(f, frame_area, input_rect, theme, editor.suggestions(), ui_state.suggestions.selected_index);
    }
}

/// Show loading spinner/text
#[cfg(feature = "gui")]
fn render_loading_indicator<T: CanvasTheme>(
    f: &mut Frame,
    frame_area: Rect,
    input_rect: Rect,
    theme: &T,
) {
    let loading_text = "Loading suggestions...";
    let loading_width = loading_text.width() as u16 + 4; // +4 for borders and padding
    let loading_height = 3;

    let dropdown_area = calculate_dropdown_position(
        input_rect,
        frame_area,
        loading_width,
        loading_height,
    );

    let loading_block = Block::default()
        .style(Style::default().bg(theme.bg()));

    let loading_paragraph = Paragraph::new(loading_text)
        .block(loading_block)
        .style(Style::default().fg(theme.fg()))
        .alignment(Alignment::Center);

    f.render_widget(loading_paragraph, dropdown_area);
}

/// Show actual suggestions list
#[cfg(feature = "gui")]
fn render_suggestions_dropdown_list<T: CanvasTheme>(
    f: &mut Frame,
    frame_area: Rect,
    input_rect: Rect,
    theme: &T,
    suggestions: &[SuggestionItem],  // Fixed: Removed <String> generic parameter
    selected_index: Option<usize>,
) {
    let display_texts: Vec<&str> = suggestions
        .iter()
        .map(|item| item.display_text.as_str())
        .collect();

    let dropdown_dimensions = calculate_dropdown_dimensions(&display_texts);
    let dropdown_area = calculate_dropdown_position(
        input_rect,
        frame_area,
        dropdown_dimensions.width,
        dropdown_dimensions.height,
    );

    // Background
    let dropdown_block = Block::default()
        .style(Style::default().bg(theme.bg()));

    // List items
    let items = create_suggestion_list_items(
        &display_texts,
        selected_index,
        dropdown_dimensions.width,
        theme,
    );

    let list = List::new(items).block(dropdown_block);
    let mut list_state = ListState::default();
    list_state.select(selected_index);

    f.render_stateful_widget(list, dropdown_area, &mut list_state);
}

/// Calculate dropdown size based on suggestions
#[cfg(feature = "gui")]
fn calculate_dropdown_dimensions(display_texts: &[&str]) -> DropdownDimensions {
    let max_width = display_texts
        .iter()
        .map(|text| text.width())
        .max()
        .unwrap_or(0) as u16;

    let horizontal_padding = 2;
    let width = (max_width + horizontal_padding).max(10);
    let height = (display_texts.len() as u16).min(5);

    DropdownDimensions { width, height }
}

/// Position dropdown to stay in bounds
#[cfg(feature = "gui")]
fn calculate_dropdown_position(
    input_rect: Rect,
    frame_area: Rect,
    dropdown_width: u16,
    dropdown_height: u16,
) -> Rect {
    let mut dropdown_area = Rect {
        x: input_rect.x,
        y: input_rect.y + 1, // below input field
        width: dropdown_width,
        height: dropdown_height,
    };

    // Keep in bounds
    if dropdown_area.bottom() > frame_area.height {
        dropdown_area.y = input_rect.y.saturating_sub(dropdown_height);
    }
    if dropdown_area.right() > frame_area.width {
        dropdown_area.x = frame_area.width.saturating_sub(dropdown_width);
    }
    dropdown_area.x = dropdown_area.x;
    dropdown_area.y = dropdown_area.y;

    dropdown_area
}

/// Create styled list items
#[cfg(feature = "gui")]
fn create_suggestion_list_items<'a, T: CanvasTheme>(
    display_texts: &'a [&'a str],
    selected_index: Option<usize>,
    dropdown_width: u16,
    theme: &T,
) -> Vec<ListItem<'a>> {
    let available_width = dropdown_width;

    display_texts
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let is_selected = selected_index == Some(i);
            let text_width = text.width() as u16;
            let padding_needed = available_width.saturating_sub(text_width);
            let padded_text = format!("{}{}", text, " ".repeat(padding_needed as usize));

            ListItem::new(padded_text).style(if is_selected {
                Style::default()
                    .fg(theme.bg())
                    .bg(theme.highlight())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg()).bg(theme.bg())
            })
        })
        .collect()
}

/// Helper struct for dropdown dimensions
#[cfg(feature = "gui")]
struct DropdownDimensions {
    width: u16,
    height: u16,
}
