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
use crate::canvas::state::SelectionState;
#[cfg(all(feature = "gui", feature = "commandline"))]
use crate::commandline::CommandLine;
#[cfg(feature = "gui")]
use crate::gui_utils::{compute_h_scroll_with_padding, display_cols_up_to, display_width};
#[cfg(feature = "gui")]
use crate::textarea::provider::{TextAreaDataProvider, TextAreaProvider};

#[cfg(feature = "gui")]
use crate::textarea::state::{
    continuation_prefix, continuation_prefix_width, count_wrapped_rows_indented, TextAreaState,
    TextOverflowMode,
};

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
fn search_match_style() -> Style {
    Style::default().fg(Color::Black).bg(Color::Yellow)
}

#[cfg(feature = "gui")]
fn active_search_match_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::LightYellow)
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
    let search = search_match_style();
    let active_search = active_search_match_style();
    let mut spans = Vec::new();

    if let Some(prefix) = prefix {
        spans.push(Span::styled(prefix, normal));
    }

    let linewise_selected = line_is_linewise_selected(state, line_idx);
    if linewise_selected {
        let selected_text = if visible.is_empty() {
            " ".to_string()
        } else {
            visible
        };
        spans.push(Span::styled(selected_text, selected));
    } else {
        let char_selection = char_selection_range(
            state,
            line_idx,
            state
                .editor
                .data_provider()
                .field_value(line_idx)
                .chars()
                .count(),
        );
        let search_matches = state.search_matches_in_line(line_idx);
        let active_match = state.active_search_match();

        if visible.is_empty() {
            if char_selection == Some((0, 0)) {
                spans.push(Span::styled(" ", selected));
            } else {
                spans.push(Span::styled(visible, normal));
            }
        } else {
            let mut current_text = String::new();
            let mut current_style: Option<Style> = None;

            for (i, ch) in visible.chars().enumerate() {
                let original_idx = original_char_offset + i;
                let style = if char_selection
                    .map(|(start, end)| original_idx >= start && original_idx <= end)
                    .unwrap_or(false)
                {
                    selected
                } else if active_match
                    .map(|m| {
                        m.line == line_idx && original_idx >= m.start && original_idx < m.end
                    })
                    .unwrap_or(false)
                {
                    active_search
                } else if search_matches
                    .iter()
                    .any(|m| original_idx >= m.start && original_idx < m.end)
                {
                    search
                } else {
                    normal
                };

                if current_style == Some(style) {
                    current_text.push(ch);
                } else {
                    if !current_text.is_empty() {
                        spans.push(Span::styled(current_text, current_style.unwrap_or(normal)));
                    }
                    current_text = ch.to_string();
                    current_style = Some(style);
                }
            }

            if !current_text.is_empty() {
                spans.push(Span::styled(current_text, current_style.unwrap_or(normal)));
            }
        }
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
    let cont_prefix_width = continuation_prefix_width(width, indent);

    let mut buf = String::new();
    let mut used: u16 = 0;
    let mut segment_start = 0;

    for (char_idx, ch) in s.chars().enumerate() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;

        if used > 0 && used.saturating_add(w) > width {
            segments.push((buf, segment_start));
            buf = String::new();
            segment_start = char_idx;
            used = cont_prefix_width;
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
    prefix: Option<String>,
) -> Line<'a> {
    if view_width == 0 {
        return Line::from(prefix.unwrap_or_default());
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
        match (prefix, show_left) {
            (Some(mut prefix), true) => {
                prefix.push(indicator);
                Some(prefix)
            }
            (Some(prefix), false) => Some(prefix),
            (None, true) => Some(indicator.to_string()),
            (None, false) => None,
        },
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
        #[cfg(feature = "commandline")]
        let textarea_area = state.commandline_textarea_area(area);
        #[cfg(not(feature = "commandline"))]
        let textarea_area = area;

        state.ensure_visible(textarea_area, self.block.as_ref());

        let inner = if let Some(b) = &self.block {
            b.clone().render(textarea_area, buf);
            b.inner(textarea_area)
        } else {
            textarea_area
        };
        let content = state.content_area(inner);

        let edited_now = state.take_edited_flag();

        let wrap_mode = matches!(state.overflow_mode, TextOverflowMode::Wrap);
        let provider = state.editor.data_provider();
        let total = provider.line_count();

        let (start, intra) = resolve_start_line_and_intra_indented(state, content);

        let mut display_lines: Vec<Line> = Vec::new();

        if total == 0 || start >= total {
            if let Some(ph) = &state.placeholder {
                let mut spans = Vec::new();
                let prefix = state.line_number_prefix(0, true);
                if !prefix.is_empty() {
                    spans.push(Span::raw(prefix));
                }
                spans.push(Span::raw(ph.clone()));
                display_lines.push(Line::from(spans));
            }
        } else if wrap_mode {
            let mut rows_left = content.height;
            let indent = state.wrap_indent_cols;
            let mut i = start;
            while i < total && rows_left > 0 {
                let s = provider.field_value(i);
                let segments = wrap_segments_with_offsets(s, content.width, indent);
                let skip = if i == start { intra as usize } else { 0 };
                for (seg_idx, (seg, offset)) in segments.into_iter().enumerate().skip(skip) {
                    let mut prefix = state.line_number_prefix(i, seg_idx == 0);
                    if seg_idx > 0 {
                        prefix.push_str(&continuation_prefix(content.width, indent));
                    }
                    display_lines.push(styled_segment_line(
                        seg,
                        i,
                        offset,
                        state,
                        (!prefix.is_empty()).then_some(prefix),
                        None,
                    ));
                    rows_left = rows_left.saturating_sub(1);
                    if rows_left == 0 {
                        break;
                    }
                }
                i += 1;
            }
        } else {
            // Indicator mode: full inner width; RIGHT_PAD only affects cursor clamp and h-scroll
            let end = (start.saturating_add(content.height as usize)).min(total);

            for i in start..end {
                let s = provider.field_value(i);
                match state.overflow_mode {
                    TextOverflowMode::Wrap => unreachable!(),
                    TextOverflowMode::Indicator { ch } => {
                        let fits = display_width(s) <= content.width;

                        let start_cols = if i == state.current_field() {
                            let col_idx = state.display_cursor_position();
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
                                target_h.max(state.h_scroll)
                            }
                        } else {
                            0
                        };

                        let prefix = state.line_number_prefix(i, true);
                        display_lines.push(clipped_line_with_selection(
                            s,
                            i,
                            state,
                            content.width,
                            ch,
                            start_cols,
                            (!prefix.is_empty()).then_some(prefix),
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

        #[cfg(feature = "commandline")]
        if let Some(commandline) = state.commandline_mut() {
            CommandLine::default().render(area, buf, commandline.state_mut());
        }
    }
}
