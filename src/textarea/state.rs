// src/textarea/state.rs
use std::ops::{Deref, DerefMut};

#[cfg(feature = "cursor-style")]
use crate::cursor::CursorManager;
use crate::editor::FormEditor;
#[cfg(feature = "commandline")]
use crate::{
    commandline::{CommandLineCommand, CommandLineRegistry, CommandLineState},
};
#[cfg(all(feature = "commandline", feature = "keybindings"))]
use crate::commandline::CommandLineSubmit;
#[cfg(all(feature = "commandline", feature = "keybindings"))]
use crate::keybindings::KeyEventOutcome;
#[cfg(feature = "gui")]
use crate::gui_utils::{compute_h_scroll_with_padding, RIGHT_PAD};
use crate::textarea::provider::{TextAreaDataProvider, TextAreaProvider};
#[cfg(feature = "cursor-style")]
use std::io;

#[cfg(feature = "gui")]
use ratatui::{layout::Rect, widgets::Block};

#[cfg(feature = "gui")]
use unicode_width::UnicodeWidthChar;

#[cfg(feature = "gui")]
fn normalize_indent(width: u16, indent: u16) -> u16 {
    indent.min(width.saturating_sub(1))
}

#[cfg(feature = "gui")]
pub(crate) fn continuation_prefix(width: u16, indent: u16) -> String {
    if width == 0 {
        return String::new();
    }

    let max_cols = width.saturating_sub(1) as usize;
    let spaces = (indent as usize).min(max_cols.saturating_sub(1));
    let mut prefix = " ".repeat(spaces);
    if prefix.chars().count() < max_cols {
        prefix.push('↪');
    }
    if prefix.chars().count() < max_cols {
        prefix.push(' ');
    }
    prefix
}

#[cfg(feature = "gui")]
pub(crate) fn continuation_prefix_width(width: u16, indent: u16) -> u16 {
    continuation_prefix(width, indent)
        .chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0) as u16)
        .sum()
}

#[cfg(feature = "gui")]
pub(crate) fn count_wrapped_rows_indented(s: &str, width: u16, indent: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let indent = normalize_indent(width, indent);
    let cont_prefix = continuation_prefix_width(width, indent);

    let mut rows: u16 = 1;
    let mut used: u16 = 0;

    for ch in s.chars() {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;

        if used > 0 && used.saturating_add(w) > width {
            rows = rows.saturating_add(1);
            used = cont_prefix;
        }
        used = used.saturating_add(w);
    }

    rows
}

#[cfg(feature = "gui")]
fn wrapped_rows_to_cursor_indented(
    s: &str,
    width: u16,
    indent: u16,
    cursor_chars: usize,
) -> (u16, u16) {
    if width == 0 {
        return (0, 0);
    }
    let indent = normalize_indent(width, indent);
    let cont_prefix = continuation_prefix_width(width, indent);

    let mut row: u16 = 0;
    let mut used: u16 = 0;

    for (i, ch) in s.chars().enumerate() {
        if i >= cursor_chars {
            break;
        }
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;

        if used > 0 && used.saturating_add(w) > width {
            row = row.saturating_add(1);
            used = cont_prefix;
        }
        used = used.saturating_add(w);
    }

    let char_count = s.chars().count();
    if cursor_chars > 0 && cursor_chars <= char_count && used >= width {
        row = row.saturating_add(1);
        used = cont_prefix;
    }

    (row, used.min(width.saturating_sub(1)))
}

#[cfg(feature = "gui")]
pub(crate) fn wrap_segment_ranges(s: &str, width: u16, indent: u16) -> Vec<(usize, usize)> {
    if width == 0 {
        return vec![(0, 0)];
    }

    let indent = normalize_indent(width, indent);
    let cont_prefix = continuation_prefix_width(width, indent);
    let mut ranges = Vec::new();
    let mut used: u16 = 0;
    let mut segment_start = 0;
    let mut char_len = 0;

    for (char_idx, ch) in s.chars().enumerate() {
        char_len = char_idx + 1;
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;

        if used > 0 && used.saturating_add(w) > width {
            ranges.push((segment_start, char_idx));
            segment_start = char_idx;
            used = cont_prefix;
        }
        used = used.saturating_add(w);
    }

    ranges.push((segment_start, char_len));
    ranges
}

#[cfg(feature = "gui")]
fn char_index_for_visual_col(
    s: &str,
    start: usize,
    end: usize,
    prefix_cols: u16,
    target_col: u16,
) -> usize {
    if start >= end {
        return start;
    }
    if target_col <= prefix_cols {
        return start;
    }

    let mut col = prefix_cols;
    for (char_idx, ch) in s
        .chars()
        .enumerate()
        .skip(start)
        .take(end.saturating_sub(start))
    {
        let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
        if col.saturating_add(w) > target_col {
            return char_idx;
        }
        col = col.saturating_add(w);
    }

    end
}

pub type TextAreaEditor<P = TextAreaProvider> = FormEditor<P>;

