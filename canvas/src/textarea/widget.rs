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
use crate::textarea::state::{TextAreaState, TextOverflowMode};

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

#[cfg(feature = "gui")]
fn slice_by_display_cols(s: &str, start_cols: u16, max_cols: u16) -> String {
    if max_cols == 0 {
        return String::new();
    }
    
    let mut current_cols: u16 = 0;
    let mut output = String::new();
    let mut output_cols: u16 = 0;
    let mut started = false;

    for ch in s.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        
        // Skip characters until we reach the start position
        if !started {
            if current_cols + char_width <= start_cols {
                current_cols += char_width;
                continue;
            } else {
                started = true;
            }
        }
        
        // Stop if adding this character would exceed our budget
        if output_cols + char_width > max_cols {
            break;
        }
        
        output.push(ch);
        output_cols += char_width;
        current_cols += char_width;
    }

    output
}

#[cfg(feature = "gui")]
fn clip_window_with_indicator(
    text: &str,
    width: u16,
    indicator: char,
    start_cols: u16,
) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }

    let total_width = display_width(text);

    // If the line fits entirely, show it as-is (no indicators, no windowing)
    if total_width <= width {
        return Line::from(Span::raw(text.to_string()));
    }

    // Left indicator only if there is real overflow and the window is shifted
    let show_left_indicator = start_cols > 0 && total_width > width;
    let left_indicator_width = if show_left_indicator { 1 } else { 0 };

    // Will we overflow to the right from this start?
    let remaining_after_start = total_width.saturating_sub(start_cols);
    let content_budget = width.saturating_sub(left_indicator_width);
    let show_right_indicator = remaining_after_start > content_budget;
    let right_indicator_width = if show_right_indicator { 1 } else { 0 };

    // Compute visible slice budget
    let actual_content_width = content_budget.saturating_sub(right_indicator_width);
    let visible_content = slice_by_display_cols(text, start_cols, actual_content_width);

    let mut spans = Vec::new();
    if show_left_indicator {
        spans.push(Span::raw(indicator.to_string()));
    }
    spans.push(Span::raw(visible_content));
    if show_right_indicator {
        spans.push(Span::raw(indicator.to_string()));
    }
    Line::from(spans)
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

        let total = state.editor.data_provider().line_count();
        let start = state.scroll_y as usize;
        let end = start
            .saturating_add(inner.height as usize)
            .min(total);

        let mut display_lines: Vec<Line> = Vec::with_capacity(end - start);

        if start >= end {
            if let Some(ph) = &state.placeholder {
                display_lines.push(Line::from(Span::raw(ph.clone())));
            }
        } else {
            for i in start..end {
                let s = state.editor.data_provider().field_value(i);
                match state.overflow_mode {
                    TextOverflowMode::Wrap => {
                        display_lines.push(Line::from(Span::raw(s.to_string())));
                    }
                    TextOverflowMode::Indicator { ch } => {
                        // Use horizontal scroll for the active line, show full text for others
                        let h_scroll_offset = if i == state.current_field() {
                            state.h_scroll
                        } else {
                            0
                        };

                        display_lines.push(clip_window_with_indicator(
                                s,
                                inner.width,
                                ch,
                                h_scroll_offset,
                        ));
                    }
                }
            }
        }

        let mut p = Paragraph::new(display_lines)
            .alignment(Alignment::Left)
            .style(self.style);

        if matches!(state.overflow_mode, TextOverflowMode::Wrap) {
            p = p.wrap(Wrap { trim: false });
        }

        p.render(inner, buf);
    }
}
