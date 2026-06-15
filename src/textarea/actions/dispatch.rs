#[cfg(feature = "keybindings")]
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(all(feature = "keybindings", feature = "commandline"))]
use crate::commandline::CommandLineEventOutcome;

#[cfg(feature = "keybindings")]
use crate::{
    canvas::modes::AppMode,
    editor::{
        behavior::KeybindingParadigm,
        product::{KeybindingProduct, handle_product_key_event},
    },
    keybindings::{CanvasKeyAction, KeyEventOutcome},
    textarea::{TextAreaDataProvider, TextAreaState},
};

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub fn handle_key_event(&mut self, evt: KeyEvent) -> KeyEventOutcome {
        // A Helix command (`f`, `t`, `r`, …) waiting for a literal character
        // captures the next key before anything else routes it.
        if self.core.keybinding_paradigm().is_helix() {
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
        if self.core.keybinding_paradigm() == KeybindingParadigm::Vim {
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
                if let Some(op) = self.core.behavior_state.vim().pending_operator() {
                    self.core.behavior_state.vim_mut().clear_pending_operator();
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

        handle_product_key_event(self, evt)
    }

    /// Whether a multi-key command is mid-flight: the shared editor pending state
    /// (key sequence, count, operator) plus this widget's literal-char captures
    /// (vim/helix `f`/`t`/`r`/surround waiting for the next character).
    pub fn is_sequence_pending(&self) -> bool {
        self.core.is_sequence_pending()
            || self.vim_pending.is_some()
            || self.helix_pending.is_some()
    }

    fn dispatch_textarea_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome {
        match self.core.keybinding_paradigm() {
            KeybindingParadigm::Helix => self.dispatch_textarea_key_action_helix(action, count),
            KeybindingParadigm::Emacs => self.dispatch_textarea_key_action_emacs(action, count),
            KeybindingParadigm::Vscode => self.dispatch_textarea_key_action_vscode(action, count),
            KeybindingParadigm::Vim => self.dispatch_textarea_key_action_vim(action, count),
        }
    }
}

#[cfg(feature = "keybindings")]
impl<P: TextAreaDataProvider> KeybindingProduct for TextAreaState<P> {
    type Provider = P;

    fn core(&self) -> &crate::editor::EditorCore<Self::Provider> {
        &self.core
    }

    fn core_mut(&mut self) -> &mut crate::editor::EditorCore<Self::Provider> {
        &mut self.core
    }

    fn handle_insert_enter(&mut self) -> KeyEventOutcome {
        if self.core.keybinding_paradigm() == KeybindingParadigm::Vscode
            && self.vscode_selection_active()
        {
            self.vscode_delete_selection();
        }
        self.insert_newline();
        KeyEventOutcome::Consumed(None)
    }

    fn handle_insert_tab(&mut self) -> KeyEventOutcome {
        self.insert_tab_spaces();
        KeyEventOutcome::Consumed(None)
    }

    fn handle_plain_insert_char(&mut self, ch: char) -> KeyEventOutcome {
        if self.core.keybinding_paradigm() == KeybindingParadigm::Vscode
            && self.vscode_selection_active()
        {
            self.vscode_delete_selection();
        }
        self.enter_edit_mode();
        #[cfg(feature = "gui")]
        {
            self.edited_this_frame = true;
        }
        if self.insert_char(ch).is_ok() {
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
        self.dispatch_textarea_key_action(action, count)
    }
}
