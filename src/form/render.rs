// src/form/render.rs
//! Canvas GUI rendering helpers.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

use crate::canvas::modes::HighlightState;
use crate::canvas::theme::{CanvasTheme, DefaultCanvasTheme};
use crate::data_provider::DataProvider;
use crate::editor::EditorCore;
use crate::gui_utils::{
    clip_inline_completion_with_indicator_padded, clip_line_with_indicator_padded,
    compute_h_scroll_with_padding, display_width, effective_right_pad,
};
use unicode_width::UnicodeWidthChar;

use std::cmp::{max, min};

#[derive(Debug, Clone, Copy)]
/// How to handle overflow when rendering a field's content.
pub enum OverflowMode {
    /// Show an indicator character at the left/right when text is truncated.
    /// Common default is '$'.
    Indicator(char),
    /// Wrap content into multiple visual lines.
    Wrap,
}

#[derive(Debug, Clone, Copy)]
/// Display options controlling canvas rendering behavior.
///
/// Override visual form widths with struct update syntax:
///
/// ```rust
/// # use canvas::{CanvasDisplayOptions, OverflowMode};
/// let opts = CanvasDisplayOptions {
///     max_label_width: 18,
///     max_input_width: Some(40),
///     ..Default::default()
/// };
/// ```
///
/// Override individual row widths with `row_input_width`:
///
/// ```rust
/// # use canvas::CanvasDisplayOptions;
/// let opts = CanvasDisplayOptions {
///     row_input_width: Some(|row, available| match row {
///         0 => 12.min(available),
///         1 => 40.min(available),
///         _ => available,
///     }),
///     ..Default::default()
/// };
/// ```
pub struct CanvasDisplayOptions {
    /// How to handle horizontal overflow for fields.
    pub overflow: OverflowMode,
    /// Maximum label column width, including the trailing space before the input.
    ///
    /// Change this by setting `CanvasDisplayOptions::max_label_width`.
    /// Labels wider than this are visually truncated with `...`.
    pub max_label_width: u16,
    /// Maximum visual width of the input column.
    ///
    /// Change this by setting `CanvasDisplayOptions::max_input_width`.
    /// `Some(width)` caps the rendered input width; `None` uses all remaining space.
    pub max_input_width: Option<u16>,
    /// Optional per-row input width override.
    ///
    /// Change this by setting `CanvasDisplayOptions::row_input_width`.
    /// The callback receives `(row_index, available_width)` and returns the visual
    /// input width for that row.
    pub row_input_width: Option<fn(usize, u16) -> u16>,
}

impl Default for CanvasDisplayOptions {
    fn default() -> Self {
        Self {
            overflow: OverflowMode::Indicator('$'),
            max_label_width: 24,
            max_input_width: Some(25),
            row_input_width: None,
        }
    }
}

impl CanvasDisplayOptions {
    fn input_width_for_row(self, row_index: usize, available_width: u16) -> u16 {
        let width = if let Some(row_input_width) = self.row_input_width {
            row_input_width(row_index, available_width)
        } else {
            self.max_input_width
                .map(|max_width| max_width.min(available_width))
                .unwrap_or(available_width)
        };

        width.min(available_width)
    }
}

/// Utility: clip a string to fit width, append indicator if overflow.
/// All spans inherit `style` so that inactive plain-text fields remain themed.
fn clip_with_indicator_line(s: &str, width: u16, indicator: char, style: Style) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }
    if display_width(s) <= width {
        return Line::from(Span::styled(s, style));
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
    Line::from(vec![
        Span::styled(out, style),
        Span::styled(indicator.to_string(), style),
    ])
}

fn clip_label_with_ellipsis(s: &str, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    if display_width(s) <= width {
        return s.to_string();
    }
    if width <= 3 {
        return ".".repeat(width as usize);
    }

    let budget = width - 3;
    let mut out = String::new();
    let mut used: u16 = 0;

    for ch in s.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if used + ch_width > budget {
            break;
        }
        out.push(ch);
        used = used.saturating_add(ch_width);
    }

    out.push_str("...");
    out
}

fn form_label_width(fields: &[&str], max_label_width: u16, area_width: u16) -> u16 {
    if area_width == 0 {
        return 0;
    }

    let longest = fields
        .iter()
        .map(|field| display_width(field))
        .max()
        .unwrap_or(0)
        .saturating_add(1);

    longest.min(max_label_width).min(area_width)
}

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

    let total_cols = display_width(typed_text);
    let (h_scroll, left_cols) = compute_h_scroll_with_padding(cursor_cols, total_cols, width);

    (
        clip_inline_completion_with_indicator_padded(
            typed_text,
            completion,
            width,
            indicator,
            h_scroll,
            theme.input_active(),
            theme.completion(),
        ),
        h_scroll,
        left_cols,
    )
}

/// Render the canvas into the provided frame using default display options.
/// Returns the rectangle of the active input field if present.
pub fn render_canvas<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &EditorCore<D>,
    theme: &T,
) -> Option<Rect> {
    let opts = CanvasDisplayOptions::default();
    render_canvas_with_options(f, area, editor, theme, opts)
}

/// Render the canvas into the provided frame with explicit display options.
///
/// This is the more configurable entrypoint for rendering and is useful for
/// tests or when callers need to override overflow handling.
pub fn render_canvas_with_options<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &EditorCore<D>,
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

