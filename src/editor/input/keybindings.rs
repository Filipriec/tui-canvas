// src/editor/input/keybindings.rs
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[cfg(feature = "keybindings")]
use crate::canvas::actions::{ActionResult, CanvasAction};
use crate::canvas::modes::AppMode;
use crate::editor::FormEditor;
use crate::DataProvider;

#[cfg(feature = "keybindings")]
use crate::integration::focus_handoff::{key_outcome_for_vertical_navigation, BoundaryExit};
#[cfg(feature = "keybindings")]
use crate::keybindings::{CanvasKeyAction, KeyEventOutcome, KeyStroke};

impl<D: DataProvider> FormEditor<D> {
    #[cfg(feature = "keybindings")]
    pub fn handle_key_event(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        let mode = self.ui_state.current_mode;

        // Convert event to normalized stroke
        let stroke = KeyStroke {
            code: evt.code,
            modifiers: evt.modifiers,
        };

        // Add key to sequence tracker
        self.seq_tracker.add_key(stroke);

        // Look up the action in configured keybindings.
        let Some(keybindings) = self.keybindings.as_ref() else {
            return KeyEventOutcome::NotMatched;
        };
        let (matched, is_prefix) = keybindings.lookup_action(mode, self.seq_tracker.sequence());

        if let Some(action) = matched.cloned() {
            let outcome = self.dispatch_canvas_action(&action);
            self.seq_tracker.reset();
            return outcome;
        }

        if is_prefix {
            // Wait for more keys
            return KeyEventOutcome::Pending;
        }

        // No match: reset sequence and try insert-char fallback in Edit
        self.seq_tracker.reset();

        if mode == AppMode::Ins {
            if let KeyCode::Char(c) = evt.code {
                // Skip control/alt combos
                let m = evt.modifiers;
                let is_plain = m.is_empty() || m == KeyModifiers::SHIFT;
                if is_plain {
                    if self.insert_char(c).is_ok() {
                        return KeyEventOutcome::Consumed(None);
                    }
                }
            }
        }

        KeyEventOutcome::NotMatched
    }

    #[cfg(feature = "keybindings")]
    fn dispatch_canvas_action(&mut self, action: &CanvasKeyAction) -> KeyEventOutcome {
        let Some(canvas_action) = Self::canvas_action_for_key_action(action) else {
            return KeyEventOutcome::NotMatched;
        };

        let vertical_boundary = Self::vertical_boundary_for_key_action(action);
        let before_field = self.current_field();
        let result = self.execute(canvas_action);

        if let Some(boundary) = vertical_boundary {
            let moved = self.current_field() != before_field;
            return key_outcome_for_vertical_navigation(moved, boundary);
        }

        Self::key_outcome_for_action_result(result)
    }