/// Outcome of feeding a single input event to a [`TextAreaState`].
///
/// Unlike the single-line input there is no `Submitted` variant: in a textarea
/// `Enter` inserts a newline rather than submitting. Hosts can use `Ignored` to
/// detect keys the textarea did not consume (e.g. to drive focus handoff).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaEventOutcome {
    /// The event was not recognized/consumed by the textarea.
    Ignored,
    /// The event was handled (text edited or cursor moved).
    Handled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverflowMode {
    Indicator { ch: char },
    Wrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAreaSearchMatch {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

#[cfg(feature = "commandline")]
pub struct TextAreaCommandLineState {
    state: CommandLineState,
    commands: CommandLineRegistry,
}

#[cfg(feature = "commandline")]
impl Default for TextAreaCommandLineState {
    fn default() -> Self {
        Self {
            state: CommandLineState::new(),
            commands: default_textarea_commandline_commands(),
        }
    }
}

#[cfg(feature = "commandline")]
impl TextAreaCommandLineState {
    pub fn state(&self) -> &CommandLineState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut CommandLineState {
        &mut self.state
    }

    pub fn commands(&self) -> &CommandLineRegistry {
        &self.commands
    }

    pub fn commands_mut(&mut self) -> &mut CommandLineRegistry {
        &mut self.commands
    }
}

#[cfg(feature = "commandline")]
fn default_textarea_commandline_commands() -> CommandLineRegistry {
    let mut registry = CommandLineRegistry::new();
    registry
        .register(
            CommandLineCommand::new("set-number")
                .alias("number")
                .alias("nu")
                .pattern(["set", "number"])
                .pattern(["set", "nu"]),
        )
        .unwrap()
        .register(
            CommandLineCommand::new("set-relative-number")
                .alias("relativenumber")
                .alias("rnu")
                .pattern(["set", "relativenumber"])
                .pattern(["set", "rnu"]),
        )
        .unwrap()
        .register(
            CommandLineCommand::new("set-no-number")
                .alias("nonumber")
                .alias("nonu")
                .pattern(["set", "nonumber"])
                .pattern(["set", "nonu"]),
        )
        .unwrap()
        .register(CommandLineCommand::new("no-highlight").alias("noh").alias("nohlsearch"))
        .unwrap();
    registry
}

#[cfg(feature = "gui")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaLineNumberMode {
    None,
    Absolute,
    Relative,
}

/// Multi-line textarea widget state.
///
/// Wraps a [`FormEditor`]. Editing, cursor, and movement methods from the engine
/// are available directly, and the engine can be reached explicitly via
/// [`TextAreaState::editor`] / [`TextAreaState::editor_mut`]. With the
/// `validation` and `computed` features enabled, the corresponding helper
/// methods are re-exposed as inherent methods on this type.
pub struct TextAreaState<P: TextAreaDataProvider = TextAreaProvider> {
    pub(crate) editor: TextAreaEditor<P>,
    pub(crate) scroll_y: u16,
    pub(crate) placeholder: Option<String>,
    pub(crate) overflow_mode: TextOverflowMode,
    pub(crate) h_scroll: u16,
    pub(crate) search_query: Option<String>,
    pub(crate) active_search_match: Option<TextAreaSearchMatch>,
    /// A Helix command waiting for the next literal character (`f`, `t`, `r`, …).
    pub(crate) helix_pending: Option<crate::textarea::actions::selection::helix::HelixPending>,
    /// The last `f`/`t`/`F`/`T` for `Alt-.` to repeat.
    pub(crate) helix_last_find: Option<crate::textarea::actions::selection::helix::HelixFind>,
    /// A Vim command waiting for the next literal character (`f`, `t`, `r`, …).
    pub(crate) vim_pending: Option<crate::textarea::actions::selection::vim::VimPending>,
    /// The last `f`/`t`/`F`/`T` for `;`/`,` to repeat.
    pub(crate) vim_last_find: Option<crate::textarea::actions::selection::vim::VimFind>,
    #[cfg(feature = "gui")]
    pub(crate) line_number_mode: TextAreaLineNumberMode,
    #[cfg(feature = "gui")]
    pub(crate) wrap_indent_cols: u16,
    #[cfg(feature = "gui")]
    pub(crate) viewport_width: u16,
    #[cfg(feature = "gui")]
    pub(crate) viewport_height: u16,
    #[cfg(feature = "gui")]
    pub(crate) edited_this_frame: bool,
    #[cfg(feature = "commandline")]
    pub(crate) commandline: Option<TextAreaCommandLineState>,
}

impl<P: TextAreaDataProvider + Default> Default for TextAreaState<P> {
    fn default() -> Self {
        Self {
            editor: FormEditor::new(P::default()),
            scroll_y: 0,
            placeholder: None,
            overflow_mode: TextOverflowMode::Indicator { ch: '$' },
            h_scroll: 0,
            search_query: None,
            active_search_match: None,
            helix_pending: None,
            helix_last_find: None,
            vim_pending: None,
            vim_last_find: None,
            #[cfg(feature = "gui")]
            line_number_mode: TextAreaLineNumberMode::None,
            #[cfg(feature = "gui")]
            wrap_indent_cols: 0,
            #[cfg(feature = "gui")]
            viewport_width: 80,
            #[cfg(feature = "gui")]
            viewport_height: 10,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
            #[cfg(feature = "commandline")]
            commandline: None,
        }
    }
}

impl<P: TextAreaDataProvider> Deref for TextAreaState<P> {
    type Target = TextAreaEditor<P>;

    fn deref(&self) -> &Self::Target {
        &self.editor
    }
}

impl<P: TextAreaDataProvider> DerefMut for TextAreaState<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.editor
    }
}

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn with_provider(provider: P) -> Self {
        Self {
            editor: FormEditor::new(provider),
            scroll_y: 0,
            placeholder: None,
            overflow_mode: TextOverflowMode::Indicator { ch: '$' },
            h_scroll: 0,
            search_query: None,
            active_search_match: None,
            helix_pending: None,
            helix_last_find: None,
            vim_pending: None,
            vim_last_find: None,
            #[cfg(feature = "gui")]
            line_number_mode: TextAreaLineNumberMode::None,
            #[cfg(feature = "gui")]
            wrap_indent_cols: 0,
            #[cfg(feature = "gui")]
            viewport_width: 80,
            #[cfg(feature = "gui")]
            viewport_height: 10,
            #[cfg(feature = "gui")]
            edited_this_frame: false,
            #[cfg(feature = "commandline")]
            commandline: None,
        }
    }

    pub fn from_text<S: Into<String>>(text: S) -> Self {
        Self::with_provider(P::from_text(text.into()))
    }

    #[cfg(feature = "commandline")]
    pub fn use_default_commandline(&mut self) {
        self.commandline = Some(TextAreaCommandLineState::default());
    }

    #[cfg(feature = "commandline")]
    pub fn commandline(&self) -> Option<&TextAreaCommandLineState> {
        self.commandline.as_ref()
    }

    #[cfg(feature = "commandline")]
    pub fn commandline_mut(&mut self) -> Option<&mut TextAreaCommandLineState> {
        self.commandline.as_mut()
    }

    #[cfg(all(feature = "gui", feature = "commandline"))]
    pub fn commandline_textarea_area(&self, area: Rect) -> Rect {
        if self.commandline.is_some() {
            Rect {
                height: area.height.saturating_sub(1),
                ..area
            }
        } else {
            area
        }
    }

    #[cfg(all(feature = "gui", feature = "commandline"))]
    pub fn cursor_with_commandline(&self, area: Rect, block: Option<&Block<'_>>) -> (u16, u16) {
        self.cursor(area, block)
    }

    #[cfg(all(feature = "commandline", feature = "keybindings", feature = "crossterm"))]
    pub fn handle_key_event_with_commandline(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> KeyEventOutcome {
        self.handle_key_event(key)
    }

    #[cfg(all(feature = "commandline", feature = "keybindings"))]
    pub(crate) fn apply_default_commandline_submit(&mut self, submit: CommandLineSubmit) {
        let is_helix = self.editor.keybinding_paradigm().is_helix();
        match submit {
            CommandLineSubmit::SearchForward(query) => {
                self.set_search_query(query);
                self.find_next();
                if is_helix {
                    self.select_active_search_match_helix();
                }
            }
            CommandLineSubmit::SearchBackward(query) => {
                self.set_search_query(query);
                self.find_previous();
                if is_helix {
                    self.select_active_search_match_helix();
                }
            }
            CommandLineSubmit::Command(command) => self.apply_default_commandline_command(&command),
        }
    }

    #[cfg(all(feature = "commandline", feature = "keybindings"))]
    pub(crate) fn apply_default_commandline_command(&mut self, command: &str) {
        let Some(commandline) = &self.commandline else {
            return;
        };
        let Ok(invocation) = commandline.commands.dispatch(command) else {
            return;
        };

        match invocation.command.name.as_str() {
            #[cfg(feature = "gui")]
            "set-number" => self.show_absolute_line_numbers(),
            #[cfg(feature = "gui")]
            "set-relative-number" => self.show_relative_line_numbers(),
            #[cfg(feature = "gui")]
            "set-no-number" => self.hide_line_numbers(),
            "no-highlight" => self.clear_search(),
            _ => {}
        }
    }

    pub fn text(&self) -> String {
        self.editor.data_provider().to_text()
    }

    pub fn set_text<S: Into<String>>(&mut self, text: S) {
        self.editor.data_provider_mut().set_text(text.into());
        self.editor.ui_state.current_field = 0;
        self.editor.set_cursor_raw(0);
        self.active_search_match = None;
    }

    pub fn set_placeholder<S: Into<String>>(&mut self, s: S) {
        self.placeholder = Some(s.into());
    }

    pub fn use_overflow_indicator(&mut self, ch: char) {
        self.overflow_mode = TextOverflowMode::Indicator { ch };
    }

    pub fn use_wrap(&mut self) {
        self.overflow_mode = TextOverflowMode::Wrap;
    }

    pub fn set_search_query<S: Into<String>>(&mut self, query: S) {
        let query = query.into();
        if query.is_empty() {
            self.clear_search();
        } else {
            self.search_query = Some(query);
            self.active_search_match = None;
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.active_search_match = None;
    }

    pub fn search_query(&self) -> Option<&str> {
        self.search_query.as_deref()
    }

    pub fn active_search_match(&self) -> Option<TextAreaSearchMatch> {
        // Self-validate: a stored match goes stale as soon as the text is
        // edited (delete/change/paste shift positions). Only report it while the
        // text it points at still equals the query, so a deleted match's
        // highlight disappears instead of landing on whatever shifted into place.
        let m = self.active_search_match?;
        let query = self.search_query.as_deref()?;
        let provider = self.editor.data_provider();
        if m.line >= provider.line_count() || m.end <= m.start {
            return None;
        }
        let actual: String = provider
            .field_value(m.line)
            .chars()
            .skip(m.start)
            .take(m.end - m.start)
            .collect();
        (actual == query).then_some(m)
    }

    pub fn search_matches_in_line(&self, line_idx: usize) -> Vec<TextAreaSearchMatch> {
        let Some(query) = self.search_query.as_deref() else {
            return Vec::new();
        };
        if query.is_empty() || line_idx >= self.editor.data_provider().line_count() {
            return Vec::new();
        }

        let line = self.editor.data_provider().field_value(line_idx);
        line.match_indices(query)
            .map(|(byte_start, matched)| {
                let start = line[..byte_start].chars().count();
                let end = start + matched.chars().count();
                TextAreaSearchMatch {
                    line: line_idx,
                    start,
                    end,
                }
            })
            .collect()
    }

    pub fn search_matches(&self) -> Vec<TextAreaSearchMatch> {
        let mut matches = Vec::new();
        let total = self.editor.data_provider().line_count();
        for line_idx in 0..total {
            matches.extend(self.search_matches_in_line(line_idx));
        }
        matches
    }

    pub fn find_next(&mut self) -> bool {
        let matches = self.search_matches();
        if matches.is_empty() {
            self.active_search_match = None;
            return false;
        }

        let cursor = (self.current_field(), self.cursor_position());
        let target = matches
            .iter()
            .copied()
            .find(|m| (m.line, m.start) > cursor)
            .unwrap_or(matches[0]);
        self.move_to_search_match(target)
    }

    pub fn find_previous(&mut self) -> bool {
        let matches = self.search_matches();
        if matches.is_empty() {
            self.active_search_match = None;
            return false;
        }

        let cursor = (self.current_field(), self.cursor_position());
        let target = matches
            .iter()
            .rev()
            .copied()
            .find(|m| (m.line, m.start) < cursor)
            .unwrap_or(*matches.last().unwrap());
        self.move_to_search_match(target)
    }

    fn move_to_search_match(&mut self, target: TextAreaSearchMatch) -> bool {
        let moved = self.editor.transition_to_field(target.line).is_ok();
        self.editor.set_cursor_for_mode(
            target.start,
            self.editor.current_text().chars().count(),
        );
        self.active_search_match = Some(target);
        moved
    }

    pub fn set_wrap_indent_cols(&mut self, cols: u16) {
        #[cfg(feature = "gui")]
        {
            self.wrap_indent_cols = cols;
        }
    }

    #[cfg(feature = "gui")]
    pub fn set_line_number_mode(&mut self, mode: TextAreaLineNumberMode) {
        self.line_number_mode = mode;
        self.h_scroll = 0;
    }

    #[cfg(feature = "gui")]
    pub fn line_number_mode(&self) -> TextAreaLineNumberMode {
        self.line_number_mode
    }

    #[cfg(feature = "gui")]
    pub fn show_absolute_line_numbers(&mut self) {
        self.set_line_number_mode(TextAreaLineNumberMode::Absolute);
    }

    #[cfg(feature = "gui")]
    pub fn show_relative_line_numbers(&mut self) {
        self.set_line_number_mode(TextAreaLineNumberMode::Relative);
    }

    #[cfg(feature = "gui")]
    pub fn hide_line_numbers(&mut self) {
        self.set_line_number_mode(TextAreaLineNumberMode::None);
    }

    #[cfg(feature = "gui")]
    pub(crate) fn line_number_gutter_width(&self) -> u16 {
        if matches!(self.line_number_mode, TextAreaLineNumberMode::None) {
            return 0;
        }

        let line_count = self.editor.data_provider().line_count().max(1);
        let digits = line_count.ilog10() as u16 + 1;
        digits.saturating_add(1)
    }

    #[cfg(feature = "gui")]
    pub(crate) fn content_area(&self, inner: Rect) -> Rect {
        let gutter_width = self.line_number_gutter_width().min(inner.width);
        Rect {
            x: inner.x.saturating_add(gutter_width),
            y: inner.y,
            width: inner.width.saturating_sub(gutter_width),
            height: inner.height,
        }
    }

    #[cfg(feature = "gui")]
    pub(crate) fn line_number_prefix(&self, line_idx: usize, first_visual_row: bool) -> String {
        let width = self.line_number_gutter_width() as usize;
        if width == 0 {
            return String::new();
        }

        if !first_visual_row {
            return " ".repeat(width);
        }

        let number = match self.line_number_mode {
            TextAreaLineNumberMode::None => return String::new(),
            TextAreaLineNumberMode::Absolute => line_idx.saturating_add(1),
            TextAreaLineNumberMode::Relative if line_idx == self.current_field() => {
                line_idx.saturating_add(1)
            }
            TextAreaLineNumberMode::Relative => line_idx.abs_diff(self.current_field()),
        };

        format!("{number:>digits$} ", digits = width.saturating_sub(1))
    }

    #[cfg(feature = "gui")]
    fn visual_rows_before_line_and_intra_indented(&self, width: u16, line_idx: usize) -> u16 {
        let provider = self.editor.data_provider();
        let mut acc: u16 = 0;
        let indent = self.wrap_indent_cols;

        for i in 0..line_idx {
            let s = provider.field_value(i);
            acc = acc.saturating_add(count_wrapped_rows_indented(s, width, indent));
        }
        acc
    }

    #[cfg(feature = "gui")]
    pub fn cursor(&self, area: Rect, block: Option<&Block<'_>>) -> (u16, u16) {
        #[cfg(feature = "commandline")]
        {
            if let Some(commandline) = &self.commandline {
                if commandline.state.is_active() {
                    return commandline.state.cursor(area);
                }
            }
        }

        #[cfg(feature = "commandline")]
        let area = self.commandline_textarea_area(area);

        let inner = if let Some(b) = block {
            b.inner(area)
        } else {
            area
        };
        let inner = self.content_area(inner);
        let line_idx = self.current_field();

        match self.overflow_mode {
            TextOverflowMode::Wrap => {
                let width = inner.width;
                let y_top = inner.y;
                let indent = self.wrap_indent_cols;

                if width == 0 {
                    let prefix = self.visual_rows_before_line_and_intra_indented(1, line_idx);
                    let y = y_top.saturating_add(prefix.saturating_sub(self.scroll_y));
                    return (inner.x, y);
                }

                let prefix_rows = self.visual_rows_before_line_and_intra_indented(width, line_idx);
                let current_line = self.current_text();
                let col_chars = self.display_cursor_position();

                let (subrow, x_cols) =
                    wrapped_rows_to_cursor_indented(current_line, width, indent, col_chars);

                let caret_vis_row = prefix_rows.saturating_add(subrow);
                let y = y_top.saturating_add(caret_vis_row.saturating_sub(self.scroll_y));
                let x = inner.x.saturating_add(x_cols);
                (x, y)
            }
            TextOverflowMode::Indicator { .. } => {
                let y = inner.y + (line_idx as u16).saturating_sub(self.scroll_y);
                let current_line = self.current_text();
                let col = self.display_cursor_position();

                let mut x_cols: u16 = 0;
                let mut total_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    let w = UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                    if i < col {
                        x_cols = x_cols.saturating_add(w);
                    }
                    total_cols = total_cols.saturating_add(w);
                }

                let left_cols = if self.h_scroll > 0 { 1 } else { 0 };

                let mut x_off_visible = x_cols
                    .saturating_sub(self.h_scroll)
                    .saturating_add(left_cols);

                let limit = inner.width.saturating_sub(1 + RIGHT_PAD);

                if x_off_visible > limit {
                    x_off_visible = limit;
                }

                let x = inner.x.saturating_add(x_off_visible);
                (x, y)
            }
        }
    }

    #[cfg(feature = "gui")]
    pub(crate) fn ensure_visible(&mut self, area: Rect, block: Option<&Block<'_>>) {
        let inner = if let Some(b) = block {
            b.inner(area)
        } else {
            area
        };
        let inner = self.content_area(inner);
        if inner.height == 0 {
            return;
        }
        self.viewport_width = inner.width;
        self.viewport_height = inner.height;

        match self.overflow_mode {
            TextOverflowMode::Indicator { .. } => {
                let line_idx_u16 = self.current_field() as u16;
                if line_idx_u16 < self.scroll_y {
                    self.scroll_y = line_idx_u16;
                } else if line_idx_u16 >= self.scroll_y + inner.height {
                    self.scroll_y = line_idx_u16.saturating_sub(inner.height - 1);
                }

                let width = inner.width;
                if width == 0 {
                    return;
                }

                let current_line = self.current_text();
                let mut total_cols: u16 = 0;
                for ch in current_line.chars() {
                    total_cols =
                        total_cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }
                if total_cols <= width {
                    self.h_scroll = 0;
                    return;
                }

                let col = self.display_cursor_position();
                let mut cursor_cols: u16 = 0;
                for (i, ch) in current_line.chars().enumerate() {
                    if i >= col {
                        break;
                    }
                    cursor_cols =
                        cursor_cols.saturating_add(UnicodeWidthChar::width(ch).unwrap_or(0) as u16);
                }

                let (target_h, _left_cols) = compute_h_scroll_with_padding(cursor_cols, width);

                if target_h > self.h_scroll {
                    self.h_scroll = target_h;
                } else if cursor_cols < self.h_scroll {
                    self.h_scroll = cursor_cols;
                }
            }
            TextOverflowMode::Wrap => {
                let width = inner.width;
                if width == 0 {
                    self.h_scroll = 0;
                    return;
                }

                let indent = self.wrap_indent_cols;
                let line_idx = self.current_field();

                let prefix_rows = self.visual_rows_before_line_and_intra_indented(width, line_idx);

                let current_line = self.current_text();
                let col = self.display_cursor_position();

                let (subrow, _x_cols) =
                    wrapped_rows_to_cursor_indented(current_line, width, indent, col);

                let caret_vis_row = prefix_rows.saturating_add(subrow);

                let top = self.scroll_y;
                let height = inner.height;

                if caret_vis_row < top {
                    self.scroll_y = caret_vis_row;
                } else {
                    let bottom = top.saturating_add(height.saturating_sub(1));
                    if caret_vis_row > bottom {
                        let shift = caret_vis_row.saturating_sub(bottom);
                        self.scroll_y = top.saturating_add(shift);
                    }
                }

                self.h_scroll = 0;
            }
        }
    }

    #[cfg(feature = "gui")]
    pub(crate) fn move_visual_line_helix(&mut self, down: bool, count: usize) -> bool {
        if !matches!(self.overflow_mode, TextOverflowMode::Wrap) || self.viewport_width == 0 {
            let mut moved = false;
            for _ in 0..count.max(1) {
                moved |= if down {
                    self.move_down()
                } else {
                    self.move_up()
                };
            }
            return moved;
        }

        let mut moved = false;
        let target_col = self
            .editor
            .ui_state
            .ideal_cursor_column
            .min(u16::MAX as usize) as u16;
        for _ in 0..count.max(1) {
            moved |= self.move_visual_line_once(down, target_col);
        }
        self.editor.ui_state.ideal_cursor_column = target_col as usize;
        moved
    }

    #[cfg(feature = "gui")]
    fn move_visual_line_once(&mut self, down: bool, target_col: u16) -> bool {
        let width = self.viewport_width;
        let indent = self.wrap_indent_cols;
        let line_idx = self.current_field();
        let current_line = self.current_text().to_string();
        let cursor = self.display_cursor_position();
        let (subrow, _cursor_col) =
            wrapped_rows_to_cursor_indented(&current_line, width, indent, cursor);
        let current_ranges = wrap_segment_ranges(&current_line, width, indent);

        let target_line = if down {
            if (subrow as usize) + 1 < current_ranges.len() {
                Some(line_idx)
            } else if line_idx + 1 < self.editor.data_provider().field_count() {
                Some(line_idx + 1)
            } else {
                None
            }
        } else if subrow > 0 {
            Some(line_idx)
        } else if line_idx > 0 {
            Some(line_idx - 1)
        } else {
            None
        };

        let Some(target_line) = target_line else {
            return false;
        };

        let target_text = self.editor.data_provider().field_value(target_line).to_string();
        let target_ranges = wrap_segment_ranges(&target_text, width, indent);
        let target_subrow = if target_line == line_idx {
            if down {
                subrow as usize + 1
            } else {
                subrow.saturating_sub(1) as usize
            }
        } else if down {
            0
        } else {
            target_ranges.len().saturating_sub(1)
        };

        let (start, end) = target_ranges
            .get(target_subrow)
            .copied()
            .unwrap_or((0, target_text.chars().count()));
        let prefix_cols = if target_subrow == 0 {
            0
        } else {
            continuation_prefix_width(width, indent)
        };
        let target_pos =
            char_index_for_visual_col(&target_text, start, end, prefix_cols, target_col);

        if target_line != line_idx && self.editor.transition_to_field(target_line).is_err() {
            return false;
        }
        let char_len = self.current_text().chars().count();
        self.editor.set_cursor_for_mode(target_pos, char_len);
        self.editor.ui_state.ideal_cursor_column = target_col as usize;
        true
    }

    #[cfg(feature = "gui")]
    pub(crate) fn take_edited_flag(&mut self) -> bool {
        let v = self.edited_this_frame;
        self.edited_this_frame = false;
        v
    }
}

impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// Borrow the underlying [`FormEditor`] engine.
    ///
    /// `TextAreaState` is a thin wrapper around a `FormEditor`. Use this for
    /// engine functionality not re-exposed directly on the wrapper.
    pub fn editor(&self) -> &TextAreaEditor<P> {
        &self.editor
    }

    /// Mutably borrow the underlying [`FormEditor`] engine.
    pub fn editor_mut(&mut self) -> &mut TextAreaEditor<P> {
        &mut self.editor
    }

    /// Install a built-in keybinding preset and its editing paradigm.
    #[cfg(feature = "keybindings")]
    pub fn use_keybinding_preset(
        &mut self,
        preset: crate::keybindings::BuiltinCanvasKeybindingPreset,
    ) {
        self.editor.set_keybinding_preset(preset);
    }

    /// Move to the start of the next word.
    pub fn move_word_next(&mut self) {
        self.editor.move_word_next();
    }

    /// Move to the start of the previous word.
    pub fn move_word_prev(&mut self) {
        self.editor.move_word_prev();
    }

    /// Move to the end of the current/next word.
    pub fn move_word_end(&mut self) {
        self.editor.move_word_end();
    }

    /// Move to the end of the previous word.
    pub fn move_word_end_prev(&mut self) {
        self.editor.move_word_end_prev();
    }

    /// Move to the start of the next WORD (whitespace-delimited, vim `W`).
    pub fn move_big_word_next(&mut self) {
        self.editor.move_big_word_next();
    }

    /// Move to the start of the previous WORD (vim `B`).
    pub fn move_big_word_prev(&mut self) {
        self.editor.move_big_word_prev();
    }

    /// Move to the end of the current/next WORD (vim `E`).
    pub fn move_big_word_end(&mut self) {
        self.editor.move_big_word_end();
    }

    /// Move to the end of the previous WORD (vim `gE`).
    pub fn move_big_word_end_prev(&mut self) {
        self.editor.move_big_word_end_prev();
    }

    /// Enter edit mode with the cursor positioned for append (vim `a`).
    pub fn enter_append_mode(&mut self) {
        self.editor.enter_append_mode();
    }

    /// The current line's display text (mask/formatter-aware when the
    /// `validation` feature is enabled; otherwise the raw text).
    #[cfg(feature = "validation")]
    pub fn current_display_text(&self) -> String {
        self.editor.current_display_text()
    }

    /// The current line's text (raw; no validation feature for masking).
    #[cfg(not(feature = "validation"))]
    pub fn current_display_text(&self) -> String {
        self.editor.current_text().to_string()
    }

    /// Cursor position in display coordinates (accounts for a display mask).
    pub fn display_cursor_position(&self) -> usize {
        self.editor.display_cursor_position()
    }

    /// Update the terminal cursor style to match the textarea's current mode.
    ///
    /// Unlike the single-line input (which is always insert-style), the textarea
    /// honours the editor's mode: in vim mode this yields a steady block cursor
    /// in normal mode and a bar cursor in insert mode. With the
    /// `cursor-style` feature disabled this is a no-op.
    #[cfg(feature = "cursor-style")]
    pub fn update_cursor_style(&self) -> io::Result<()> {
        CursorManager::update_for_mode(self.editor.mode())
    }

    /// No-op when the `cursor-style` feature is disabled.
    #[cfg(not(feature = "cursor-style"))]
    pub fn update_cursor_style(&self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Validation helpers, re-exposed from the underlying [`FormEditor`] so they are
/// part of `TextAreaState`'s own public API rather than only reachable through
/// `Deref`.
#[cfg(feature = "validation")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn set_validation_enabled(&mut self, enabled: bool) {
        self.editor.set_validation_enabled(enabled);
    }

    pub fn is_validation_enabled(&self) -> bool {
        self.editor.is_validation_enabled()
    }

    pub fn set_field_validation(
        &mut self,
        field_index: usize,
        config: crate::validation::ValidationConfig,
    ) {
        self.editor.set_field_validation(field_index, config);
    }

    pub fn remove_field_validation(&mut self, field_index: usize) {
        self.editor.remove_field_validation(field_index);
    }

    pub fn validate_current_field(&mut self) -> crate::validation::ValidationResult {
        self.editor.validate_current_field()
    }

    pub fn validate_field(
        &mut self,
        field_index: usize,
    ) -> Option<crate::validation::ValidationResult> {
        self.editor.validate_field(field_index)
    }

    pub fn clear_validation_results(&mut self) {
        self.editor.clear_validation_results();
    }

    pub fn validation_summary(&self) -> crate::validation::ValidationSummary {
        self.editor.validation_summary()
    }

    pub fn can_switch_fields(&self) -> bool {
        self.editor.can_switch_fields()
    }

    pub fn field_switch_block_reason(&self) -> Option<String> {
        self.editor.field_switch_block_reason()
    }

    pub fn last_switch_block(&self) -> Option<&str> {
        self.editor.last_switch_block()
    }

    pub fn current_limits_status_text(&self) -> Option<String> {
        self.editor.current_limits_status_text()
    }

    pub fn current_formatter_warning(&self) -> Option<String> {
        self.editor.current_formatter_warning()
    }

    pub fn external_validation_of(
        &self,
        field_index: usize,
    ) -> crate::validation::ExternalValidationState {
        self.editor.external_validation_of(field_index)
    }

    pub fn clear_all_external_validation(&mut self) {
        self.editor.clear_all_external_validation();
    }

    pub fn clear_external_validation(&mut self, field_index: usize) {
        self.editor.clear_external_validation(field_index);
    }

    pub fn set_external_validation(
        &mut self,
        field_index: usize,
        state: crate::validation::ExternalValidationState,
    ) {
        self.editor.set_external_validation(field_index, state);
    }

    pub fn set_external_validation_callback<F>(&mut self, callback: F)
    where
        F: FnMut(usize, &str) -> crate::validation::ExternalValidationState + Send + Sync + 'static,
    {
        self.editor.set_external_validation_callback(callback);
    }
}

