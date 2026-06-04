use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use crate::gui_utils::{
    clip_window_with_indicator_padded, compute_h_scroll_with_padding, display_cols_up_to,
    display_width,
};

use super::state::CommandLineState;

#[derive(Debug, Clone)]
pub struct CommandLine {
    pub(crate) style: Style,
    pub(crate) prompt_style: Style,
    pub(crate) placeholder_style: Style,
    pub(crate) overflow_indicator: char,
}

impl Default for CommandLine {
    fn default() -> Self {
        Self {
            style: Style::default(),
            prompt_style: Style::default().fg(Color::Yellow),
            placeholder_style: Style::default().fg(Color::DarkGray),
            overflow_indicator: '$',
        }
    }
}

impl CommandLine {
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn prompt_style(mut self, style: Style) -> Self {
        self.prompt_style = style;
        self
    }

    pub fn placeholder_style(mut self, style: Style) -> Self {
        self.placeholder_style = style;
        self
    }

    pub fn overflow_indicator(mut self, ch: char) -> Self {
        self.overflow_indicator = ch;
        self
    }
}

impl StatefulWidget for CommandLine {
    type State = CommandLineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.is_active() {
            return;
        }

        let prompt = state.prompt();
        let prompt_width = display_width(prompt);
        let input_width = area.width.saturating_sub(prompt_width);
        let input_area = Rect {
            x: area.x.saturating_add(prompt_width),
            y: area.y,
            width: input_width,
            height: area.height,
        };
        state.input.ensure_visible(input_area, None);

        let text = state.input();
        let input_line = if text.is_empty() {
            Line::from(Span::styled("", self.placeholder_style))
        } else {
            let fits = display_width(text) <= input_width;
            let start_cols = if fits {
                state.input.h_scroll
            } else {
                let cursor_cols = display_cols_up_to(text, state.input.display_cursor_position());
                let (target_h, _) = compute_h_scroll_with_padding(cursor_cols, input_width);
                target_h.max(state.input.h_scroll)
            };
            clip_window_with_indicator_padded(
                text,
                input_width,
                self.overflow_indicator,
                start_cols,
            )
        };

        let mut spans = Vec::new();
        spans.push(Span::styled(prompt.to_string(), self.prompt_style));
        spans.extend(input_line.spans);

        let paragraph = Paragraph::new(Line::from(spans))
            .alignment(Alignment::Left)
            .style(self.style);
        paragraph.render(area, buf);
    }
}
