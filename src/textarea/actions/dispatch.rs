#[cfg(feature = "keybindings")]
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(all(feature = "keybindings", feature = "commandline"))]
use crate::commandline::CommandLineEventOutcome;

#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    editor::behavior::KeybindingParadigm,
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn handle_key_event(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        // A Helix command (`f`, `t`, `r`, …) waiting for a literal character
        // captures the next key before anything else routes it.
        if self.editor.keybinding_paradigm().is_helix() {
            if let Some(pending) = self.helix_pending {
                if evt.kind != KeyEventKind::Press {
                    return KeyEventOutcome::Consumed(None);
                }
                self.helix_pending = None;
                if let KeyCode::Char(ch) = evt.code {
                    if !evt.modifiers.contains(KeyModifiers::CONTROL)
                        && !evt.modifiers.contains(KeyModifiers::ALT)
                    {
                        self.resolve_helix_pending(pending, ch);
                    }
                }
                // Esc or any non-character key simply cancels.
                return KeyEventOutcome::Consumed(None);
            }
        }

        // Vim's `f`/`t`/`r` likewise capture the next literal character.
        if self.editor.keybinding_paradigm() == KeybindingParadigm::Vim {
            if let Some(pending) = self.vim_pending {
                if evt.kind != KeyEventKind::Press {
                    return KeyEventOutcome::Consumed(None);
                }
                self.vim_pending = None;
                let mut resolved = false;
                if let KeyCode::Char(ch) = evt.code {
                    if !evt.modifiers.contains(KeyModifiers::CONTROL)
                        && !evt.modifiers.contains(KeyModifiers::ALT)
                    {
                        self.resolve_vim_pending(pending, ch);
                        resolved = true;
                    }
                }

                // If the pending char was a find motion delimiting an operator
                // (`dfx`, `dtx`), apply the operator now that the cursor moved;
                // `f`/`t` are inclusive, `F`/`T` exclusive (inclusive == forward).
                if let Some(op) = self.editor.behavior_state.vim().pending_operator() {
                    self.editor.behavior_state.vim_mut().clear_pending_operator();
                    if resolved {
                        if let crate::textarea::actions::selection::vim::VimPending::Find {
                            forward,
                            ..
                        } = pending
                        {
                            self.finish_operator_charwise_vim(op.operator, op.anchor, forward);
                        }
                    }
                }
                // Esc or any non-character key simply cancels.
                return KeyEventOutcome::Consumed(None);
            }
        }

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

        if mode == AppMode::Ins && matches!(evt.code, KeyCode::Enter) {
            // VSCode: typing over a selection (here, a newline) replaces it.
            if self.editor.keybinding_paradigm() == KeybindingParadigm::Vscode
                && self.vscode_selection_active()
            {
                self.vscode_delete_selection();
            }
            self.insert_newline();
            return KeyEventOutcome::Consumed(None);
        }

        if mode != AppMode::Ins
            && self.editor.keybinding_paradigm() == KeybindingParadigm::Vim
        {
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
        // An unmatched key (e.g. Esc) cancels a half-typed operator.
        self.editor.behavior_state.vim_mut().clear_pending_operator();

        if mode == AppMode::Ins {
            match evt.code {
                KeyCode::Tab => {
                    self.insert_tab_spaces();
                    return KeyEventOutcome::Consumed(None);
                }
                KeyCode::Char(c) => {
                    let m = evt.modifiers;
                    let is_plain = m.is_empty() || m == KeyModifiers::SHIFT;
                    if is_plain {
                        // VSCode: typing over a selection replaces it.
                        if self.editor.keybinding_paradigm() == KeybindingParadigm::Vscode
                            && self.vscode_selection_active()
                        {
                            self.vscode_delete_selection();
                        }
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
        match self.editor.keybinding_paradigm() {
            KeybindingParadigm::Helix => self.dispatch_textarea_key_action_helix(action, count),
            KeybindingParadigm::Emacs => self.dispatch_textarea_key_action_emacs(action, count),
            KeybindingParadigm::Vscode => self.dispatch_textarea_key_action_vscode(action, count),
            KeybindingParadigm::Vim => self.dispatch_textarea_key_action_vim(action, count),
        }
    }
}
