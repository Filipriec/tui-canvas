#[cfg(feature = "keybindings")]
mod emacs;
#[cfg(feature = "keybindings")]
mod helix;
#[cfg(feature = "keybindings")]
mod shared;
mod vim;

#[cfg(feature = "keybindings")]
use crate::editor::behavior::KeybindingParadigm;
#[cfg(feature = "keybindings")]
use crate::editor::EditorCore;
#[cfg(feature = "keybindings")]
use crate::DataProvider;

#[cfg(feature = "keybindings")]
impl<D: DataProvider> EditorCore<D> {
    pub(crate) fn apply_after_mode_change_for_paradigm(&mut self) {
        match self.behavior_state.paradigm() {
            KeybindingParadigm::Helix => self.apply_after_mode_change_helix(),
            KeybindingParadigm::Emacs | KeybindingParadigm::Vscode => {
                self.apply_after_mode_change_emacs()
            }
            KeybindingParadigm::Vim => self.apply_after_mode_change_vim(),
        }
    }
}
