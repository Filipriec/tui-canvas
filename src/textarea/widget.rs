// src/textarea/widget.rs
#[cfg(feature = "gui")]
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};

#[cfg(feature = "gui")]
use crate::gui_utils::{
    compute_h_scroll_with_padding, display_cols_up_to, display_width,
};
#[cfg(feature = "gui")]
use crate::canvas::state::SelectionState;
#[cfg(feature = "gui")]
use crate::textarea::provider::{TextAreaDataProvider, TextAreaProvider};

#[cfg(feature = "gui")]
use crate::textarea::state::{count_wrapped_rows_indented, TextAreaState, TextOverflowMode};

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

#[cfg(feature = "gui")]
#[derive(Debug, Clone)]
pub struct TextArea<'a, P: TextAreaDataProvider = TextAreaProvider> {
    pub(crate) block: Option<Block<'a>>,
    pub(crate) style: Style,
    pub(crate) border_type: BorderType,
    pub(crate) _provider: std::marker::PhantomData<P>,
}

#[cfg(feature = "gui")]
impl<'a, P: TextAreaDataProvider> Default for TextArea<'a, P> {
    fn default() -> Self {
        Self {
            block: Some(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            style: Style::default(),
            border_type: BorderType::Rounded,
            _provider: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "gui")]
impl<'a, P: TextAreaDataProvider> TextArea<'a, P> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn border_type(mut self, ty: BorderType) -> Self {
        self.border_type = ty;
        if let Some(b) = &mut self.block {
            *b = b.clone().border_type(ty);
        }
        self
    }
}

#[cfg(feature = "gui")]
fn selection_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .bg(Color::Blue)
        .add_modifier(Modifier::BOLD)
}

#[cfg(feature = "gui")]
fn char_selection_range<P: TextAreaDataProvider>(
    state: &TextAreaState<P>,
    line_idx: usize,
    text_len: usize,
) -> Option<(usize, usize)> {
    let SelectionState::Characterwise { anchor } = *state.selection_state() else {
        return None;
    };

    let cursor = (state.current_field(), state.cursor_position());
    let start = anchor.min(cursor);
    let end = anchor.max(cursor);

    if line_idx < start.0 || line_idx > end.0 {
        return None;
    }

    if start.0 == end.0 {
        return Some((start.1.min(text_len), end.1.min(text_len)));
    }

    if line_idx == start.0 {
        Some((start.1.min(text_len), text_len.saturating_sub(1)))
    } else if line_idx == end.0 {
        Some((0, end.1.min(text_len)))
    } else {
        Some((0, text_len.saturating_sub(1)))
    }
}

#[cfg(feature = "gui")]
fn line_is_linewise_selected<P: TextAreaDataProvider>(
    state: &TextAreaState<P>,
    line_idx: usize,
) -> bool {
    let SelectionState::Linewise { anchor_field } = *state.selection_state() else {
        return false;
    };

    let start = anchor_field.min(state.current_field());
    let end = anchor_field.max(state.current_field());
    line_idx >= start && line_idx <= end
}

#[cfg(feature = "gui")]
fn styled_segment_line<'a, P: TextAreaDataProvider>(
    visible: String,
    line_idx: usize,
    original_char_offset: usize,
    state: &TextAreaState<P>,
    prefix: Option<String>,
    suffix: Option<String>,
) -> Line<'a> {
    let normal = Style::default();
    let selected = selection_style();
    let mut spans = Vec::new();

    if let Some(prefix) = prefix {
        spans.push(Span::styled(prefix, normal));
    }

    if line_is_linewise_selected(state, line_idx) {
        let selected_text = if visible.is_empty() {
            " ".to_string()
        } else {
            visible
        };
        spans.push(Span::styled(selected_text, selected));
    } else if let Some((start, end)) =
        char_selection_range(
            state,
            line_idx,
            state
                .editor
                .data_provider()
                .field_value(line_idx)
                .chars()
                .count(),
        )
    {
        let mut before = String::new();
        let mut highlighted = String::new();
        let mut after = String::new();

        for (i, ch) in visible.chars().enumerate() {
            let original_idx = original_char_offset + i;
            if original_idx >= start && original_idx <= end {
                highlighted.push(ch);
            } else if original_idx < start {
                before.push(ch);
            } else {
                after.push(ch);
            }
        }

        if !before.is_empty() {
            spans.push(Span::styled(before, normal));
        }
        if highlighted.is_empty() && visible.is_empty() {
            spans.push(Span::styled(" ", selected));
        } else if !highlighted.is_empty() {
            spans.push(Span::styled(highlighted, selected));
        }
        if !after.is_empty() {
            spans.push(Span::styled(after, normal));
        }
    } else {
        spans.push(Span::styled(visible, normal));
    }

    if let Some(suffix) = suffix {
        spans.push(Span::styled(suffix, normal));
    }

    Line::from(spans)
}

