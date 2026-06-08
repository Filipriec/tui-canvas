use crate::textarea::{TextAreaDataProvider, TextAreaState};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    #[cfg(feature = "keybindings")]
    pub(crate) fn yank_selection(&mut self) {
        self.core.yank_selection_core();
    }
}
