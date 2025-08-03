// src/canvas/gui.rs
//! Canvas GUI updated to work with FormEditor

#[cfg(feature = "gui")]
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph},
    Frame,
};

#[cfg(feature = "gui")]
use crate::canvas::theme::{CanvasTheme, DefaultCanvasTheme};
use crate::canvas::modes::HighlightState;
use crate::data_provider::DataProvider;
use crate::editor::FormEditor;

#[cfg(feature = "gui")]
use std::cmp::{max, min};

/// Render ONLY the canvas form fields - no autocomplete
/// Updated to work with FormEditor instead of CanvasState trait
#[cfg(feature = "gui")]
pub fn render_canvas<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &FormEditor<D>,
    theme: &T,
) -> Option<Rect> {
    // Convert SelectionState to HighlightState
    let highlight_state = convert_selection_to_highlight(editor.ui_state().selection_state());
    render_canvas_with_highlight(f, area, editor, theme, &highlight_state)
}

/// Render canvas with explicit highlight state (for advanced use)
#[cfg(feature = "gui")]
pub fn render_canvas_with_highlight<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &FormEditor<D>,
    theme: &T,
    highlight_state: &HighlightState,
) -> Option<Rect> {
    let ui_state = editor.ui_state();
    let data_provider = editor.data_provider();

    // Build field information
    let field_count = data_provider.field_count();
    let mut fields: Vec<&str> = Vec::with_capacity(field_count);
    let mut inputs: Vec<String> = Vec::with_capacity(field_count);

    for i in 0..field_count {
        fields.push(data_provider.field_name(i));
        inputs.push(data_provider.field_value(i).to_string());
    }

    let current_field_idx = ui_state.current_field();
    let is_edit_mode = matches!(ui_state.mode(), crate::canvas::modes::AppMode::Edit);

    render_canvas_fields(
        f,
        area,
        &fields,
        &current_field_idx,
        &inputs,
        theme,
        is_edit_mode,
        highlight_state, // Now using the actual highlight state!
        ui_state.cursor_position(),
        false, // TODO: track unsaved changes in editor
        |i| {
            data_provider.display_value(i).unwrap_or(data_provider.field_value(i)).to_string()
        },
        |i| data_provider.display_value(i).is_some(),
    )
}

/// Convert SelectionState to HighlightState for rendering
#[cfg(feature = "gui")]
fn convert_selection_to_highlight(selection: &crate::canvas::state::SelectionState) -> HighlightState {
    use crate::canvas::state::SelectionState;

    match selection {
        SelectionState::None => HighlightState::Off,
        SelectionState::Characterwise { anchor } => HighlightState::Characterwise { anchor: *anchor },
        SelectionState::Linewise { anchor_field } => HighlightState::Linewise { anchor_line: *anchor_field },
    }
}

/// Core canvas field rendering
#[cfg(feature = "gui")]
fn render_canvas_fields<T: CanvasTheme, F1, F2>(
    f: &mut Frame,
    area: Rect,
    fields: &[&str],
    current_field_idx: &usize,
    inputs: &[String],
    theme: &T,
    is_edit_mode: bool,
    highlight_state: &HighlightState,
    current_cursor_pos: usize,
    has_unsaved_changes: bool,
    get_display_value: F1,
    has_display_override: F2,
) -> Option<Rect>
where
    F1: Fn(usize) -> String,
    F2: Fn(usize) -> bool,
{
    // Create layout
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Border style based on state
    let border_style = if has_unsaved_changes {
        Style::default().fg(theme.warning())
    } else if is_edit_mode {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.secondary())
    };

    // Input container
    let input_container = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg()));

    let input_block = Rect {
        x: columns[1].x,
        y: columns[1].y,
        width: columns[1].width,
        height: fields.len() as u16 + 2,
    };

    f.render_widget(&input_container, input_block);

    // Input area layout
    let input_area = input_container.inner(input_block);
    let input_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); fields.len()])
        .split(input_area);

    // Render field labels
    render_field_labels(f, columns[0], input_block, fields, theme);

    // Render field values and return active field rect
    render_field_values(
        f,
        input_rows.to_vec(),
        inputs,
        current_field_idx,
        theme,
        highlight_state,
        current_cursor_pos,
        get_display_value,
        has_display_override,
    )
}

