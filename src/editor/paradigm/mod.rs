mod helix;
mod shared;
mod vim;

use crate::editor::behavior::KeybindingParadigm;
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    pub(crate) fn apply_after_mode_change_for_paradigm(&mut self) {
        match self.behavior_state.paradigm() {
            KeybindingParadigm::Helix => self.apply_after_mode_change_helix(),
            KeybindingParadigm::Vim | KeybindingParadigm::Emacs => {
                self.apply_after_mode_change_vim()
            }
        }
    }
}