fn render_canvas_with_highlight_and_options<T: CanvasTheme, D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &EditorCore<D>,
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

    render_canvas_fields_with_options(
        f,
        area,
        &fields,
        &current_field_idx,
        &inputs,
        theme,
        highlight_state,
        editor.display_cursor_position(),
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
fn render_canvas_fields_with_options<T: CanvasTheme, F1, F2>(
    f: &mut Frame,
    area: Rect,
    fields: &[&str],
    current_field_idx: &usize,
    inputs: &[String],
    theme: &T,
    highlight_state: &HighlightState,
    current_cursor_pos: usize,
    get_display_value: F1,
    has_display_override: F2,
    active_completion: Option<String>,
    opts: CanvasDisplayOptions,
) -> Option<Rect>
where
    F1: Fn(usize) -> String,
    F2: Fn(usize) -> bool,
{
    // Fill the canvas area with the theme's background so the surface
    // never inherits whatever the parent frame had underneath.
    let bg_block = Block::default().style(theme.background());
    f.render_widget(bg_block, area);

    let label_width = form_label_width(fields, opts.max_label_width, area.width);
    let available_input_width = area.width.saturating_sub(label_width);

    render_field_labels(f, area, label_width, fields, theme);

    let mut active_field_input_rect = None;

    for i in 0..inputs.len() {
        let is_active = i == *current_field_idx;
        let typed_text = get_display_value(i);
        let input_row = Rect {
            x: area.x + label_width,
            y: area.y + i as u16,
            width: opts.input_width_for_row(i, available_input_width),
            height: 1,
        };
        let inner_width = input_row.width;

        let mut h_scroll_for_cursor: u16 = 0;
        let mut left_offset_for_cursor: u16 = 0;

        let line = match highlight_state {
            HighlightState::Characterwise { .. } | HighlightState::Linewise { .. } => {
                let highlighted = apply_highlighting(
                    &typed_text,
                    i,
                    current_field_idx,
                    current_cursor_pos,
                    highlight_state,
                    theme,
                    is_active,
                );

                if is_active {
                    // Compute scroll values for cursor positioning, exactly as
                    // render_active_line_with_indicator does for the no-selection path.
                    let mut cursor_cols: u16 = 0;
                    for (j, ch) in typed_text.chars().enumerate() {
                        if j >= current_cursor_pos {
                            break;
                        }
                        cursor_cols = cursor_cols
                            .saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                    }
                    let total_cols = display_width(&typed_text);
                    let (hs, lc) =
                        compute_h_scroll_with_padding(cursor_cols, total_cols, inner_width);
                    h_scroll_for_cursor = hs;
                    left_offset_for_cursor = lc;
                }

                // Clip the highlighted line when text overflows, preserving
                // span styles through horizontal scrolling.
                match opts.overflow {
                    OverflowMode::Indicator(ind) => {
                        if inner_width > 0 && display_width(&typed_text) > inner_width {
                            clip_line_with_indicator_padded(
                                highlighted,
                                inner_width,
                                ind,
                                h_scroll_for_cursor,
                            )
                        } else {
                            highlighted
                        }
                    }
                    OverflowMode::Wrap => highlighted,
                }
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
                        Line::from(Span::styled(typed_text.clone(), theme.input()))
                    } else {
                        clip_with_indicator_line(
                            &typed_text,
                            inner_width,
                            ind,
                            theme.input(),
                        )
                    }
                }

                OverflowMode::Wrap => {
                    if is_active {
                        let mut spans: Vec<Span> = Vec::new();
                        spans.push(Span::styled(
                            typed_text.clone(),
                            theme.input_active(),
                        ));
                        if let Some(completion) = &active_completion {
                            if !completion.is_empty() {
                                spans.push(Span::styled(
                                    completion.clone(),
                                    theme.completion(),
                                ));
                            }
                        }
                        Line::from(spans)
                    } else {
                        Line::from(Span::styled(typed_text.clone(), theme.input()))
                    }
                }
            },
        };

        let mut p = Paragraph::new(line).alignment(Alignment::Left);

        if matches!(opts.overflow, OverflowMode::Wrap) {
            p = p.wrap(Wrap { trim: false });
        }

        f.render_widget(p, input_row);

        if is_active {
            active_field_input_rect = Some(input_row);
            set_cursor_position_scrolled(
                f,
                input_row,
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
fn render_field_labels<T: CanvasTheme>(
    f: &mut Frame,
    area: Rect,
    label_width: u16,
    fields: &[&str],
    theme: &T,
) {
    if label_width == 0 {
        return;
    }

    let label_text_width = label_width.saturating_sub(1);

    for (i, field) in fields.iter().enumerate() {
        let clipped_label = clip_label_with_ellipsis(field, label_text_width);
        let label = Paragraph::new(Line::from(Span::styled(
            clipped_label,
            theme.label(),
        )));
        f.render_widget(
            label,
            Rect {
                x: area.x,
                y: area.y + i as u16,
                width: label_text_width,
                height: 1,
            },
        );
    }
}

/// Apply highlighting based on highlight state
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
        HighlightState::Off => Line::from(Span::styled(text, theme.input())),
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

    let highlight_style = theme.selection();

    let normal_style = theme.input();

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

    let highlight_style = theme.selection();

    let normal_style = theme.input();

    if field_index >= start_field && field_index <= end_field {
        Line::from(Span::styled(text, highlight_style))
    } else {
        Line::from(Span::styled(text, normal_style))
    }
}

/// Set cursor position (x clamp only; no Y offset with wrap in this version)
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

    let total_cols = display_width(text);
    let limit = field_rect
        .width
        .saturating_sub(1 + effective_right_pad(cols, total_cols));
    if visible_x > limit {
        visible_x = limit;
    }

    let cursor_x = field_rect.x.saturating_add(visible_x);
    let cursor_y = field_rect.y;
    f.set_cursor_position((cursor_x, cursor_y));
}

pub fn render_canvas_default<D: DataProvider>(
    f: &mut Frame,
    area: Rect,
    editor: &EditorCore<D>,
) -> Option<Rect> {
    let theme = DefaultCanvasTheme;
    render_canvas(f, area, editor, &theme)
}
