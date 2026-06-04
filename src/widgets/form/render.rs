// src/widgets/form/render.rs
//! Canvas GUI rendering helpers.

#[cfg(feature = "gui")]
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

use crate::canvas::modes::HighlightState;
#[cfg(feature = "gui")]
use crate::canvas::theme::{CanvasTheme, DefaultCanvasTheme};
use crate::data_provider::DataProvider;
use crate::editor::FormEditor;
#[cfg(feature = "gui")]
use crate::gui_utils::{
    clip_inline_completion_with_indicator_padded, compute_h_scroll_with_padding, display_width,
    RIGHT_PAD,
};
#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

#[cfg(feature = "gui")]
use std::cmp::{max, min};

#[cfg(feature = "gui")]
#[derive(Debug, Clone, Copy)]
/// How to handle overflow when rendering a field's content.
pub enum OverflowMode {
    /// Show an indicator character at the left/right when text is truncated.
    /// Common default is '$'.
    Indicator(char),
    /// Wrap content into multiple visual lines.
    Wrap,
}

#[cfg(feature = "gui")]
#[derive(Debug, Clone, Copy)]
/// Display options controlling canvas rendering behavior.
pub struct CanvasDisplayOptions {
    /// How to handle horizontal overflow for fields.
    pub overflow: OverflowMode,
}

#[cfg(feature = "gui")]
impl Default for CanvasDisplayOptions {
    fn default() -> Self {
        Self {
            overflow: OverflowMode::Indicator('$'),
        }
    }
}

/// Utility: clip a string to fit width, append indicator if overflow
#[cfg(feature = "gui")]
fn clip_with_indicator_line<'a>(s: &'a str, width: u16, indicator: char) -> Line<'a> {
    if width == 0 {
        return Line::from("");
    }
    if display_width(s) <= width {
        return Line::from(Span::raw(s));
    }
    let budget = width.saturating_sub(1);
    let mut out = String::new();
    let mut used: u16 = 0;
    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if used + w > budget {
            break;
        }
        out.push(ch);
        used = used.saturating_add(w);
    }
    Line::from(vec![Span::raw(out), Span::raw(indicator.to_string())])
}

#[cfg(feature = "gui")]
fn render_active_line_with_indicator<T: CanvasTheme>(
    typed_text: &str,
    completion: Option<&str>,
    width: u16,
    indicator: char,
    cursor_chars: usize,
    theme: &T,
) -> (Line<'static>, u16, u16) {
    if width == 0 {
        return (Line::from(""), 0, 0);
    }

    // Cursor display column
    let mut cursor_cols: u16 = 0;
    for (i, ch) in typed_text.chars().enumerate() {
        if i >= cursor_chars {
            break;
        }
        cursor_cols = cursor_cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
    }

    let (h_scroll, left_cols) = compute_h_scroll_with_padding(cursor_cols, width);

    (
        clip_inline_completion_with_indicator_padded(
            typed_text,
            completion,
            width,
            indicator,
            h_scroll,
            Style::default().fg(theme.fg()),
            Style::default().fg(theme.suggestion_gray()),
        ),
        h_scroll,
        left_cols,
    )
}

#[cfg(feature = "gui")]
/// Render the canvas into the provided frame using default display options.
///
/// Returns the rectangle of the active input field if present.
pub fn render_canvas<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &FormEditor<D>,
    theme: &T,
) -> Option<Rect> {
    let opts = CanvasDisplayOptions::default();
    render_canvas_with_options(f, area, editor, theme, opts)
}

