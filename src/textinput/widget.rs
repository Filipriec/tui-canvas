#[cfg(feature = "gui")]
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, StatefulWidget, Widget},
};

#[cfg(feature = "gui")]
use crate::gui_utils::{
    clip_inline_completion_with_indicator_padded, clip_window_with_indicator_padded,
    compute_h_scroll_with_padding, display_cols_up_to, display_width,
};
#[cfg(feature = "gui")]
use crate::textinput::provider::{TextInputDataProvider, TextInputProvider};

#[cfg(feature = "gui")]
use crate::textinput::state::TextInputState;

#[cfg(feature = "gui")]
#[derive(Debug, Clone)]
pub struct TextInput<'a, P: TextInputDataProvider = TextInputProvider> {
    pub(crate) block: Option<Block<'a>>,
    pub(crate) style: Style,
    pub(crate) suggestion_style: Style,
    pub(crate) border_type: BorderType,
    pub(crate) _provider: std::marker::PhantomData<P>,
}

#[cfg(feature = "gui")]
impl<'a, P: TextInputDataProvider> Default for TextInput<'a, P> {
    fn default() -> Self {
        Self {
            block: Some(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            style: Style::default(),
            suggestion_style: Style::default().fg(Color::DarkGray),
            border_type: BorderType::Rounded,
            _provider: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "gui")]
impl<'a, P: TextInputDataProvider> TextInput<'a, P> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn suggestion_style(mut self, style: Style) -> Self {
        self.suggestion_style = style;
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
impl<'a, P: TextInputDataProvider> StatefulWidget for TextInput<'a, P> {
    type State = TextInputState<P>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.ensure_visible(area, self.block.as_ref());

        let inner = if let Some(b) = &self.block {
            b.clone().render(area, buf);
            b.inner(area)
        } else {
            area
        };

        let edited_now = state.take_edited_flag();
        let text = state.current_display_text_for_render();
        let suggestion = state.suggestion_suffix();

        let line = if text.is_empty() && suggestion.is_none() {
            Line::from(Span::raw(state.placeholder.clone().unwrap_or_default()))
        } else {
            let fits = display_width(&text) <= inner.width;
            let start_cols = if fits {
                if edited_now {
                    0
                } else {
                    state.h_scroll
                }
            } else {
                let cursor_cols = display_cols_up_to(&text, state.display_cursor_position());
                let (target_h, _) =
                    compute_h_scroll_with_padding(cursor_cols, display_width(&text), inner.width);
                target_h.max(state.h_scroll)
            };

            if suggestion.is_some() {
                clip_inline_completion_with_indicator_padded(
                    &text,
                    suggestion,
                    inner.width,
                    state.overflow_indicator,
                    start_cols,
                    self.style,
                    self.suggestion_style,
                )
            } else {
                clip_window_with_indicator_padded(
                    &text,
                    inner.width,
                    state.overflow_indicator,
                    start_cols,
                )
            }
        };

        let p = Paragraph::new(vec![line])
            .alignment(Alignment::Left)
            .style(self.style);

        p.render(inner, buf);
    }
}
