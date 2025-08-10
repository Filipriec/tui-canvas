// src/editor.rs
//! Main API for the canvas library - FormEditor with library-owned state

#[cfg(feature = "cursor-style")]
use crate::canvas::CursorManager;


use anyhow::Result;
use crate::canvas::state::EditorState;
use crate::data_provider::{DataProvider, SuggestionsProvider, SuggestionItem};
use crate::canvas::modes::AppMode;
use crate::canvas::state::SelectionState;

/// Main editor that manages UI state internally and delegates data to user
pub struct FormEditor<D: DataProvider> {
    // Library owns all UI state
    ui_state: EditorState,

    // User owns business data
    data_provider: D,

    // Autocomplete suggestions (library manages UI, user provides data)
    pub(crate) suggestions: Vec<SuggestionItem>,

    #[cfg(feature = "validation")]
    external_validation_callback: Option<Box<dyn FnMut(usize, &str) + Send + Sync>>,
}

impl<D: DataProvider> FormEditor<D> {
    /// Convert a char index to a byte index in a string
    fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
        s.char_indices()
            .nth(char_idx)
            .map(|(byte_idx, _)| byte_idx)
            .unwrap_or_else(|| s.len())
    }

    #[allow(dead_code)]
    /// Convert a byte index to a char index in a string
    fn byte_to_char_index(s: &str, byte_idx: usize) -> usize {
        s[..byte_idx].chars().count()
    }
    pub fn new(data_provider: D) -> Self {
        let mut editor = Self {
            ui_state: EditorState::new(),
            data_provider,
            suggestions: Vec::new(),
            #[cfg(feature = "validation")]
            external_validation_callback: None,
        };

        // Initialize validation configurations if validation feature is enabled
        #[cfg(feature = "validation")]
        {
            editor.initialize_validation();
        }

        editor
    }

    /// Get current field text (convenience method)
    fn current_text(&self) -> &str {
        // Convenience wrapper, kept for compatibility with existing code
        let field_index = self.ui_state.current_field;
        if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        }
    }

    /// Compute inline completion for current selection and current text.
    fn compute_current_completion(&self) -> Option<String> {
        let typed = self.current_text();
        let idx = self.ui_state.suggestions.selected_index?;
        let sugg = self.suggestions.get(idx)?;
        if let Some(rest) = sugg.value_to_store.strip_prefix(typed) {
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
        None
    }

    /// Update UI state's completion text from current selection
    pub fn update_inline_completion(&mut self) {
        self.ui_state.suggestions.completion_text = self.compute_current_completion();
    }

    /// Initialize validation configurations from data provider
    #[cfg(feature = "validation")]
    fn initialize_validation(&mut self) {
        let field_count = self.data_provider.field_count();
        for field_index in 0..field_count {
            if let Some(config) = self.data_provider.validation_config(field_index) {
                self.ui_state.validation.set_field_config(field_index, config);
            }
        }
    }

    // ===================================================================
    // READ-ONLY ACCESS: User can fetch UI state
    // ===================================================================

    /// Get current field index (for user's compatibility)
    pub fn current_field(&self) -> usize {
        self.ui_state.current_field()
    }

    /// Get current cursor position (for user's compatibility)
    pub fn cursor_position(&self) -> usize {
        self.ui_state.cursor_position()
    }

    /// Get current mode (for user's mode-dependent logic)
    pub fn mode(&self) -> AppMode {
        self.ui_state.mode()
    }

    /// Check if suggestions dropdown is active (for user's logic)
    pub fn is_suggestions_active(&self) -> bool {
        self.ui_state.is_suggestions_active()
    }

    /// Get current field text for display.
    ///
    /// Policies:
    /// - Feature 4 (custom formatter):
    ///   - While editing the focused field: ALWAYS show raw (no custom formatting).
    /// - Mask-only fields: mask applies even in Edit mode (preserve legacy behavior).
    /// - Otherwise: raw.
    #[cfg(feature = "validation")]
    pub fn current_display_text(&self) -> String {
        let field_index = self.ui_state.current_field;
        let raw = if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        };

        if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
            // 1) Mask-only fields: mask applies even in Edit (legacy behavior)
            if cfg.custom_formatter.is_none() {
                if let Some(mask) = &cfg.display_mask {
                    return mask.apply_to_display(raw);
                }
            }

            // 2) Feature 4 fields: raw while editing, formatted otherwise
            if cfg.custom_formatter.is_some() {
                if matches!(self.ui_state.current_mode, AppMode::Edit) {
                    return raw.to_string();
                }
                if let Some((formatted, _mapper, _warning)) = cfg.run_custom_formatter(raw) {
                    return formatted;
                }
            }

            // 3) Fallback to mask if present (when formatter didn't produce output)
            if let Some(mask) = &cfg.display_mask {
                return mask.apply_to_display(raw);
            }
        }

        raw.to_string()
    }

    /// Get reference to UI state for rendering
    pub fn ui_state(&self) -> &EditorState {
        &self.ui_state
    }

    /// Mutable access to UI state for internal crate use only.
    pub(crate) fn ui_state_mut(&mut self) -> &mut EditorState {
        &mut self.ui_state
    }

    /// Open the suggestions UI for `field_index` (UI-only; does not fetch).
    pub fn open_suggestions(&mut self, field_index: usize) {
        self.ui_state.open_suggestions(field_index);
    }

    /// Close suggestions UI and clear the current suggestion results.
    pub fn close_suggestions(&mut self) {
        self.ui_state.close_suggestions();
        self.suggestions.clear();
    }

    /// Set external validation state for a field (Feature 5)
    #[cfg(feature = "validation")]
    pub fn set_external_validation(
        &mut self,
        field_index: usize,
        state: crate::validation::ExternalValidationState,
    ) {
        self.ui_state
            .validation
            .set_external_validation(field_index, state);
    }

    /// Clear external validation state for a specific field
    #[cfg(feature = "validation")]
    pub fn clear_external_validation(&mut self, field_index: usize) {
        self.ui_state.validation.clear_external_validation(field_index);
    }

    /// Set external validation callback (Feature 5)
    #[cfg(feature = "validation")]
    pub fn set_external_validation_callback<F>(&mut self, callback: F)
    where
        F: FnMut(usize, &str) + Send + Sync + 'static,
    {
        self.external_validation_callback = Some(Box::new(callback));
    }

    /// Get effective display text for any field index.
    ///
    /// Policies:
    /// - Feature 4 fields (with custom formatter):
    ///   - If the field is currently focused AND in Edit mode: return raw (no formatting).
    /// - Mask-only fields: mask applies regardless of mode (legacy behavior).
    /// - Otherwise: raw.
    #[cfg(feature = "validation")]
    pub fn display_text_for_field(&self, field_index: usize) -> String {
        let raw = if field_index < self.data_provider.field_count() {
            self.data_provider.field_value(field_index)
        } else {
            ""
        };

        if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
            // Mask-only fields: mask applies even in Edit mode
            if cfg.custom_formatter.is_none() {
                if let Some(mask) = &cfg.display_mask {
                    return mask.apply_to_display(raw);
                }
            }

            // Feature 4 fields:
            if cfg.custom_formatter.is_some() {
                // Focused + Edit -> raw
                if field_index == self.ui_state.current_field
                    && matches!(self.ui_state.current_mode, AppMode::Edit)
                {
                    return raw.to_string();
                }
                // Not editing -> formatted
                if let Some((formatted, _mapper, _warning)) = cfg.run_custom_formatter(raw) {
                    return formatted;
                }
            }

            // Fallback to mask if present (in case formatter didn't return output)
            if let Some(mask) = &cfg.display_mask {
                return mask.apply_to_display(raw);
            }
        }

        raw.to_string()
    }

    /// Get reference to data provider for rendering
    pub fn data_provider(&self) -> &D {
        &self.data_provider
    }

    /// Get autocomplete suggestions for rendering (read-only)
    pub fn suggestions(&self) -> &[SuggestionItem] {
        &self.suggestions
    }

    /// Get validation state (for user's business logic)
    /// Only available when the 'validation' feature is enabled
    #[cfg(feature = "validation")]
    pub fn validation_state(&self) -> &crate::validation::ValidationState {
        self.ui_state.validation_state()
    }

    /// Get validation result for current field
    #[cfg(feature = "validation")]
    pub fn current_field_validation(&self) -> Option<&crate::validation::ValidationResult> {
        self.ui_state.validation.get_field_result(self.ui_state.current_field)
    }

    /// Get validation result for specific field
    #[cfg(feature = "validation")]
    pub fn field_validation(&self, field_index: usize) -> Option<&crate::validation::ValidationResult> {
        self.ui_state.validation.get_field_result(field_index)
    }

    // ===================================================================
    // SYNC OPERATIONS: No async needed for basic editing
    // ===================================================================

    /// Centralized field transition logic
    #[cfg_attr(not(feature = "validation"), allow(unused_variables))]
    pub fn transition_to_field(&mut self, new_field: usize) -> Result<()> {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return Ok(());
        }

        let prev_field = self.ui_state.current_field;

        // 1. Bounds check
        let mut target_field = new_field.min(field_count - 1);

        // 2. Computed field skipping
        #[cfg(feature = "computed")]
        {
            if let Some(computed_state) = &self.ui_state.computed {
                if computed_state.is_computed_field(target_field) {
                    // Determine direction and search for nearest non-computed field
                    if target_field >= prev_field {
                        // Moving down: search forward
                        for i in (target_field + 1)..field_count {
                            if !computed_state.is_computed_field(i) {
                                target_field = i;
                                break;
                            }
                        }
                    } else {
                        // Moving up: search backward
                        let mut i = target_field;
                        loop {
                            if !computed_state.is_computed_field(i) {
                                target_field = i;
                                break;
                            }
                            if i == 0 {
                                break;
                            }
                            i -= 1;
                        }
                    }
                }
            }
        }

        // No-op if the resolved target is the same as current
        if target_field == prev_field {
            return Ok(());
        }

        // 3. Blocking validation before leaving current field
        #[cfg(feature = "validation")]
        {
            let current_text = self.current_text();
            if !self.ui_state.validation.allows_field_switch(prev_field, current_text) {
                if let Some(reason) = self
                    .ui_state
                    .validation
                    .field_switch_block_reason(prev_field, current_text)
                {
                    tracing::debug!("Field switch blocked: {}", reason);
                    return Err(anyhow::anyhow!("Cannot switch fields: {}", reason));
                }
            }
        }

        // 4. Exit hook for current field (content validation + external validation trigger)
        #[cfg(feature = "validation")]
        {
            let text = self.data_provider.field_value(prev_field).to_string();
            let _ = self
                .ui_state
                .validation
                .validate_field_content(prev_field, &text);

            if let Some(cfg) = self.ui_state.validation.get_field_config(prev_field) {
                // If external validation is enabled for this field and there is content
                if cfg.external_validation_enabled && !text.is_empty() {
                    // Trigger external validation state
                    self.set_external_validation(prev_field, crate::validation::ExternalValidationState::Validating);

                    // Invoke external callback if registered
                    if let Some(cb) = self.external_validation_callback.as_mut() {
                        cb(prev_field, &text);
                    }
                }
            }
        }

        #[cfg(feature = "computed")]
        {
            // Placeholder for recompute hook if needed (requires provider)
            // Could call on_field_changed with a provider when available.
        }

        // 5. Move to new field
        self.ui_state.move_to_field(target_field, field_count);

        // 6. Clamp cursor to new field
        let current_text = self.current_text();
        let max_pos = current_text.chars().count();
        self.ui_state.set_cursor(
            self.ui_state.ideal_cursor_column,
            max_pos,
            self.ui_state.current_mode == AppMode::Edit,
        );

        Ok(())
    }

    /// Handle character insertion with proper mask/limit coordination
    pub fn insert_char(&mut self, ch: char) -> Result<()> {
        if self.ui_state.current_mode != AppMode::Edit {
            return Ok(()); // Ignore in non-edit modes
        }

        let field_index = self.ui_state.current_field;
        let raw_cursor_pos = self.ui_state.cursor_pos;
        let current_raw_text = self.data_provider.field_value(field_index);

        // Mask gate: reject input that doesn't fit the mask at current position
        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_cursor_pos = mask.raw_pos_to_display_pos(raw_cursor_pos);

                    // Reject if at/after end of mask pattern (in char positions)
                    let pattern_char_len = mask.pattern().chars().count();
                    if display_cursor_pos >= pattern_char_len {
                        return Ok(());
                    }

                    // Reject if on a separator, not an input position
                    if !mask.is_input_position(display_cursor_pos) {
                        return Ok(());
                    }

                    // Reject if mask is already full
                    let input_slots = (0..pattern_char_len)
                        .filter(|&pos| mask.is_input_position(pos))
                        .count();
                    if current_raw_text.chars().count() >= input_slots {
                        return Ok(());
                    }
                }
            }
        }

        // Validation: character insertion
        #[cfg(feature = "validation")]
        {
            let vr = self.ui_state.validation.validate_char_insertion(
                field_index,
                current_raw_text,
                raw_cursor_pos,
                ch,
            );
            if !vr.is_acceptable() {
                return Ok(()); // Silently reject invalid input
            }
        }

        // Build new raw text with inserted character at char index raw_cursor_pos
        let new_raw_text = {
            let mut temp = current_raw_text.to_string();
            let byte_pos = Self::char_to_byte_index(current_raw_text, raw_cursor_pos);
            temp.insert(byte_pos, ch);
            temp
        };

        // Post-validate full content and mask capacity
        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                // Check character limits
                if let Some(limits) = &cfg.character_limits {
                    if let Some(result) = limits.validate_content(&new_raw_text) {
                        if !result.is_acceptable() {
                            return Ok(()); // Silently reject
                        }
                    }
                }
                // Check mask capacity again against new length
                if let Some(mask) = &cfg.display_mask {
                    let pattern_char_len = mask.pattern().chars().count();
                    let input_slots = (0..pattern_char_len)
                        .filter(|&pos| mask.is_input_position(pos))
                        .count();
                    if new_raw_text.chars().count() > input_slots {
                        return Ok(());
                    }
                }
            }
        }

        // Commit
        self.data_provider
            .set_field_value(field_index, new_raw_text.clone());

        // Move cursor
        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let new_raw_pos = raw_cursor_pos + 1;
                    let display_pos = mask.raw_pos_to_display_pos(new_raw_pos);
                    let next_input_display = mask.next_input_position(display_pos);
                    let next_raw_pos = mask.display_pos_to_raw_pos(next_input_display);
                    let max_raw = new_raw_text.chars().count();

                    self.ui_state.cursor_pos = next_raw_pos.min(max_raw);
                    self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
                    return Ok(());
                }
            }
        }

        // No mask -> simple increment
        self.ui_state.cursor_pos = raw_cursor_pos + 1;
        self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
        Ok(())
    }

    /// Move to previous field (vim k / up arrow)
    pub fn move_up(&mut self) -> Result<()> {
        let new_field = self.ui_state.current_field.saturating_sub(1);
        self.transition_to_field(new_field)
    }

    /// Move to next field (vim j / down arrow)
    pub fn move_down(&mut self) -> Result<()> {
        let new_field = (self.ui_state.current_field + 1).min(self.data_provider.field_count().saturating_sub(1));
        self.transition_to_field(new_field)
    }

    /// Move to next field (vim j / down arrow)
    pub fn move_to_next_field(&mut self) -> Result<()> {
        let field_count = self.data_provider.field_count();
        if field_count == 0 {
            return Ok(());
        }

        let new_field = (self.ui_state.current_field + 1) % field_count;
        self.transition_to_field(new_field)
    }

    /// Restore left and right movement within the current field
    /// Move cursor left within current field
    pub fn move_left(&mut self) -> Result<()> {
        let mut moved = false;
        // Try mask-aware movement if validation/mask config exists
        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos = mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                    if let Some(prev_input) = mask.prev_input_position(display_pos) {
                        let raw_pos = mask.display_pos_to_raw_pos(prev_input);
                        let max_pos = self.current_text().chars().count();
                        self.ui_state.cursor_pos = raw_pos.min(max_pos);
                        self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
                        moved = true;
                    } else {
                        self.ui_state.cursor_pos = 0;
                        self.ui_state.ideal_cursor_column = 0;
                        moved = true;
                    }
                }
            }
        }
        if !moved {
            // Fallback to simple left movement
            if self.ui_state.cursor_pos > 0 {
                self.ui_state.cursor_pos -= 1;
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
            }
        }
        Ok(())
    }

    /// Move cursor right within current field
    pub fn move_right(&mut self) -> Result<()> {
        let mut moved = false;
        let field_index = self.ui_state.current_field;
        // Try mask-aware movement if mask is configured for this field
        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos = mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                    // Next input position now returns usize
                    let next_display_pos = mask.next_input_position(display_pos);
                    let next_pos = mask.display_pos_to_raw_pos(next_display_pos);
                    let max_pos = self.current_text().chars().count();
                    self.ui_state.cursor_pos = next_pos.min(max_pos);
                    self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
                    moved = true;
                }
            }
        }
        if !moved {
            // Fallback to simple right movement
            let max_pos = self.current_text().chars().count();
            if self.ui_state.cursor_pos < max_pos {
                self.ui_state.cursor_pos += 1;
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
            }
        }
        Ok(())
    }

    /// Change mode (for vim compatibility)
    pub fn set_mode(&mut self, mode: AppMode) {
        match (self.ui_state.current_mode, mode) {
            // Entering highlight mode from read-only
            (AppMode::ReadOnly, AppMode::Highlight) => {
                self.enter_highlight_mode();
            }
            // Exiting highlight mode
            (AppMode::Highlight, AppMode::ReadOnly) => {
                self.exit_highlight_mode();
            }
            // Other transitions
            (_, new_mode) => {
                self.ui_state.current_mode = new_mode;
                if new_mode != AppMode::Highlight {
                    self.ui_state.selection = SelectionState::None;
                }

                #[cfg(feature = "cursor-style")]
                {
                    let _ = CursorManager::update_for_mode(new_mode);
                }
            }
        }
    }

    /// Enter edit mode with cursor positioned for append (vim 'a' command)
    pub fn enter_append_mode(&mut self) {
        let current_text = self.current_text();

        // Calculate append position: always move right, even at line end
        let char_len = current_text.chars().count();
        let append_pos = if current_text.is_empty() {
            0
        } else {
            (self.ui_state.cursor_pos + 1).min(char_len)
        };

        // Set cursor position for append
        self.ui_state.cursor_pos = append_pos;
        self.ui_state.ideal_cursor_column = append_pos;

        // Enter edit mode (which will update cursor style)
        self.set_mode(AppMode::Edit);
    }

    // ===================================================================
    // VALIDATION METHODS (only available with validation feature)
    // ===================================================================

    /// Enable or disable validation
    #[cfg(feature = "validation")]
    pub fn set_validation_enabled(&mut self, enabled: bool) {
        self.ui_state.validation.set_enabled(enabled);
    }

    /// Check if validation is enabled
    #[cfg(feature = "validation")]
    pub fn is_validation_enabled(&self) -> bool {
        self.ui_state.validation.is_enabled()
    }

    /// Set validation configuration for a specific field
    #[cfg(feature = "validation")]
    pub fn set_field_validation(&mut self, field_index: usize, config: crate::validation::ValidationConfig) {
        self.ui_state.validation.set_field_config(field_index, config);
    }

    /// Remove validation configuration for a specific field
    #[cfg(feature = "validation")]
    pub fn remove_field_validation(&mut self, field_index: usize) {
        self.ui_state.validation.remove_field_config(field_index);
    }

    /// Manually validate current field content
    #[cfg(feature = "validation")]
    pub fn validate_current_field(&mut self) -> crate::validation::ValidationResult {
        let field_index = self.ui_state.current_field;
        let current_text = self.current_text().to_string();
        self.ui_state.validation.validate_field_content(field_index, &current_text)
    }

    /// Manually validate specific field content
    #[cfg(feature = "validation")]
    pub fn validate_field(&mut self, field_index: usize) -> Option<crate::validation::ValidationResult> {
        if field_index < self.data_provider.field_count() {
            let text = self.data_provider.field_value(field_index).to_string();
            Some(self.ui_state.validation.validate_field_content(field_index, &text))
        } else {
            None
        }
    }

    /// Clear validation results for all fields
    #[cfg(feature = "validation")]
    pub fn clear_validation_results(&mut self) {
        self.ui_state.validation.clear_all_results();
    }

    /// Get validation summary for all fields
    #[cfg(feature = "validation")]
    pub fn validation_summary(&self) -> crate::validation::ValidationSummary {
        self.ui_state.validation.summary()
    }

    /// Check if field switching is allowed from current field
    #[cfg(feature = "validation")]
    pub fn can_switch_fields(&self) -> bool {
        let current_text = self.current_text();
        self.ui_state.validation.allows_field_switch(self.ui_state.current_field, current_text)
    }

    /// Get reason why field switching is blocked (if any)
    #[cfg(feature = "validation")]
    pub fn field_switch_block_reason(&self) -> Option<String> {
        let current_text = self.current_text();
        self.ui_state.validation.field_switch_block_reason(self.ui_state.current_field, current_text)
    }

    // ===================================================================
    // ASYNC OPERATIONS: Only suggestions need async
    // ===================================================================

    /// Trigger suggestions (async because it fetches data)
    pub async fn trigger_suggestions<A>(&mut self, provider: &mut A) -> Result<()>
    where
        A: SuggestionsProvider,
    {
        let field_index = self.ui_state.current_field;

        if !self.data_provider.supports_suggestions(field_index) {
            return Ok(());
        }

        // Activate suggestions UI
        self.ui_state.activate_suggestions(field_index);

        // Fetch suggestions from user (no conversion needed!)
        let query = self.current_text();
        self.suggestions = provider.fetch_suggestions(field_index, query).await?;

        // Update UI state
        self.ui_state.suggestions.is_loading = false;
        if !self.suggestions.is_empty() {
            self.ui_state.suggestions.selected_index = Some(0);
            // Compute initial inline completion from first suggestion
            self.update_inline_completion();
        }

        Ok(())
    }

    /// Navigate suggestions
    pub fn suggestions_next(&mut self) {
        if !self.ui_state.suggestions.is_active || self.suggestions.is_empty() {
            return;
        }

        let current = self.ui_state.suggestions.selected_index.unwrap_or(0);
        let next = (current + 1) % self.suggestions.len();
        self.ui_state.suggestions.selected_index = Some(next);

        // Update inline completion to reflect new highlighted item
        self.update_inline_completion();
    }

    /// Apply selected suggestion
    /// Apply selected suggestion
    pub fn apply_suggestion(&mut self) -> Option<String> {
        if let Some(selected_index) = self.ui_state.suggestions.selected_index {
            if let Some(suggestion) = self.suggestions.get(selected_index).cloned() {
                let field_index = self.ui_state.current_field;

                // Apply to user's data
                self.data_provider.set_field_value(
                    field_index,
                    suggestion.value_to_store.clone()
                );

                // Update cursor position
                self.ui_state.cursor_pos = suggestion.value_to_store.len();
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;

                // Close suggestions
                self.ui_state.deactivate_suggestions();
                self.suggestions.clear();

                // Validate the new content if validation is enabled
                #[cfg(feature = "validation")]
                {
                    let _validation_result = self.ui_state.validation.validate_field_content(
                        field_index,
                        &suggestion.value_to_store,
                    );
                }

                return Some(suggestion.display_text);
            }
        }
        None
    }

    // ===================================================================
    // MOVEMENT METHODS (keeping existing implementations)
    // ===================================================================

    /// Move to previous field (vim k / up arrow)
    pub fn move_up_only(&mut self) -> Result<()> {
        self.move_up()
    }

    /// Move to next field (vim j / down arrow)
    pub fn move_down_only(&mut self) -> Result<()> {
        self.move_down()
    }

    /// Move to first line (vim gg)
    pub fn move_first_line(&mut self) -> Result<()> {
        self.transition_to_field(0)
    }

    /// Move to last line (vim G)
    pub fn move_last_line(&mut self) -> Result<()> {
        let last_field = self.data_provider.field_count().saturating_sub(1);
        self.transition_to_field(last_field)
    }

    /// Move to previous field (alternative to move_up)
    pub fn prev_field(&mut self) -> Result<()> {
        self.move_up()
    }

    // ================================================================================================
    // COMPUTED FIELDS (behind 'computed' feature)
    // ================================================================================================

    /// Initialize computed fields from provider and computed provider
    #[cfg(feature = "computed")]
    pub fn set_computed_provider<C>(&mut self, mut provider: C)
    where
        C: crate::computed::ComputedProvider,
    {
        // Initialize computed state
        self.ui_state.computed = Some(crate::computed::ComputedState::new());

        // Register computed fields and their dependencies
        let field_count = self.data_provider.field_count();
        for field_index in 0..field_count {
            if provider.handles_field(field_index) {
                let deps = provider.field_dependencies(field_index);
                if let Some(computed_state) = &mut self.ui_state.computed {
                    computed_state.register_computed_field(field_index, deps);
                }
            }
        }

        // Initial computation of all computed fields
        self.recompute_all_fields(&mut provider);
    }

    /// Recompute specific computed fields
    #[cfg(feature = "computed")]
    pub fn recompute_fields<C>(&mut self, provider: &mut C, field_indices: &[usize])
    where
        C: crate::computed::ComputedProvider,
    {
        if let Some(computed_state) = &mut self.ui_state.computed {
            // Collect all field values for context
            let field_values: Vec<String> = (0..self.data_provider.field_count())
                .map(|i| {
                    if computed_state.is_computed_field(i) {
                        // Use cached computed value
                        computed_state
                            .get_computed_value(i)
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        // Use regular field value
                        self.data_provider.field_value(i).to_string()
                    }
                })
                .collect();

            let field_refs: Vec<&str> = field_values.iter().map(|s| s.as_str()).collect();

            // Recompute specified fields
            for &field_index in field_indices {
                if provider.handles_field(field_index) {
                    let context = crate::computed::ComputedContext {
                        field_values: &field_refs,
                        target_field: field_index,
                        current_field: Some(self.ui_state.current_field),
                    };

                    let computed_value = provider.compute_field(context);
                    computed_state.set_computed_value(field_index, computed_value);
                }
            }
        }
    }

    /// Recompute all computed fields
    #[cfg(feature = "computed")]
    pub fn recompute_all_fields<C>(&mut self, provider: &mut C)
    where
        C: crate::computed::ComputedProvider,
    {
        if let Some(computed_state) = &self.ui_state.computed {
            let computed_fields: Vec<usize> = computed_state.computed_fields().collect();
            self.recompute_fields(provider, &computed_fields);
        }
    }

    /// Trigger recomputation when field changes (call this after set_field_value)
    #[cfg(feature = "computed")]
    pub fn on_field_changed<C>(&mut self, provider: &mut C, changed_field: usize)
    where
        C: crate::computed::ComputedProvider,
    {
        if let Some(computed_state) = &self.ui_state.computed {
            let fields_to_update = computed_state.fields_to_recompute(changed_field);
            if !fields_to_update.is_empty() {
                self.recompute_fields(provider, &fields_to_update);
            }
        }
    }

    /// Enhanced getter that returns computed values for computed fields when available
    #[cfg(feature = "computed")]
    pub fn effective_field_value(&self, field_index: usize) -> String {
        if let Some(computed_state) = &self.ui_state.computed {
            if let Some(computed_value) = computed_state.get_computed_value(field_index) {
                return computed_value.clone();
            }
        }
        self.data_provider.field_value(field_index).to_string()
    }

    /// Move to next field (alternative to move_down)
    pub fn next_field(&mut self) -> Result<()> {
        self.move_down()
    }

    /// Move to start of current field (vim 0)
    pub fn move_line_start(&mut self) {
        use crate::canvas::actions::movement::line::line_start_position;
        let new_pos = line_start_position();
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Move to end of current field (vim $)
    pub fn move_line_end(&mut self) {
        use crate::canvas::actions::movement::line::line_end_position;
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;

        let new_pos = line_end_position(current_text, is_edit_mode);
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Move to start of next word (vim w)
    pub fn move_word_next(&mut self) {
        use crate::canvas::actions::movement::word::find_next_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            return;
        }

        let new_pos = find_next_word_start(current_text, self.ui_state.cursor_pos);
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;

        // Clamp to valid bounds for current mode
        let char_len = current_text.chars().count();
        let final_pos = if is_edit_mode {
            new_pos.min(char_len)
        } else {
            new_pos.min(char_len.saturating_sub(1))
        };

        self.ui_state.cursor_pos = final_pos;
        self.ui_state.ideal_cursor_column = final_pos;
    }

    /// Move to start of previous word (vim b)
    pub fn move_word_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_word_start;
        let current_text = self.current_text();

        if current_text.is_empty() {
            return;
        }

        let new_pos = find_prev_word_start(current_text, self.ui_state.cursor_pos);
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Move to end of current/next word (vim e)
    pub fn move_word_end(&mut self) {
        use crate::canvas::actions::movement::word::find_word_end;
        let current_text = self.current_text();

        if current_text.is_empty() {
            return;
        }

        let current_pos = self.ui_state.cursor_pos;
        let char_len = current_text.chars().count();
        let new_pos = find_word_end(current_text, current_pos);

        // If we didn't move, try next word
        let final_pos = if new_pos == current_pos && current_pos + 1 < char_len {
            find_word_end(current_text, current_pos + 1)
        } else {
            new_pos
        };

        // Clamp for read-only mode
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;
        let clamped_pos = if is_edit_mode {
            final_pos.min(char_len)
        } else {
            final_pos.min(char_len.saturating_sub(1))
        };

        self.ui_state.cursor_pos = clamped_pos;
        self.ui_state.ideal_cursor_column = clamped_pos;
    }

    /// Move to end of previous word (vim ge)
    pub fn move_word_end_prev(&mut self) {
        use crate::canvas::actions::movement::word::find_prev_word_end;
        let current_text = self.current_text();

        if current_text.is_empty() {
            return;
        }

        let new_pos = find_prev_word_end(current_text, self.ui_state.cursor_pos);
        self.ui_state.cursor_pos = new_pos;
        self.ui_state.ideal_cursor_column = new_pos;
    }

    /// Delete character before cursor (vim x in insert mode / backspace)
    pub fn delete_backward(&mut self) -> Result<()> {
        if self.ui_state.current_mode != AppMode::Edit {
            return Ok(());
        }
        if self.ui_state.cursor_pos == 0 {
            return Ok(());
        }

        let field_index = self.ui_state.current_field;
        let mut current_text = self.data_provider.field_value(field_index).to_string();

        let new_cursor = self.ui_state.cursor_pos.saturating_sub(1);

        let start = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos - 1);
        let end = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos);
        current_text.replace_range(start..end, "");
        self.data_provider.set_field_value(field_index, current_text.clone());

        // Always run reposition logic
        let mut target_cursor = new_cursor;
        #[cfg(feature = "validation")]
        {
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                if let Some(mask) = &cfg.display_mask {
                    let display_pos = mask.raw_pos_to_display_pos(new_cursor);
                    if let Some(prev_input) = mask.prev_input_position(display_pos) {
                        target_cursor = mask.display_pos_to_raw_pos(prev_input);
                    }
                }
            }
        }

        self.ui_state.cursor_pos = target_cursor;
        self.ui_state.ideal_cursor_column = target_cursor;

        #[cfg(feature = "validation")]
        {
            let _ = self.ui_state.validation.validate_field_content(
                field_index,
                &current_text,
            );
        }

        Ok(())
    }

    /// Delete character under cursor (vim x / delete key)
    pub fn delete_forward(&mut self) -> Result<()> {
        if self.ui_state.current_mode != AppMode::Edit {
            return Ok(());
        }

        let field_index = self.ui_state.current_field;
        let mut current_text = self.data_provider.field_value(field_index).to_string();

        if self.ui_state.cursor_pos < current_text.chars().count() {
            let start = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos);
            let end = Self::char_to_byte_index(&current_text, self.ui_state.cursor_pos + 1);
            current_text.replace_range(start..end, "");
            self.data_provider.set_field_value(field_index, current_text.clone());

            let mut target_cursor = self.ui_state.cursor_pos;
            #[cfg(feature = "validation")]
            {
                if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                    if let Some(mask) = &cfg.display_mask {
                        let display_pos = mask.raw_pos_to_display_pos(self.ui_state.cursor_pos);
                        let next_input = mask.next_input_position(display_pos);
                        target_cursor = mask.display_pos_to_raw_pos(next_input)
                            .min(current_text.chars().count());
                    }
                }
            }

            self.ui_state.cursor_pos = target_cursor;
            self.ui_state.ideal_cursor_column = target_cursor;

            #[cfg(feature = "validation")]
            {
                let _ = self.ui_state.validation.validate_field_content(
                    field_index,
                    &current_text,
                );
            }
        }

        Ok(())
    }

    /// Exit edit mode to read-only mode (vim Escape)
    pub fn exit_edit_mode(&mut self) -> Result<()> {
        // Validate current field content when exiting edit mode
        #[cfg(feature = "validation")]
        {
            let current_text = self.current_text();
            if !self.ui_state.validation.allows_field_switch(self.ui_state.current_field, current_text) {
                if let Some(reason) = self.ui_state.validation.field_switch_block_reason(self.ui_state.current_field, current_text) {
                    return Err(anyhow::anyhow!("Cannot exit edit mode: {}", reason));
                }
            }
        }

        // Adjust cursor position when transitioning from edit to normal mode
        let current_text = self.current_text();
        if !current_text.is_empty() {
            // In normal mode, cursor must be ON a character, not after the last one
            let max_normal_pos = current_text.chars().count().saturating_sub(1);
            if self.ui_state.cursor_pos > max_normal_pos {
                self.ui_state.cursor_pos = max_normal_pos;
                self.ui_state.ideal_cursor_column = self.ui_state.cursor_pos;
            }
        }

        self.set_mode(AppMode::ReadOnly);
        // Deactivate suggestions when exiting edit mode
        self.ui_state.deactivate_suggestions();

        Ok(())
    }

    /// Enter edit mode from read-only mode (vim i/a/o)
    pub fn enter_edit_mode(&mut self) {
        #[cfg(feature = "computed")]
        {
            if let Some(computed_state) = &self.ui_state.computed {
                if computed_state.is_computed_field(self.ui_state.current_field) {
                    // Can't edit computed fields - silently ignore
                    return;
                }
            }
        }
        self.set_mode(AppMode::Edit);
    }

    // ===================================================================
    // HELPER METHODS
    // ===================================================================

    /// Clamp cursor position to valid bounds for current field and mode
    fn clamp_cursor_to_current_field(&mut self) {
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;

        use crate::canvas::actions::movement::line::safe_cursor_position;
        let safe_pos = safe_cursor_position(
            current_text,
            self.ui_state.ideal_cursor_column,
            is_edit_mode
        );

        self.ui_state.cursor_pos = safe_pos;
    }


    /// Set the value of the current field
    pub fn set_current_field_value(&mut self, value: String) {
        let field_index = self.ui_state.current_field;
        self.data_provider.set_field_value(field_index, value.clone());
        // Reset cursor to start of field
        self.ui_state.cursor_pos = 0;
        self.ui_state.ideal_cursor_column = 0;

        // Validate the new content if validation is enabled
        #[cfg(feature = "validation")]
        {
            let _validation_result = self.ui_state.validation.validate_field_content(
                field_index,
                &value,
            );
        }
    }

    /// Set the value of a specific field by index
    pub fn set_field_value(&mut self, field_index: usize, value: String) {
        if field_index < self.data_provider.field_count() {
            self.data_provider.set_field_value(field_index, value.clone());
            // If we're modifying the current field, reset cursor
            if field_index == self.ui_state.current_field {
                self.ui_state.cursor_pos = 0;
                self.ui_state.ideal_cursor_column = 0;
            }

            // Validate the new content if validation is enabled
            #[cfg(feature = "validation")]
            {
                let _validation_result = self.ui_state.validation.validate_field_content(
                    field_index,
                    &value,
                );
            }
        }
    }

    /// Clear the current field (set to empty string)
    pub fn clear_current_field(&mut self) {
        self.set_current_field_value(String::new());
    }

    /// Get mutable access to data provider (for advanced operations)
    pub fn data_provider_mut(&mut self) -> &mut D {
        &mut self.data_provider
    }

    /// Set cursor to exact position (for vim-style movements like f, F, t, T)
    pub fn set_cursor_position(&mut self, position: usize) {
        let current_text = self.current_text();
        let is_edit_mode = self.ui_state.current_mode == AppMode::Edit;

        // Clamp to valid bounds for current mode
        let char_len = current_text.chars().count();
        let max_pos = if is_edit_mode {
            char_len
        } else {
            char_len.saturating_sub(1)
        };

        let clamped_pos = position.min(max_pos);

        // Update cursor position directly
        self.ui_state.cursor_pos = clamped_pos;
        self.ui_state.ideal_cursor_column = clamped_pos;
    }

    /// Get cursor position for display (maps raw cursor to display position with formatter/mask)
    pub fn display_cursor_position(&self) -> usize {
        let current_text = self.current_text();

        // Count characters, not bytes
        let char_count = current_text.chars().count();

        // Clamp raw_pos based on mode
        let raw_pos = match self.ui_state.current_mode {
            AppMode::Edit => self.ui_state.cursor_pos.min(char_count),
            _ => {
                if char_count == 0 {
                    0
                } else {
                    self.ui_state.cursor_pos.min(char_count.saturating_sub(1))
                }
            }
        };

        #[cfg(feature = "validation")]
        {
            let field_index = self.ui_state.current_field;
            if let Some(cfg) = self.ui_state.validation.get_field_config(field_index) {
                // Apply custom formatter mapping if not editing
                if !matches!(self.ui_state.current_mode, AppMode::Edit) {
                    if let Some((formatted, mapper, _)) = cfg.run_custom_formatter(current_text) {
                        return mapper.raw_to_formatted(current_text, &formatted, raw_pos);
                    }
                }
                // Apply mask mapping using clamped raw_pos
                if let Some(mask) = &cfg.display_mask {
                    return mask.raw_pos_to_display_pos(raw_pos);
                }
            }
        }

        raw_pos
    }

    /// Cleanup cursor style (call this when shutting down)
    pub fn cleanup_cursor(&self) -> std::io::Result<()> {
        #[cfg(feature = "cursor-style")]
        {
            crate::canvas::CursorManager::reset()
        }
        #[cfg(not(feature = "cursor-style"))]
        {
            Ok(())
        }
    }


    // ===================================================================
    // HIGHLIGHT MODE
    // ===================================================================

    /// Enter highlight mode (visual mode)
    pub fn enter_highlight_mode(&mut self) {
        if self.ui_state.current_mode == AppMode::ReadOnly {
            self.ui_state.current_mode = AppMode::Highlight;
            self.ui_state.selection = SelectionState::Characterwise {
                anchor: (self.ui_state.current_field, self.ui_state.cursor_pos),
            };

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Highlight);
            }
        }
    }

    /// Enter highlight line mode (visual line mode)
    pub fn enter_highlight_line_mode(&mut self) {
        if self.ui_state.current_mode == AppMode::ReadOnly {
            self.ui_state.current_mode = AppMode::Highlight;
            self.ui_state.selection = SelectionState::Linewise {
                anchor_field: self.ui_state.current_field,
            };

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::Highlight);
            }
        }
    }

    /// Exit highlight mode back to read-only
    pub fn exit_highlight_mode(&mut self) {
        if self.ui_state.current_mode == AppMode::Highlight {
            self.ui_state.current_mode = AppMode::ReadOnly;
            self.ui_state.selection = SelectionState::None;

            #[cfg(feature = "cursor-style")]
            {
                let _ = CursorManager::update_for_mode(AppMode::ReadOnly);
            }
        }
    }

    /// Check if currently in highlight mode
    pub fn is_highlight_mode(&self) -> bool {
        self.ui_state.current_mode == AppMode::Highlight
    }

    /// Get current selection state
    pub fn selection_state(&self) -> &SelectionState {
        &self.ui_state.selection
    }

    /// Enhanced movement methods that update selection in highlight mode
    pub fn move_left_with_selection(&mut self) {
        let _ = self.move_left();
        // Selection anchor stays in place, cursor position updates automatically
    }

    pub fn move_right_with_selection(&mut self) {
        let _ = self.move_right();
        // Selection anchor stays in place, cursor position updates automatically
    }

    pub fn move_up_with_selection(&mut self) {
        let _ = self.move_up();
        // Selection anchor stays in place, cursor position updates automatically
    }

    pub fn move_down_with_selection(&mut self) {
        let _ = self.move_down();
        // Selection anchor stays in place, cursor position updates automatically
    }

    // Add similar methods for word movement, line movement, etc.
    pub fn move_word_next_with_selection(&mut self) {
        self.move_word_next();
    }

    pub fn move_word_prev_with_selection(&mut self) {
        self.move_word_prev();
    }

    pub fn move_line_start_with_selection(&mut self) {
        self.move_line_start();
    }

    pub fn move_line_end_with_selection(&mut self) {
        self.move_line_end();
    }
}

// Add Drop implementation for automatic cleanup
impl<D: DataProvider> Drop for FormEditor<D> {
    fn drop(&mut self) {
        // Reset cursor to default when FormEditor is dropped
        let _ = self.cleanup_cursor();
    }
}