#[cfg(feature = "gui")]
/// Render the canvas into the provided frame with explicit display options.
///
/// This is the more configurable entrypoint for rendering and is useful for
/// tests or when callers need to override overflow handling.
pub fn render_canvas_with_options<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &FormEditor<D>,
    theme: &T,
    opts: CanvasDisplayOptions,
) -> Option<Rect> {
    let highlight_state = convert_selection_to_highlight(editor.ui_state().selection_state());

    #[cfg(feature = "suggestions")]
    let active_completion = if editor.ui_state().is_suggestions_active()
        && editor.ui_state().suggestions.active_field == Some(editor.ui_state().current_field())
    {
        editor.ui_state().suggestions.completion_text.clone()
    } else {
        None
    };
    #[cfg(not(feature = "suggestions"))]
    let active_completion: Option<String> = None;

    render_canvas_with_highlight_and_options(
        f,
        area,
        editor,
        theme,
        &highlight_state,
        active_completion,
        opts,
    )
}

#[cfg(feature = "gui")]
fn render_canvas_with_highlight_and_options<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &FormEditor<D>,
    theme: &T,
    highlight_state: &HighlightState,
    active_completion: Option<String>,
    opts: CanvasDisplayOptions,
) -> Option<Rect> {
    let ui_state = editor.ui_state();
    let data_provider = editor.data_provider();

    let field_count = data_provider.field_count();
    let mut fields: Vec<&str> = Vec::with_capacity(field_count);
    let mut inputs: Vec<String> = Vec::with_capacity(field_count);

    for i in 0..field_count {
        fields.push(data_provider.field_name(i));
        #[cfg(feature = "validation")]
        {
            inputs.push(editor.display_text_for_field(i));
        }
        #[cfg(not(feature = "validation"))]
        {
            inputs.push(data_provider.field_value(i).to_string());
        }
    }

    let current_field_idx = ui_state.current_field();
    let is_edit_mode = matches!(ui_state.mode(), crate::canvas::modes::AppMode::Edit);

    render_canvas_fields_with_options(
        f,
        area,
        &fields,
        &current_field_idx,
        &inputs,
        theme,
        is_edit_mode,
        highlight_state,
        editor.display_cursor_position(),
        false,
        #[cfg(feature = "validation")]
        |field_idx| editor.display_text_for_field(field_idx),
        #[cfg(not(feature = "validation"))]
        |field_idx| data_provider.field_value(field_idx).to_string(),
        #[cfg(feature = "validation")]
        |field_idx| {
            editor
                .ui_state()
                .validation_state()
                .get_field_config(field_idx)
                .map(|cfg| cfg.custom_formatter.is_some() || cfg.display_mask.is_some())
                .unwrap_or(false)
        },
        #[cfg(not(feature = "validation"))]
        |_field_idx| false,
        active_completion,
        opts,
    )
}

#[cfg(feature = "gui")]
fn convert_selection_to_highlight(
    selection: &crate::canvas::state::SelectionState,
) -> HighlightState {
    use crate::canvas::state::SelectionState;

    match selection {
        SelectionState::None => HighlightState::Off,
        SelectionState::Characterwise { anchor } => {
            HighlightState::Characterwise { anchor: *anchor }
        }
        SelectionState::Linewise { anchor_field } => HighlightState::Linewise {
            anchor_line: *anchor_field,
        },
    }
}

