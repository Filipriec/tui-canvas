// src/textarea/widget.rs
#[cfg(feature = "gui")]
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};

#[cfg(feature = "gui")]
use crate::gui_utils::{
    clip_window_with_indicator_padded, compute_h_scroll_with_padding, display_cols_up_to,
    display_width,
};
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
fn wrap_segments_with_indent(s: &str, width: u16, indent: u16) -> Vec<String> {
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

                        display_lines.push(clip_window_with_indicator_padded(
                            s,
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
