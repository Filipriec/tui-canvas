#[cfg(feature = "keybindings")]
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[cfg(feature = "keybindings")]
use crate::{
    DataProvider,
    canvas::modes::AppMode,
    editor::{EditorCore, behavior::KeybindingParadigm},
    keybindings::{CanvasKeyAction, KeyEventOutcome, KeyStroke},
};

#[cfg(feature = "keybindings")]
pub(crate) trait KeybindingProduct {
    type Provider: DataProvider;

    fn core(&self) -> &EditorCore<Self::Provider>;

    fn core_mut(&mut self) -> &mut EditorCore<Self::Provider>;

    fn handle_insert_enter(&mut self) -> KeyEventOutcome;

    fn handle_insert_tab(&mut self) -> KeyEventOutcome;

    fn handle_plain_insert_char(&mut self, ch: char) -> KeyEventOutcome;

    fn dispatch_product_key_action(
        &mut self,
        action: &CanvasKeyAction,
        count: usize,
    ) -> KeyEventOutcome;

    fn take_vim_count(&mut self) -> usize {
        self.core_mut().behavior_state.vim_mut().take_count_or_one()
    }

    fn clear_unmatched_pending(&mut self) {
        self.core_mut()
            .behavior_state
            .vim_mut()
            .clear_pending_operator();
    }
}

#[cfg(feature = "keybindings")]
pub(crate) fn handle_product_key_event<P: KeybindingProduct>(
    product: &mut P,
    evt: KeyEvent,
) -> KeyEventOutcome {
    if evt.kind != KeyEventKind::Press {
        return KeyEventOutcome::NotMatched;
    }

    let mode = product.core().ui_state.mode();

    if mode == AppMode::Ins && matches!(evt.code, KeyCode::Enter) {
        return product.handle_insert_enter();
    }

    if mode != AppMode::Ins && product.core().keybinding_paradigm() == KeybindingParadigm::Vim {
        if let KeyCode::Char(ch) = evt.code {
            if let Some(digit) = ch.to_digit(10) {
                let vim = product.core_mut().behavior_state.vim_mut();
                if digit > 0 || vim.has_count() {
                    vim.push_count_digit(digit as usize);
                    return KeyEventOutcome::Pending;
                }
            }
        }
    }

    let stroke = KeyStroke {
        code: evt.code,
        modifiers: evt.modifiers,
    };

    product.core_mut().seq_tracker.add_key(stroke);

    let (matched, is_prefix) = {
        let core = product.core();
        let Some(keybindings) = core.keybindings.as_ref() else {
            return KeyEventOutcome::NotMatched;
        };
        let (matched, is_prefix) = keybindings.lookup_action(mode, core.seq_tracker.sequence());
        (matched.cloned(), is_prefix)
    };

    if let Some(action) = matched {
        let count = product.take_vim_count();
        product.core_mut().seq_tracker.reset();
        return product.dispatch_product_key_action(&action, count);
    }

    if is_prefix {
        return KeyEventOutcome::Pending;
    }

    product.core_mut().seq_tracker.reset();
    product.core_mut().behavior_state.vim_mut().reset_count();
    product.clear_unmatched_pending();

    if mode == AppMode::Ins {
        match evt.code {
            KeyCode::Tab => {
                return product.handle_insert_tab();
            }
            KeyCode::Char(c) => {
                let m = evt.modifiers;
                let is_plain = m.is_empty() || m == KeyModifiers::SHIFT;
                if is_plain {
                    return product.handle_plain_insert_char(c);
                }
            }
            _ => {}
        }
    }

    KeyEventOutcome::NotMatched
}