/// Core canvas field rendering with options
#[cfg(feature = "gui")]
fn render_canvas_fields_with_options<T: CanvasTheme, F1, F2>(
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
    active_completion: Option<String>,
    opts: CanvasDisplayOptions,
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

    let input_area = input_container.inner(input_block);

    let input_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); fields.len()])
        .split(input_area);

    render_field_labels(f, columns[0], input_block, fields, theme);

    let mut active_field_input_rect = None;

    for i in 0..inputs.len() {
        let is_active = i == *current_field_idx;
        let typed_text = get_display_value(i);
        let inner_width = input_rows[i].width;

        let mut h_scroll_for_cursor: u16 = 0;
        let mut left_offset_for_cursor: u16 = 0;

        let line = match highlight_state {
            HighlightState::Characterwise { .. } | HighlightState::Linewise { .. } => {
                apply_highlighting(
                    &typed_text,
                    i,
                    current_field_idx,
                    current_cursor_pos,
                    highlight_state,
                    theme,
                    is_active,
                )
            }

            HighlightState::Off => match opts.overflow {
                OverflowMode::Indicator(ind) => {
                    if is_active {
                        let (l, hs, left_cols) = render_active_line_with_indicator(
                            &typed_text,
                            active_completion.as_deref(),
                            inner_width,
                            ind,
                            current_cursor_pos,
                            theme,
                        );
                        h_scroll_for_cursor = hs;
                        left_offset_for_cursor = left_cols;
                        l
                    } else if display_width(&typed_text) <= inner_width {
                        Line::from(Span::raw(typed_text.clone()))
                    } else {
                        clip_with_indicator_line(&typed_text, inner_width, ind)
                    }
                }

                OverflowMode::Wrap => {
                    if is_active {
                        let mut spans: Vec<Span> = Vec::new();
                        spans.push(Span::styled(
                            typed_text.clone(),
                            Style::default().fg(theme.fg()),
                        ));
                        if let Some(completion) = &active_completion {
                            if !completion.is_empty() {
                                spans.push(Span::styled(
                                    completion.clone(),
                                    Style::default().fg(theme.suggestion_gray()),
                                ));
                            }
                        }
                        Line::from(spans)
                    } else {
                        Line::from(Span::raw(typed_text.clone()))
                    }
                }
            },
        };

        let mut p = Paragraph::new(line).alignment(Alignment::Left);

        if matches!(opts.overflow, OverflowMode::Wrap) {
            p = p.wrap(Wrap { trim: false });
        }

        f.render_widget(p, input_rows[i]);

        if is_active {
            active_field_input_rect = Some(input_rows[i]);
            set_cursor_position_scrolled(
                f,
                input_rows[i],
                &typed_text,
                current_cursor_pos,
                has_display_override(i),
                h_scroll_for_cursor,
                left_offset_for_cursor,
            );
        }
    }

    active_field_input_rect
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
            format!("{field}:"),
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
        HighlightState::Off => Line::from(Span::styled(text, Style::default().fg(theme.fg()))),
        HighlightState::Characterwise { anchor } => apply_characterwise_highlighting(
            text,
            text_len,
            field_index,
            current_field_idx,
            current_cursor_pos,
            anchor,
            theme,
            is_active,
        ),
        HighlightState::Linewise { anchor_line } => apply_linewise_highlighting(
            text,
            field_index,
            current_field_idx,
            anchor_line,
            theme,
            is_active,
        ),
    }
}

