#[cfg(feature = "crossterm")]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use std::ops::{Deref, DerefMut};

use crate::canvas::actions::{ActionResult, CanvasAction};
use crate::{editor::EditorCore, DataProvider};

#[cfg(feature = "keybindings")]
use crate::{
    editor::behavior::{VimOperator, VimPendingOperator},
    editor::product::{handle_product_key_event, KeybindingProduct},
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
            CanvasKeyAction::JoinLineBelow
            | CanvasKeyAction::MoveLineUp
            | CanvasKeyAction::MoveLineDown
            | CanvasKeyAction::DuplicateLineUp
            | CanvasKeyAction::DuplicateLineDown
            | CanvasKeyAction::CutLine
            | CanvasKeyAction::PasteAfter
            | CanvasKeyAction::PasteBefore => KeyEventOutcome::Consumed(None),
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
