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
                        display_lines.push(clip_with_indicator(s, inner.width, ch));
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
