// src/textarea/widget.rs
#[cfg(feature = "gui")]
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, StatefulWidget, Widget, Wrap,
    },
};

#[cfg(feature = "gui")]
use crate::data_provider::DataProvider;

#[cfg(feature = "gui")]
use crate::textarea::state::{
    compute_h_scroll_with_padding,
    count_wrapped_rows_indented,
    TextAreaState,
    TextOverflowMode,
};

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

#[cfg(feature = "gui")]
#[derive(Debug, Clone)]
pub struct TextArea<'a> {
    pub(crate) block: Option<Block<'a>>,
    pub(crate) style: Style,
    pub(crate) border_type: BorderType,
}

#[cfg(feature = "gui")]
impl<'a> Default for TextArea<'a> {
    fn default() -> Self {
        Self {
            block: Some(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            style: Style::default(),
            border_type: BorderType::Rounded,
        }
    }
}

#[cfg(feature = "gui")]
impl<'a> TextArea<'a> {
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
fn display_width(s: &str) -> u16 {
    s.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0) as u16)
        .sum()
}

#[cfg(feature = "gui")]
fn display_cols_up_to(s: &str, char_count: usize) -> u16 {
    let mut cols: u16 = 0;
    for (i, ch) in s.chars().enumerate() {
        if i >= char_count {
            break;
        }
        cols = cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
    }
    cols
}

#[cfg(feature = "gui")]
fn clip_with_indicator(s: &str, width: u16, indicator: char) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }

    if display_width(s) <= width {
        return Line::from(Span::raw(s.to_string()));
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

// anchor: near other helpers
#[cfg(feature = "gui")]
fn slice_by_display_cols(s: &str, start_cols: u16, max_cols: u16) -> String {
    if max_cols == 0 {
        return String::new();
    }

    let mut current_cols: u16 = 0;
    let mut output = String::new();
    let mut taken: u16 = 0;
    let mut started = false;

    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;

        if !started {
            if current_cols.saturating_add(w) <= start_cols {
                current_cols = current_cols.saturating_add(w);
                continue;
            } else {
                started = true;
            }
        }

        if taken.saturating_add(w) > max_cols {
            break;
        }

        output.push(ch);
        taken = taken.saturating_add(w);
        current_cols = current_cols.saturating_add(w);
    }

    output
}

#[cfg(feature = "gui")]
fn clip_window_with_indicator_padded(
    text: &str,
    view_width: u16,
    indicator: char,
    start_cols: u16,
) -> Line<'static> {
    if view_width == 0 {
        return Line::from("");
    }

    let total = display_width(text);

    // Left indicator if we scrolled
    let show_left = start_cols > 0;
    let left_cols: u16 = if show_left { 1 } else { 0 };

    // Capacity for text if we also need a right indicator
    let cap_with_right = view_width.saturating_sub(left_cols + 1);

    // Do we still have content beyond this window?
    let remaining = total.saturating_sub(start_cols);
    let show_right = remaining > cap_with_right;

    // Final capacity for visible text
    let max_visible = if show_right {
        cap_with_right
    } else {
        view_width.saturating_sub(left_cols)
    };

    let visible = slice_by_display_cols(text, start_cols, max_visible);

    let mut spans: Vec<Span> = Vec::new();
    if show_left {
        spans.push(Span::raw(indicator.to_string()));
    }

    // Visible text
    spans.push(Span::raw(visible.clone()));

    // Place $ flush-right
    if show_right {
        let used_cols = left_cols + display_width(&visible);
        let right_pos = view_width.saturating_sub(1);
        let filler = right_pos.saturating_sub(used_cols);
        if filler > 0 {
            spans.push(Span::raw(" ".repeat(filler as usize)));
        }
        spans.push(Span::raw(indicator.to_string()));
    }

    Line::from(spans)
}

#[cfg(feature = "gui")]
fn wrap_segments_with_indent(
    s: &str,
    width: u16,
    indent: u16,
) -> Vec<String> {
    let mut segments: Vec<String> = Vec::new();
    if width == 0 {
        segments.push(String::new());
        return segments;
    }

    let indent = indent.min(width.saturating_sub(1));
    let cont_cap = width.saturating_sub(indent);
    let indent_str = " ".repeat(indent as usize);

    let mut buf = String::new();
    let mut used: u16 = 0;
    let mut first = true;

    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        let cap = if first { width } else { cont_cap };

        // Early-wrap: wrap before filling the last cell (and avoid empty segment)
        if used > 0 && used.saturating_add(w) >= cap {
            segments.push(buf);
            buf = String::new();
            used = 0;
            first = false;
            if indent > 0 {
                buf.push_str(&indent_str);
                used = indent;
            }
        }

        buf.push(ch);
        used = used.saturating_add(w);
    }

    segments.push(buf);
    segments
}

// Map visual row offset to (logical line, intra segment)
#[cfg(feature = "gui")]
fn resolve_start_line_and_intra_indented(
    state: &TextAreaState,
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
impl<'a> StatefulWidget for TextArea<'a> {
    type State = TextAreaState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.ensure_visible(area, self.block.as_ref());

        let inner = if let Some(b) = &self.block {
            b.clone().render(area, buf);
            b.inner(area)
        } else {
            area
        };

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
            // manual pre-wrap path (unchanged)
            let mut rows_left = inner.height;
            let indent = state.wrap_indent_cols;
            let mut i = start;
            while i < total && rows_left > 0 {
                let s = provider.field_value(i);
                let segments = wrap_segments_with_indent(s, inner.width, indent);
                let skip = if i == start { intra as usize } else { 0 };
                for seg in segments.into_iter().skip(skip) {
                    display_lines.push(Line::from(Span::raw(seg)));
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
                        // Same-frame h-scroll so text shifts immediately
                        let start_cols = if i == state.current_field() {
                            let col_idx = state.display_cursor_position();
                            let cursor_cols = display_cols_up_to(s, col_idx);
                            let (target_h, _left_cols) =
                                compute_h_scroll_with_padding(cursor_cols, inner.width);
                            target_h.max(state.h_scroll)
                        } else {
                            0
                        };

                        display_lines.push(clip_window_with_indicator_padded(
                                s,
                                inner.width, // full view width
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
