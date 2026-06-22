use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use crate::canvas::AppMode;
use crate::gui_utils::{
    clip_window_with_indicator_padded, compute_h_scroll_with_padding, display_cols_up_to,
    display_width,
};

use super::state::CommandLineState;

/// Placement policy for [`CommandLine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandLinePlacement {
    /// Render into the last row of the provided area.
    Bottom,
    /// Render into the provided area exactly.
    Inline,
}

/// Renderable command-line prompt.
///
/// This widget draws nothing while [`CommandLineState`] is inactive. By
/// default it renders into the last row of the provided area, so rendering it
/// over the terminal frame reserves the terminal's bottom row for command
/// input. Use [`CommandLine::inline`] when an app wants to place it manually.
#[derive(Debug, Clone)]
pub struct CommandLine {
    pub(crate) style: Style,
    pub(crate) prompt_style: Style,
    pub(crate) placeholder_style: Style,
    pub(crate) cursor_normal_style: Style,
    pub(crate) cursor_insert_style: Style,
    pub(crate) cursor_select_style: Style,
    pub(crate) overflow_indicator: char,
    pub(crate) placement: CommandLinePlacement,
}

impl Default for CommandLine {
    fn default() -> Self {
        Self {
            style: Style::default(),
            prompt_style: Style::default().fg(Color::Yellow),
            placeholder_style: Style::default().fg(Color::DarkGray),
            cursor_normal_style: Style::default().fg(Color::Black).bg(Color::White),
            cursor_insert_style: Style::default().fg(Color::Black).bg(Color::Green),
            cursor_select_style: Style::default().fg(Color::Black).bg(Color::Blue),
            overflow_indicator: '$',
            placement: CommandLinePlacement::Bottom,
        }
    }
}

impl CommandLine {
    pub(crate) fn placement_area(area: Rect, placement: CommandLinePlacement) -> Rect {
        match placement {
            CommandLinePlacement::Bottom => Rect {
                x: area.x,
                y: area.y.saturating_add(area.height.saturating_sub(1)),
                width: area.width,
                height: area.height.min(1),
            },
            CommandLinePlacement::Inline => area,
        }
    }

    /// Render into the last row of the provided area.
    pub fn bottom(mut self) -> Self {
        self.placement = CommandLinePlacement::Bottom;
        self
    }

    /// Render into the provided area exactly.
    pub fn inline(mut self) -> Self {
        self.placement = CommandLinePlacement::Inline;
        self
    }

    /// Set the placement policy.
    pub fn placement(mut self, placement: CommandLinePlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Style applied to the input text.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Style applied to the prompt prefix (`:`, `/`, or `?`).
    pub fn prompt_style(mut self, style: Style) -> Self {
        self.prompt_style = style;
        self
    }

    /// Reserved for empty-input placeholder styling.
    pub fn placeholder_style(mut self, style: Style) -> Self {
        self.placeholder_style = style;
        self
    }

    pub fn cursor_styles(mut self, normal: Style, insert: Style, select: Style) -> Self {
        self.cursor_normal_style = normal;
        self.cursor_insert_style = insert;
        self.cursor_select_style = select;
        self
    }

    /// Character shown when input is horizontally clipped.
    pub fn overflow_indicator(mut self, ch: char) -> Self {
        self.overflow_indicator = ch;
        self
    }
}

fn cursor_style_for_mode(mode: AppMode, normal: Style, insert: Style, select: Style) -> Style {
    match mode {
        AppMode::Ins => insert,
        AppMode::Sel => select,
        AppMode::Nor | AppMode::General | AppMode::Command => normal,
    }
}

impl StatefulWidget for CommandLine {
    type State = CommandLineState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.is_active() {
            return;
        }

        let area = Self::placement_area(area, self.placement);
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
                let (target_h, _) =
                    compute_h_scroll_with_padding(cursor_cols, display_width(text), input_width);
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

        let (cursor_x, cursor_y) = state.cursor_inline(area);
        if cursor_x >= input_area.x
            && cursor_x < input_area.right()
            && cursor_y >= input_area.y
            && cursor_y < input_area.bottom()
        {
            if let Some(cell) = buf.cell_mut((cursor_x, cursor_y)) {
                if state.input.display_cursor_position() >= text.chars().count() {
                    cell.set_symbol(" ");
                }
                cell.set_style(cursor_style_for_mode(
                    state.input.mode(),
                    self.cursor_normal_style,
                    self.cursor_insert_style,
                    self.cursor_select_style,
                ));
            }
        }
    }
}
