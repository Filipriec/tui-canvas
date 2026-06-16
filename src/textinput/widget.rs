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
#[cfg(feature = "gui")]
use crate::gui_utils::{
    clip_inline_completion_with_indicator_padded, clip_line_with_indicator_padded,
    clip_window_with_indicator_padded, compute_h_scroll_with_padding, display_cols_up_to,
    display_width,
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
    pub(crate) highlight_style: Style,
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
            highlight_style: Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD),
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

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
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

        let _ = state.take_edited_flag();
        let text = state.current_display_text_for_render();
        let suggestion = state.suggestion_suffix();

        let line = if text.is_empty() && suggestion.is_none() {
            Line::from(Span::raw(state.placeholder.clone().unwrap_or_default()))
        } else if suggestion.is_some()
            || !matches!(
                state.core().ui_state().selection_state(),
                SelectionState::Characterwise { .. }
            )
        {
            // Plain text rendering (with optional suggestion) when there is no
            // active characterwise selection.
            let fits = display_width(&text) <= inner.width;
            let start_cols = if fits {
                state.h_scroll
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
        } else {
            // Characterwise selection is active: render with highlighting.
            let SelectionState::Characterwise { anchor } =
                state.core().ui_state().selection_state()
            else {
                unreachable!()
            };
            let (_, anchor_char) = *anchor;
            let cursor_pos = state.display_cursor_position();
            let text_len = text.chars().count();

            let start = anchor_char.min(cursor_pos).min(text_len);
            let end = anchor_char.max(cursor_pos).min(text_len);

            let fits = display_width(&text) <= inner.width;
            let start_cols = if fits {
                state.h_scroll
            } else {
                let cursor_cols = display_cols_up_to(&text, cursor_pos);
                let (target_h, _) =
                    compute_h_scroll_with_padding(cursor_cols, display_width(&text), inner.width);
                target_h.max(state.h_scroll)
            };

            let full_line = if start == end {
                // Single-char anchor at cursor (e.g. Helix primary selection):
                // highlight just the cursor character.
                let before: String = text.chars().take(start).collect();
                let highlighted: String = text.chars().skip(start).take(1).collect();
                let after: String = text.chars().skip(start + 1).collect();
                Line::from(vec![
                    Span::styled(before, self.style),
                    Span::styled(highlighted, self.highlight_style),
                    Span::styled(after, self.style),
                ])
            } else {
                let before: String = text.chars().take(start).collect();
                let highlighted: String = text
                    .chars()
                    .skip(start)
                    .take(end.saturating_sub(start) + 1)
                    .collect();
                let after: String = text.chars().skip(end + 1).collect();
                Line::from(vec![
                    Span::styled(before, self.style),
                    Span::styled(highlighted, self.highlight_style),
                    Span::styled(after, self.style),
                ])
            };

            if fits {
                full_line
            } else {
                clip_line_with_indicator_padded(
                    full_line,
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

#[cfg(all(test, feature = "gui"))]
mod tests {
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Style},
        widgets::StatefulWidget,
    };

    use super::TextInput;
    use crate::{
        canvas::state::SelectionState,
        textinput::{TextInputProvider, TextInputState},
    };

    #[test]
    fn characterwise_selection_highlight_uses_inclusive_end() {
        let mut input = TextInputState::<TextInputProvider>::from_text("abcd");
        input.set_cursor_position(2);
        input.form.core.ui_state.selection = SelectionState::Characterwise { anchor: (0, 0) };

        let mut widget = TextInput::default().highlight_style(Style::default().bg(Color::Red));
        widget.block = None;

        let area = Rect::new(0, 0, 4, 1);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf, &mut input);

        assert_eq!(buf.cell((0, 0)).unwrap().bg, Color::Red);
        assert_eq!(buf.cell((1, 0)).unwrap().bg, Color::Red);
        assert_eq!(buf.cell((2, 0)).unwrap().bg, Color::Red);
        assert_ne!(buf.cell((3, 0)).unwrap().bg, Color::Red);
    }
}