/// Render field labels
#[cfg(feature = "gui")]
fn render_field_labels<T: CanvasTheme>(
    f: &mut Frame,
    label_area: Rect,
    input_block: Rect,
    fields: &[&str],
    theme: &T,
) {
    for (i, field) in fields.iter().enumerate() {
        let label = Paragraph::new(Line::from(Span::styled(
            format!("{}:", field),
            Style::default().fg(theme.fg()),
        )));
        f.render_widget(
            label,
            Rect {
                x: label_area.x,
                y: input_block.y + 1 + i as u16,
                width: label_area.width,
                height: 1,
            },
        );
    }
}

/// Render field values with highlighting
#[cfg(feature = "gui")]
fn render_field_values<T: CanvasTheme, F1, F2>(
    f: &mut Frame,
    input_rows: Vec<Rect>,
    inputs: &[String],
    current_field_idx: &usize,
    theme: &T,
    highlight_state: &HighlightState,
    current_cursor_pos: usize,
    get_display_value: F1,
    has_display_override: F2,
) -> Option<Rect>
where
    F1: Fn(usize) -> String,
    F2: Fn(usize) -> bool,
{
    let mut active_field_input_rect = None;

    for (i, _input) in inputs.iter().enumerate() {
        let is_active = i == *current_field_idx;
        let text = get_display_value(i);

        // Apply highlighting
        let line = apply_highlighting(
            &text,
            i,
            current_field_idx,
            current_cursor_pos,
            highlight_state,
            theme,
            is_active,
        );

        let input_display = Paragraph::new(line).alignment(Alignment::Left);
        f.render_widget(input_display, input_rows[i]);

        // Set cursor for active field
        if is_active {
            active_field_input_rect = Some(input_rows[i]);
            set_cursor_position(f, input_rows[i], &text, current_cursor_pos, has_display_override(i));
        }
    }

    active_field_input_rect
}

/// Apply highlighting based on highlight state
#[cfg(feature = "gui")]
fn apply_highlighting<'a, T: CanvasTheme>(
    text: &'a str,
    field_index: usize,
    current_field_idx: &usize,
    current_cursor_pos: usize,
    highlight_state: &HighlightState,
    theme: &T,
    is_active: bool,
) -> Line<'a> {
    let text_len = text.chars().count();

    match highlight_state {
        HighlightState::Off => {
            Line::from(Span::styled(
                text,
                Style::default().fg(theme.fg())
            ))
        }
        HighlightState::Characterwise { anchor } => {
            apply_characterwise_highlighting(text, text_len, field_index, current_field_idx, current_cursor_pos, anchor, theme, is_active)
        }
        HighlightState::Linewise { anchor_line } => {
            apply_linewise_highlighting(text, field_index, current_field_idx, anchor_line, theme, is_active)
        }
    }
}