#[cfg(feature = "gui")]
fn wrap_segments_with_offsets(s: &str, width: u16, indent: u16) -> Vec<(String, usize)> {
    let mut segments: Vec<(String, usize)> = Vec::new();
    if width == 0 {
        segments.push((String::new(), 0));
        return segments;
    }

    let indent = indent.min(width.saturating_sub(1));
    let cont_cap = width.saturating_sub(indent);
    let indent_str = " ".repeat(indent as usize);

    let mut buf = String::new();
    let mut used: u16 = 0;
    let mut first = true;
    let mut segment_start = 0;

    for (char_idx, ch) in s.chars().enumerate() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        let cap = if first { width } else { cont_cap };

        if used > 0 && used.saturating_add(w) >= cap {
            segments.push((buf, segment_start));
            buf = String::new();
            used = 0;
            first = false;
            segment_start = char_idx;
            if indent > 0 {
                buf.push_str(&indent_str);
                used = indent;
            }
        }

        buf.push(ch);
        used = used.saturating_add(w);
    }

    segments.push((buf, segment_start));
    segments
}

#[cfg(feature = "gui")]
fn slice_by_display_cols_with_offset(s: &str, start_cols: u16, max_cols: u16) -> (String, usize) {
    if max_cols == 0 {
        return (String::new(), 0);
    }

    let mut cols: u16 = 0;
    let mut out = String::new();
    let mut taken: u16 = 0;
    let mut started = false;
    let mut start_char = 0;

    for (char_idx, ch) in s.chars().enumerate() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        let next = cols.saturating_add(w);

        if !started {
            if next <= start_cols {
                cols = next;
                continue;
            }
            started = true;
            start_char = char_idx;
        }

        if taken.saturating_add(w) > max_cols {
            break;
        }

        out.push(ch);
        taken = taken.saturating_add(w);
        cols = next;
    }

    (out, start_char)
}

#[cfg(feature = "gui")]
fn clipped_line_with_selection<'a, P: TextAreaDataProvider>(
    text: &str,
    line_idx: usize,
    state: &TextAreaState<P>,
    view_width: u16,
    indicator: char,
    start_cols: u16,
) -> Line<'a> {
    if view_width == 0 {
        return Line::from("");
    }

    let total = display_width(text);
    let show_left = start_cols > 0;
    let left_cols: u16 = if show_left { 1 } else { 0 };
    let cap_with_right = view_width.saturating_sub(left_cols + 1);
    let remaining = total.saturating_sub(start_cols);
    let show_right = remaining > cap_with_right;
    let max_visible = if show_right {
        cap_with_right
    } else {
        view_width.saturating_sub(left_cols)
    };

    let (visible, char_offset) = slice_by_display_cols_with_offset(text, start_cols, max_visible);

    let suffix = if show_right {
        let used_cols = left_cols + display_width(&visible);
        let right_pos = view_width.saturating_sub(1);
        let filler = right_pos.saturating_sub(used_cols);
        let mut suffix = String::new();
        if filler > 0 {
            suffix.push_str(&" ".repeat(filler as usize));
        }
        suffix.push(indicator);
        Some(suffix)
    } else {
        None
    };

    styled_segment_line(
        visible,
        line_idx,
        char_offset,
        state,
        show_left.then(|| indicator.to_string()),
        suffix,
    )
}

