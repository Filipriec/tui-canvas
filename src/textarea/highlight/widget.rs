// src/textarea/highlight/widget.rs
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};

use super::chunks::{clip_chunks_window_with_indicator_padded, wrap_chunks_indented};
use super::state::TextAreaSyntaxState;

use crate::data_provider::DataProvider;
use crate::gui_utils::{compute_h_scroll_with_padding, display_cols_up_to, display_width};
use crate::textarea::state::{count_wrapped_rows_indented, TextOverflowMode};

#[derive(Debug, Clone)]
pub struct TextAreaSyntax<'a> {
    pub block: Option<Block<'a>>,
    pub style: Style,
    pub border_type: BorderType,
}

impl<'a> Default for TextAreaSyntax<'a> {
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

impl<'a> TextAreaSyntax<'a> {
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

fn resolve_start_line_and_intra_indented(state: &TextAreaSyntaxState, inner: Rect) -> (usize, u16) {
    let provider = state.textarea.editor.data_provider();
    let total = provider.line_count();

    if total == 0 {
        return (0, 0);
    }

    let wrap = matches!(state.textarea.overflow_mode, TextOverflowMode::Wrap);
    let width = inner.width;
    let target_vis = state.textarea.scroll_y;

    if !wrap {
        let start = (target_vis as usize).min(total);
        return (start, 0);
    }

    let indent = state.textarea.wrap_indent_cols;

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

fn prepend_line_prefix(mut line: Line<'static>, prefix: String) -> Line<'static> {
    if !prefix.is_empty() {
        line.spans.insert(0, Span::raw(prefix));
    }
    line
}

impl<'a> StatefulWidget for TextAreaSyntax<'a> {
    type State = TextAreaSyntaxState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Reuse existing scroll logic
        state.textarea.ensure_visible(area, self.block.as_ref());

        let inner = if let Some(b) = &self.block {
            b.clone().render(area, buf);
            b.inner(area)
        } else {
            area
        };
        let content = state.textarea.content_area(inner);

        let edited_now = state.textarea.take_edited_flag();

        let wrap_mode = matches!(state.textarea.overflow_mode, TextOverflowMode::Wrap);
        let provider = state.textarea.editor.data_provider();
        let total = provider.line_count();

        let (start, intra) = resolve_start_line_and_intra_indented(state, content);

        let mut display_lines: Vec<Line> = Vec::new();

        if total == 0 || start >= total {
            if let Some(ph) = &state.textarea.placeholder {
                let mut spans = Vec::new();
                let prefix = state.textarea.line_number_prefix(0, true);
                if !prefix.is_empty() {
                    spans.push(Span::raw(prefix));
                }
                spans.push(Span::raw(ph.clone()));
                display_lines.push(Line::from(spans));
            }
        } else if wrap_mode {
            let mut rows_left = content.height;
            let indent = state.textarea.wrap_indent_cols;

            let mut i = start;
            while i < total && rows_left > 0 {
                let s = provider.field_value(i);

                let chunks = state.engine.highlight_line_cached(i, s, provider);

                let lines = wrap_chunks_indented(&chunks, content.width, indent);
                let skip = if i == start { intra as usize } else { 0 };
                for (line_idx, line) in lines.into_iter().enumerate().skip(skip) {
                    let prefix = state.textarea.line_number_prefix(i, line_idx == 0);
                    display_lines.push(prepend_line_prefix(line, prefix));
                    rows_left = rows_left.saturating_sub(1);
                    if rows_left == 0 {
                        break;
                    }
                }
                i += 1;
            }
        } else {
            let end = (start.saturating_add(content.height as usize)).min(total);

            for i in start..end {
                let s = provider.field_value(i);

                let chunks = state.engine.highlight_line_cached(i, s, provider);

                let fits = display_width(s) <= content.width;
                let start_cols = if i == state.textarea.current_field() {
                    let col_idx = state.textarea.display_cursor_position();
                    let cursor_cols = display_cols_up_to(s, col_idx);
                    let (target_h, _left_cols) =
                        compute_h_scroll_with_padding(cursor_cols, content.width);

                    if fits {
                        if edited_now {
                            target_h
                        } else {
                            0
                        }
                    } else {
                        target_h.max(state.textarea.h_scroll)
                    }
                } else {
                    0
                };

                if let TextOverflowMode::Indicator { ch } = state.textarea.overflow_mode {
                    let prefix = state.textarea.line_number_prefix(i, true);
                    display_lines.push(prepend_line_prefix(
                        clip_chunks_window_with_indicator_padded(
                            &chunks,
                            content.width,
                            ch,
                            start_cols,
                        ),
                        prefix,
                    ));
                }
            }
        }

        let p = Paragraph::new(display_lines)
            .alignment(Alignment::Left)
            .style(self.style);

        p.render(inner, buf);
    }
}
