// canvas/src/gui/render.rs

#[cfg(feature = "gui")]
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::state::CanvasState;
use crate::modes::HighlightState;
#[cfg(feature = "gui")]
use super::theme::CanvasTheme;
#[cfg(feature = "gui")]
use std::cmp::{max, min};

/// Render canvas using the CanvasState trait and CanvasTheme
#[cfg(feature = "gui")]
pub fn render_canvas<T: CanvasTheme>(
    f: &mut Frame,
    area: Rect,
    form_state: &impl CanvasState,
    theme: &T,
    is_edit_mode: bool,
    highlight_state: &HighlightState,
) -> Option<Rect> {
    let fields: Vec<&str> = form_state.fields();
    let current_field_idx = form_state.current_field();
    let inputs: Vec<&String> = form_state.inputs();
    
    render_canvas_impl(
        f,
        area,
        &fields,
        &current_field_idx,
        &inputs,
        theme,
        is_edit_mode,
        highlight_state,
        form_state.current_cursor_pos(),
        form_state.has_unsaved_changes(),
        |i| form_state.get_display_value_for_field(i).to_string(),
        |i| form_state.has_display_override(i),
    )
}

/// Internal implementation of canvas rendering
#[cfg(feature = "gui")]
fn render_canvas_impl<T: CanvasTheme, F1, F2>(
    f: &mut Frame,
    area: Rect,
    fields: &[&str],
    current_field_idx: &usize,
    inputs: &[&String],
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
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    let border_style = if has_unsaved_changes {
        Style::default().fg(theme.warning())
    } else if is_edit_mode {
        Style::default().fg(theme.accent())
    } else {
        Style::default().fg(theme.secondary())
    };
    
    let input_container = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg()));

    let input_block = Rect {
        x: columns[1].x,
        y: columns[1].y,
        width: columns[1].width,
        height: fields.len() as u16 + 2,
    };

    f.render_widget(&input_container, input_block);

    let input_area = input_container.inner(input_block);
    let input_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); fields.len()])
        .split(input_area);

    let mut active_field_input_rect = None;

    // Render field labels
    for (i, field) in fields.iter().enumerate() {
        let label = Paragraph::new(Line::from(Span::styled(
            format!("{}:", field),
            Style::default().fg(theme.fg()),
        )));
        f.render_widget(
            label,
            Rect {
                x: columns[0].x,
                y: input_block.y + 1 + i as u16,
                width: columns[0].width,
                height: 1,
            },
        );
    }

    // Render field values
    for (i, _input) in inputs.iter().enumerate() {
        let is_active = i == *current_field_idx;

        // Use the provided closure to get display value
        let text = get_display_value(i);
        let text_len = text.chars().count();
        let line: Line;

        match highlight_state {
            HighlightState::Off => {
                line = Line::from(Span::styled(
                    &text,
                    if is_active {
                        Style::default().fg(theme.highlight())
                    } else {
                        Style::default().fg(theme.fg())
                    },
                ));
            }
            HighlightState::Characterwise { anchor } => {
                let (anchor_field, anchor_char) = *anchor;
                let start_field = min(anchor_field, *current_field_idx);
                let end_field = max(anchor_field, *current_field_idx);

                let (start_char, end_char) = if anchor_field == *current_field_idx {
                    (min(anchor_char, current_cursor_pos), max(anchor_char, current_cursor_pos))
                } else if anchor_field < *current_field_idx {
                    (anchor_char, current_cursor_pos)
                } else {
                    (current_cursor_pos, anchor_char)
                };

                let highlight_style = Style::default()
                    .fg(theme.highlight())
                    .bg(theme.highlight_bg())
                    .add_modifier(Modifier::BOLD);
                let normal_style_in_highlight = Style::default().fg(theme.highlight());
                let normal_style_outside = Style::default().fg(theme.fg());

                if i >= start_field && i <= end_field {
                    if start_field == end_field {
                        let clamped_start = start_char.min(text_len);
                        let clamped_end = end_char.min(text_len);

                        let before: String = text.chars().take(clamped_start).collect();
                        let highlighted: String = text.chars()
                            .skip(clamped_start)
                            .take(clamped_end.saturating_sub(clamped_start) + 1)
                            .collect();
                        let after: String = text.chars().skip(clamped_end + 1).collect();

                        line = Line::from(vec![
                            Span::styled(before, normal_style_in_highlight),
                            Span::styled(highlighted, highlight_style),
                            Span::styled(after, normal_style_in_highlight),
                        ]);
                    } else if i == start_field {
                        let safe_start = start_char.min(text_len);
                        let before: String = text.chars().take(safe_start).collect();
                        let highlighted: String = text.chars().skip(safe_start).collect();
                        line = Line::from(vec![
                            Span::styled(before, normal_style_in_highlight),
                            Span::styled(highlighted, highlight_style),
                        ]);
                    } else if i == end_field {
                        let safe_end_inclusive = if text_len > 0 { end_char.min(text_len - 1) } else { 0 };
                        let highlighted: String = text.chars().take(safe_end_inclusive + 1).collect();
                        let after: String = text.chars().skip(safe_end_inclusive + 1).collect();
                        line = Line::from(vec![
                            Span::styled(highlighted, highlight_style),
                            Span::styled(after, normal_style_in_highlight),
                        ]);
                    } else {
                        line = Line::from(Span::styled(&text, highlight_style));
                    }
                } else {
                    line = Line::from(Span::styled(
                        &text,
                        if is_active { normal_style_in_highlight } else { normal_style_outside }
                    ));
                }
            }
            HighlightState::Linewise { anchor_line } => {
                let start_field = min(*anchor_line, *current_field_idx);
                let end_field = max(*anchor_line, *current_field_idx);
                let highlight_style = Style::default()
                    .fg(theme.highlight())
                    .bg(theme.highlight_bg())
                    .add_modifier(Modifier::BOLD);
                let normal_style_in_highlight = Style::default().fg(theme.highlight());
                let normal_style_outside = Style::default().fg(theme.fg());

                if i >= start_field && i <= end_field {
                    line = Line::from(Span::styled(&text, highlight_style));
                } else {
                    line = Line::from(Span::styled(
                        &text,
                        if is_active { normal_style_in_highlight } else { normal_style_outside }
                    ));
                }
            }
        }

        let input_display = Paragraph::new(line).alignment(Alignment::Left);
        f.render_widget(input_display, input_rows[i]);

        if is_active {
            active_field_input_rect = Some(input_rows[i]);

            // Use the provided closure to check for display override
            let cursor_x = if has_display_override(i) {
                // If an override exists, place the cursor at the end.
                input_rows[i].x + text.chars().count() as u16
            } else {
                // Otherwise, use the real cursor position.
                input_rows[i].x + current_cursor_pos as u16
            };
            let cursor_y = input_rows[i].y;
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }

    active_field_input_rect
}
