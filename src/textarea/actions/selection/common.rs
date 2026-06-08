use crate::textarea::{TextAreaDataProvider, TextAreaState};

impl<P: TextAreaDataProvider> TextAreaState<P> {
    /// Delete the active selection once. The structural behavior (remove rows vs
    /// clear in place) lives in the shared engine on `EditorCore`; the text area
    /// uses the provider's `Dynamic` policy. This wrapper only forwards the call
    /// and tracks the GUI redraw flag.
    pub(crate) fn delete_selection_once(&mut self, yank: bool) -> bool {
        let changed = self.core.delete_selection_once_core(yank);
        #[cfg(feature = "gui")]
        if changed {
            self.edited_this_frame = true;
        }
        changed
    }
}