    #[cfg(feature = "keybindings")]
    fn canvas_action_for_key_action(action: &CanvasKeyAction) -> Option<CanvasAction> {
        match action {
            CanvasKeyAction::MoveLeft => Some(CanvasAction::MoveLeft),
            CanvasKeyAction::MoveRight => Some(CanvasAction::MoveRight),
            CanvasKeyAction::MoveUp => Some(CanvasAction::MoveUp),
            CanvasKeyAction::MoveDown => Some(CanvasAction::MoveDown),
            CanvasKeyAction::NextField => Some(CanvasAction::NextField),
            CanvasKeyAction::PrevField => Some(CanvasAction::PrevField),
            CanvasKeyAction::MoveLineStart => Some(CanvasAction::MoveLineStart),
            CanvasKeyAction::MoveLineEnd => Some(CanvasAction::MoveLineEnd),
            CanvasKeyAction::MoveFirstLine => Some(CanvasAction::MoveFirstLine),
            CanvasKeyAction::MoveLastLine => Some(CanvasAction::MoveLastLine),
            CanvasKeyAction::MoveWordNext => Some(CanvasAction::MoveWordNext),
            CanvasKeyAction::MoveWordPrev => Some(CanvasAction::MoveWordPrev),
            CanvasKeyAction::MoveWordEnd => Some(CanvasAction::MoveWordEnd),
            CanvasKeyAction::MoveWordEndPrev => Some(CanvasAction::MoveWordEndPrev),
            CanvasKeyAction::MoveBigWordNext => Some(CanvasAction::MoveBigWordNext),
            CanvasKeyAction::MoveBigWordPrev => Some(CanvasAction::MoveBigWordPrev),
            CanvasKeyAction::MoveBigWordEnd => Some(CanvasAction::MoveBigWordEnd),
            CanvasKeyAction::MoveBigWordEndPrev => Some(CanvasAction::MoveBigWordEndPrev),
            CanvasKeyAction::DeleteCharBackward => Some(CanvasAction::DeleteBackward),
            CanvasKeyAction::DeleteCharForward => Some(CanvasAction::DeleteForward),
            CanvasKeyAction::Undo => Some(CanvasAction::Undo),
            CanvasKeyAction::Redo => Some(CanvasAction::Redo),
            CanvasKeyAction::OpenLineBelow => Some(CanvasAction::OpenLineBelow),
            CanvasKeyAction::OpenLineAbove => Some(CanvasAction::OpenLineAbove),
            CanvasKeyAction::EnterEditModeBefore => Some(CanvasAction::EnterEditMode),
            CanvasKeyAction::EnterEditModeAfter => Some(CanvasAction::EnterEditModeAfter),
            CanvasKeyAction::Exit | CanvasKeyAction::ExitEditMode => {
                Some(CanvasAction::ExitEditMode)
            }
            CanvasKeyAction::EnterHighlightMode => Some(CanvasAction::EnterHighlightMode),
            CanvasKeyAction::EnterHighlightModeLinewise => {
                Some(CanvasAction::EnterHighlightModeLinewise)
            }
            CanvasKeyAction::ExitHighlightMode => Some(CanvasAction::ExitHighlightMode),
            CanvasKeyAction::MoveHalfPageUp | CanvasKeyAction::MoveHalfPageDown => None,
            CanvasKeyAction::EnterEditModeLineStart
            | CanvasKeyAction::EnterEditModeLineEnd
            | CanvasKeyAction::DeleteLine
            | CanvasKeyAction::DeleteToLineEnd
            | CanvasKeyAction::ChangeLine
            | CanvasKeyAction::ChangeToLineEnd
            | CanvasKeyAction::JoinLineBelow
            | CanvasKeyAction::YankLine
            | CanvasKeyAction::PasteAfter
            | CanvasKeyAction::PasteBefore
            | CanvasKeyAction::DeleteSelection
            | CanvasKeyAction::DeleteSelectionNoYank
            | CanvasKeyAction::ChangeSelection
            | CanvasKeyAction::ChangeSelectionNoYank
            | CanvasKeyAction::YankSelection
            | CanvasKeyAction::CollapseSelection
            | CanvasKeyAction::ExtendLineBelow
            | CanvasKeyAction::ExtendToLineBounds
            | CanvasKeyAction::SearchNext
            | CanvasKeyAction::SearchPrev
            | CanvasKeyAction::SelectAll
            | CanvasKeyAction::FlipSelections
            | CanvasKeyAction::SwitchCase
            | CanvasKeyAction::SwitchToLowercase
            | CanvasKeyAction::SwitchToUppercase
            | CanvasKeyAction::TrimSelections
            | CanvasKeyAction::GotoFirstNonWhitespace
            | CanvasKeyAction::MovePageUp
            | CanvasKeyAction::MovePageDown
            | CanvasKeyAction::SearchSelection
            | CanvasKeyAction::EnsureSelectionForward
            | CanvasKeyAction::MatchBrackets
            | CanvasKeyAction::IndentSelection
            | CanvasKeyAction::UnindentSelection
            | CanvasKeyAction::IncrementNumber
            | CanvasKeyAction::DecrementNumber
            | CanvasKeyAction::FindNextChar
            | CanvasKeyAction::FindPrevChar
            | CanvasKeyAction::TillNextChar
            | CanvasKeyAction::TillPrevChar
            | CanvasKeyAction::ReplaceChar
            | CanvasKeyAction::RepeatLastFind
            | CanvasKeyAction::SurroundAdd
            | CanvasKeyAction::SurroundDelete
            | CanvasKeyAction::SurroundReplace
            | CanvasKeyAction::DeleteWordBackward
            | CanvasKeyAction::DeleteToLineStart
            | CanvasKeyAction::DeleteWordForward
            | CanvasKeyAction::ClearSearch => None,
            #[cfg(feature = "suggestions")]
            CanvasKeyAction::OpenSuggestions => Some(CanvasAction::TriggerSuggestions),
            #[cfg(feature = "suggestions")]
            CanvasKeyAction::ApplySuggestion | CanvasKeyAction::EnterDecider => {
                Some(CanvasAction::SelectSuggestion)
            }
            #[cfg(feature = "suggestions")]
            CanvasKeyAction::SuggestionDown => Some(CanvasAction::SuggestionDown),
            #[cfg(feature = "suggestions")]
            CanvasKeyAction::SuggestionUp => Some(CanvasAction::SuggestionUp),
            CanvasKeyAction::Unknown(_) => None,
            #[cfg(not(feature = "suggestions"))]
            CanvasKeyAction::OpenSuggestions
            | CanvasKeyAction::ApplySuggestion
            | CanvasKeyAction::EnterDecider
            | CanvasKeyAction::SuggestionDown
            | CanvasKeyAction::SuggestionUp => None,
        }
    }

    #[cfg(feature = "keybindings")]
    fn vertical_boundary_for_key_action(action: &CanvasKeyAction) -> Option<BoundaryExit> {
        match action {
            CanvasKeyAction::MoveUp | CanvasKeyAction::PrevField => Some(BoundaryExit::Top),
            CanvasKeyAction::MoveDown | CanvasKeyAction::NextField => Some(BoundaryExit::Bottom),
            _ => None,
        }
    }

    #[cfg(feature = "keybindings")]
    fn key_outcome_for_action_result(result: ActionResult) -> KeyEventOutcome {
        match result {
            ActionResult::Success => KeyEventOutcome::Consumed(None),
            ActionResult::Message(msg) | ActionResult::Error(msg) => {
                KeyEventOutcome::Consumed(Some(msg))
            }
        }
    }
}
