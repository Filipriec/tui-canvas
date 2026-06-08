#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
#[cfg(feature = "cursor-style")]
use std::io;

use std::ops::{Deref, DerefMut};

use crate::canvas::actions::{ActionResult, CanvasAction};
#[cfg(feature = "keybindings")]
use crate::canvas::modes::AppMode;
#[cfg(feature = "keybindings")]
use crate::canvas::state::SelectionState;
#[cfg(feature = "gui")]
use crate::gui_utils::{display_cols_up_to, display_width};
use crate::{editor::EditorCore, DataProvider};
#[cfg(feature = "cursor-style")]
use crate::CursorManager;
#[cfg(feature = "gui")]
use ratatui::{layout::Rect, widgets::Block};

#[cfg(feature = "keybindings")]
use crate::{
    editor::{
        behavior::{KeybindingParadigm, VimOperator, VimPendingOperator, YankRegister},
        paradigm::helix_word::HelixWordTarget,
        product::{handle_product_key_event, KeybindingProduct},
    },
    integration::focus_handoff::{key_outcome_for_vertical_navigation, BoundaryExit},
    keybindings::{CanvasKeyAction, CanvasKeyBindings, KeyEventOutcome},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormEventOutcome {
    Ignored,
    Handled,
    Submitted,
}

#[derive(Debug)]
pub struct TextFormState<D: DataProvider> {
    pub(crate) core: EditorCore<D>,
    fixed_field_count: usize,
}

impl<D: DataProvider + Default> Default for TextFormState<D> {
    fn default() -> Self {
        Self::new(D::default())
    }
}

impl<D: DataProvider> TextFormState<D> {
    pub fn new(data_provider: D) -> Self {
        let fixed_field_count = data_provider.field_count();
        Self {
            core: EditorCore::new(data_provider),
            fixed_field_count,
        }
    }

    pub fn with_provider(data_provider: D) -> Self {
        Self::new(data_provider)
    }

    pub fn core(&self) -> &EditorCore<D> {
        &self.core
    }

    pub fn fixed_field_count(&self) -> usize {
        self.fixed_field_count
    }

    fn assert_fixed_rows(&mut self) {
        let actual = self.core.data_provider().field_count();
        assert_eq!(
            actual, self.fixed_field_count,
            "TextFormState invariant violated: fixed field count changed"
        );

        if self.fixed_field_count == 0 {
            self.core.ui_state.current_field = 0;
            return;
        }

        if self.core.ui_state.current_field >= self.fixed_field_count {
            let target = self.fixed_field_count - 1;
            self.core.ui_state.current_field = target;
            let len = self.core.current_text().chars().count();
            let cursor = self.core.cursor_position().min(len);
            self.core.set_cursor_raw(cursor);
        }
    }

    fn with_fixed_rows<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.assert_fixed_rows();
        let result = f(self);
        self.assert_fixed_rows();
        result
    }

    #[cfg(feature = "crossterm")]
    pub fn handle_event(&mut self, event: Event) -> TextFormEventOutcome {
        self.with_fixed_rows(|this| match event {
            Event::Key(key) => this.input(key),
            Event::Paste(text) => this.paste(&text),
            _ => TextFormEventOutcome::Ignored,
        })
    }

    pub fn paste(&mut self, text: &str) -> TextFormEventOutcome {
        self.with_fixed_rows(|this| {
            let filtered: String = text
                .chars()
                .filter(|&ch| ch != '\n' && ch != '\r')
                .collect();

            if filtered.is_empty() {
                return TextFormEventOutcome::Ignored;
            }

            this.core.enter_edit_mode();
            let _ = this.core.insert_text(&filtered);
            TextFormEventOutcome::Handled
        })
    }

    #[cfg(feature = "crossterm")]
    pub fn input(&mut self, key: KeyEvent) -> TextFormEventOutcome {
        self.with_fixed_rows(|this| {
            if key.kind != KeyEventKind::Press {
                return TextFormEventOutcome::Ignored;
            }

            match (key.code, key.modifiers) {
                (KeyCode::Enter, _) => this.enter_next_field_or_submit(),
                (KeyCode::Tab, _) => {
                    let _ = this.core.next_field();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::BackTab, _) => {
                    let _ = this.core.prev_field();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Backspace, _) => {
                    let _ = this.core.delete_backward();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Delete, _) => {
                    let _ = this.core.delete_forward();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL) => {
                    this.core.move_word_prev();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL) => {
                    this.core.move_word_next();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Left, _) => {
                    let _ = this.core.move_left();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Right, _) => {
                    let _ = this.core.move_right();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Up, _) => {
                    let _ = this.core.move_up();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Down, _) => {
                    let _ = this.core.move_down();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Home, _) | (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    this.core.move_line_start();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::End, _) | (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                    this.core.move_line_end();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Char('b'), KeyModifiers::ALT) => {
                    this.core.move_word_prev();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Char('f'), KeyModifiers::ALT) => {
                    this.core.move_word_next();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Char('e'), KeyModifiers::ALT) => {
                    this.core.move_word_end();
                    TextFormEventOutcome::Handled
                }
                (KeyCode::Esc, _) => {
                    if this.core.mode() == crate::canvas::modes::AppMode::Ins {
                        let _ = this.core.exit_edit_mode();
                        TextFormEventOutcome::Handled
                    } else {
                        TextFormEventOutcome::Ignored
                    }
                }
                (KeyCode::Char(c), m)
                    if !m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT) =>
                {
                    this.core.enter_edit_mode();
                    let _ = this.core.insert_char(c);
                    TextFormEventOutcome::Handled
                }
                _ => TextFormEventOutcome::Ignored,
            }
        })
    }

    #[cfg(feature = "keybindings")]
    pub fn handle_key_event(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        self.with_fixed_rows(|this| handle_product_key_event(this, evt))
    }

    #[cfg(feature = "cursor-style")]
    pub fn update_cursor_style(&self) -> io::Result<()> {
        CursorManager::update_for_mode(self.core.mode())
    }

    #[cfg(not(feature = "cursor-style"))]
    pub fn update_cursor_style(&self) -> std::io::Result<()> {
        Ok(())
    }

    #[cfg(feature = "gui")]
    pub fn cursor(&self, area: Rect, block: Option<&Block<'_>>) -> (u16, u16) {
        let inner = if let Some(block) = block {
            block.inner(area)
        } else {
            area
        };
        let provider = self.core.data_provider();
        let label_width = (0..provider.field_count())
            .map(|index| display_width(provider.field_name(index)))
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let row = self.core.current_field() as u16;
        let current_text = self.core.current_text();
        let cursor_cols = display_cols_up_to(current_text, self.core.display_cursor_position());

        (
            inner.x.saturating_add(label_width).saturating_add(cursor_cols),
            inner.y.saturating_add(row),
        )
    }

    #[cfg(feature = "keybindings")]
    pub fn use_keybinding_preset(
        &mut self,
        preset: crate::keybindings::BuiltinCanvasKeybindingPreset,
    ) {
        self.core.set_keybinding_preset(preset);
    }

    #[cfg(feature = "keybindings")]
    pub fn set_keybindings(&mut self, keybindings: CanvasKeyBindings) {
        self.core.set_keybindings(keybindings);
    }

    pub fn clear_current_field(&mut self) {
        self.with_fixed_rows(|this| this.core.set_current_field_value(String::new()))
    }

    pub fn clear_field(&mut self, field_index: usize) {
        self.with_fixed_rows(|this| {
            if field_index < this.fixed_field_count {
                this.core.set_field_value(field_index, String::new());
            }
        })
    }

    pub fn clear_current_and_following_fields(&mut self, count: usize) {
        self.with_fixed_rows(|this| {
            if this.fixed_field_count == 0 {
                return;
            }

            let start = this.core.current_field();
            let end = start
                .saturating_add(count.max(1))
                .saturating_sub(1)
                .min(this.fixed_field_count - 1);
            this.clear_field_range(start, end);
        })
    }

    fn clear_field_range(&mut self, start: usize, end: usize) {
        if self.fixed_field_count == 0 {
            return;
        }

        let start = start.min(self.fixed_field_count - 1);
        let end = end.min(self.fixed_field_count - 1);
        if start > end {
            return;
        }

        for field_index in start..=end {
            self.core.set_field_value(field_index, String::new());
        }
    }

    #[cfg(feature = "keybindings")]
    fn extend_line_below_helix(&mut self, count: usize) {
        if self.fixed_field_count == 0 {
            return;
        }

        match self.core.selection_state().clone() {
            SelectionState::Linewise { .. } => {}
            SelectionState::Characterwise { anchor } => {
                self.core.ui_state.current_mode = AppMode::Sel;
                self.core.ui_state.selection = SelectionState::Linewise {
                    anchor_field: anchor.0,
                };
                return;
            }
            SelectionState::None => {
                let current = self.core.current_field();
                self.core.ui_state.current_mode = AppMode::Sel;
                self.core.ui_state.selection = SelectionState::Linewise {
                    anchor_field: current,
                };
                return;
            }
        }

        let target = self
            .core
            .current_field()
            .saturating_add(count.max(1))
            .min(self.fixed_field_count - 1);
        let _ = self.core.transition_to_field(target);
    }

    #[cfg(feature = "keybindings")]
    fn extend_to_line_bounds_helix(&mut self) {
        let current = self.core.current_field();
        self.core.ui_state.current_mode = AppMode::Sel;
        self.core.ui_state.selection = SelectionState::Linewise {
            anchor_field: current,
        };
    }

    #[cfg(feature = "keybindings")]
    fn collapse_selection_helix(&mut self) {
        self.core.ui_state.current_mode = AppMode::Nor;
        self.core.ui_state.selection = SelectionState::Characterwise {
            anchor: (self.core.current_field(), self.core.cursor_position()),
        };
    }

    #[cfg(feature = "keybindings")]
    fn field_char_len(&self, field_index: usize) -> usize {
        self.core
            .data_provider()
            .field_value(field_index)
            .chars()
            .count()
    }

    #[cfg(feature = "keybindings")]
    fn extract_characterwise_text(
        &self,
        start: (usize, usize),
        end: (usize, usize),
    ) -> Vec<String> {
        if start.0 == end.0 {
            let text: String = self
                .core
                .data_provider()
                .field_value(start.0)
                .chars()
                .skip(start.1)
                .take(end.1.saturating_sub(start.1) + 1)
                .collect();
            return vec![text];
        }

        let mut yanked = Vec::new();
        let first: String = self
            .core
            .data_provider()
            .field_value(start.0)
            .chars()
            .skip(start.1)
            .collect();
        yanked.push(first);
        for field_index in start.0 + 1..end.0 {
            yanked.push(
                self.core
                    .data_provider()
                    .field_value(field_index)
                    .to_string(),
            );
        }
        let last: String = self
            .core
            .data_provider()
            .field_value(end.0)
            .chars()
            .take(end.1 + 1)
            .collect();
        yanked.push(last);
        yanked
    }

    #[cfg(feature = "keybindings")]
    fn yank_selection_helix(&mut self) {
        match self.core.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
                let current = self.core.current_field();
                let start = anchor_field.min(current);
                let end = anchor_field.max(current).min(self.fixed_field_count.saturating_sub(1));
                if start > end || self.fixed_field_count == 0 {
                    return;
                }

                let lines: Vec<String> = (start..=end)
                    .map(|field_index| {
                        self.core
                            .data_provider()
                            .field_value(field_index)
                            .to_string()
                    })
                    .collect();
                self.core
                    .behavior_state
                    .yank_mut()
                    .set_line_register(lines);
            }
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.core.current_field(), self.core.cursor_position());
                let start = anchor.min(cursor);
                let end = anchor.max(cursor);
                if start.0 >= self.fixed_field_count || end.0 >= self.fixed_field_count {
                    return;
                }

                let yanked = self.extract_characterwise_text(start, end);
                if yanked.iter().all(|text| text.is_empty()) {
                    return;
                }
                self.core
                    .behavior_state
                    .yank_mut()
                    .set_text_register(yanked);
            }
            SelectionState::None => {}
        }
    }

    #[cfg(feature = "keybindings")]
    fn ensure_helix_primary_selection(&mut self) {
        self.core.ui_state.current_mode = AppMode::Nor;
        self.core.ui_state.selection = SelectionState::Characterwise {
            anchor: (self.core.current_field(), self.core.cursor_position()),
        };
    }

    #[cfg(feature = "keybindings")]
    fn delete_primary_character_helix(&mut self, yank: bool) -> bool {
        let field_index = self.core.current_field();
        if field_index >= self.fixed_field_count {
            return false;
        }

        let cursor = self.core.cursor_position();
        let current = self.core.data_provider().field_value(field_index).to_string();
        let line_len = current.chars().count();
        if cursor >= line_len {
            return false;
        }

        if yank {
            let ch: String = current.chars().skip(cursor).take(1).collect();
            self.core
                .behavior_state
                .yank_mut()
                .set_text_register(vec![ch]);
        }
        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Delete);
        let kept: String = current
            .chars()
            .enumerate()
            .filter_map(|(idx, ch)| if idx == cursor { None } else { Some(ch) })
            .collect();
        self.core
            .data_provider_mut()
            .set_field_value(field_index, kept);
        let len = self.field_char_len(field_index);
        self.core.ui_state.set_cursor(cursor.min(len), len, false);
        true
    }

    #[cfg(feature = "keybindings")]
    fn delete_characterwise_selection_helix(
        &mut self,
        anchor: (usize, usize),
        yank: bool,
    ) -> bool {
        let cursor = (self.core.current_field(), self.core.cursor_position());
        if anchor == cursor {
            return self.delete_primary_character_helix(yank);
        }

        let start = anchor.min(cursor);
        let end = anchor.max(cursor);
        if start.0 >= self.fixed_field_count || end.0 >= self.fixed_field_count {
            return false;
        }

        if yank {
            let yanked = self.extract_characterwise_text(start, end);
            self.core
                .behavior_state
                .yank_mut()
                .set_text_register(yanked);
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Delete);

        if start.0 == end.0 {
            let line = self.core.data_provider().field_value(start.0).to_string();
            let new_line: String = line
                .chars()
                .enumerate()
                .filter_map(|(idx, ch)| {
                    if idx < start.1 || idx > end.1 {
                        Some(ch)
                    } else {
                        None
                    }
                })
                .collect();
            self.core
                .data_provider_mut()
                .set_field_value(start.0, new_line);
        } else {
            let first: String = self
                .core
                .data_provider()
                .field_value(start.0)
                .chars()
                .take(start.1)
                .collect();
            self.core
                .data_provider_mut()
                .set_field_value(start.0, first);

            for field_index in start.0 + 1..end.0 {
                self.core
                    .data_provider_mut()
                    .set_field_value(field_index, String::new());
            }

            let last: String = self
                .core
                .data_provider()
                .field_value(end.0)
                .chars()
                .skip(end.1 + 1)
                .collect();
            self.core.data_provider_mut().set_field_value(end.0, last);
        }

        let _ = self.core.transition_to_field(start.0);
        let len = self.field_char_len(start.0);
        self.core.ui_state.set_cursor(start.1.min(len), len, false);
        true
    }

    #[cfg(feature = "keybindings")]
    fn delete_selection_once_helix(&mut self, yank: bool) -> bool {
        match self.core.selection_state().clone() {
            SelectionState::Linewise { anchor_field } => {
                let current = self.core.current_field();
                let start = anchor_field.min(current);
                let end = anchor_field.max(current).min(self.fixed_field_count.saturating_sub(1));
                if self.fixed_field_count == 0 || start > end {
                    return false;
                }

                if yank {
                    let lines: Vec<String> = (start..=end)
                        .map(|field_index| {
                            self.core
                                .data_provider()
                                .field_value(field_index)
                                .to_string()
                        })
                        .collect();
                    self.core
                        .behavior_state
                        .yank_mut()
                        .set_line_register(lines);
                }

                self.clear_field_range(start, end);
                let _ = self.core.transition_to_field(start);
                self.core.move_line_start();
                true
            }
            SelectionState::Characterwise { anchor } => {
                self.delete_characterwise_selection_helix(anchor, yank)
            }
            SelectionState::None => self.delete_primary_character_helix(yank),
        }
    }

    #[cfg(feature = "keybindings")]
    fn delete_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once_helix(yank) {
                break;
            }
        }
        if self.core.mode() == AppMode::Nor {
            self.ensure_helix_primary_selection();
        }
    }

    #[cfg(feature = "keybindings")]
    fn change_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.delete_selection_once_helix(yank) {
                break;
            }
        }
        self.core.enter_edit_mode();
    }

    #[cfg(feature = "keybindings")]
    fn character_paste_position_helix(&self, after: bool) -> (usize, usize) {
        match self.core.selection_state() {
            SelectionState::Characterwise { anchor } => {
                let cursor = (self.core.current_field(), self.core.cursor_position());
                let start = (*anchor).min(cursor);
                let end = (*anchor).max(cursor);
                if after {
                    (end.0, end.1.saturating_add(1))
                } else {
                    start
                }
            }
            _ => {
                let field = self.core.current_field();
                let len = self.field_char_len(field);
                let cursor = self.core.cursor_position().min(len);
                if after {
                    (field, cursor.saturating_add(1).min(len))
                } else {
                    (field, cursor)
                }
            }
        }
    }

    #[cfg(feature = "keybindings")]
    fn repeated_register_text_lines(lines: &[String], count: usize) -> Vec<String> {
        let repeat = count.max(1);
        let mut repeated = Vec::with_capacity(lines.len().saturating_mul(repeat));
        for _ in 0..repeat {
            repeated.extend(lines.iter().cloned());
        }
        repeated
    }

    #[cfg(feature = "keybindings")]
    fn paste_text_register_helix(&mut self, after: bool, count: usize, lines: Vec<String>) {
        let lines = Self::repeated_register_text_lines(&lines, count);
        if lines.is_empty() || self.fixed_field_count == 0 {
            return;
        }

        let (field, col) = self.character_paste_position_helix(after);
        if field >= self.fixed_field_count {
            return;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let col = col.min(self.field_char_len(field));
        let current = self.core.data_provider().field_value(field).to_string();
        let prefix: String = current.chars().take(col).collect();
        let suffix: String = current.chars().skip(col).collect();

        if lines.len() == 1 {
            let inserted = &lines[0];
            self.core
                .data_provider_mut()
                .set_field_value(field, format!("{prefix}{inserted}{suffix}"));
            let cursor = col.saturating_add(inserted.chars().count());
            let len = self.field_char_len(field);
            let _ = self.core.transition_to_field(field);
            self.core.ui_state.set_cursor(cursor.min(len), len, false);
        } else {
            self.core
                .data_provider_mut()
                .set_field_value(field, format!("{prefix}{}{suffix}", lines[0]));

            let mut target_field = field;
            let mut target_col = col.saturating_add(lines[0].chars().count());
            for (offset, text) in lines.iter().enumerate().skip(1) {
                let next_field = field.saturating_add(offset);
                if next_field >= self.fixed_field_count {
                    break;
                }
                self.core
                    .data_provider_mut()
                    .set_field_value(next_field, text.clone());
                target_field = next_field;
                target_col = text.chars().count();
            }

            let len = self.field_char_len(target_field);
            let _ = self.core.transition_to_field(target_field);
            self.core
                .ui_state
                .set_cursor(target_col.min(len), len, false);
        }

        self.ensure_helix_primary_selection();
    }

    #[cfg(feature = "keybindings")]
    fn paste_line_register_helix(&mut self, after: bool, count: usize, lines: Vec<String>) {
        let lines = Self::repeated_register_text_lines(&lines, count);
        if lines.is_empty() || self.fixed_field_count == 0 {
            return;
        }

        let current = self.core.current_field();
        let start = if after {
            current.saturating_add(1)
        } else {
            current
        };
        if start >= self.fixed_field_count {
            return;
        }

        self.core
            .record_checkpoint(crate::editor::features::history::EditKind::Other);

        let mut target = start;
        for (offset, line) in lines.into_iter().enumerate() {
            let field = start.saturating_add(offset);
            if field >= self.fixed_field_count {
                break;
            }
            self.core.data_provider_mut().set_field_value(field, line);
            target = field;
        }

        let _ = self.core.transition_to_field(target);
        self.core.move_line_start();
        self.ensure_helix_primary_selection();
    }

    #[cfg(feature = "keybindings")]
    fn paste_register_helix(&mut self, after: bool, count: usize) {
        let Some(register) = self.core.behavior_state.yank().register().cloned() else {
            return;
        };

        match register {
            YankRegister::Lines(lines) => self.paste_line_register_helix(after, count, lines),
            YankRegister::Text(lines) => self.paste_text_register_helix(after, count, lines),
        }
    }

    pub fn change_current_field(&mut self) {
        self.with_fixed_rows(|this| {
            this.core.set_current_field_value(String::new());
            this.core.enter_edit_mode();
        })
    }

    pub fn delete_to_field_end(&mut self) {
        self.with_fixed_rows(|this| {
            let cursor = this.core.cursor_position();
            let kept: String = this.core.current_text().chars().take(cursor).collect();
            this.core.set_current_field_value(kept);
            this.core.set_cursor_position(cursor);
        })
    }

    pub fn execute(&mut self, action: CanvasAction) -> ActionResult {
        self.with_fixed_rows(|this| match action {
            CanvasAction::OpenLineBelow => {
                let _ = this.core.open_line_below();
                ActionResult::Success
            }
            CanvasAction::OpenLineAbove => {
                let _ = this.core.open_line_above();
                ActionResult::Success
            }
            other => this.core.execute(other),
        })
    }

    pub fn undo(&mut self) -> bool {
        self.with_fixed_rows(|this| this.core.undo())
    }

    pub fn redo(&mut self) -> bool {
        self.with_fixed_rows(|this| this.core.redo())
    }

    #[cfg(feature = "crossterm")]
    fn enter_next_field_or_submit(&mut self) -> TextFormEventOutcome {
        let last = self.core.data_provider().field_count().saturating_sub(1);
        if self.core.current_field() >= last {
            TextFormEventOutcome::Submitted
        } else {
            let _ = self.core.next_field();
            TextFormEventOutcome::Handled
        }
    }

    #[cfg(feature = "keybindings")]
    fn move_next_field_count(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.core.next_field();
        }
    }

    #[cfg(feature = "keybindings")]
    fn move_prev_field_count(&mut self, count: usize) {
        for _ in 0..count {
            let _ = self.core.prev_field();
        }
    }

    #[cfg(feature = "keybindings")]
    fn execute_canvas_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        let Some(canvas_action) = action.to_canvas_action() else {
            return KeyEventOutcome::NotMatched;
        };

        let boundary = match action {
            CanvasKeyAction::MoveUp | CanvasKeyAction::PrevField => Some(BoundaryExit::Top),
            CanvasKeyAction::MoveDown | CanvasKeyAction::NextField => Some(BoundaryExit::Bottom),
            _ => None,
        };
        let before_field = self.core.current_field();

        let mut result = ActionResult::Success;
        for _ in 0..count {
            result = self.execute(canvas_action.clone());
        }

        if let Some(boundary) = boundary {
            let moved = self.core.current_field() != before_field;
            return key_outcome_for_vertical_navigation(moved, boundary);
        }

        match result {
            ActionResult::Success => KeyEventOutcome::Consumed(None),
            ActionResult::Message(msg) | ActionResult::Error(msg) => {
                KeyEventOutcome::Consumed(Some(msg))
            }
        }
    }

    #[cfg(feature = "keybindings")]
    fn begin_operator_vim(&mut self, operator: VimOperator, count: usize) {
        let anchor = (self.core.current_field(), self.core.cursor_position());
        self.core
            .behavior_state
            .vim_mut()
            .set_pending_operator(VimPendingOperator {
                operator,
                count: count.max(1),
                anchor,
            });
    }

    #[cfg(feature = "keybindings")]
    fn apply_operator_motion_vim(
        &mut self,
        action: &CanvasKeyAction,
        motion_count: usize,
    ) -> KeyEventOutcome {
        let Some(pending) = self.core.behavior_state.vim().pending_operator() else {
            return self.execute_canvas_key_action(action, motion_count);
        };

        self.core.behavior_state.vim_mut().clear_pending_operator();
        let total = pending
            .count
            .saturating_mul(motion_count.max(1))
            .max(1);

        if matches!(
            action,
            CanvasKeyAction::OperatorDelete
                | CanvasKeyAction::OperatorChange
                | CanvasKeyAction::OperatorYank
        ) {
            let start = pending.anchor.0;
            let end = start.saturating_add(total).saturating_sub(1);
            match pending.operator {
                VimOperator::Delete => self.clear_field_range(start, end),
                VimOperator::Change => {
                    self.clear_field_range(start, end);
                    self.core.enter_edit_mode();
                }
                VimOperator::Yank => {}
            }
            return KeyEventOutcome::Consumed(None);
        }

        let linewise_target = match action {
            CanvasKeyAction::MoveUp => Some(pending.anchor.0.saturating_sub(total)),
            CanvasKeyAction::MoveDown => Some(
                pending
                    .anchor
                    .0
                    .saturating_add(total)
                    .min(self.fixed_field_count.saturating_sub(1)),
            ),
            CanvasKeyAction::MoveFirstLine => Some(0),
            CanvasKeyAction::MoveLastLine => Some(self.fixed_field_count.saturating_sub(1)),
            _ => None,
        };

        if let Some(target) = linewise_target {
            let start = pending.anchor.0.min(target);
            let end = pending.anchor.0.max(target);
            match pending.operator {
                VimOperator::Delete => self.clear_field_range(start, end),
                VimOperator::Change => {
                    self.clear_field_range(start, end);
                    self.core.enter_edit_mode();
                }
                VimOperator::Yank => {}
            }
            return KeyEventOutcome::Consumed(None);
        }

        match pending.operator {
            VimOperator::Delete => {
                self.delete_to_field_end();
                KeyEventOutcome::Consumed(None)
            }
            VimOperator::Change => {
                self.delete_to_field_end();
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            VimOperator::Yank => KeyEventOutcome::Consumed(None),
        }
    }
}