// Map visual row offset to (logical line, intra segment)
#[cfg(feature = "gui")]
fn resolve_start_line_and_intra_indented(
    state: &TextAreaState<impl TextAreaDataProvider>,
    inner: Rect,
) -> (usize, u16) {
    let provider = state.editor.data_provider();
    let total = provider.line_count();

    if total == 0 {
        return (0, 0);
    }

    let wrap = matches!(state.overflow_mode, TextOverflowMode::Wrap);
    let width = inner.width;
    let target_vis = state.scroll_y;

    if !wrap {
        let start = (target_vis as usize).min(total);
        return (start, 0);
    }

    let indent = state.wrap_indent_cols;

    let mut acc: u16 = 0;
    for i in 0..total {
        let s = provider.field_value(i);
        let rows = count_wrapped_rows_indented(s, width, indent);
        if acc.saturating_add(rows) > target_vis {
            let intra = target_vis.saturating_sub(acc);
            return (i, intra);
        }
        acc = acc.saturating_add(rows);
    }

    (total.saturating_sub(1), 0)
}

#[cfg(feature = "gui")]
impl<'a, P: TextAreaDataProvider> StatefulWidget for TextArea<'a, P> {
    type State = TextAreaState<P>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.ensure_visible(area, self.block.as_ref());

        let inner = if let Some(b) = &self.block {
            b.clone().render(area, buf);
            b.inner(area)
        } else {
            area
        };

        let edited_now = state.take_edited_flag();

        let wrap_mode = matches!(state.overflow_mode, TextOverflowMode::Wrap);
        let provider = state.editor.data_provider();
        let total = provider.line_count();

        let (start, intra) = resolve_start_line_and_intra_indented(state, inner);

        let mut display_lines: Vec<Line> = Vec::new();

        if total == 0 || start >= total {
            if let Some(ph) = &state.placeholder {
                display_lines.push(Line::from(Span::raw(ph.clone())));
            }
        } else if wrap_mode {
            let mut rows_left = inner.height;
            let indent = state.wrap_indent_cols;
            let mut i = start;
            while i < total && rows_left > 0 {
                let s = provider.field_value(i);
                let segments = wrap_segments_with_offsets(s, inner.width, indent);
                let skip = if i == start { intra as usize } else { 0 };
                for (seg, offset) in segments.into_iter().skip(skip) {
                    display_lines.push(styled_segment_line(seg, i, offset, state, None, None));
                    rows_left = rows_left.saturating_sub(1);
                    if rows_left == 0 {
                        break;
                    }
                }
                i += 1;
            }
        } else {
            // Indicator mode: full inner width; RIGHT_PAD only affects cursor clamp and h-scroll
            let end = (start.saturating_add(inner.height as usize)).min(total);

            for i in start..end {
                let s = provider.field_value(i);
                match state.overflow_mode {
                    TextOverflowMode::Wrap => unreachable!(),
                    TextOverflowMode::Indicator { ch } => {
                        let fits = display_width(s) <= inner.width;

                        let start_cols = if i == state.current_field() {
                            let col_idx = state.display_cursor_position();
                            let cursor_cols = display_cols_up_to(s, col_idx);
                            let (target_h, _left_cols) =
                                compute_h_scroll_with_padding(cursor_cols, inner.width);

                            if fits {
                                if edited_now {
                                    target_h
                                } else {
                                    0
                                }
                            } else {
                                target_h.max(state.h_scroll)
                            }
                        } else {
                            0
                        };

                        display_lines.push(clipped_line_with_selection(
                            s,
                            i,
                            state,
                            inner.width,
                            ch,
                            start_cols,
                        ));
                    }
                }
            }
        }

        let p = Paragraph::new(display_lines)
            .alignment(Alignment::Left)
            .style(self.style);

        // No Paragraph::wrap/scroll in wrap mode — we pre-wrap.
        p.render(inner, buf);
    }
}
