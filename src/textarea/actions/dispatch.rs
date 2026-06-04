#[cfg(feature = "keybindings")]
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(all(feature = "keybindings", feature = "commandline"))]
use crate::commandline::CommandLineEventOutcome;

#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn handle_key_event(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        #[cfg(feature = "commandline")]
        {
            let should_route_commandline = self
                .commandline
                .as_ref()
                .map(|commandline| {
                    commandline.state().is_active()
                        || matches!(self.mode(), AppMode::Nor | AppMode::Sel)
                })
                .unwrap_or(false);

            if should_route_commandline {
                let outcome = self
                    .commandline
                    .as_mut()
                    .expect("checked commandline presence")
                    .state_mut()
                    .input_key(evt);

                match outcome {
                    CommandLineEventOutcome::Ignored => {}
                    CommandLineEventOutcome::Handled | CommandLineEventOutcome::Cancelled => {
                        return KeyEventOutcome::Consumed(None);
                    }
                    CommandLineEventOutcome::Submitted(submit) => {
                        self.apply_default_commandline_submit(submit);
                        return KeyEventOutcome::Consumed(None);
                    }
                }
            }
        }

        self.handle_key_event_inner(evt)
    }

    fn handle_key_event_inner(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        if evt.kind != KeyEventKind::Press {
            return KeyEventOutcome::NotMatched;
        }

        let mode = self.editor.ui_state.mode();

        if mode != AppMode::Ins {
            if let KeyCode::Char(ch) = evt.code {
                if let Some(digit) = ch.to_digit(10) {
                    let vim = self.editor.behavior_state.vim_mut();
                    if digit > 0 || vim.has_count() {
                        vim.push_count_digit(digit as usize);
                        return KeyEventOutcome::Pending;
                    }
                }
            }
        }

        let stroke = crate::keybindings::KeyStroke {
            code: evt.code,
            modifiers: evt.modifiers,
        };

        self.editor.seq_tracker.add_key(stroke);

        let Some(keybindings) = self.editor.keybindings.as_ref() else {
            return KeyEventOutcome::NotMatched;
        };
        let (matched, is_prefix) =
            keybindings.lookup_action(mode, self.editor.seq_tracker.sequence());

        if let Some(action) = matched.cloned() {
            let count = self.take_vim_count();
            self.editor.seq_tracker.reset();
            return self.dispatch_textarea_key_action(&action, count);
        }

        if is_prefix {
            return KeyEventOutcome::Pending;
        }

        self.editor.seq_tracker.reset();
        self.editor.behavior_state.vim_mut().reset_count();

        if mode == AppMode::Ins {
            match evt.code {
                KeyCode::Enter => {
                    self.insert_newline();
                    return KeyEventOutcome::Consumed(None);
                }
                KeyCode::Tab => {
                    self.insert_tab_spaces();
                    return KeyEventOutcome::Consumed(None);
                }
                KeyCode::Char(c) => {
                    let m = evt.modifiers;
                    let is_plain = m.is_empty() || m == KeyModifiers::SHIFT;
                    if is_plain {
                        self.enter_edit_mode();
                        #[cfg(feature = "gui")]
                        {
                            self.edited_this_frame = true;
                        }
                        if self.insert_char(c).is_ok() {
                            return KeyEventOutcome::Consumed(None);
                        }
                    }
                }
                _ => {}
            }
        }

        KeyEventOutcome::NotMatched
    }

    fn take_vim_count(&mut self) -> usize {
        self.editor.behavior_state.vim_mut().take_count_or_one()
    }

    fn dispatch_textarea_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        match action {
            CanvasKeyAction::NextField if self.mode() == AppMode::Ins => {
                self.insert_tab_spaces();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteCharBackward => {
                for _ in 0..count {
                    self.delete_backward_preserving_mode();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteCharForward => {
                for _ in 0..count {
                    self.delete_forward_preserving_mode();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineBelow => {
                for _ in 0..count {
                    self.open_line_below();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::OpenLineAbove => {
                for _ in 0..count {
                    self.open_line_above();
                }
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveHalfPageUp => {
                self.move_half_page_up(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::MoveHalfPageDown => {
                self.move_half_page_down(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterEditModeLineStart => {
                self.enter_line_start_insert_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::EnterEditModeLineEnd => {
                self.enter_line_end_insert_mode();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteLine => {
                self.delete_current_lines(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::DeleteToLineEnd => {
                self.delete_to_line_end();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeLine => {
                self.change_current_line();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::ChangeToLineEnd => {
                self.change_to_line_end();
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::JoinLineBelow => {
                self.join_lines_below(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::YankLine => {
                self.yank_current_lines(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteAfter => {
                self.paste_after(count);
                KeyEventOutcome::Consumed(None)
            }
            CanvasKeyAction::PasteBefore => {
                self.paste_before(count);
                KeyEventOutcome::Consumed(None)
            }
            _ => {
                let Some(canvas_action) = action.to_canvas_action() else {
                    return KeyEventOutcome::NotMatched;
                };
                let mut result = crate::canvas::actions::ActionResult::Success;
                for _ in 0..count {
                    result = self.editor.execute(canvas_action.clone());
                }
                match result {
                    crate::canvas::actions::ActionResult::Success => {
                        KeyEventOutcome::Consumed(None)
                    }
                    crate::canvas::actions::ActionResult::Message(msg)
                    | crate::canvas::actions::ActionResult::Error(msg) => {
                        KeyEventOutcome::Consumed(Some(msg))
                    }
                }
            }
        }
    }
}