/// Apply characterwise highlighting.
#[cfg(feature = "gui")]
fn apply_characterwise_highlighting<'a, T: CanvasTheme>(
    text: &'a str,
    text_len: usize,
    field_index: usize,
    current_field_idx: &usize,
    current_cursor_pos: usize,
    anchor: &(usize, usize),
    theme: &T,
    _is_active: bool,
) -> Line<'a> {
    let (anchor_field, anchor_char) = *anchor;
    let start_field = min(anchor_field, *current_field_idx);
    let end_field = max(anchor_field, *current_field_idx);

    let highlight_style = Style::default()
        .fg(theme.highlight())
        .bg(theme.highlight_bg())
        .add_modifier(Modifier::BOLD);

    let normal_style = Style::default().fg(theme.fg());

    if field_index >= start_field && field_index <= end_field {
        if start_field == end_field {
            let (start_char, end_char) = if anchor_field == *current_field_idx {
                (
                    min(anchor_char, current_cursor_pos),
                    max(anchor_char, current_cursor_pos),
                )
            } else if anchor_field < *current_field_idx {
                (anchor_char, current_cursor_pos)
            } else {
                (current_cursor_pos, anchor_char)
            };

            let clamped_start = start_char.min(text_len);
            let clamped_end = end_char.min(text_len);

            let before: String = text.chars().take(clamped_start).collect();
            let highlighted: String = text
                .chars()
                .skip(clamped_start)
                .take(clamped_end.saturating_sub(clamped_start) + 1)
                .collect();
            let after: String = text.chars().skip(clamped_end + 1).collect();

            Line::from(vec![
                Span::styled(before, normal_style),
                Span::styled(highlighted, highlight_style),
                Span::styled(after, normal_style),
            ])
        } else if field_index == anchor_field {
            if anchor_field < *current_field_idx {
                let clamped_start = anchor_char.min(text_len);
                let before: String = text.chars().take(clamped_start).collect();
                let highlighted: String = text.chars().skip(clamped_start).collect();

                Line::from(vec![
                    Span::styled(before, normal_style),
                    Span::styled(highlighted, highlight_style),
                ])
            } else {
                let clamped_end = anchor_char.min(text_len);
                let highlighted: String = text.chars().take(clamped_end + 1).collect();
                let after: String = text.chars().skip(clamped_end + 1).collect();

                Line::from(vec![
                    Span::styled(highlighted, highlight_style),
                    Span::styled(after, normal_style),
                ])
            }
        } else if field_index == *current_field_idx {
            if anchor_field < *current_field_idx {
                let clamped_end = current_cursor_pos.min(text_len);
                let highlighted: String = text.chars().take(clamped_end + 1).collect();
                let after: String = text.chars().skip(clamped_end + 1).collect();

                Line::from(vec![
                    Span::styled(highlighted, highlight_style),
                    Span::styled(after, normal_style),
                ])
            } else {
                let clamped_start = current_cursor_pos.min(text_len);
                let before: String = text.chars().take(clamped_start).collect();
                let highlighted: String = text.chars().skip(clamped_start).collect();

                Line::from(vec![
                    Span::styled(before, normal_style),
                    Span::styled(highlighted, highlight_style),
                ])
            }
        } else {
            Line::from(Span::styled(text, highlight_style))
        }
    } else {
        Line::from(Span::styled(text, normal_style))
    }
}

/// Apply linewise highlighting.
#[cfg(feature = "gui")]
fn apply_linewise_highlighting<'a, T: CanvasTheme>(
    text: &'a str,
    field_index: usize,
    current_field_idx: &usize,
    anchor_line: &usize,
    theme: &T,
    _is_active: bool,
) -> Line<'a> {
    let start_field = min(*anchor_line, *current_field_idx);
    let end_field = max(*anchor_line, *current_field_idx);

    let highlight_style = Style::default()
        .fg(theme.highlight())
        .bg(theme.highlight_bg())
        .add_modifier(Modifier::BOLD);

    let normal_style = Style::default().fg(theme.fg());

    if field_index >= start_field && field_index <= end_field {
        Line::from(Span::styled(text, highlight_style))
    } else {
        Line::from(Span::styled(text, normal_style))
    }
}

/// Set cursor position (x clamp only; no Y offset with wrap in this version)
#[cfg(feature = "gui")]
fn set_cursor_position_scrolled(
    f: &mut Frame,
    field_rect: Rect,
    text: &str,
    current_cursor_pos: usize,
    _has_display_override: bool,
    h_scroll: u16,
    left_offset: u16,
) {
    let mut cols: u16 = 0;
    for (i, ch) in text.chars().enumerate() {
        if i >= current_cursor_pos {
            break;
        }
        cols = cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
    }

    let mut visible_x = cols.saturating_sub(h_scroll).saturating_add(left_offset);

    let limit = field_rect.width.saturating_sub(1 + RIGHT_PAD);
    if visible_x > limit {
        visible_x = limit;
    }

    let cursor_x = field_rect.x.saturating_add(visible_x);
    let cursor_y = field_rect.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

#[cfg(feature = "gui")]
pub fn render_canvas_default<D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &FormEditor<D>,
) -> Option<Rect> {
    let theme = DefaultCanvasTheme;
    render_canvas(f, area, editor, &theme)
}
