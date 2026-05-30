// src/editor/core.rs
#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;

use crate::canvas::modes::AppMode;
use crate::canvas::state::EditorState;
use crate::DataProvider;
#[cfg(feature = "suggestions")]
use crate::SuggestionItem;
use derivative::Derivative;

#[cfg(feature = "keymap")]
use crate::keymap::{CanvasKeyMap, KeySequenceTracker};

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct FormEditor<D: DataProvider> {
    pub(crate) ui_state: EditorState,
    pub(crate) data_provider: D,
    #[cfg(feature = "suggestions")]
    pub(crate) suggestions: Vec<SuggestionItem>,

    #[cfg(feature = "validation")]
    #[derivative(Debug = "ignore")]
    #[derivative(Default(value = "None"))]
    pub(crate) external_validation_callback: Option<
        Box<dyn FnMut(usize, &str) -> crate::validation::ExternalValidationState + Send + Sync>,
    >,
    #[cfg(feature = "keymap")]
    #[derivative(Default(value = "None"))]
    pub(crate) keymap: Option<CanvasKeyMap>,

    #[cfg(feature = "keymap")]
    #[derivative(Default(value = "KeySequenceTracker::new(400)"))]
    pub(crate) seq_tracker: KeySequenceTracker,

    pub(crate) undo_stack: Vec<crate::editor::history::EditSnapshot>,
    pub(crate) redo_stack: Vec<crate::editor::history::EditSnapshot>,
    #[derivative(Default(value = "crate::editor::history::DEFAULT_HISTORY_LIMIT"))]
    pub(crate) history_limit: usize,
    pub(crate) history_last_kind: Option<crate::editor::history::EditKind>,
    #[derivative(Default(value = "true"))]
    pub(crate) history_enabled: bool,
}

impl<D: DataProvider> FormEditor<D> {
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
            #[cfg(feature = "keymap")]
            keymap: None,
            #[cfg(feature = "keymap")]
            seq_tracker: KeySequenceTracker::new(400),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history_limit: crate::editor::history::DEFAULT_HISTORY_LIMIT,
            history_last_kind: None,
            history_enabled: true,
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

    /// Set the keymap for this editor instance
    #[cfg(feature = "keymap")]
    pub fn set_keymap(&mut self, keymap: CanvasKeyMap) {
        self.keymap = Some(keymap);
    }

    /// Check if this editor has a keymap configured
    #[cfg(feature = "keymap")]
    pub fn has_keymap(&self) -> bool {
        self.keymap.is_some()
    }

    /// Set the timeout for multi-key sequences (in milliseconds)
    #[cfg(feature = "keymap")]
    pub fn set_key_sequence_timeout_ms(&mut self, timeout_ms: u64) {
        self.seq_tracker = KeySequenceTracker::new(timeout_ms);
    }

    pub fn current_text(&self) -> &str {
        let field_index = self.ui_state.current_field;
        if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        }
    }

    pub(crate) fn set_cursor_raw(&mut self, pos: usize) {
        self.ui_state.set_cursor(pos, pos, true);
    }

    pub(crate) fn set_cursor_for_mode(&mut self, pos: usize, max_len: usize) {
        self.ui_state
            .set_cursor(pos, max_len, self.ui_state.current_mode == AppMode::Edit);
    }

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