/// Computed-field helpers, re-exposed from the underlying [`FormEditor`].
#[cfg(feature = "computed")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn register_computed_provider<C>(&mut self, provider: &C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.register_computed_provider(provider);
    }

    pub fn set_computed_provider<C>(&mut self, provider: C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.set_computed_provider(provider);
    }

    pub fn recompute_fields<C>(&mut self, provider: &mut C, field_indices: &[usize])
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.recompute_fields(provider, field_indices);
    }

    pub fn recompute_all_fields<C>(&mut self, provider: &mut C)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.recompute_all_fields(provider);
    }

    pub fn on_field_changed<C>(&mut self, provider: &mut C, changed_field: usize)
    where
        C: crate::computed::ComputedProvider,
    {
        self.editor.on_field_changed(provider, changed_field);
    }

    pub fn effective_field_value(&self, field_index: usize) -> String {
        self.editor.effective_field_value(field_index)
    }
}

/// Undo/redo, re-exposed from the underlying [`FormEditor`].
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn undo(&mut self) -> bool {
        self.editor.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.editor.redo()
    }

    pub fn can_undo(&self) -> bool {
        self.editor.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.editor.can_redo()
    }

    pub fn clear_history(&mut self) {
        self.editor.clear_history();
    }

    pub fn set_history_limit(&mut self, limit: usize) {
        self.editor.set_history_limit(limit);
    }
}

