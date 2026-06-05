use crate::textarea::{TextAreaDataProvider, TextAreaState};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// Emacs `M-w` — copy region to kill ring without deleting.
    pub(crate) fn copy_region_emacs(&mut self) {
        self.yank_selection();
    }

    /// Emacs `C-w` — kill region (yank then delete, deactivate mark).
    pub(crate) fn kill_region_emacs(&mut self) {
        self.yank_selection();
        let _ = self.delete_selection_once(false);
        self.exit_highlight_mode_emacs();
    }

    /// Delete active region without pushing to the kill ring.
    pub(crate) fn delete_region_emacs(&mut self) {
        let _ = self.delete_selection_once(false);
        self.exit_highlight_mode_emacs();
    }

    pub(crate) fn yank_primary_selection_emacs(&mut self) {
        self.copy_region_emacs();
    }
}