/// Apply characterwise highlighting - DIRECTION-AWARE VERSION
#[cfg(feature = "gui")]
fn apply_characterwise_highlighting<'a, T: CanvasTheme>(
    text: &'a str,
    text_len: usize,
    field_index: usize,
    current_field_idx: &usize,
    current_cursor_pos: usize,
    anchor: &(usize, usize),
    theme: &T,
    is_active: bool,
) -> Line<'a> {
    let (anchor_field, anchor_char) = *anchor;
    let start_field = min(anchor_field, *current_field_idx);
    let end_field = max(anchor_field, *current_field_idx);

    let highlight_style = Style::default()
        .fg(theme.highlight())
        .bg(theme.highlight_bg())
        .add_modifier(Modifier::BOLD);
    let normal_style_in_highlight = Style::default().fg(theme.highlight());
    let normal_style_outside = Style::default().fg(theme.fg());

    if field_index >= start_field && field_index <= end_field {
        if start_field == end_field {
            // Single field selection - same as before
            let (start_char, end_char) = if anchor_field == *current_field_idx {
                (min(anchor_char, current_cursor_pos), max(anchor_char, current_cursor_pos))
            } else if anchor_field < *current_field_idx {
                (anchor_char, current_cursor_pos)
            } else {
                (current_cursor_pos, anchor_char)
            };

            let clamped_start = start_char.min(text_len);
            let clamped_end = end_char.min(text_len);

            let before: String = text.chars().take(clamped_start).collect();
            let highlighted: String = text.chars()
                .skip(clamped_start)
                .take(clamped_end.saturating_sub(clamped_start) + 1)
                .collect();
            let after: String = text.chars().skip(clamped_end + 1).collect();

            Line::from(vec![
                Span::styled(before, normal_style_in_highlight),
                Span::styled(highlighted, highlight_style),
                Span::styled(after, normal_style_in_highlight),
            ])
        } else {
            // Multi-field selection - think in terms of anchor→current direction
            if field_index == anchor_field {
                // Anchor field: highlight from anchor position toward the selection
                if anchor_field < *current_field_idx {
                    // Downward selection: highlight from anchor to end of field
                    let clamped_start = anchor_char.min(text_len);
                    let before: String = text.chars().take(clamped_start).collect();
                    let highlighted: String = text.chars().skip(clamped_start).collect();

                    Line::from(vec![
                        Span::styled(before, normal_style_in_highlight),
                        Span::styled(highlighted, highlight_style),
                    ])
                } else {
                    // Upward selection: highlight from start of field to anchor
                    let clamped_end = anchor_char.min(text_len);
                    let highlighted: String = text.chars().take(clamped_end + 1).collect();
                    let after: String = text.chars().skip(clamped_end + 1).collect();

                    Line::from(vec![
                        Span::styled(highlighted, highlight_style),
                        Span::styled(after, normal_style_in_highlight),
                    ])
                }
            } else if field_index == *current_field_idx {
                // Current field: highlight toward the cursor position
                if anchor_field < *current_field_idx {
                    // Downward selection: highlight from start of field to cursor
                    let clamped_end = current_cursor_pos.min(text_len);
                    let highlighted: String = text.chars().take(clamped_end + 1).collect();
                    let after: String = text.chars().skip(clamped_end + 1).collect();

                    Line::from(vec![
                        Span::styled(highlighted, highlight_style),
                        Span::styled(after, normal_style_in_highlight),
                    ])
                } else {
                    // Upward selection: highlight from cursor to end of field
                    let clamped_start = current_cursor_pos.min(text_len);
                    let before: String = text.chars().take(clamped_start).collect();
                    let highlighted: String = text.chars().skip(clamped_start).collect();

                    Line::from(vec![
                        Span::styled(before, normal_style_in_highlight),
                        Span::styled(highlighted, highlight_style),
                    ])
                }
            } else {
                // Middle field between anchor and current: highlight entire field
                Line::from(Span::styled(text, highlight_style))
            }
        }
    } else {
        Line::from(Span::styled(
            text,
            if is_active { normal_style_in_highlight } else { normal_style_outside }
        ))
    }
}

/// Apply linewise highlighting - VISUALLY DISTINCT VERSION
#[cfg(feature = "gui")]
fn apply_linewise_highlighting<'a, T: CanvasTheme>(
    text: &'a str,
    field_index: usize,
    current_field_idx: &usize,
    anchor_line: &usize,
    theme: &T,
    is_active: bool,
) -> Line<'a> {
    let start_field = min(*anchor_line, *current_field_idx);
    let end_field = max(*anchor_line, *current_field_idx);

    // Use the SAME style as characterwise highlighting
    let highlight_style = Style::default()
        .fg(theme.highlight())
        .bg(theme.highlight_bg())
        .add_modifier(Modifier::BOLD);

    let normal_style_in_highlight = Style::default().fg(theme.highlight());
    let normal_style_outside = Style::default().fg(theme.fg());

    if field_index >= start_field && field_index <= end_field {
        // ALWAYS highlight entire line - no markers, just full line highlighting
        Line::from(Span::styled(text, highlight_style))
    } else {
        Line::from(Span::styled(
            text,
            if is_active { normal_style_in_highlight } else { normal_style_outside }
        ))
    }
}

/// Set cursor position
#[cfg(feature = "gui")]
fn set_cursor_position(
    f: &mut Frame,
    field_rect: Rect,
    text: &str,
    current_cursor_pos: usize,
    has_display_override: bool,
) {
    let cursor_x = if has_display_override {
        field_rect.x + text.chars().count() as u16
    } else {
        field_rect.x + current_cursor_pos as u16
    };
    let cursor_y = field_rect.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

/// Set default theme if custom not specified
#[cfg(feature = "gui")]
pub fn render_canvas_default<D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &FormEditor<D>,
) -> Option<Rect> {
    let theme = DefaultCanvasTheme::default();
    render_canvas(f, area, editor, &theme)
}