/// Dropdown suggestions, re-exposed from the underlying [`FormEditor`] so that
/// `TextInput`, `TextArea`, and `FormEditor` all share one suggestions
/// mechanism. Render the dropdown with
/// `canvas::suggestions::render::render_suggestions_dropdown(.., self.editor())`.
#[cfg(feature = "suggestions")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn open_suggestions(&mut self, field_index: usize) {
        self.editor.open_suggestions(field_index);
    }

    pub fn trigger_suggestions(&mut self) -> Option<(usize, String)> {
        self.editor.trigger_suggestions()
    }

    pub fn apply_suggestions(&mut self, items: Vec<crate::SuggestionItem>) {
        self.editor.apply_suggestions(items);
    }

    pub fn update_suggestions(&mut self, items: Vec<crate::SuggestionItem>) {
        self.editor.update_suggestions(items);
    }

    pub fn dismiss_suggestions(&mut self) {
        self.editor.dismiss_suggestions();
    }

    pub fn cancel_suggestions(&mut self) {
        self.editor.cancel_suggestions();
    }

    pub fn suggestions_next(&mut self) {
        self.editor.suggestions_next();
    }

    pub fn suggestions_prev(&mut self) {
        self.editor.suggestions_prev();
    }

    pub fn apply_suggestion(&mut self) -> Option<String> {
        self.editor.apply_suggestion()
    }

    pub fn is_suggestions_active(&self) -> bool {
        self.editor.is_suggestions_active()
    }

    pub fn is_suggestions_loading(&self) -> bool {
        self.editor.ui_state().is_suggestions_loading()
    }

    pub fn dropdown_suggestions(&self) -> &[crate::SuggestionItem] {
        self.editor.suggestions()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "crossterm")]
    use super::TextAreaEventOutcome;
    use super::TextAreaState;
    use crate::textarea::provider::TextAreaProvider;

    #[test]
    fn paste_splits_lines() {
        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("ab");
        textarea.enter_edit_mode();
        textarea.set_cursor_position(2);

        textarea.paste("c\r\nd\nef");

        assert_eq!(textarea.text(), "abc\nd\nef");
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn input_reports_handled_and_ignored() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("");

        // A printable character is consumed.
        let out = textarea.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(out, TextAreaEventOutcome::Handled);
        assert_eq!(textarea.text(), "x");

        // Enter inserts a newline and is handled (never "submitted").
        let out = textarea.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(out, TextAreaEventOutcome::Handled);
        assert_eq!(textarea.text(), "x\n");

        // An unrecognized key is ignored so a host can react to it.
        let out = textarea.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(out, TextAreaEventOutcome::Ignored);
    }

    #[test]
    fn undo_textarea_newline_and_typing() {
        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("");
        textarea.enter_edit_mode();
        let _ = textarea.insert_text("ab"); // Insert run
        textarea.insert_newline(); // structural -> its own step
        let _ = textarea.insert_text("cd"); // new Insert run
        assert_eq!(textarea.text(), "ab\ncd");

        assert!(textarea.undo()); // undo "cd"
        assert_eq!(textarea.text(), "ab\n");
        assert!(textarea.undo()); // undo the newline
        assert_eq!(textarea.text(), "ab");
        assert!(textarea.undo()); // undo "ab"
        assert_eq!(textarea.text(), "");
        assert!(!textarea.undo());

        assert!(textarea.redo()); // redo "ab"
        assert_eq!(textarea.text(), "ab");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_open_line_below_inserts_textarea_line() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\n\ntwo");
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.cursor_position(), 0);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_open_line_above_inserts_textarea_line() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        let _ = textarea.move_down();

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\n\ntwo");
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.cursor_position(), 0);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_edit_enter_inserts_newline() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        textarea.enter_edit_mode();
        textarea.set_cursor_position(3);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\n");
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.cursor_position(), 0);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_edit_backspace_and_delete_join_textarea_lines() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        textarea.enter_edit_mode();
        let _ = textarea.move_down();

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "onetwo");
        assert_eq!(textarea.current_field(), 0);
        assert_eq!(textarea.cursor_position(), 3);

        textarea.set_text("one\ntwo");
        textarea.enter_edit_mode();
        textarea.set_cursor_position(3);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "onetwo");
        assert_eq!(textarea.current_field(), 0);
        assert_eq!(textarea.cursor_position(), 3);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_normal_x_and_x_delete_without_entering_edit_mode() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        let _ = textarea.move_right();

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ac");
        assert_eq!(textarea.cursor_position(), 1);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::SHIFT));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "c");
        assert_eq!(textarea.cursor_position(), 0);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_edit_tab_inserts_spaces_in_textarea() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("ab");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        textarea.enter_edit_mode();
        textarea.set_cursor_position(1);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "a    b");
        assert_eq!(textarea.cursor_position(), 5);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_keybinding_path_ignores_key_releases() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        textarea.enter_edit_mode();

        let out = textarea.handle_key_event(KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        });

        assert!(matches!(out, KeyEventOutcome::NotMatched));
        assert_eq!(textarea.text(), "");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_line_end_preserves_preferred_column_across_vertical_moves() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abcde\nxy\nabcdef");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('$'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 0);
        assert_eq!(textarea.cursor_position(), 4);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.cursor_position(), 1);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 2);
        assert_eq!(textarea.cursor_position(), 4);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_capital_i_and_a_enter_insert_at_line_edges() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abcd");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        textarea.set_cursor_position(2);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 0);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);

        let _ = textarea.exit_edit_mode();
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 4);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_d_and_c_change_to_line_end() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abcdef");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        textarea.set_cursor_position(2);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ab");
        assert_eq!(textarea.cursor_position(), 1);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);

        textarea.set_text("abcdef");
        textarea.set_cursor_position(2);
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ab");
        assert_eq!(textarea.cursor_position(), 2);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_dd_and_cc_delete_or_change_current_line() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo\nthree");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        let _ = textarea.move_down();

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\nthree");
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\n");
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.cursor_position(), 0);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_j_joins_line_below() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE));

        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "onetwo");
        assert_eq!(textarea.current_field(), 0);
        assert_eq!(textarea.cursor_position(), 3);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_counts_repeat_motion_and_delete_actions() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abcde\none\ntwo\nthree");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 3);

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "two\nthree");
        assert_eq!(textarea.current_field(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_ctrl_u_and_ctrl_d_move_half_page() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea =
            TextAreaState::<TextAreaProvider>::from_text("0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        textarea.viewport_height = 6;

        let out =
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 3);

        let out =
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 0);

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out =
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 6);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_yank_line_and_paste_after_or_before() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo\nthree");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());
        let _ = textarea.move_down();

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\ntwo\ntwo\nthree");
        assert_eq!(textarea.current_field(), 2);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\ntwo\ntwo\ntwo\nthree");
        assert_eq!(textarea.current_field(), 2);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_visual_y_yanks_selection_and_exits_highlight_mode() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo\nthree");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('V'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Sel);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);
        assert_eq!(textarea.text(), "one\ntwo\nthree");

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\ntwo\none\ntwo\nthree");
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_visual_d_and_c_act_on_selection() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::CanvasKeyBindings;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let n = KeyModifiers::NONE;
        let press = |t: &mut TextAreaState<TextAreaProvider>, c: char| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), n));
        };

        // Visual `d` deletes the (inclusive) selection and returns to normal.
        let mut t = TextAreaState::<TextAreaProvider>::from_text("hello");
        t.set_keybindings(CanvasKeyBindings::vim_defaults());
        press(&mut t, 'v'); // anchor at 0
        press(&mut t, 'l'); // head -> 1
        press(&mut t, 'l'); // head -> 2  (covers "hel")
        press(&mut t, 'd');
        assert_eq!(t.text(), "lo");
        assert_eq!(t.mode(), AppMode::Nor);

        // Visual `c` deletes the selection and drops into insert mode.
        let mut t = TextAreaState::<TextAreaProvider>::from_text("hello");
        t.set_keybindings(CanvasKeyBindings::vim_defaults());
        press(&mut t, 'v');
        press(&mut t, 'l');
        press(&mut t, 'l');
        press(&mut t, 'c');
        assert_eq!(t.text(), "lo");
        assert_eq!(t.mode(), AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_find_char_with_repeat_and_reverse() {
        use crate::keybindings::CanvasKeyBindings;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let n = KeyModifiers::NONE;
        let press = |t: &mut TextAreaState<TextAreaProvider>, c: char| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), n));
        };

        let mut t = TextAreaState::<TextAreaProvider>::from_text("hello world");
        t.set_keybindings(CanvasKeyBindings::vim_defaults());

        press(&mut t, 'f');
        press(&mut t, 'o'); // first 'o' at index 4
        assert_eq!(t.cursor_position(), 4);

        press(&mut t, ';'); // repeat -> 'o' at index 7
        assert_eq!(t.cursor_position(), 7);

        press(&mut t, ','); // reverse -> back to index 4
        assert_eq!(t.cursor_position(), 4);

        // `t` (till) stops one short of the target.
        let mut t = TextAreaState::<TextAreaProvider>::from_text("hello");
        t.set_keybindings(CanvasKeyBindings::vim_defaults());
        press(&mut t, 't');
        press(&mut t, 'l'); // first 'l' at index 2 -> till lands on index 1
        assert_eq!(t.cursor_position(), 1);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_replace_char_and_toggle_case() {
        use crate::keybindings::CanvasKeyBindings;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let n = KeyModifiers::NONE;
        let press = |t: &mut TextAreaState<TextAreaProvider>, c: char| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), n));
        };

        // `r` replaces the single character under the cursor.
        let mut t = TextAreaState::<TextAreaProvider>::from_text("hello");
        t.set_keybindings(CanvasKeyBindings::vim_defaults());
        press(&mut t, 'r');
        press(&mut t, 'x');
        assert_eq!(t.text(), "xello");
        assert_eq!(t.cursor_position(), 0);

        // `~` toggles case and advances the cursor.
        let mut t = TextAreaState::<TextAreaProvider>::from_text("abc");
        t.set_keybindings(CanvasKeyBindings::vim_defaults());
        press(&mut t, '~');
        assert_eq!(t.text(), "Abc");
        assert_eq!(t.cursor_position(), 1);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm", feature = "commandline"))]
    #[test]
    fn helix_active_search_match_invalidated_by_edit() {
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let n = KeyModifiers::NONE;
        let mut t = TextAreaState::<TextAreaProvider>::from_text("foo bar foo baz foo");
        t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        t.use_default_commandline();
        let k = |t: &mut TextAreaState<TextAreaProvider>, c: char, m: KeyModifiers| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), m));
        };

        k(&mut t, 'e', n); // select first "foo"
        k(&mut t, '*', n); // search "foo"
        k(&mut t, 'n', n); // active match = second "foo"
        assert!(t.active_search_match().is_some());

        // Deleting the matched word removes its highlight (no stale match
        // landing on the text that shifts into its place) and leaves the other
        // matches highlighted at their new positions.
        k(&mut t, 'd', n);
        assert_eq!(t.text(), "foo bar  baz foo");
        assert!(t.active_search_match().is_none());
        let starts: Vec<usize> = t.search_matches().iter().map(|m| m.start).collect();
        assert_eq!(starts, vec![0, 13]);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm", feature = "commandline"))]
    #[test]
    fn helix_star_then_n_and_shift_n_navigate() {
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let n = KeyModifiers::NONE;
        let mut t = TextAreaState::<TextAreaProvider>::from_text("foo bar foo baz foo");
        t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        t.use_default_commandline();
        let k = |t: &mut TextAreaState<TextAreaProvider>, c: char, m: KeyModifiers| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), m));
        };

        // Select the first "foo" (chars 0..2) and search for it.
        k(&mut t, 'e', n);
        k(&mut t, '*', n);
        assert_eq!(t.search_query(), Some("foo"));

        // `n` advances to the second match (chars 8..10).
        k(&mut t, 'n', n);
        assert_eq!(t.cursor_position(), 10);

        // `N` goes back to the first match — previously this stayed put because
        // navigation was cursor-relative instead of match-relative.
        k(&mut t, 'N', KeyModifiers::SHIFT);
        assert_eq!(t.cursor_position(), 2);

        // `N` again wraps around to the last match (chars 16..18).
        k(&mut t, 'N', KeyModifiers::SHIFT);
        assert_eq!(t.cursor_position(), 18);

        // The search (and its highlight) persists for navigation...
        assert_eq!(t.search_matches().len(), 3);
        // ...until Esc clears it.
        let _ = t.handle_key_event(KeyEvent::new(KeyCode::Esc, n));
        assert_eq!(t.search_query(), None);
        assert!(t.search_matches().is_empty());
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_wave4_surround_and_insert_deletes() {
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mk = |text: &str| {
            let mut t = TextAreaState::<TextAreaProvider>::from_text(text);
            t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
            t
        };
        let k = |t: &mut TextAreaState<TextAreaProvider>, c: char, m: KeyModifiers| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), m));
        };
        let n = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;
        let alt = KeyModifiers::ALT;

        // `ms(` wraps the selection in a pair.
        let mut t = mk("abc");
        k(&mut t, '%', n);
        k(&mut t, 'm', n);
        k(&mut t, 's', n);
        k(&mut t, '(', n);
        assert_eq!(t.text(), "(abc)");

        // `md(` removes the surrounding pair.
        let mut t = mk("(abc)");
        t.set_cursor_position(2);
        k(&mut t, 'm', n);
        k(&mut t, 'd', n);
        k(&mut t, '(', n);
        assert_eq!(t.text(), "abc");

        // `mr({` replaces the surrounding pair (two pending chars).
        let mut t = mk("(abc)");
        t.set_cursor_position(2);
        k(&mut t, 'm', n);
        k(&mut t, 'r', n);
        k(&mut t, '(', n);
        k(&mut t, '{', n);
        assert_eq!(t.text(), "{abc}");

        // Insert-mode deletes.
        let mut t = mk("foo bar");
        k(&mut t, 'A', n); // insert at end of line
        k(&mut t, 'w', ctrl);
        assert_eq!(t.text(), "foo ");

        let mut t = mk("foo bar");
        k(&mut t, 'A', n);
        k(&mut t, 'u', ctrl);
        assert_eq!(t.text(), "");

        let mut t = mk("foo bar");
        k(&mut t, 'i', n); // insert at start of line
        k(&mut t, 'd', alt);
        assert_eq!(t.text(), "bar");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_wave3_find_till_replace() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mk = |text: &str| {
            let mut t = TextAreaState::<TextAreaProvider>::from_text(text);
            t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
            t
        };
        let k = |t: &mut TextAreaState<TextAreaProvider>, c: char, m: KeyModifiers| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), m));
        };
        let n = KeyModifiers::NONE;
        let alt = KeyModifiers::ALT;

        // `f<char>` selects up to and including the next match.
        let mut t = mk("one two three");
        k(&mut t, 'f', n);
        k(&mut t, 'o', n);
        assert_eq!(t.cursor_position(), 6);
        assert!(matches!(
            t.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));

        // `t<char>` stops one short of the match.
        let mut t = mk("one two three");
        k(&mut t, 't', n);
        k(&mut t, 'o', n);
        assert_eq!(t.cursor_position(), 5);

        // `F<char>` searches backward.
        let mut t = mk("one two three");
        t.set_cursor_position(6);
        k(&mut t, 'F', n);
        k(&mut t, 'o', n);
        assert_eq!(t.cursor_position(), 0);

        // `Alt-.` repeats the last find.
        let mut t = mk("oxoxox");
        k(&mut t, 'f', n);
        k(&mut t, 'x', n);
        assert_eq!(t.cursor_position(), 1);
        k(&mut t, '.', alt);
        assert_eq!(t.cursor_position(), 3);

        // `r<char>` replaces every selected char; single-char and word.
        let mut t = mk("abcd");
        k(&mut t, 'w', n);
        k(&mut t, 'r', n);
        k(&mut t, 'x', n);
        assert_eq!(t.text(), "xxxx");

        let mut t = mk("abcd");
        k(&mut t, 'r', n);
        k(&mut t, 'Z', n);
        assert_eq!(t.text(), "Zbcd");

        // Esc cancels a pending command without editing.
        let mut t = mk("abcd");
        k(&mut t, 'r', n);
        let _ = t.handle_key_event(KeyEvent::new(KeyCode::Esc, n));
        assert_eq!(t.text(), "abcd");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_wave2_indent_and_number_ops() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mk = |text: &str| {
            let mut t = TextAreaState::<TextAreaProvider>::from_text(text);
            t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
            t
        };
        let k = |t: &mut TextAreaState<TextAreaProvider>, c: char, m: KeyModifiers| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), m));
        };
        let n = KeyModifiers::NONE;
        let ctrl = KeyModifiers::CONTROL;

        // `>` indents every selected line; `<` unindents back.
        let mut t = mk("ab\ncd");
        k(&mut t, '%', n);
        k(&mut t, '>', n);
        assert_eq!(t.text(), "    ab\n    cd");
        k(&mut t, '<', n);
        assert_eq!(t.text(), "ab\ncd");

        // `Ctrl-a` increments the number, selecting it.
        let mut t = mk("x 5 y");
        k(&mut t, 'a', ctrl);
        assert_eq!(t.text(), "x 6 y");
        assert_eq!(t.cursor_position(), 2);
        assert!(matches!(
            t.selection_state(),
            SelectionState::Characterwise { anchor: (0, 2) }
        ));

        // `Ctrl-x` decrements; negative numbers are handled.
        let mut t = mk("x 5 y");
        k(&mut t, 'x', ctrl);
        assert_eq!(t.text(), "x 4 y");

        let mut t = mk("v -3 w");
        k(&mut t, 'a', ctrl);
        assert_eq!(t.text(), "v -2 w");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm", feature = "commandline"))]
    #[test]
    fn helix_wave1_movement_and_selection_ops() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mk = |text: &str| {
            let mut t = TextAreaState::<TextAreaProvider>::from_text(text);
            t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
            t
        };
        let k = |t: &mut TextAreaState<TextAreaProvider>, c: char, m: KeyModifiers| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), m));
        };
        let n = KeyModifiers::NONE;
        let alt = KeyModifiers::ALT;

        // `mm` jumps between matching brackets.
        let mut t = mk("a(bc)d");
        t.set_cursor_position(1);
        k(&mut t, 'm', n);
        k(&mut t, 'm', n);
        assert_eq!(t.cursor_position(), 4);
        k(&mut t, 'm', n);
        k(&mut t, 'm', n);
        assert_eq!(t.cursor_position(), 1);

        // `*` sets the search pattern from the selection.
        let mut t = mk("bc x bc");
        k(&mut t, 'w', n);
        k(&mut t, '*', n);
        assert_eq!(t.search_query(), Some("bc "));

        // `Alt-:` makes a backward selection face forward.
        let mut t = mk("one two");
        t.set_cursor_position(4);
        k(&mut t, 'b', n);
        assert_eq!(t.cursor_position(), 0);
        k(&mut t, ':', alt);
        assert_eq!(t.cursor_position(), 3);
        assert!(matches!(
            t.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));

        // `J` joins the next line with a single space.
        let mut t = mk("ab\ncd");
        k(&mut t, 'J', n);
        assert_eq!(t.text(), "ab cd");

        // `G` jumps to the last line.
        let mut t = mk("l0\nl1\nl2");
        k(&mut t, 'G', n);
        assert_eq!(t.current_field(), 2);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_selection_and_case_ops() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mk = |text: &str| {
            let mut t = TextAreaState::<TextAreaProvider>::from_text(text);
            t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
            t
        };
        let k = |t: &mut TextAreaState<TextAreaProvider>, c: char, m: KeyModifiers| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), m));
        };
        let n = KeyModifiers::NONE;
        let alt = KeyModifiers::ALT;

        // `%` selects the whole document.
        let mut t = mk("ab\ncd");
        k(&mut t, '%', n);
        assert_eq!((t.current_field(), t.cursor_position()), (1, 1));
        assert!(matches!(
            t.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));

        // Case operations transform the selection in place.
        let mut t = mk("Hello");
        k(&mut t, '%', n);
        k(&mut t, '~', n);
        assert_eq!(t.text(), "hELLO");

        let mut t = mk("Hello");
        k(&mut t, '%', n);
        k(&mut t, '`', n);
        assert_eq!(t.text(), "hello");

        let mut t = mk("Hello");
        k(&mut t, '%', n);
        k(&mut t, '`', alt);
        assert_eq!(t.text(), "HELLO");

        // `_` trims leading/trailing whitespace from the selection.
        let mut t = mk("  hi");
        k(&mut t, '%', n);
        k(&mut t, '_', n);
        assert_eq!(t.cursor_position(), 3);
        assert!(matches!(
            t.selection_state(),
            SelectionState::Characterwise { anchor: (0, 2) }
        ));

        // `gs` jumps to the first non-whitespace char of the line.
        let mut t = mk("   abc");
        let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char('g'), n));
        let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char('s'), n));
        assert_eq!(t.cursor_position(), 3);

        // `Alt-;` flips anchor and head without changing the covered range.
        let mut t = mk("one two three");
        k(&mut t, 'w', n); // selects "one ", anchor (0,0), head 3
        k(&mut t, ';', alt);
        assert_eq!(t.cursor_position(), 0);
        assert!(matches!(
            t.selection_state(),
            SelectionState::Characterwise { anchor: (0, 3) }
        ));
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_w_crosses_line_boundary_cleanly() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let key = |t: &mut TextAreaState<TextAreaProvider>, c: char| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        };

        let mut t = TextAreaState::<TextAreaProvider>::from_text("ab cd\nef gh");
        t.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Within line 0: "ab " then "cd".
        key(&mut t, 'w');
        assert_eq!((t.current_field(), t.cursor_position()), (0, 2));
        key(&mut t, 'w');
        assert_eq!((t.current_field(), t.cursor_position()), (0, 4));

        // `w` at the end of line 0 crosses into line 1 and selects its first
        // word "ef " entirely on line 1 — no span back into line 0, and the
        // anchor is not the previous word's last char.
        key(&mut t, 'w');
        assert_eq!((t.current_field(), t.cursor_position()), (1, 2));
        assert!(matches!(
            t.selection_state(),
            SelectionState::Characterwise { anchor: (1, 0) }
        ));

        // Deleting that word removes exactly "ef " from line 1.
        key(&mut t, 'd');
        assert_eq!(t.text(), "ab cd\ngh");
        assert_eq!(t.current_field(), 1);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_counted_yank_and_paste_repeat_lines() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo\nthree\nfour");
        textarea.set_keybindings(CanvasKeyBindings::vim_defaults());

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        assert!(matches!(
            textarea.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE)),
            KeyEventOutcome::Pending
        ));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "one\none\ntwo\none\ntwo\ntwo\nthree\nfour");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_d_deletes_primary_selection_without_pending_operator() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        let _ = textarea.move_right();

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ac");
        assert_eq!(textarea.cursor_position(), 1);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_y_and_p_yank_and_paste_selection() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        let _ = textarea.move_right();

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "onne\ntwo");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_defaults_set_keybindings_installs_helix_paradigm() {
        use crate::keybindings::{CanvasKeyBindings, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.set_keybindings(CanvasKeyBindings::helix_defaults());

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ac");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_linewise_yank_pastes_as_lines() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo\nthree");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        assert_eq!(textarea.text(), "one\ntwo\none\nthree");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_goto_mode_motions_use_helix_keys() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo\nthree");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Pending));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 2);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Pending));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 0);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Pending));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 2);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_word_motions_replace_primary_selection() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one two three");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 3);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 6);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 3) }
        ));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 7);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 6) }
        ));

        textarea.set_cursor_position(8);
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        // `b` from the start of "three" selects "two " backwards, landing the
        // block cursor on the 't' of "two" (char 4) with the inclusive anchor
        // on the trailing space (char 7).
        assert_eq!(textarea.cursor_position(), 4);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 7) }
        ));

        // Repeated forward word motions must keep advancing (the previous patch
        // got stuck on the trailing space after the first `w`).
        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one two three four");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        let expected = [(3usize, 0usize), (7, 4), (13, 8), (17, 14)];
        for (cursor, anchor) in expected {
            let out =
                textarea.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
            assert!(matches!(out, KeyEventOutcome::Consumed(None)));
            assert_eq!(textarea.cursor_position(), cursor);
            assert!(matches!(
                textarea.selection_state(),
                SelectionState::Characterwise { anchor: (0, a) } if *a == anchor
            ));
        }
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_select_mode_word_motions_extend_with_pinned_anchor() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one two three four");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Enter select/extend mode; the anchor pins at the cursor.
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Sel);

        // The heads must match normal-mode word motions (3, 7, 12) exactly,
        // while the anchor stays pinned at (0, 0) and the mode stays Sel.
        for (k, head) in [('w', 3usize), ('w', 7), ('e', 12)] {
            let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char(k), KeyModifiers::NONE));
            assert!(matches!(out, KeyEventOutcome::Consumed(None)));
            assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Sel);
            assert_eq!(textarea.cursor_position(), head);
            assert!(matches!(
                textarea.selection_state(),
                SelectionState::Characterwise { anchor: (0, 0) }
            ));
        }

        // Extending backward moves only the head; the anchor is still pinned.
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.cursor_position(), 8);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm", feature = "commandline"))]
    #[test]
    fn helix_search_selects_match_and_n_advances() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc foo xyz foo end");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        textarea.use_default_commandline();

        let key = |t: &mut TextAreaState<TextAreaProvider>, c: char| {
            let _ = t.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        };

        // `/foo` then Enter: the first match after the cursor becomes the
        // primary selection (anchor on 'f', head on the last 'o').
        key(&mut textarea, '/');
        for c in "foo".chars() {
            key(&mut textarea, c);
        }
        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(textarea.cursor_position(), 6);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 4) }
        ));

        // `n` advances to the next match and selects it.
        key(&mut textarea, 'n');
        assert_eq!(textarea.cursor_position(), 14);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 12) }
        ));

        // `n` again wraps back to the first match.
        key(&mut textarea, 'n');
        assert_eq!(textarea.cursor_position(), 6);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 4) }
        ));
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_c_enters_insert_after_deleting_selection() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        let _ = textarea.move_right();

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ac");
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_U_redoes_change() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        let _ = textarea.move_right();

        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(textarea.text(), "ac");
        assert!(textarea.undo());
        assert_eq!(textarea.text(), "abc");

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('U'), KeyModifiers::SHIFT));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ac");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_x_extends_line_selection() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crate::canvas::state::SelectionState;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo\nthree\nfour");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Linewise { anchor_field: 0 }
        ));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 1);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Linewise { anchor_field: 0 }
        ));

        // Third and fourth presses must keep growing downward (anchor stays on
        // line 0) instead of collapsing onto the current line.
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 2);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Linewise { anchor_field: 0 }
        ));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.current_field(), 3);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Linewise { anchor_field: 0 }
        ));

        // `;` collapses the line selection back to a single-char cursor.
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char(';'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (3, _) }
        ));
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_visual_mode_y_yanks_and_returns_to_normal() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one\ntwo");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Sel);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "onone\ntwo");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn emacs_ctrl_space_sets_mark_and_esc_deactivates() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Emacs);

        let out = textarea.handle_key_event(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
        ));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Sel);
        assert!(matches!(
            textarea.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);
        assert!(matches!(textarea.selection_state(), SelectionState::None));
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn emacs_ctrl_w_kills_region() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Emacs);
        let _ = textarea.move_right();

        let _ = textarea.handle_key_event(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
        ));
        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ac");
        assert_eq!(textarea.cursor_position(), 1);
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn emacs_alt_w_copies_region_without_deleting() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Emacs);
        let _ = textarea.move_right();

        let _ = textarea.handle_key_event(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
        ));
        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::ALT));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "abc");
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Sel);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn emacs_ctrl_y_yanks_killed_text_in_insert_mode() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("abc");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Emacs);
        let _ = textarea.move_right();

        let _ = textarea.handle_key_event(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
        ));
        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(textarea.text(), "ac");

        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert_eq!(textarea.mode(), crate::canvas::modes::AppMode::Ins);

        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "abc");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn emacs_ctrl_y_preserves_newlines_in_insert_mode() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("ab\ncd");
        textarea.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Emacs);

        let _ = textarea.handle_key_event(KeyEvent::new(
            KeyCode::Char(' '),
            KeyModifiers::CONTROL,
        ));
        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "d");

        let _ = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        let out = textarea.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert!(matches!(out, KeyEventOutcome::Consumed(None)));
        assert_eq!(textarea.text(), "ab\ncd");
    }

    #[test]
    fn textarea_search_collects_matches_by_line_and_char_offsets() {
        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("alpha beta\nbeta alpha");
        textarea.set_search_query("alpha");

        assert_eq!(
            textarea.search_matches(),
            vec![
                super::TextAreaSearchMatch {
                    line: 0,
                    start: 0,
                    end: 5,
                },
                super::TextAreaSearchMatch {
                    line: 1,
                    start: 5,
                    end: 10,
                },
            ]
        );
    }

    #[test]
    fn textarea_find_next_and_previous_wrap() {
        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one two\nthree two\none");
        textarea.set_search_query("two");

        assert!(textarea.find_next());
        assert_eq!(textarea.current_field(), 0);
        assert_eq!(textarea.cursor_position(), 4);

        assert!(textarea.find_next());
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.cursor_position(), 6);

        assert!(textarea.find_next());
        assert_eq!(textarea.current_field(), 0);
        assert_eq!(textarea.cursor_position(), 4);

        assert!(textarea.find_previous());
        assert_eq!(textarea.current_field(), 1);
        assert_eq!(textarea.cursor_position(), 6);
    }

    #[test]
    fn textarea_clear_search_removes_query_and_active_match() {
        let mut textarea = TextAreaState::<TextAreaProvider>::from_text("one two");
        textarea.set_search_query("two");
        assert!(textarea.find_next());

        textarea.clear_search();

        assert_eq!(textarea.search_query(), None);
        assert_eq!(textarea.active_search_match(), None);
        assert!(textarea.search_matches().is_empty());
    }
}