impl<D: DataProvider> Deref for TextFormState<D> {
    type Target = EditorCore<D>;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl<D: DataProvider> DerefMut for TextFormState<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

#[cfg(feature = "keybindings")]
impl<D: DataProvider> KeybindingProduct for TextFormState<D> {
    type Provider = D;

    fn core(&self) -> &EditorCore<Self::Provider> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut EditorCore<Self::Provider> {
        &mut self.core
    }

    fn handle_insert_enter(&mut self) -> KeyEventOutcome {
        self.move_next_field_count(1);
        KeyEventOutcome::Consumed(None)
    }

    fn handle_insert_tab(&mut self) -> KeyEventOutcome {
        self.move_next_field_count(1);
        KeyEventOutcome::Consumed(None)
    }

    fn handle_plain_insert_char(&mut self, ch: char) -> KeyEventOutcome {
        self.core.enter_edit_mode();
        if self.core.insert_char(ch).is_ok() {
            KeyEventOutcome::Consumed(None)
        } else {
            KeyEventOutcome::NotMatched
        }
    }

    fn dispatch_product_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        if self.core.behavior_state.vim().has_pending_operator() {
            return self.apply_operator_motion_vim(action, count);
        }

        if self.core.keybinding_paradigm() == KeybindingParadigm::Helix {
            match action {
                CanvasKeyAction::MoveWordNext => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::NextWordStart);
                    return KeyEventOutcome::Consumed(None);
                }
                CanvasKeyAction::MoveWordPrev => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::PrevWordStart);
                    return KeyEventOutcome::Consumed(None);
                }
                CanvasKeyAction::MoveWordEnd => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::NextWordEnd);
                    return KeyEventOutcome::Consumed(None);
                }
                CanvasKeyAction::MoveWordEndPrev => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::PrevWordEnd);
                    return KeyEventOutcome::Consumed(None);
                }
                CanvasKeyAction::MoveBigWordNext => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::NextLongWordStart);
                    return KeyEventOutcome::Consumed(None);
                }
                CanvasKeyAction::MoveBigWordPrev => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::PrevLongWordStart);
                    return KeyEventOutcome::Consumed(None);
                }
                CanvasKeyAction::MoveBigWordEnd => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::NextLongWordEnd);
                    return KeyEventOutcome::Consumed(None);
                }
                CanvasKeyAction::MoveBigWordEndPrev => {
                    self.core
                        .select_word_motion_helix(count, HelixWordTarget::PrevLongWordEnd);
                    return KeyEventOutcome::Consumed(None);
                }
                _ => {}
            }
        }

        match action {
            CanvasKeyAction::OperatorDelete => {
                self.begin_operator_vim(VimOperator::Delete, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OperatorChange => {
                self.begin_operator_vim(VimOperator::Change, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OperatorYank => {
                self.begin_operator_vim(VimOperator::Yank, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::NextField => {
                self.move_next_field_count(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PrevField => {
                self.move_prev_field_count(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineBelow => {
                self.move_next_field_count(count);
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineAbove => {
                self.move_prev_field_count(count);
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterEditModeLineStart => {
                self.core.move_line_start();
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterEditModeLineEnd => {
                self.core.move_line_end();
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteLine => {
                self.clear_current_and_following_fields(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteToLineEnd => {
                self.delete_to_field_end();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeLine => {
                self.change_current_field();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeToLineEnd => {
                self.delete_to_field_end();
                self.core.enter_edit_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ExtendLineBelow => {
                self.extend_line_below_helix(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ExtendToLineBounds => {
                self.extend_to_line_bounds_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::CollapseSelection => {
                self.collapse_selection_helix();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteSelection | CanvasKeyAction::DeleteSelectionNoYank => {
                self.delete_selection_helix(
                    matches!(action, CanvasKeyAction::DeleteSelection),
                    count,
                );
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeSelection | CanvasKeyAction::ChangeSelectionNoYank => {
                self.change_selection_helix(
                    matches!(action, CanvasKeyAction::ChangeSelection),
                    count,
                );
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::YankSelection => {
                for _ in 0..count {
                    self.yank_selection_helix();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::JoinLineBelow
            | CanvasKeyAction::MoveLineUp
            | CanvasKeyAction::MoveLineDown
            | CanvasKeyAction::DuplicateLineUp
            | CanvasKeyAction::DuplicateLineDown
            | CanvasKeyAction::CutLine => KeyEventOutcome::Consumed(None),
            CanvasKeyAction::PasteAfter => {
                self.paste_register_helix(true, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteBefore => {
                self.paste_register_helix(false, count);
                KeyEventOutcome::Consumed(None)
            }
            _ => self.execute_canvas_key_action(action, count),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "crossterm")]
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::TextFormState;
    #[cfg(feature = "crossterm")]
    use super::TextFormEventOutcome;
    use crate::DataProvider;

    #[derive(Default)]
    struct TestProvider {
        fields: [String; 2],
    }

    impl DataProvider for TestProvider {
        fn field_count(&self) -> usize {
            2
        }

        fn field_name(&self, index: usize) -> &str {
            match index {
                0 => "first",
                1 => "second",
                _ => "",
            }
        }

        fn field_value(&self, index: usize) -> &str {
            self.fields.get(index).map(String::as_str).unwrap_or("")
        }

        fn set_field_value(&mut self, index: usize, value: String) {
            if let Some(field) = self.fields.get_mut(index) {
                *field = value;
            }
        }
    }

    #[derive(Default)]
    struct VecProvider {
        fields: Vec<String>,
    }

    impl DataProvider for VecProvider {
        fn field_count(&self) -> usize {
            self.fields.len()
        }

        fn field_name(&self, index: usize) -> &str {
            match index {
                0 => "first",
                1 => "second",
                _ => "",
            }
        }

        fn field_value(&self, index: usize) -> &str {
            self.fields.get(index).map(String::as_str).unwrap_or("")
        }

        fn set_field_value(&mut self, index: usize, value: String) {
            if let Some(field) = self.fields.get_mut(index) {
                *field = value;
            }
        }
    }

    #[cfg(feature = "crossterm")]
    #[test]
    fn enter_moves_between_fields_then_submits() {
        let mut form = TextFormState::new(TestProvider::default());

        let first = form.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(first, TextFormEventOutcome::Handled);
        assert_eq!(form.current_field(), 1);

        let second = form.input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(second, TextFormEventOutcome::Submitted);
        assert_eq!(form.current_field(), 1);
    }

    #[test]
    fn delete_line_clears_current_field_without_removing_it() {
        let mut form = TextFormState::new(TestProvider {
            fields: ["abc".to_string(), "def".to_string()],
        });
        form.clear_current_field();

        assert_eq!(form.data_provider().field_count(), 2);
        assert_eq!(form.current_text(), "");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn keybinding_enter_moves_to_next_field_without_splitting_rows() {
        use crate::keybindings::KeyEventOutcome;

        let mut form = TextFormState::new(TestProvider {
            fields: ["abc".to_string(), "def".to_string()],
        });
        form.enter_edit_mode();

        let outcome = form.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert_eq!(outcome, KeyEventOutcome::Consumed(None));
        assert_eq!(form.data_provider().field_count(), 2);
        assert_eq!(form.current_field(), 1);
        assert_eq!(form.data_provider().field_value(0), "abc");
        assert_eq!(form.current_text(), "def");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_dd_clears_current_field_without_shifting_following_fields() {
        let mut form = TextFormState::new(TestProvider {
            fields: ["row1".to_string(), "row2".to_string()],
        });
        form.set_keybindings(crate::keybindings::CanvasKeyBindings::vim_defaults());

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(form.fixed_field_count(), 2);
        assert_eq!(form.data_provider().field_count(), 2);
        assert_eq!(form.data_provider().field_value(0), "");
        assert_eq!(form.data_provider().field_value(1), "row2");
        assert_eq!(form.current_field(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_counted_dd_clears_fixed_slots_without_shifting_later_fields() {
        let mut form = TextFormState::new(VecProvider {
            fields: vec![
                "row1".to_string(),
                "row2".to_string(),
                "row3".to_string(),
            ],
        });
        form.set_keybindings(crate::keybindings::CanvasKeyBindings::vim_defaults());

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(form.fixed_field_count(), 3);
        assert_eq!(form.data_provider().capture_content(), vec!["", "", "row3"]);
        assert_eq!(form.current_field(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn vim_join_line_below_does_not_merge_or_remove_fixed_fields() {
        let mut form = TextFormState::new(TestProvider {
            fields: ["row1".to_string(), "row2".to_string()],
        });
        form.set_keybindings(crate::keybindings::CanvasKeyBindings::vim_defaults());

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT));

        assert_eq!(form.fixed_field_count(), 2);
        assert_eq!(form.data_provider().field_value(0), "row1");
        assert_eq!(form.data_provider().field_value(1), "row2");
        assert_eq!(form.current_field(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_x_then_d_clears_selected_fixed_slot_without_shifting_following_fields() {
        let mut form = TextFormState::new(TestProvider {
            fields: ["row1".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(crate::keybindings::BuiltinCanvasKeybindingPreset::Helix);

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(form.fixed_field_count(), 2);
        assert_eq!(form.data_provider().field_count(), 2);
        assert_eq!(form.data_provider().field_value(0), "");
        assert_eq!(form.data_provider().field_value(1), "row2");
        assert_eq!(form.current_field(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_extended_line_delete_clears_fixed_slots_without_shifting_later_fields() {
        let mut form = TextFormState::new(VecProvider {
            fields: vec![
                "row1".to_string(),
                "row2".to_string(),
                "row3".to_string(),
            ],
        });
        form.use_keybinding_preset(crate::keybindings::BuiltinCanvasKeybindingPreset::Helix);

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(form.fixed_field_count(), 3);
        assert_eq!(form.data_provider().capture_content(), vec!["", "", "row3"]);
        assert_eq!(form.current_field(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_word_motion_sets_characterwise_selection_for_highlight() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        let mut form = TextFormState::new(TestProvider {
            fields: ["one two three".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let outcome = form.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

        assert!(matches!(outcome, KeyEventOutcome::Consumed(None)));
        assert_eq!(form.cursor_position(), 3);
        assert!(matches!(
            form.selection_state(),
            SelectionState::Characterwise { anchor: (0, 0) }
        ));
        assert_eq!(form.data_provider().field_value(1), "row2");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_word_then_delete_removes_selection_without_clearing_field() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        let mut form = TextFormState::new(TestProvider {
            fields: ["one two three".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
        let outcome = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert!(matches!(outcome, KeyEventOutcome::Consumed(None)));
        assert_eq!(form.fixed_field_count(), 2);
        assert_eq!(form.data_provider().field_value(0), "two three");
        assert_eq!(form.data_provider().field_value(1), "row2");
        assert_eq!(form.current_field(), 0);
        assert_eq!(form.cursor_position(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_cross_field_character_delete_does_not_merge_fixed_fields() {
        use crate::canvas::state::SelectionState;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;

        let mut form = TextFormState::new(TestProvider {
            fields: ["abc".to_string(), "def".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
        let _ = form.transition_to_field(1);
        form.set_cursor_position(1);
        form.core.ui_state.selection = SelectionState::Characterwise { anchor: (0, 1) };

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(form.fixed_field_count(), 2);
        assert_eq!(form.data_provider().field_value(0), "a");
        assert_eq!(form.data_provider().field_value(1), "f");
        assert_eq!(form.current_field(), 0);
        assert_eq!(form.cursor_position(), 0);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_characterwise_yank_then_paste_inserts_inside_field() {
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        let mut form = TextFormState::new(TestProvider {
            fields: ["one two".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        let outcome = form.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

        assert!(matches!(outcome, KeyEventOutcome::Consumed(None)));
        assert_eq!(form.fixed_field_count(), 2);
        assert_eq!(form.data_provider().field_value(0), "one one two");
        assert_eq!(form.data_provider().field_value(1), "row2");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_linewise_yank_then_paste_writes_fixed_slots_without_shifting() {
        use crate::keybindings::BuiltinCanvasKeybindingPreset;

        let mut form = TextFormState::new(VecProvider {
            fields: vec![
                "row1".to_string(),
                "row2".to_string(),
                "row3".to_string(),
            ],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

        assert_eq!(form.fixed_field_count(), 3);
        assert_eq!(form.data_provider().capture_content(), vec!["row1", "row1", "row3"]);
    }

    #[test]
    #[should_panic(expected = "TextFormState invariant violated: fixed field count changed")]
    fn guard_rejects_field_count_changes_before_textform_mutation() {
        let mut form = TextFormState::new(VecProvider {
            fields: vec!["one".to_string(), "two".to_string()],
        });
        form.core.data_provider_mut().fields.pop();

        let _ = form.paste("x");
    }
}
