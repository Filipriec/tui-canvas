#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
#[cfg(feature = "cursor-style")]
use std::io;

use std::ops::{Deref, DerefMut};

use crate::canvas::actions::{ActionResult, CanvasAction};
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

#[cfg(feature = "keybindings")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextFormActionPolicy {
    SharedCore,
    ProductHandled,
    StructuralNoOp,
    Unsupported,
}

#[cfg(feature = "keybindings")]
fn textform_action_policy(action: &CanvasKeyAction) -> TextFormActionPolicy {
    use CanvasKeyAction::*;

    match action {
        MoveLeft
        | MoveRight
        | MoveUp
        | MoveDown
        | MoveLineStart
        | MoveLineEnd
        | MoveHalfPageUp
        | MoveHalfPageDown
        | MoveFirstLine
        | MoveLastLine
        | MoveWordNext
        | MoveWordPrev
        | MoveWordEnd
        | MoveWordEndPrev
        | MoveBigWordNext
        | MoveBigWordPrev
        | MoveBigWordEnd
        | MoveBigWordEndPrev
        | DeleteCharBackward
        | DeleteCharForward
        | Undo
        | Redo
        | OpenSuggestions
        | ApplySuggestion
        | SuggestionDown
        | SuggestionUp
        | EnterEditModeBefore
        | EnterEditModeAfter
        | ExitEditMode
        | EnterHighlightMode
        | EnterHighlightModeLinewise
        | ExitHighlightMode => TextFormActionPolicy::SharedCore,

        NextField
        | PrevField
        | OpenLineBelow
        | OpenLineAbove
        | EnterEditModeLineStart
        | EnterEditModeLineEnd
        | DeleteLine
        | DeleteToLineEnd
        | ChangeLine
        | ChangeToLineEnd
        | OperatorDelete
        | OperatorChange
        | OperatorYank
        | YankLine
        | CopyLine
        | CutLine
        | PasteAfter
        | PasteBefore
        | DeleteSelection
        | DeleteSelectionNoYank
        | ChangeSelection
        | ChangeSelectionNoYank
        | YankSelection
        | CollapseSelection
        | ExtendLineBelow
        | ExtendToLineBounds => TextFormActionPolicy::ProductHandled,

        JoinLineBelow
        | MoveLineUp
        | MoveLineDown
        | DuplicateLineUp
        | DuplicateLineDown => TextFormActionPolicy::StructuralNoOp,

        EnterDecider
        | Exit
        | SearchNext
        | SearchPrev
        | SelectAll
        | FlipSelections
        | SwitchCase
        | SwitchToLowercase
        | SwitchToUppercase
        | TrimSelections
        | GotoFirstNonWhitespace
        | MovePageUp
        | MovePageDown
        | SearchSelection
        | EnsureSelectionForward
        | MatchBrackets
        | IndentSelection
        | UnindentSelection
        | IncrementNumber
        | DecrementNumber
        | FindNextChar
        | FindPrevChar
        | TillNextChar
        | TillPrevChar
        | ReplaceChar
        | RepeatLastFind
        | RepeatLastFindReverse
        | SurroundAdd
        | SurroundDelete
        | SurroundReplace
        | DeleteWordBackward
        | DeleteToLineStart
        | DeleteWordForward
        | ClearSearch
        | SelectLeft
        | SelectRight
        | SelectUp
        | SelectDown
        | SelectWordPrev
        | SelectWordNext
        | SelectLineStart
        | SelectLineEnd
        | SelectDocStart
        | SelectDocEnd
        | Unknown(_) => TextFormActionPolicy::Unsupported,
    }
}

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
            // A form is a fixed-row buffer: its dispatch calls the `fixed`
            // row operations (clear/overwrite slots) rather than the `dynamic`
            // ones a text area uses.
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

    #[cfg(feature = "keybindings")]
    fn selected_fixed_field_values(&self, start: usize, end: usize) -> Vec<String> {
        if self.fixed_field_count == 0 || start > end {
            return Vec::new();
        }

        let end = end.min(self.fixed_field_count - 1);
        (start..=end)
            .map(|field_index| {
                self.core
                    .data_provider()
                    .field_value(field_index)
                    .to_string()
            })
            .collect()
    }

    #[cfg(feature = "keybindings")]
    fn yank_current_and_following_fields(&mut self, count: usize) {
        if self.fixed_field_count == 0 {
            return;
        }

        let start = self.core.current_field();
        let end = start
            .saturating_add(count.max(1))
            .saturating_sub(1)
            .min(self.fixed_field_count - 1);
        let lines = self.selected_fixed_field_values(start, end);
        if !lines.is_empty() {
            self.core
                .behavior_state
                .yank_mut()
                .set_line_register(lines);
        }
    }

    #[cfg(feature = "keybindings")]
    fn cut_current_and_following_fields(&mut self, count: usize) {
        self.yank_current_and_following_fields(count);
        self.clear_current_and_following_fields(count);
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
    fn field_char_len(&self, field_index: usize) -> usize {
        self.core
            .data_provider()
            .field_value(field_index)
            .chars()
            .count()
    }

    #[cfg(feature = "keybindings")]
    fn delete_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.core.delete_selection_once_fixed(yank) {
                break;
            }
        }
        self.core.finish_helix_selection_edit();
    }

    #[cfg(feature = "keybindings")]
    fn change_selection_helix(&mut self, yank: bool, count: usize) {
        for _ in 0..count.max(1) {
            if !self.core.delete_selection_once_fixed(yank) {
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
    fn paste_register_helix(&mut self, after: bool, count: usize) {
        let Some(register) = self.core.behavior_state.yank().register().cloned() else {
            return;
        };

        match register {
            YankRegister::Lines(lines) => {
                self.core.paste_lines_fixed(after, count, lines);
                self.core.ensure_helix_primary_selection();
            }
            YankRegister::Text(lines) => {
                let text = crate::editor::rows::repeated_text(&lines, count);
                if text.is_empty() || self.fixed_field_count == 0 {
                    return;
                }
                let (field, col) = self.character_paste_position_helix(after);
                if field >= self.fixed_field_count {
                    return;
                }
                let (target_field, target_col) =
                    self.core.insert_text_fixed(field, col, &text);
                let _ = self.core.transition_to_field(target_field);
                self.core.set_cursor_position(target_col);
                self.core.ensure_helix_primary_selection();
            }
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
                VimOperator::Yank => {
                    let lines = self.selected_fixed_field_values(start, end);
                    if !lines.is_empty() {
                        self.core
                            .behavior_state
                            .yank_mut()
                            .set_line_register(lines);
                    }
                }
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
                VimOperator::Yank => {
                    let lines = self.selected_fixed_field_values(start, end);
                    if !lines.is_empty() {
                        self.core
                            .behavior_state
                            .yank_mut()
                            .set_line_register(lines);
                    }
                }
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
            VimOperator::Yank => {
                let field = self.core.current_field();
                let line = self.core.data_provider().field_value(field).to_string();
                self.core
                    .behavior_state
                    .yank_mut()
                    .set_text_register(vec![line]);
                KeyEventOutcome::Consumed(None)
            }
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
        let policy = textform_action_policy(action);

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
            CanvasKeyAction::YankLine | CanvasKeyAction::CopyLine => {
                self.yank_current_and_following_fields(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::CutLine => {
                self.cut_current_and_following_fields(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ExtendLineBelow => {
                for _ in 0..count {
                    self.core.extend_line_below_helix();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ExtendToLineBounds => {
                for _ in 0..count {
                    self.core.extend_to_line_bounds_helix();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::CollapseSelection => {
                self.core.collapse_selection_helix();
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
                    self.core.yank_primary_selection_helix();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::JoinLineBelow
            | CanvasKeyAction::MoveLineUp
            | CanvasKeyAction::MoveLineDown
            | CanvasKeyAction::DuplicateLineUp
            | CanvasKeyAction::DuplicateLineDown => KeyEventOutcome::Consumed(None),
            CanvasKeyAction::PasteAfter => {
                self.paste_register_helix(true, count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteBefore => {
                self.paste_register_helix(false, count);
                KeyEventOutcome::Consumed(None)
            }
            _ => match policy {
                TextFormActionPolicy::SharedCore => self.execute_canvas_key_action(action, count),
                TextFormActionPolicy::StructuralNoOp => KeyEventOutcome::Consumed(None),
                TextFormActionPolicy::Unsupported => KeyEventOutcome::NotMatched,
                TextFormActionPolicy::ProductHandled => KeyEventOutcome::Consumed(Some(format!(
                    "Unhandled textform action: {}",
                    action.as_str()
                ))),
            },
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

    #[cfg(feature = "keybindings")]
    #[derive(Clone)]
    struct FixedPolicyProvider {
        names: [&'static str; 3],
        fields: [String; 3],
    }

    #[cfg(feature = "keybindings")]
    impl FixedPolicyProvider {
        fn new() -> Self {
            Self {
                names: ["first", "second", "third"],
                fields: ["alpha beta".into(), "gamma delta".into(), "epsilon".into()],
            }
        }

        fn names(&self) -> Vec<String> {
            self.names.iter().map(|name| (*name).to_string()).collect()
        }
    }

    #[cfg(feature = "keybindings")]
    impl DataProvider for FixedPolicyProvider {
        fn field_count(&self) -> usize {
            self.fields.len()
        }

        fn field_name(&self, index: usize) -> &str {
            self.names.get(index).copied().unwrap_or("")
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

    #[cfg(feature = "keybindings")]
    fn all_known_key_actions() -> Vec<crate::keybindings::CanvasKeyAction> {
        use crate::keybindings::CanvasKeyAction::*;

        vec![
            MoveLeft,
            MoveRight,
            MoveUp,
            MoveDown,
            NextField,
            PrevField,
            MoveLineStart,
            MoveLineEnd,
            MoveHalfPageUp,
            MoveHalfPageDown,
            MoveFirstLine,
            MoveLastLine,
            MoveWordNext,
            MoveWordPrev,
            MoveWordEnd,
            MoveWordEndPrev,
            MoveBigWordNext,
            MoveBigWordPrev,
            MoveBigWordEnd,
            MoveBigWordEndPrev,
            DeleteCharBackward,
            DeleteCharForward,
            Undo,
            Redo,
            OpenLineBelow,
            OpenLineAbove,
            EnterEditModeLineStart,
            EnterEditModeLineEnd,
            DeleteLine,
            DeleteToLineEnd,
            ChangeLine,
            ChangeToLineEnd,
            OperatorDelete,
            OperatorChange,
            OperatorYank,
            JoinLineBelow,
            YankLine,
            PasteAfter,
            PasteBefore,
            OpenSuggestions,
            ApplySuggestion,
            EnterDecider,
            SuggestionDown,
            SuggestionUp,
            EnterEditModeBefore,
            EnterEditModeAfter,
            Exit,
            ExitEditMode,
            EnterHighlightMode,
            EnterHighlightModeLinewise,
            ExitHighlightMode,
            DeleteSelection,
            DeleteSelectionNoYank,
            ChangeSelection,
            ChangeSelectionNoYank,
            YankSelection,
            CollapseSelection,
            ExtendLineBelow,
            ExtendToLineBounds,
            SearchNext,
            SearchPrev,
            SelectAll,
            FlipSelections,
            SwitchCase,
            SwitchToLowercase,
            SwitchToUppercase,
            TrimSelections,
            GotoFirstNonWhitespace,
            MovePageUp,
            MovePageDown,
            SearchSelection,
            EnsureSelectionForward,
            MatchBrackets,
            IndentSelection,
            UnindentSelection,
            IncrementNumber,
            DecrementNumber,
            FindNextChar,
            FindPrevChar,
            TillNextChar,
            TillPrevChar,
            ReplaceChar,
            RepeatLastFind,
            RepeatLastFindReverse,
            SurroundAdd,
            SurroundDelete,
            SurroundReplace,
            DeleteWordBackward,
            DeleteToLineStart,
            DeleteWordForward,
            ClearSearch,
            MoveLineUp,
            MoveLineDown,
            DuplicateLineUp,
            DuplicateLineDown,
            CopyLine,
            CutLine,
            SelectLeft,
            SelectRight,
            SelectUp,
            SelectDown,
            SelectWordPrev,
            SelectWordNext,
            SelectLineStart,
            SelectLineEnd,
            SelectDocStart,
            SelectDocEnd,
        ]
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

    #[cfg(feature = "keybindings")]
    #[test]
    fn every_known_key_action_preserves_textform_fixed_slots() {
        use crate::editor::product::KeybindingProduct;
        use crate::keybindings::{BuiltinCanvasKeybindingPreset, KeyEventOutcome};

        for action in all_known_key_actions() {
            let provider = FixedPolicyProvider::new();
            let expected_names = provider.names();
            let mut form = TextFormState::new(provider);
            form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);
            form.core
                .behavior_state
                .yank_mut()
                .set_text_register(vec!["paste".to_string()]);

            let outcome = form.dispatch_product_key_action(&action, 1);
            assert!(
                !matches!(
                    outcome,
                    KeyEventOutcome::Consumed(Some(ref msg))
                        if msg.starts_with("Unhandled textform action:")
                ),
                "action is classified as product-handled but not implemented: {}",
                action.as_str()
            );

            assert_eq!(
                form.data_provider().field_count(),
                3,
                "action changed field count: {}",
                action.as_str()
            );
            assert_eq!(
                form.fixed_field_count(),
                3,
                "action changed fixed count: {}",
                action.as_str()
            );
            let names: Vec<String> = (0..form.data_provider().field_count())
                .map(|index| form.data_provider().field_name(index).to_string())
                .collect();
            assert_eq!(
                names,
                expected_names,
                "action changed field identity/order: {}",
                action.as_str()
            );
        }
    }

    #[cfg(feature = "keybindings")]
    #[test]
    fn structural_noop_actions_leave_fixed_field_values_unchanged() {
        use crate::editor::product::KeybindingProduct;
        use crate::keybindings::CanvasKeyAction;

        let actions = [
            CanvasKeyAction::JoinLineBelow,
            CanvasKeyAction::MoveLineUp,
            CanvasKeyAction::MoveLineDown,
            CanvasKeyAction::DuplicateLineUp,
            CanvasKeyAction::DuplicateLineDown,
        ];

        for action in actions {
            let provider = FixedPolicyProvider::new();
            let expected = provider.capture_content();
            let mut form = TextFormState::new(provider);

            let _ = form.dispatch_product_key_action(&action, 1);

            assert_eq!(
                form.data_provider().capture_content(),
                expected,
                "structural no-op changed fixed field values: {}",
                action.as_str()
            );
        }
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

    #[cfg(feature = "keybindings")]
    #[test]
    fn cut_line_clears_fixed_slot_without_shifting_later_fields() {
        use crate::editor::product::KeybindingProduct;
        use crate::keybindings::CanvasKeyAction;

        let mut form = TextFormState::new(VecProvider {
            fields: vec![
                "row1".to_string(),
                "row2".to_string(),
                "row3".to_string(),
            ],
        });
        let _ = form.transition_to_field(1);

        let _ = form.dispatch_product_key_action(&CanvasKeyAction::CutLine, 1);

        assert_eq!(form.fixed_field_count(), 3);
        assert_eq!(form.data_provider().capture_content(), vec!["row1", "", "row3"]);
        assert_eq!(form.current_field(), 1);
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

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_extend_line_stays_in_normal_mode_like_textarea() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;

        let mut form = TextFormState::new(TestProvider {
            fields: ["row1".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Helix `x` selects the whole line but stays in normal mode (there is no
        // separate visual mode for it) — exactly like the text area.
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(form.mode(), AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_delete_after_extend_returns_to_normal_mode() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;

        let mut form = TextFormState::new(TestProvider {
            fields: ["row1".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // `x` then `d`: the slot clears and we end in normal mode (no stuck SEL).
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(form.mode(), AppMode::Nor);
        assert_eq!(form.data_provider().field_value(0), "");
        assert_eq!(form.data_provider().field_value(1), "row2");
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_highlight_yank_returns_to_normal_mode() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;

        let mut form = TextFormState::new(TestProvider {
            fields: ["row1".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // `v` enters the genuine highlight (select) mode; `y` yanks and must
        // leave it, matching the text area.
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        assert_eq!(form.mode(), AppMode::Sel);

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert_eq!(form.mode(), AppMode::Nor);
    }

    #[cfg(all(feature = "keybindings", feature = "crossterm"))]
    #[test]
    fn helix_append_on_last_character_inserts_after_it() {
        use crate::canvas::modes::AppMode;
        use crate::keybindings::BuiltinCanvasKeybindingPreset;

        let mut form = TextFormState::new(TestProvider {
            fields: ["abc".to_string(), "row2".to_string()],
        });
        form.use_keybinding_preset(BuiltinCanvasKeybindingPreset::Helix);

        // Selection on the last character of the field; `a` must position the
        // cursor *after* it, not clamp back onto it.
        form.move_line_end();
        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(form.mode(), AppMode::Ins);
        assert_eq!(form.cursor_position(), 3);

        let _ = form.handle_key_event(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
        assert_eq!(form.current_text(), "abcX");
    }

    #[cfg(feature = "keybindings")]
    #[test]
    fn multiline_text_register_paste_preserves_suffix_in_fixed_slot() {
        use crate::editor::product::KeybindingProduct;
        use crate::keybindings::CanvasKeyAction;

        let mut form = TextFormState::new(VecProvider {
            fields: vec![
                "aaZZ".to_string(),
                "row2".to_string(),
                "row3".to_string(),
            ],
        });
        form.set_cursor_position(2);
        form.core
            .behavior_state
            .yank_mut()
            .set_text_register(vec!["X".to_string(), "Y".to_string()]);

        let _ = form.dispatch_product_key_action(&CanvasKeyAction::PasteBefore, 1);

        assert_eq!(form.fixed_field_count(), 3);
        assert_eq!(form.data_provider().capture_content(), vec!["aaX", "YZZ", "row3"]);
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
