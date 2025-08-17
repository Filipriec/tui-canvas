// src/editor/core.rs

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;

use crate::canvas::modes::AppMode;
use crate::canvas::state::EditorState;
use crate::DataProvider;
#[cfg(feature = "suggestions")]
use crate::SuggestionItem;

pub struct FormEditor<D: DataProvider> {
    pub(crate) ui_state: EditorState,
    pub(crate) data_provider: D,
    #[cfg(feature = "suggestions")]
    pub(crate) suggestions: Vec<SuggestionItem>,

    #[cfg(feature = "validation")]
    pub(crate) external_validation_callback: Option<
        Box<
            dyn FnMut(usize, &str) -> crate::validation::ExternalValidationState
                + Send
                + Sync,
        >,
    >,
}

impl<D: DataProvider> FormEditor<D> {
    // Make helpers visible to sibling modules in this crate
    pub(crate) fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or_else(|| s.len())
    }

    #[allow(dead_code)]
    pub(crate) fn byte_to_char_index(s: &str, byte_idx: usize) -> usize {
        s[..byte_idx].chars().count()
    }

    pub fn new(data_provider: D) -> Self {
        let editor = Self {
            ui_state: EditorState::new(),
            data_provider,
            #[cfg(feature = "suggestions")]
            suggestions: Vec::new(),
            #[cfg(feature = "validation")]
            external_validation_callback: None,
        };

        #[cfg(feature = "validation")]
        {
            let mut editor = editor;
            editor.initialize_validation();

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(editor.ui_state.current_mode);
            }
            editor
        }
        #[cfg(not(feature = "validation"))]
        {
            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(editor.ui_state.current_mode);
            }
            editor
        }
    }

    // Library-internal, used by multiple modules
    pub(crate) fn current_text(&self) -> &str {
        let field_index = self.ui_state.current_field;
        if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        }
    }

    // Read-only getters
    pub fn current_field(&self) -> usize {
        self.ui_state.current_field()
    }
    pub fn cursor_position(&self) -> usize {
        self.ui_state.cursor_position()
    }
    pub fn mode(&self) -> AppMode {
        self.ui_state.mode()
    }
    #[cfg(feature = "suggestions")]
    pub fn is_suggestions_active(&self) -> bool {
        self.ui_state.is_suggestions_active()
    }
    pub fn ui_state(&self) -> &EditorState {
        &self.ui_state
    }
    pub fn data_provider(&self) -> &D {
        &self.data_provider
    }
    pub fn data_provider_mut(&mut self) -> &mut D {
        &mut self.data_provider
    }
    #[cfg(feature = "suggestions")]
    pub fn suggestions(&self) -> &[SuggestionItem] {
        &self.suggestions
    }

    #[cfg(feature = "validation")]
    pub fn validation_state(&self) -> &crate::validation::ValidationState {
        self.ui_state.validation_state()
    }

    // Cursor cleanup
    #[cfg(feature = "cursor-style")]
    pub fn cleanup_cursor(&self) -> std::io::Result<()> {
        CursorManager::reset()
    }
    #[cfg(not(feature = "cursor-style"))]
    pub fn cleanup_cursor(&self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<D: DataProvider> Drop for FormEditor<D> {
    fn drop(&mut self) {
        let _ = self.cleanup_cursor();
    }
}
