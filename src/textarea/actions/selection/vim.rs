use crate::textarea::{TextAreaDataProvider, TextAreaState};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    pub(crate) fn yank_primary_selection_vim(&mut self) {
        self.yank_selection();
        self.exit_highlight_mode_vim();
    }
}
